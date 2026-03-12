//! Monte Carlo Tree Search (MCTS) with policy prior and value leaf evaluation.
//!
//! Implements PUCT-based MCTS suitable for AlphaGo/AlphaZero-style
//! policy-guided search over the WH40K action space.
//!
//! # Architecture
//!
//! - `MctsNode`: A single node in the search tree (visits, value, prior, children)
//! - `MctsTree`: The complete search tree with arena-based node storage
//! - `MctsConfig`: Configuration parameters (simulations, PUCT constant, temperature)
//! - `MctsSearch`: Implements `AiWorker` trait for integration with search stack
//!
//! # Policy Sources
//!
//! - Uniform prior (default, no trained policy)
//! - Heuristic prior (from evaluator-based action ranking)
//! - Neural network prior (from policy/value network, future)
//!
//! Source: implementation_v3.md Phase 10, Section 14

use serde::{Deserialize, Serialize};

use rand::Rng;

use wh40k_core_types::PlayerId;
use wh40k_game_core::{GameState, CommandValidator, CommandExecutor};
use wh40k_eval_heuristic::{HeuristicEvaluator, Evaluator};
use wh40k_search_abstraction::{ActionGenerator, MacroAction, CandidateSet};

use crate::{SearchResult, SearchStats, Score, AiWorker};

// ============================================================================
// MCTS Configuration
// ============================================================================

/// Configuration for MCTS search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MctsConfig {
    /// Number of simulations (tree walks) per search.
    pub num_simulations: u32,
    /// PUCT exploration constant (c_puct in AlphaGo: typically 1.0-2.5).
    pub c_puct: f32,
    /// Temperature for action selection from root visit counts.
    /// 1.0 = proportional to visits, 0.0 = argmax, >1.0 = more exploration.
    pub temperature: f32,
    /// Dirichlet noise alpha for root exploration (0.0 = no noise).
    pub dirichlet_alpha: f32,
    /// Dirichlet noise weight (epsilon, typically 0.25).
    pub dirichlet_epsilon: f32,
    /// Maximum tree depth before forced leaf evaluation.
    pub max_depth: u32,
    /// Whether to use heuristic priors when no policy network is available.
    pub use_heuristic_prior: bool,
    /// Number of random rollout steps for value estimation when no value network.
    pub rollout_depth: u32,
    /// FPU (First Play Urgency) reduction for unexplored children.
    pub fpu_reduction: f32,
}

impl MctsConfig {
    /// Default configuration for exploration-heavy MCTS.
    pub fn default_exploration() -> Self {
        Self {
            num_simulations: 800,
            c_puct: 2.0,
            temperature: 1.0,
            dirichlet_alpha: 0.3,
            dirichlet_epsilon: 0.25,
            max_depth: 50,
            use_heuristic_prior: true,
            rollout_depth: 0,
            fpu_reduction: 0.0,
        }
    }

    /// Fast MCTS for testing (fewer simulations).
    pub fn fast() -> Self {
        Self {
            num_simulations: 100,
            c_puct: 2.0,
            temperature: 1.0,
            dirichlet_alpha: 0.0,
            dirichlet_epsilon: 0.0,
            max_depth: 30,
            use_heuristic_prior: true,
            rollout_depth: 0,
            fpu_reduction: 0.0,
        }
    }

    /// Competition-strength MCTS (more simulations, lower temperature).
    pub fn competition() -> Self {
        Self {
            num_simulations: 1600,
            c_puct: 1.5,
            temperature: 0.1,
            dirichlet_alpha: 0.3,
            dirichlet_epsilon: 0.25,
            max_depth: 80,
            use_heuristic_prior: true,
            rollout_depth: 0,
            fpu_reduction: 0.0,
        }
    }
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self::default_exploration()
    }
}

// ============================================================================
// MCTS Node
// ============================================================================

/// Index into the MctsTree node arena.
type NodeIndex = usize;

/// A single node in the MCTS tree.
#[derive(Debug, Clone)]
struct MctsNode {
    /// The action that led to this node (None for root).
    action: Option<MacroAction>,
    /// Index of parent node (None for root).
    parent: Option<NodeIndex>,
    /// Indices of child nodes.
    children: Vec<NodeIndex>,
    /// Number of times this node has been visited.
    visits: u32,
    /// Sum of values backpropagated through this node.
    value_sum: f64,
    /// Prior probability from policy network or heuristic.
    prior: f32,
    /// Whether this node has been expanded (children generated).
    is_expanded: bool,
    /// Whether this is a terminal game state.
    is_terminal: bool,
    /// Terminal value (if is_terminal).
    terminal_value: f64,
    /// The perspective player at this node.
    perspective: PlayerId,
    /// Depth in the tree (root = 0).
    depth: u32,
}

impl MctsNode {
    /// Create a new root node.
    fn root(perspective: PlayerId) -> Self {
        Self {
            action: None,
            parent: None,
            children: Vec::new(),
            visits: 0,
            value_sum: 0.0,
            prior: 1.0,
            is_expanded: false,
            is_terminal: false,
            terminal_value: 0.0,
            perspective,
            depth: 0,
        }
    }

    /// Create a child node.
    fn child(action: MacroAction, parent: NodeIndex, prior: f32, perspective: PlayerId, depth: u32) -> Self {
        Self {
            action: Some(action),
            parent: Some(parent),
            children: Vec::new(),
            visits: 0,
            value_sum: 0.0,
            prior,
            is_expanded: false,
            is_terminal: false,
            terminal_value: 0.0,
            perspective,
            depth,
        }
    }

    /// Get the mean value (Q) of this node.
    fn q_value(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.value_sum / self.visits as f64
        }
    }
}

// ============================================================================
// MCTS Tree
// ============================================================================

/// Arena-based MCTS tree for efficient node allocation.
struct MctsTree {
    /// All nodes stored in a flat vector (arena).
    nodes: Vec<MctsNode>,
    /// Index of the root node.
    root: NodeIndex,
}

impl MctsTree {
    /// Create a new tree with a root node.
    fn new(perspective: PlayerId) -> Self {
        let root = MctsNode::root(perspective);
        Self {
            nodes: vec![root],
            root: 0,
        }
    }

    /// Add a new node and return its index.
    fn add_node(&mut self, node: MctsNode) -> NodeIndex {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    /// Get a reference to a node.
    fn node(&self, index: NodeIndex) -> &MctsNode {
        &self.nodes[index]
    }

    /// Get a mutable reference to a node.
    fn node_mut(&mut self, index: NodeIndex) -> &mut MctsNode {
        &mut self.nodes[index]
    }

    /// Total nodes in the tree.
    fn size(&self) -> usize {
        self.nodes.len()
    }
}

// ============================================================================
// MCTS Search
// ============================================================================

/// MCTS search engine implementing the AiWorker trait.
pub struct MctsSearch {
    config: MctsConfig,
}

impl MctsSearch {
    /// Create a new MCTS search with the given configuration.
    pub fn new(config: MctsConfig) -> Self {
        Self { config }
    }

    /// Run MCTS search on the given state and return the best action.
    pub fn search(&mut self, state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
        let (result, _) = self.search_internal(state, perspective)?;
        Some(result)
    }

    /// Run MCTS search and also return the policy target distribution from root
    /// visit counts. Used for generating training data for the policy/value network.
    ///
    /// Returns `(SearchResult, policy_target)` where `policy_target` is a vector of
    /// `(child_index, visit_proportion)` pairs from the root's children.
    pub fn search_for_training(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<(SearchResult, Vec<(usize, f32)>)> {
        self.search_internal(state, perspective)
    }

    /// Internal search that returns both the result and the policy target.
    fn search_internal(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<(SearchResult, Vec<(usize, f32)>)> {
        if !state.is_in_progress() {
            return None;
        }

        let evaluator = HeuristicEvaluator::with_defaults();
        let mut tree = MctsTree::new(perspective);
        let root_idx = tree.root;

        // Expand root
        self.expand_node(&mut tree, root_idx, state, &evaluator);

        if tree.node(tree.root).children.is_empty() {
            return None;
        }

        // Add Dirichlet noise to root priors for exploration
        if self.config.dirichlet_alpha > 0.0 && self.config.dirichlet_epsilon > 0.0 {
            self.add_dirichlet_noise(&mut tree);
        }

        // Run simulations
        for _ in 0..self.config.num_simulations {
            let mut sim_state = state.clone();
            let mut current = tree.root;

            // SELECT: Walk down the tree using PUCT
            while tree.node(current).is_expanded && !tree.node(current).children.is_empty() {
                current = self.select_child(&tree, current);

                // Apply the action to advance the simulation state
                if let Some(ref action) = tree.node(current).action {
                    for command in &action.commands {
                        let validation = CommandValidator::validate(&sim_state, command);
                        if validation.is_legal() {
                            let _ = CommandExecutor::execute(&mut sim_state, command);
                        }
                    }
                }
            }

            // EXPAND AND EVALUATE
            let value = if tree.node(current).is_terminal {
                tree.node(current).terminal_value
            } else if !sim_state.is_in_progress() {
                // Terminal state reached during simulation
                let v = self.terminal_value(&sim_state, perspective);
                tree.node_mut(current).is_terminal = true;
                tree.node_mut(current).terminal_value = v;
                v
            } else if tree.node(current).depth >= self.config.max_depth {
                // Max depth reached, evaluate with heuristic
                let score = evaluator.evaluate(&sim_state, perspective);
                normalize_score(score)
            } else {
                // EXPAND: generate children
                self.expand_node(&mut tree, current, &sim_state, &evaluator);

                // EVALUATE: use heuristic evaluation at leaf
                let score = evaluator.evaluate(&sim_state, perspective);
                normalize_score(score)
            };

            // BACKUP: walk parent pointers from leaf back to root
            let mut backup_node = current;
            loop {
                let node = tree.node_mut(backup_node);
                node.visits += 1;
                // Value is from the perspective of the root player
                if node.perspective == perspective {
                    node.value_sum += value;
                } else {
                    node.value_sum += 1.0 - value;
                }
                match node.parent {
                    Some(parent_idx) => backup_node = parent_idx,
                    None => break, // Reached root
                }
            }
        }

        // Extract policy target from root visit counts
        let policy_target = self.extract_policy_target(&tree);

        // SELECT ACTION from root based on visit counts
        let (best_child_idx, _visit_distribution) = self.select_root_action(&tree);
        let best_node = tree.node(best_child_idx);
        let best_action = best_node.action.clone()?;

        // Build candidate scores from root children
        let candidate_scores: Vec<(MacroAction, Score)> = tree.node(tree.root).children.iter()
            .filter_map(|&child_idx| {
                let child = tree.node(child_idx);
                child.action.as_ref().map(|a| {
                    (a.clone(), (child.q_value() * 1000.0) as Score)
                })
            })
            .collect();

        let tree_node_count = tree.size() as u64;

        let stats = SearchStats {
            nodes_evaluated: tree_node_count,
            leaf_evaluations: self.config.num_simulations as u64,
            beta_cutoffs: 0,
            tt_cutoffs: 0,
            tt_ordering_hits: 0,
            max_depth_reached: tree.nodes.iter().map(|n| n.depth as u8).max().unwrap_or(0),
            root_candidates: tree.node(tree.root).children.len(),
            depth_requested: self.config.max_depth as u8,
            chance_samples: 0,
            iterations_completed: self.config.num_simulations as u8,
            aspiration_resets: 0,
            quiescence_nodes: 0,
            extensions: 0,
            time_elapsed_ms: 0,
            nps: 0,
            pv_changes: 0,
            seldepth: 0,
        };

        let best_score = (best_node.q_value() * 1000.0) as Score;

        // Build PV (principal variation) from best path
        let mut pv = Vec::new();
        let mut pv_node = best_child_idx;
        while let Some(ref action) = tree.node(pv_node).action {
            pv.push(action.clone());
            // Follow the most-visited child
            if let Some(&best_child) = tree.node(pv_node).children.iter()
                .max_by_key(|&&c| tree.node(c).visits)
            {
                if tree.node(best_child).visits > 0 {
                    pv_node = best_child;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Some((SearchResult {
            best_action,
            score: best_score,
            stats,
            pv,
            candidate_scores,
        }, policy_target))
    }

    /// Expand a node by generating all legal actions as children.
    fn expand_node(
        &self,
        tree: &mut MctsTree,
        node_idx: NodeIndex,
        state: &GameState,
        evaluator: &HeuristicEvaluator,
    ) {
        if tree.node(node_idx).is_expanded {
            return;
        }

        let perspective = state.decision_owner;
        let candidates = ActionGenerator::generate(state, perspective);

        if candidates.candidates.is_empty() {
            tree.node_mut(node_idx).is_expanded = true;
            return;
        }

        // Compute priors
        let priors = if self.config.use_heuristic_prior {
            compute_heuristic_priors(&candidates, state, perspective, evaluator)
        } else {
            // Uniform prior
            let uniform = 1.0 / candidates.candidates.len() as f32;
            vec![uniform; candidates.candidates.len()]
        };

        let depth = tree.node(node_idx).depth + 1;

        // Create child nodes
        let child_indices: Vec<NodeIndex> = candidates.candidates.iter()
            .zip(priors.iter())
            .map(|(action, &prior)| {
                let child = MctsNode::child(action.clone(), node_idx, prior, perspective, depth);
                tree.add_node(child)
            })
            .collect();

        tree.node_mut(node_idx).children = child_indices;
        tree.node_mut(node_idx).is_expanded = true;
    }

    /// Select the best child using PUCT formula.
    fn select_child(&self, tree: &MctsTree, parent_idx: NodeIndex) -> NodeIndex {
        let parent = tree.node(parent_idx);
        let parent_visits_sqrt = (parent.visits as f64).sqrt();

        let mut best_idx = parent.children[0];
        let mut best_ucb = f64::NEG_INFINITY;

        for &child_idx in &parent.children {
            let child = tree.node(child_idx);

            let q = if child.visits == 0 {
                // FPU: use parent value minus reduction
                parent.q_value() - self.config.fpu_reduction as f64
            } else {
                child.q_value()
            };

            // PUCT = Q(s,a) + c_puct * P(s,a) * sqrt(N(s)) / (1 + N(s,a))
            let u = self.config.c_puct as f64
                * child.prior as f64
                * parent_visits_sqrt
                / (1.0 + child.visits as f64);

            let ucb = q + u;

            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = child_idx;
            }
        }

        best_idx
    }

    /// Add Dirichlet noise to root node priors for exploration.
    fn add_dirichlet_noise(&self, tree: &mut MctsTree) {
        let root_children = tree.node(tree.root).children.clone();
        let n = root_children.len();
        if n == 0 {
            return;
        }

        // Generate Dirichlet noise using Gamma distribution approximation
        let noise = generate_dirichlet_noise(n, self.config.dirichlet_alpha);
        let eps = self.config.dirichlet_epsilon;

        for (i, &child_idx) in root_children.iter().enumerate() {
            let child = tree.node_mut(child_idx);
            child.prior = (1.0 - eps) * child.prior + eps * noise[i];
        }
    }

    /// Select the action from root based on visit counts and temperature.
    fn select_root_action(&self, tree: &MctsTree) -> (NodeIndex, Vec<f32>) {
        let root = tree.node(tree.root);
        let children = &root.children;

        if children.is_empty() {
            return (tree.root, Vec::new());
        }

        // Compute visit distribution
        let visits: Vec<u32> = children.iter()
            .map(|&c| tree.node(c).visits)
            .collect();

        let total_visits: u32 = visits.iter().sum();

        if self.config.temperature <= 0.01 {
            // Argmax selection (temperature ≈ 0)
            let best_idx = children.iter()
                .enumerate()
                .max_by_key(|(_, &c)| tree.node(c).visits)
                .map(|(_, &c)| c)
                .unwrap_or(children[0]);

            let distribution: Vec<f32> = visits.iter()
                .map(|&v| v as f32 / total_visits.max(1) as f32)
                .collect();

            (best_idx, distribution)
        } else {
            // Temperature-based selection
            let inv_temp = 1.0 / self.config.temperature;

            let powered: Vec<f64> = visits.iter()
                .map(|&v| (v as f64).powf(inv_temp as f64))
                .collect();

            let sum_powered: f64 = powered.iter().sum();

            let probabilities: Vec<f32> = powered.iter()
                .map(|&p| (p / sum_powered) as f32)
                .collect();

            // Select proportionally (using max for determinism in engine)
            let best_idx_local = probabilities.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            (children[best_idx_local], probabilities)
        }
    }

    /// Compute terminal value from game outcome.
    fn terminal_value(&self, state: &GameState, perspective: PlayerId) -> f64 {
        match state.game_outcome {
            wh40k_core_types::GameOutcome::Victory(winner) => {
                if winner == perspective { 1.0 } else { 0.0 }
            }
            wh40k_core_types::GameOutcome::Draw => 0.5,
            wh40k_core_types::GameOutcome::InProgress => 0.5,
        }
    }

    /// Extract the policy distribution from root visit counts.
    /// Returns (child_index, visit_proportion) pairs suitable for training targets.
    fn extract_policy_target(&self, tree: &MctsTree) -> Vec<(usize, f32)> {
        let root = tree.node(tree.root);
        let total_visits: u32 = root.children.iter()
            .map(|&c| tree.node(c).visits)
            .sum();

        if total_visits == 0 {
            return Vec::new();
        }

        root.children.iter().enumerate()
            .map(|(i, &child_idx)| {
                let child = tree.node(child_idx);
                (i, child.visits as f32 / total_visits as f32)
            })
            .collect()
    }
}

impl AiWorker for MctsSearch {
    fn choose_action(&mut self, state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
        self.search(state, perspective)
    }

    fn name(&self) -> &str {
        "MCTS"
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Compute heuristic priors for candidate actions using the evaluator.
fn compute_heuristic_priors(
    candidates: &CandidateSet,
    state: &GameState,
    perspective: PlayerId,
    evaluator: &HeuristicEvaluator,
) -> Vec<f32> {
    let n = candidates.candidates.len();
    if n == 0 {
        return Vec::new();
    }

    // Score each action by applying it and evaluating
    let mut scores: Vec<f64> = Vec::with_capacity(n);

    for action in &candidates.candidates {
        let mut sim_state = state.clone();
        let mut valid = true;

        for command in &action.commands {
            let validation = CommandValidator::validate(&sim_state, command);
            if !validation.is_legal() {
                valid = false;
                break;
            }
            if CommandExecutor::execute(&mut sim_state, command).is_err() {
                valid = false;
                break;
            }
        }

        if valid {
            let score = evaluator.evaluate(&sim_state, perspective);
            scores.push(score as f64);
        } else {
            scores.push(f64::NEG_INFINITY);
        }
    }

    // Convert scores to probabilities using softmax
    softmax_f32(&scores)
}

/// Softmax transformation from scores to probabilities.
fn softmax_f32(scores: &[f64]) -> Vec<f32> {
    if scores.is_empty() {
        return Vec::new();
    }

    let max_score = scores.iter()
        .filter(|s| s.is_finite())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    if max_score == f64::NEG_INFINITY {
        // All invalid - uniform
        let uniform = 1.0 / scores.len() as f32;
        return vec![uniform; scores.len()];
    }

    let temperature = 1.0; // Softmax temperature for prior computation
    let exps: Vec<f64> = scores.iter()
        .map(|&s| {
            if s.is_finite() {
                ((s - max_score) / temperature).exp()
            } else {
                0.0
            }
        })
        .collect();

    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        let uniform = 1.0 / scores.len() as f32;
        return vec![uniform; scores.len()];
    }

    exps.iter().map(|&e| (e / sum) as f32).collect()
}

/// Normalize a heuristic score to [0, 1] range for MCTS value.
fn normalize_score(score: Score) -> f64 {
    // Scores typically range from -10000 to 10000
    // Map to 0..1 with sigmoid-like transformation
    let s = score as f64 / 1000.0;
    1.0 / (1.0 + (-s).exp())
}

/// Generate Dirichlet noise using Gamma distribution approximation.
/// Uses Marsaglia-Tsang method for Gamma samples.
fn generate_dirichlet_noise(n: usize, alpha: f32) -> Vec<f32> {
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // Use a time-based seed for noise diversity across searches
    let seed_val = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    let mut rng = SmallRng::seed_from_u64(seed_val);

    // Simple approximation: generate Gamma(alpha, 1) samples
    // For small alpha, use rejection sampling approximation
    let mut samples: Vec<f32> = Vec::with_capacity(n);

    for _ in 0..n {
        // Simple Gamma approximation using the fact that
        // Gamma(alpha, 1) ≈ (alpha * (1 + U))^alpha for small alpha
        // Using a more standard approach: sum of alpha exponentials
        let gamma_sample = if alpha >= 1.0 {
            // For alpha >= 1, use Marsaglia-Tsang method simplified
            let d = alpha - 1.0 / 3.0;
            let c = 1.0 / (9.0 * d).sqrt();
            loop {
                let x: f32 = rng.gen::<f32>() * 2.0 - 1.0;
                let v = (1.0 + c * x).powi(3);
                if v > 0.0 {
                    let u: f32 = rng.gen();
                    if u < 1.0 - 0.0331 * x.powi(4)
                        || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln())
                    {
                        break d * v;
                    }
                }
            }
        } else {
            // For alpha < 1, use the relation Gamma(alpha) = Gamma(alpha+1) * U^(1/alpha)
            let d = alpha + 1.0 - 1.0 / 3.0;
            let c = 1.0 / (9.0 * d).sqrt();
            let g1 = loop {
                let x: f32 = rng.gen::<f32>() * 2.0 - 1.0;
                let v = (1.0 + c * x).powi(3);
                if v > 0.0 {
                    let u: f32 = rng.gen();
                    if u < 1.0 - 0.0331 * x.powi(4)
                        || u.ln() < 0.5 * x * x + d * (1.0 - v + v.ln())
                    {
                        break d * v;
                    }
                }
            };
            let u: f32 = rng.gen();
            g1 * u.powf(1.0 / alpha)
        };

        samples.push(gamma_sample.max(1e-10));
    }

    // Normalize to sum to 1
    let sum: f32 = samples.iter().sum();
    if sum > 0.0 {
        samples.iter_mut().for_each(|s| *s /= sum);
    } else {
        let uniform = 1.0 / n as f32;
        samples.iter_mut().for_each(|s| *s = uniform);
    }

    samples
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Run MCTS search with default exploration config.
pub fn mcts_choose(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut search = MctsSearch::new(MctsConfig::default_exploration());
    search.search(state, perspective)
}

/// Run fast MCTS search (fewer simulations).
pub fn mcts_choose_fast(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut search = MctsSearch::new(MctsConfig::fast());
    search.search(state, perspective)
}

/// Run competition-strength MCTS.
pub fn mcts_choose_competition(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut search = MctsSearch::new(MctsConfig::competition());
    search.search(state, perspective)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{FactionId, MissionId, Phase};
    use wh40k_command_system::Command;
    use wh40k_game_core::ScenarioLoader;
    use wh40k_search_abstraction::{MacroActionId, TacticalIntent};

    fn create_test_state() -> GameState {
        ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            Some(MissionId::new(1)),
            [42u8; 32],
        )
    }

    #[test]
    fn test_mcts_config_defaults() {
        let config = MctsConfig::default();
        assert_eq!(config.num_simulations, 800);
        assert!(config.c_puct > 0.0);
        assert!(config.temperature > 0.0);
    }

    #[test]
    fn test_mcts_config_fast() {
        let config = MctsConfig::fast();
        assert_eq!(config.num_simulations, 100);
    }

    #[test]
    fn test_mcts_config_competition() {
        let config = MctsConfig::competition();
        assert_eq!(config.num_simulations, 1600);
        assert!(config.temperature < 1.0);
    }

    #[test]
    fn test_mcts_node_root() {
        let node = MctsNode::root(PlayerId::new(0));
        assert!(node.action.is_none());
        assert!(node.parent.is_none());
        assert_eq!(node.visits, 0);
        assert_eq!(node.q_value(), 0.0);
    }

    #[test]
    fn test_mcts_node_q_value() {
        let mut node = MctsNode::root(PlayerId::new(0));
        node.visits = 10;
        node.value_sum = 7.0;
        assert!((node.q_value() - 0.7).abs() < 1e-6);
    }

    #[test]
    fn test_mcts_tree_creation() {
        let tree = MctsTree::new(PlayerId::new(0));
        assert_eq!(tree.size(), 1);
        assert_eq!(tree.root, 0);
    }

    #[test]
    fn test_mcts_tree_add_node() {
        let mut tree = MctsTree::new(PlayerId::new(0));
        let cmd = Command::EndPhase { phase: Phase::Movement };
        let child = MctsNode::child(
            MacroAction::from_command(MacroActionId(0), cmd, TacticalIntent::PhaseControl),
            0,
            0.5,
            PlayerId::new(0),
            1,
        );
        let idx = tree.add_node(child);
        assert_eq!(idx, 1);
        assert_eq!(tree.size(), 2);
    }

    #[test]
    fn test_softmax() {
        let scores = vec![1.0, 2.0, 3.0];
        let probs = softmax_f32(&scores);
        assert_eq!(probs.len(), 3);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        // Higher score should have higher probability
        assert!(probs[2] > probs[1]);
        assert!(probs[1] > probs[0]);
    }

    #[test]
    fn test_softmax_all_equal() {
        let scores = vec![5.0, 5.0, 5.0];
        let probs = softmax_f32(&scores);
        for p in &probs {
            assert!((p - 1.0 / 3.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_softmax_with_neg_infinity() {
        let scores = vec![f64::NEG_INFINITY, 1.0, 2.0];
        let probs = softmax_f32(&scores);
        assert!((probs[0]).abs() < 0.01); // Invalid action gets ~0 probability
        assert!(probs[2] > probs[1]); // Higher score gets more
    }

    #[test]
    fn test_normalize_score() {
        let v = normalize_score(0);
        assert!((v - 0.5).abs() < 0.01);

        let high = normalize_score(5000);
        assert!(high > 0.9);

        let low = normalize_score(-5000);
        assert!(low < 0.1);
    }

    #[test]
    fn test_dirichlet_noise() {
        let noise = generate_dirichlet_noise(10, 0.3);
        assert_eq!(noise.len(), 10);
        let sum: f32 = noise.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        for &n in &noise {
            assert!(n >= 0.0);
        }
    }

    #[test]
    fn test_mcts_search_runs() {
        let state = create_test_state();
        let config = MctsConfig {
            num_simulations: 10, // Very few for test speed
            ..MctsConfig::fast()
        };
        let mut search = MctsSearch::new(config);
        let result = search.search(&state, PlayerId::new(0));
        // Should return some result (game is in progress)
        assert!(result.is_some() || !state.is_in_progress());
    }

    #[test]
    fn test_mcts_ai_worker_name() {
        let search = MctsSearch::new(MctsConfig::fast());
        assert_eq!(search.name(), "MCTS");
    }

    #[test]
    fn test_mcts_convenience_functions() {
        let state = create_test_state();
        // Just verify they don't panic
        // Use very fast config internally
        let config = MctsConfig {
            num_simulations: 5,
            ..MctsConfig::fast()
        };
        let mut search = MctsSearch::new(config);
        let _ = search.search(&state, PlayerId::new(0));
    }
}
