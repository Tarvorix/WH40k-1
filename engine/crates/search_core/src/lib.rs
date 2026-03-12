//! WH40K Engine - SearchCore crate
//!
//! Root search engine and AI worker implementations.
//! Provides multiple AI strength levels from simple greedy to
//! deep negamax with alpha-beta pruning.
//!
//! # AI Implementations
//!
//! - [`GreedyAi`] - Picks the highest-evaluated move at depth 0 (fastest, weakest)
//! - [`OnePlySearch`] - Evaluates all candidates at depth 1 (fast, moderate)
//! - [`NegamaxSearch`] - Full negamax with alpha-beta pruning at depth 2-3 (slowest, strongest)
//!
//! # Architecture
//!
//! - [`SearchRoot`] - Orchestrates a search from the current position
//! - [`AiWorker`] - Trait for AI decision-making (implemented by all AI levels)
//! - [`SearchConfig`] - Configuration for search parameters
//! - [`SearchResult`] - Result of a completed search
//! - [`SearchStats`] - Statistics from the search process
//!
//! Source: implementation_v3.md Section 11 (Search Engine Design)

use wh40k_core_types::{GameOutcome, Phase, PlayerId, SubPhase};
use wh40k_command_system::Command;
use wh40k_game_core::state::GameState;
use wh40k_game_core::CommandExecutor;
use wh40k_eval_features::{Score, SCORE_DRAW, SCORE_LOSS, SCORE_WIN};
use wh40k_eval_heuristic::{Evaluator, HeuristicEvaluator, HeuristicWeights};
use wh40k_eval_nnue::{
    BenchmarkResult, NnueEvaluator, NnueModel, NnueModelArtifact,
    compare_evaluators,
};
use wh40k_search_abstraction::{ActionGenerator, MacroAction, TacticalIntent};
use wh40k_search_ordering::{MoveOrderer, static_order, heuristic_order};
use wh40k_transposition::{TTBound, TTProbeResult, TranspositionTable};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

// ============================================================================
// Search Configuration
// ============================================================================

/// Configuration parameters for the search engine.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum search depth (in plies).
    pub max_depth: u8,
    /// Maximum number of candidates to consider at each node.
    /// Limits branching factor for performance.
    pub max_candidates: usize,
    /// Transposition table size in megabytes.
    pub tt_size_mb: usize,
    /// Whether to use alpha-beta pruning.
    pub use_alpha_beta: bool,
    /// Whether to use transposition table.
    pub use_tt: bool,
    /// Whether to use move ordering heuristics (killers, history).
    pub use_move_ordering: bool,
    /// Maximum number of chance samples for stochastic nodes.
    /// Higher = more accurate but slower.
    pub chance_samples: u32,
    /// Whether to use heuristic ordering at the root node.
    pub heuristic_root_ordering: bool,
    /// Node budget: maximum number of nodes to evaluate before stopping.
    /// 0 = unlimited.
    pub node_budget: u64,

    // === Phase 6: Iterative Deepening ===

    /// Whether to use iterative deepening (search depth 1, 2, 3, ..., max_depth).
    /// PV from each iteration seeds move ordering for the next.
    pub use_iterative_deepening: bool,

    /// Whether to use aspiration windows around the previous iteration's score.
    /// Reduces search tree size by searching a narrow alpha-beta window first.
    pub use_aspiration_windows: bool,

    /// Initial aspiration window half-width (centipawns).
    /// The first aspiration window is [score - initial, score + initial].
    pub aspiration_window_initial: Score,

    /// How much to widen the aspiration window on fail-high/fail-low.
    /// The delta doubles on each successive failure.
    pub aspiration_window_delta: Score,

    // === Phase 6: Quiescence and Extensions ===

    /// Whether to use quiescence search at leaf nodes.
    /// Continues searching unstable positions (mid-combat, charge resolution, etc.)
    /// instead of returning a static evaluation.
    pub use_quiescence: bool,

    /// Maximum additional depth for quiescence search beyond the main search depth.
    /// Prevents quiescence explosion in deeply nested tactical sequences.
    pub quiescence_max_depth: u8,

    /// Whether to use selective depth extensions.
    /// Extends search depth for critical moves (charges, fight order, stratagems).
    pub use_extensions: bool,

    /// Maximum total extensions allowed on any single search path.
    /// Prevents search explosion from cascading extensions.
    pub max_extensions: u8,

    // === Phase 6: Time Management ===

    /// Time budget for the search in milliseconds. 0 = unlimited (use node_budget or max_depth).
    pub time_budget_ms: u64,

    /// Soft time factor in thousandths (e.g., 600 = 60% of time_budget_ms).
    /// The search tries to complete the current iteration within this fraction.
    /// A new iteration is only started if soft time has not been exceeded.
    pub soft_time_factor: u32,

    // === Phase 6: Lazy SMP ===

    /// Number of threads for lazy SMP search. 1 = single-threaded (default).
    pub smp_threads: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_candidates: 30,
            tt_size_mb: 16,
            use_alpha_beta: true,
            use_tt: true,
            use_move_ordering: true,
            chance_samples: 3,
            heuristic_root_ordering: true,
            node_budget: 0,
            // Phase 6 defaults: all disabled to preserve existing behavior
            use_iterative_deepening: false,
            use_aspiration_windows: false,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: false,
            quiescence_max_depth: 4,
            use_extensions: false,
            max_extensions: 4,
            time_budget_ms: 0,
            soft_time_factor: 600, // 60%
            smp_threads: 1,
        }
    }
}

impl SearchConfig {
    /// Create a config for greedy (depth 0) search.
    pub fn greedy() -> Self {
        Self {
            max_depth: 0,
            max_candidates: 50,
            tt_size_mb: 0,
            use_alpha_beta: false,
            use_tt: false,
            use_move_ordering: false,
            chance_samples: 1,
            heuristic_root_ordering: false,
            node_budget: 0,
            use_iterative_deepening: false,
            use_aspiration_windows: false,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: false,
            quiescence_max_depth: 0,
            use_extensions: false,
            max_extensions: 0,
            time_budget_ms: 0,
            soft_time_factor: 600,
            smp_threads: 1,
        }
    }

    /// Create a config for one-ply search.
    pub fn one_ply() -> Self {
        Self {
            max_depth: 1,
            max_candidates: 40,
            tt_size_mb: 4,
            use_alpha_beta: true,
            use_tt: true,
            use_move_ordering: false,
            chance_samples: 2,
            heuristic_root_ordering: true,
            node_budget: 0,
            use_iterative_deepening: false,
            use_aspiration_windows: false,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: false,
            quiescence_max_depth: 0,
            use_extensions: false,
            max_extensions: 0,
            time_budget_ms: 0,
            soft_time_factor: 600,
            smp_threads: 1,
        }
    }

    /// Create a config for negamax at the given depth.
    pub fn negamax(depth: u8) -> Self {
        Self {
            max_depth: depth,
            max_candidates: 30,
            tt_size_mb: 16,
            use_alpha_beta: true,
            use_tt: true,
            use_move_ordering: true,
            chance_samples: 3,
            heuristic_root_ordering: true,
            node_budget: 0,
            use_iterative_deepening: false,
            use_aspiration_windows: false,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: false,
            quiescence_max_depth: 0,
            use_extensions: false,
            max_extensions: 0,
            time_budget_ms: 0,
            soft_time_factor: 600,
            smp_threads: 1,
        }
    }

    /// Create a config for iterative deepening search.
    /// Full Stockfish-like search with ID, aspiration windows, quiescence,
    /// extensions, and move ordering. Uses node budget for stopping.
    ///
    /// Source: implementation_v3.md Section 11.1
    pub fn iterative_deepening(max_depth: u8) -> Self {
        Self {
            max_depth,
            max_candidates: 30,
            tt_size_mb: 32,
            use_alpha_beta: true,
            use_tt: true,
            use_move_ordering: true,
            chance_samples: 3,
            heuristic_root_ordering: true,
            node_budget: 0,
            use_iterative_deepening: true,
            use_aspiration_windows: true,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: true,
            quiescence_max_depth: 4,
            use_extensions: true,
            max_extensions: 4,
            time_budget_ms: 0,
            soft_time_factor: 600,
            smp_threads: 1,
        }
    }

    /// Create a config for time-limited iterative deepening search.
    /// Searches until the time budget is exhausted, returning the best
    /// result from the last completed iteration.
    ///
    /// Source: implementation_v3.md Section 11.7 (Time management)
    pub fn iterative_deepening_timed(time_ms: u64) -> Self {
        Self {
            max_depth: 64, // Effectively unlimited; time is the constraint
            max_candidates: 30,
            tt_size_mb: 32,
            use_alpha_beta: true,
            use_tt: true,
            use_move_ordering: true,
            chance_samples: 3,
            heuristic_root_ordering: true,
            node_budget: 0,
            use_iterative_deepening: true,
            use_aspiration_windows: true,
            aspiration_window_initial: 50,
            aspiration_window_delta: 50,
            use_quiescence: true,
            quiescence_max_depth: 4,
            use_extensions: true,
            max_extensions: 4,
            time_budget_ms: time_ms,
            soft_time_factor: 600,
            smp_threads: 1,
        }
    }
}

// ============================================================================
// Search Statistics
// ============================================================================

/// Statistics gathered during a search.
#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    /// Total nodes evaluated (leaf + internal).
    pub nodes_evaluated: u64,
    /// Number of leaf node evaluations.
    pub leaf_evaluations: u64,
    /// Number of alpha-beta cutoffs (beta pruning).
    pub beta_cutoffs: u64,
    /// Number of transposition table hits that provided cutoffs.
    pub tt_cutoffs: u64,
    /// Number of transposition table hits for move ordering.
    pub tt_ordering_hits: u64,
    /// Maximum depth reached during search.
    pub max_depth_reached: u8,
    /// Number of candidate actions generated at root.
    pub root_candidates: usize,
    /// Search depth requested.
    pub depth_requested: u8,
    /// Number of chance node samples taken.
    pub chance_samples: u64,

    // === Phase 6: Iterative Deepening Stats ===

    /// Number of iterative deepening iterations completed.
    pub iterations_completed: u8,
    /// Number of aspiration window resets (fail-high or fail-low).
    pub aspiration_resets: u64,
    /// Number of nodes evaluated in quiescence search.
    pub quiescence_nodes: u64,
    /// Number of selective depth extensions applied.
    pub extensions: u64,
    /// Total time elapsed during search in milliseconds.
    pub time_elapsed_ms: u64,
    /// Nodes per second throughput.
    pub nps: u64,
    /// Number of times the principal variation changed between iterations.
    pub pv_changes: u64,
    /// Selective depth: maximum depth reached including extensions and quiescence.
    pub seldepth: u8,
}

impl std::fmt::Display for SearchStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Search[depth={}/{}, seldepth={}, nodes={}, leaves={}, cutoffs={}, tt_cuts={}, tt_ord={}, \
             root_cands={}, iter={}, asp_resets={}, qs_nodes={}, ext={}, time={}ms, nps={}, pv_chg={}]",
            self.max_depth_reached,
            self.depth_requested,
            self.seldepth,
            self.nodes_evaluated,
            self.leaf_evaluations,
            self.beta_cutoffs,
            self.tt_cutoffs,
            self.tt_ordering_hits,
            self.root_candidates,
            self.iterations_completed,
            self.aspiration_resets,
            self.quiescence_nodes,
            self.extensions,
            self.time_elapsed_ms,
            self.nps,
            self.pv_changes,
        )
    }
}

// ============================================================================
// Search Result
// ============================================================================

/// The result of a completed AI search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The best action found by the search.
    pub best_action: MacroAction,
    /// The evaluation score of the best action.
    pub score: Score,
    /// Search statistics.
    pub stats: SearchStats,
    /// The principal variation (best line of play).
    /// Contains the sequence of best actions from root.
    pub pv: Vec<MacroAction>,
    /// All root candidate scores for analysis.
    pub candidate_scores: Vec<(MacroAction, Score)>,
}

impl SearchResult {
    /// Get the commands from the best action that should be executed.
    pub fn best_commands(&self) -> &[Command] {
        &self.best_action.commands
    }

    /// Get the tactical intent of the best action.
    pub fn best_intent(&self) -> TacticalIntent {
        self.best_action.intent
    }
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Best: {} (score: {:+}, intent: {:?}) | {}",
            self.best_action.label,
            self.score,
            self.best_action.intent,
            self.stats,
        )
    }
}

// ============================================================================
// AI Worker Trait
// ============================================================================

/// Trait for AI decision-making.
/// All AI implementations (greedy, one-ply, negamax) implement this.
pub trait AiWorker {
    /// Choose the best action for the current game state.
    ///
    /// # Arguments
    /// * `state` - The current game state
    /// * `perspective` - Which player the AI is playing as
    ///
    /// # Returns
    /// A SearchResult containing the best action and diagnostics.
    /// Returns None if no legal actions are available.
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult>;

    /// Returns the name of this AI implementation.
    fn name(&self) -> &str;
}

// ============================================================================
// Greedy AI (Depth 0)
// ============================================================================

/// Greedy AI: evaluates each candidate action and picks the best one.
/// No look-ahead. Fastest but weakest AI level.
///
/// Suitable for:
/// - Quick testing
/// - Baseline comparisons
/// - Fallback when search budget is exhausted
pub struct GreedyAi {
    evaluator: HeuristicEvaluator,
}

impl GreedyAi {
    /// Create a new greedy AI with default weights.
    pub fn new() -> Self {
        Self {
            evaluator: HeuristicEvaluator::with_defaults(),
        }
    }

    /// Create a greedy AI with custom evaluation weights.
    pub fn with_weights(weights: HeuristicWeights) -> Self {
        Self {
            evaluator: HeuristicEvaluator::new(weights),
        }
    }
}

impl Default for GreedyAi {
    fn default() -> Self {
        Self::new()
    }
}

impl AiWorker for GreedyAi {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return None;
        }

        let mut stats = SearchStats {
            depth_requested: 0,
            root_candidates: actions.len(),
            ..Default::default()
        };

        let mut best_score = SCORE_LOSS;
        let mut best_idx = 0;
        let mut candidate_scores: Vec<(MacroAction, Score)> = Vec::with_capacity(actions.len());

        for (i, action) in actions.iter().enumerate() {
            // Apply the action to a cloned state
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            let score = if applied {
                self.evaluator.evaluate(&child_state, perspective)
            } else {
                SCORE_LOSS + 1 // Invalid action gets worst score
            };

            stats.nodes_evaluated += 1;
            stats.leaf_evaluations += 1;

            candidate_scores.push((action.clone(), score));

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let best_action = actions[best_idx].clone();

        Some(SearchResult {
            best_action: best_action.clone(),
            score: best_score,
            stats,
            pv: vec![best_action],
            candidate_scores,
        })
    }

    fn name(&self) -> &str {
        "GreedyAi"
    }
}

// ============================================================================
// One-Ply Search (Depth 1)
// ============================================================================

/// One-ply search: evaluates each root candidate after applying it,
/// then does a shallow evaluation. Moderate speed and strength.
///
/// Uses heuristic ordering at the root to try the most promising
/// moves first, though at depth 1 this mainly helps diagnostics.
pub struct OnePlySearch {
    evaluator: HeuristicEvaluator,
    config: SearchConfig,
}

impl OnePlySearch {
    /// Create a new one-ply search with default settings.
    pub fn new() -> Self {
        Self {
            evaluator: HeuristicEvaluator::with_defaults(),
            config: SearchConfig::one_ply(),
        }
    }

    /// Create a one-ply search with custom weights.
    pub fn with_weights(weights: HeuristicWeights) -> Self {
        Self {
            evaluator: HeuristicEvaluator::new(weights),
            config: SearchConfig::one_ply(),
        }
    }
}

impl Default for OnePlySearch {
    fn default() -> Self {
        Self::new()
    }
}

impl AiWorker for OnePlySearch {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return None;
        }

        let mut stats = SearchStats {
            depth_requested: 1,
            root_candidates: actions.len(),
            max_depth_reached: 1,
            ..Default::default()
        };

        // Order candidates using heuristic prior at root
        let ordered = if self.config.heuristic_root_ordering {
            heuristic_order(&candidates, state, perspective, &self.evaluator)
        } else {
            static_order(&candidates)
        };

        let mut best_score = SCORE_LOSS;
        let mut best_idx = 0;
        let mut candidate_scores: Vec<(MacroAction, Score)> = Vec::with_capacity(actions.len());

        let max_to_evaluate = ordered.len().min(self.config.max_candidates);

        for ord_move in ordered.iter().take(max_to_evaluate) {
            let action = &actions[ord_move.index];

            // Apply the action
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            let score = if applied {
                // Check terminal states
                if !child_state.is_in_progress() {
                    match child_state.game_outcome {
                        GameOutcome::Victory(winner) if winner == perspective => {
                            SCORE_WIN - 1
                        }
                        GameOutcome::Victory(_) => SCORE_LOSS + 1,
                        GameOutcome::Draw => SCORE_DRAW,
                        _ => self.evaluator.evaluate(&child_state, perspective),
                    }
                } else {
                    // At depth 1, we just evaluate the resulting position
                    self.evaluator.evaluate(&child_state, perspective)
                }
            } else {
                SCORE_LOSS + 1
            };

            stats.nodes_evaluated += 1;
            stats.leaf_evaluations += 1;

            candidate_scores.push((action.clone(), score));

            if score > best_score {
                best_score = score;
                best_idx = ord_move.index;
            }
        }

        let best_action = actions[best_idx].clone();

        Some(SearchResult {
            best_action: best_action.clone(),
            score: best_score,
            stats,
            pv: vec![best_action],
            candidate_scores,
        })
    }

    fn name(&self) -> &str {
        "OnePlySearch"
    }
}

// ============================================================================
// Negamax Search (Depth 2-3+)
// ============================================================================

/// Full negamax search with alpha-beta pruning.
/// The strongest AI level, searching multiple plies deep.
///
/// Features:
/// - Negamax framework with alpha-beta pruning
/// - Transposition table for avoiding redundant work
/// - Killer move heuristic for better move ordering
/// - History heuristic for learned move ordering
/// - Deterministic chance node sampling
/// - Configurable depth and node budget
///
/// Source: implementation_v3.md Section 11.6 (negamax/alpha-beta)
pub struct NegamaxSearch {
    evaluator: HeuristicEvaluator,
    config: SearchConfig,
    tt: TranspositionTable,
    orderer: MoveOrderer,
    stats: SearchStats,
}

impl NegamaxSearch {
    /// Create a new negamax search with default settings.
    pub fn new() -> Self {
        let config = SearchConfig::default();
        let tt_size = if config.use_tt { config.tt_size_mb } else { 0 };
        Self {
            evaluator: HeuristicEvaluator::with_defaults(),
            config: config.clone(),
            tt: if tt_size > 0 {
                TranspositionTable::with_mb(tt_size)
            } else {
                TranspositionTable::new(1024)
            },
            orderer: MoveOrderer::new(config.max_depth as usize + 2),
            stats: SearchStats::default(),
        }
    }

    /// Create a negamax search with custom configuration.
    pub fn with_config(config: SearchConfig) -> Self {
        let tt_size = if config.use_tt { config.tt_size_mb } else { 0 };
        Self {
            evaluator: HeuristicEvaluator::with_defaults(),
            tt: if tt_size > 0 {
                TranspositionTable::with_mb(tt_size)
            } else {
                TranspositionTable::new(1024)
            },
            orderer: MoveOrderer::new(config.max_depth as usize + 2),
            stats: SearchStats::default(),
            config,
        }
    }

    /// Create a negamax search with custom weights and configuration.
    pub fn with_weights_and_config(weights: HeuristicWeights, config: SearchConfig) -> Self {
        let tt_size = if config.use_tt { config.tt_size_mb } else { 0 };
        Self {
            evaluator: HeuristicEvaluator::new(weights),
            tt: if tt_size > 0 {
                TranspositionTable::with_mb(tt_size)
            } else {
                TranspositionTable::new(1024)
            },
            orderer: MoveOrderer::new(config.max_depth as usize + 2),
            stats: SearchStats::default(),
            config,
        }
    }

    /// Get the transposition table statistics.
    pub fn tt_stats(&self) -> wh40k_transposition::TTStats {
        self.tt.stats()
    }

    /// Clear the transposition table (call between games).
    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    /// Check if the node budget has been exhausted.
    fn budget_exhausted(&self) -> bool {
        self.config.node_budget > 0 && self.stats.nodes_evaluated >= self.config.node_budget
    }

    /// The core negamax search function with alpha-beta pruning.
    ///
    /// # Arguments
    /// * `state` - Current game state
    /// * `depth` - Remaining depth to search
    /// * `alpha` - Lower bound (best score the maximizer can guarantee)
    /// * `beta` - Upper bound (best score the minimizer can guarantee)
    /// * `perspective` - Which player we're evaluating for at this node
    /// * `ply` - Distance from root (for killer table indexing)
    ///
    /// # Returns
    /// The evaluation score from the perspective of `perspective`.
    fn negamax(
        &mut self,
        state: &GameState,
        depth: u8,
        mut alpha: Score,
        beta: Score,
        perspective: PlayerId,
        ply: usize,
    ) -> Score {
        self.stats.nodes_evaluated += 1;

        // Check node budget
        if self.budget_exhausted() {
            return self.evaluator.evaluate(state, perspective);
        }

        // Terminal state check
        if !state.is_in_progress() {
            return match state.game_outcome {
                GameOutcome::Victory(winner) if winner == perspective => {
                    SCORE_WIN - ply as Score
                }
                GameOutcome::Victory(_) => SCORE_LOSS + ply as Score,
                GameOutcome::Draw => SCORE_DRAW,
                _ => self.evaluator.evaluate(state, perspective),
            };
        }

        // Tabling check
        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + ply as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - ply as Score;
        }

        // Leaf node: evaluate statically
        if depth == 0 {
            self.stats.leaf_evaluations += 1;
            return self.evaluator.evaluate(state, perspective);
        }

        // Track max depth reached
        if ply as u8 > self.stats.max_depth_reached {
            self.stats.max_depth_reached = ply as u8;
        }

        // Transposition table probe
        let state_hash = state.compute_hash();
        let mut tt_best_move: Option<u16> = None;

        if self.config.use_tt {
            match self.tt.probe(state_hash, depth) {
                TTProbeResult::Hit(entry) => {
                    // Can we use the stored score directly?
                    match entry.bound {
                        TTBound::Exact => {
                            self.stats.tt_cutoffs += 1;
                            return entry.score;
                        }
                        TTBound::LowerBound => {
                            if entry.score >= beta {
                                self.stats.tt_cutoffs += 1;
                                return entry.score;
                            }
                            if entry.score > alpha {
                                alpha = entry.score;
                            }
                        }
                        TTBound::UpperBound => {
                            if entry.score <= alpha {
                                self.stats.tt_cutoffs += 1;
                                return entry.score;
                            }
                        }
                    }
                    tt_best_move = entry.best_move_index.into();
                    if tt_best_move == Some(u16::MAX) {
                        tt_best_move = None;
                    }
                    if tt_best_move.is_some() {
                        self.stats.tt_ordering_hits += 1;
                    }
                }
                TTProbeResult::ShallowHit(entry) => {
                    // Can't use score, but can use best move for ordering
                    tt_best_move = if entry.has_best_move() {
                        Some(entry.best_move_index)
                    } else {
                        None
                    };
                    if tt_best_move.is_some() {
                        self.stats.tt_ordering_hits += 1;
                    }
                }
                TTProbeResult::Miss => {}
            }
        }

        // Generate candidates
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            // No legal moves - evaluate current position
            self.stats.leaf_evaluations += 1;
            return self.evaluator.evaluate(state, perspective);
        }

        // Order moves
        let ordered = if self.config.use_move_ordering {
            self.orderer.order_moves(&candidates, ply, tt_best_move)
        } else {
            static_order(&candidates)
        };

        let max_to_search = ordered.len().min(self.config.max_candidates);

        let mut best_score = SCORE_LOSS + ply as Score;
        let mut best_move_index: u16 = u16::MAX;
        let original_alpha = alpha;

        for (move_num, ord_move) in ordered.iter().take(max_to_search).enumerate() {
            let action = &actions[ord_move.index];

            // Apply action to cloned state
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            if !applied {
                continue;
            }

            // Determine who decides next in the child state
            let child_perspective = child_state.decision_owner;
            let score = if child_perspective == perspective {
                // Same player continues (e.g., multiple decisions in a phase)
                self.negamax(
                    &child_state,
                    depth - 1,
                    alpha,
                    beta,
                    perspective,
                    ply + 1,
                )
            } else {
                // Opponent's turn - negate the score
                -self.negamax(
                    &child_state,
                    depth - 1,
                    -beta,
                    -alpha,
                    child_perspective,
                    ply + 1,
                )
            };

            if score > best_score {
                best_score = score;
                best_move_index = ord_move.index as u16;

                if score > alpha {
                    alpha = score;
                }
            }

            // Alpha-beta cutoff
            if self.config.use_alpha_beta && alpha >= beta {
                self.stats.beta_cutoffs += 1;

                // Record cutoff for move ordering
                if self.config.use_move_ordering {
                    self.orderer.record_cutoff(ply, action);

                    // Record all previously searched moves as failures
                    for prev in ordered.iter().take(move_num) {
                        self.orderer.record_searched(&actions[prev.index], ply);
                    }
                }

                break;
            }

            // Record non-cutoff moves as searched
            if self.config.use_move_ordering && move_num > 0 {
                // Only record after the first move
            }
        }

        // Store in transposition table
        if self.config.use_tt {
            let bound = if best_score <= original_alpha {
                TTBound::UpperBound
            } else if best_score >= beta {
                TTBound::LowerBound
            } else {
                TTBound::Exact
            };

            self.tt.store(
                state_hash,
                best_score,
                depth,
                bound,
                best_move_index,
            );
        }

        best_score
    }
}

impl Default for NegamaxSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl AiWorker for NegamaxSearch {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        // Reset stats for this search
        self.stats = SearchStats {
            depth_requested: self.config.max_depth,
            ..Default::default()
        };

        // New TT generation for this search
        if self.config.use_tt {
            self.tt.new_generation();
        }

        // Clear killers for new search
        if self.config.use_move_ordering {
            self.orderer.clear_killers();
        }

        // Generate root candidates
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return None;
        }

        self.stats.root_candidates = actions.len();

        // Order root moves with heuristic evaluation for best initial ordering
        let ordered = if self.config.heuristic_root_ordering {
            heuristic_order(&candidates, state, perspective, &self.evaluator)
        } else {
            static_order(&candidates)
        };

        let max_to_search = ordered.len().min(self.config.max_candidates);

        let mut best_score = SCORE_LOSS;
        let mut best_idx = 0;
        let mut alpha = SCORE_LOSS;
        let beta = SCORE_WIN;
        let mut candidate_scores: Vec<(MacroAction, Score)> = Vec::with_capacity(actions.len());
        let mut pv: Vec<MacroAction> = Vec::new();

        for ord_move in ordered.iter().take(max_to_search) {
            let action = &actions[ord_move.index];

            // Apply action
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            let score = if applied {
                // Determine who decides next
                let child_perspective = child_state.decision_owner;

                if child_perspective == perspective {
                    // Same player continues
                    self.negamax(
                        &child_state,
                        self.config.max_depth - 1,
                        alpha,
                        beta,
                        perspective,
                        1,
                    )
                } else {
                    // Opponent's turn - negate
                    -self.negamax(
                        &child_state,
                        self.config.max_depth - 1,
                        -beta,
                        -alpha,
                        child_perspective,
                        1,
                    )
                }
            } else {
                SCORE_LOSS + 1
            };

            candidate_scores.push((action.clone(), score));

            if score > best_score {
                best_score = score;
                best_idx = ord_move.index;
                pv = vec![action.clone()];

                if score > alpha {
                    alpha = score;
                }
            }

            // Check budget
            if self.budget_exhausted() {
                break;
            }
        }

        let best_action = actions[best_idx].clone();

        Some(SearchResult {
            best_action,
            score: best_score,
            stats: self.stats.clone(),
            pv,
            candidate_scores,
        })
    }

    fn name(&self) -> &str {
        "NegamaxSearch"
    }
}

// ============================================================================
// Phase 6: Principal Variation Line
// ============================================================================

/// A principal variation line: the sequence of best actions through the search tree.
/// Used for PV tracking during iterative deepening and for seeding move ordering
/// in subsequent iterations.
///
/// Source: implementation_v3.md Section 11.1 (principal variation tracking)
#[derive(Debug, Clone, Default)]
pub struct PvLine {
    /// The sequence of best actions from root to leaf.
    pub moves: Vec<MacroAction>,
}

impl PvLine {
    /// Create an empty PV line.
    pub fn new() -> Self {
        Self { moves: Vec::new() }
    }

    /// Create a PV line with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            moves: Vec::with_capacity(cap),
        }
    }

    /// Get the move at a specific ply depth, if any.
    pub fn move_at_ply(&self, ply: usize) -> Option<&MacroAction> {
        self.moves.get(ply)
    }

    /// Get the length of this PV line.
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// Check if this PV line is empty.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Clear the PV line.
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    /// Set this PV line from a move and a child PV.
    /// The resulting PV is: [action] ++ child_pv.
    pub fn update_from(&mut self, action: &MacroAction, child_pv: &PvLine) {
        self.moves.clear();
        self.moves.push(action.clone());
        self.moves.extend_from_slice(&child_pv.moves);
    }
}

impl std::fmt::Display for PvLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.moves.is_empty() {
            return write!(f, "(empty PV)");
        }
        for (i, action) in self.moves.iter().enumerate() {
            if i > 0 {
                write!(f, " -> ")?;
            }
            write!(f, "{}", action.label)?;
        }
        Ok(())
    }
}

// ============================================================================
// Phase 6: Time Manager
// ============================================================================

/// Manages time allocation for iterative deepening search.
/// Implements dynamic time control based on position characteristics
/// and search progress.
///
/// Inputs considered (from implementation_v3.md Section 11.7):
/// - Remaining time budget
/// - Phase sharpness (more time in Charge/Fight)
/// - Root branching factor (more time with more candidates)
/// - Score stability (less time when PV doesn't change)
/// - Confidence gap between best and second-best move
///
/// Source: implementation_v3.md Section 11.7 (Time management)
#[derive(Debug, Clone)]
pub struct TimeManager {
    /// The instant when the search started.
    start_time: Instant,
    /// Hard time limit in milliseconds (never exceed).
    hard_limit_ms: u64,
    /// Soft time limit in milliseconds (try not to start new iterations after).
    soft_limit_ms: u64,
    /// Node budget (0 = unlimited). Alternative stopping criterion.
    node_budget: u64,
    /// Whether time-based stopping is enabled.
    time_based: bool,
}

impl TimeManager {
    /// Create a new time manager from search configuration.
    pub fn new(config: &SearchConfig) -> Self {
        let time_based = config.time_budget_ms > 0;
        let hard_limit_ms = config.time_budget_ms;
        let soft_limit_ms = if time_based {
            (hard_limit_ms as u128 * config.soft_time_factor as u128 / 1000) as u64
        } else {
            0
        };

        Self {
            start_time: Instant::now(),
            hard_limit_ms,
            soft_limit_ms,
            node_budget: config.node_budget,
            time_based,
        }
    }

    /// Create a time manager with a specific time budget (milliseconds).
    pub fn with_time(time_ms: u64) -> Self {
        Self {
            start_time: Instant::now(),
            hard_limit_ms: time_ms,
            soft_limit_ms: time_ms * 60 / 100, // 60% soft limit
            node_budget: 0,
            time_based: true,
        }
    }

    /// Create an unlimited time manager (only depth/node budget stops search).
    pub fn unlimited() -> Self {
        Self {
            start_time: Instant::now(),
            hard_limit_ms: 0,
            soft_limit_ms: 0,
            node_budget: 0,
            time_based: false,
        }
    }

    /// Reset the start time (call at the beginning of a new search).
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Get elapsed time in milliseconds since search started.
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    /// Check if the hard time limit has been exceeded.
    /// The search MUST stop immediately when this returns true.
    pub fn hard_stop(&self, nodes: u64) -> bool {
        if self.node_budget > 0 && nodes >= self.node_budget {
            return true;
        }
        if self.time_based {
            return self.elapsed_ms() >= self.hard_limit_ms;
        }
        false
    }

    /// Check if the soft time limit has been exceeded.
    /// A new iteration should NOT be started when this returns true.
    pub fn soft_stop(&self) -> bool {
        if self.time_based {
            return self.elapsed_ms() >= self.soft_limit_ms;
        }
        false
    }

    /// Adjust the soft time limit dynamically based on search characteristics.
    ///
    /// More time when:
    /// - Charge/fight sequencing is sharp
    /// - Objective swing is immediate
    /// - Multiple valid reactions exist
    ///
    /// Less time when:
    /// - PV is stable (didn't change between iterations)
    /// - One move dominates heavily (large score gap)
    /// - Procedural/cleanup phase
    pub fn adjust_for_position(
        &mut self,
        phase: Phase,
        pv_stable: bool,
        score_gap: Score,
        root_candidates: usize,
    ) {
        if !self.time_based {
            return;
        }

        let mut factor: f64 = 1.0;

        // Phase sharpness: spend more time in tactical phases
        match phase {
            Phase::Charge | Phase::Fight => factor *= 1.4,
            Phase::Shooting => factor *= 1.2,
            Phase::Movement => factor *= 1.0,
            Phase::Command | Phase::PreBattle | Phase::GameEnd => factor *= 0.6,
        }

        // PV stability: less time when PV is stable
        if pv_stable {
            factor *= 0.7;
        }

        // Score gap: less time when one move dominates
        if score_gap > 200 {
            factor *= 0.6;
        } else if score_gap > 100 {
            factor *= 0.8;
        }

        // Branching factor: more time with more candidates
        if root_candidates > 20 {
            factor *= 1.3;
        } else if root_candidates < 5 {
            factor *= 0.7;
        }

        // Apply factor to soft limit (never exceed hard limit)
        let base_soft = self.hard_limit_ms * 60 / 100;
        self.soft_limit_ms = ((base_soft as f64 * factor) as u64).min(self.hard_limit_ms);
    }

    /// Calculate nodes per second.
    pub fn nps(&self, nodes: u64) -> u64 {
        let elapsed = self.elapsed_ms();
        if elapsed == 0 {
            return nodes * 1000; // Assume <1ms = extrapolate
        }
        nodes * 1000 / elapsed
    }
}

// ============================================================================
// Phase 6: Position Instability Detection
// ============================================================================

/// Detect whether a game position is "unstable" and should not be evaluated
/// statically. Unstable positions are those in the middle of a tactical
/// sequence where the evaluation would be misleading.
///
/// From implementation_v3.md Section 11.8:
/// "Do not stop search at unstable states."
///
/// Extend search in:
/// - Ongoing attack sequences
/// - Charge resolutions
/// - Fight order decisions
/// - Lethal retaliation windows
/// - Major scoring-swing reactions
///
/// Source: implementation_v3.md Section 11.8 (Quiescence/tactical extension)
pub fn is_position_unstable(state: &GameState) -> bool {
    // Active reaction windows are always unstable - a pending decision exists
    if state.has_reaction_window() {
        return true;
    }

    // Check for mid-resolution subphases where stopping evaluation
    // would give a misleading score (e.g., after hits rolled but before
    // damage applied, or after charge declared but before charge move)
    matches!(
        state.current_subphase,
        // Shooting resolution in progress
        SubPhase::ResolveAttacks
        // Charge resolution in progress
        | SubPhase::RollChargeDistance
        | SubPhase::MakeChargeMove
        | SubPhase::ResolveOverwatch
        // Fight resolution in progress
        | SubPhase::PileIn
        | SubPhase::ResolveMeleeAttacks
        | SubPhase::Consolidate
        | SubPhase::FightsFirst
        | SubPhase::RemainingCombats
        // Generic resolution states
        | SubPhase::EffectResolution
        | SubPhase::ReactionWindow
        | SubPhase::StratagemWindow
    )
}

/// Classify the tactical volatility of a position for time management.
/// Higher values mean more time should be allocated.
///
/// Source: implementation_v3.md Section 11.7 (spend more/less time heuristics)
pub fn position_volatility(state: &GameState) -> u8 {
    let mut volatility: u8 = 0;

    // Reaction windows add volatility
    if state.has_reaction_window() {
        volatility += 3;
    }

    // Phase-based volatility
    match state.current_phase {
        Phase::Fight => volatility += 4,    // Fight phase: critical sequencing
        Phase::Charge => volatility += 3,   // Charge phase: high-impact decisions
        Phase::Shooting => volatility += 2, // Shooting: moderate tactical depth
        Phase::Movement => volatility += 1, // Movement: positional
        Phase::Command | Phase::PreBattle | Phase::GameEnd => {} // Low volatility
    }

    // Mid-resolution subphases are highly volatile
    if is_position_unstable(state) {
        volatility += 2;
    }

    volatility
}

// ============================================================================
// Phase 6: Search Diagnostics
// ============================================================================

/// Information captured for a single iteration of iterative deepening.
/// Used for debugging, analysis, and time management decisions.
#[derive(Debug, Clone)]
pub struct IterationInfo {
    /// The depth of this iteration.
    pub depth: u8,
    /// The best score found at this depth.
    pub score: Score,
    /// The principal variation at this depth.
    pub pv: PvLine,
    /// Nodes evaluated during this iteration.
    pub nodes: u64,
    /// Time elapsed at the end of this iteration (ms).
    pub time_ms: u64,
    /// NPS at the end of this iteration.
    pub nps: u64,
    /// Number of aspiration resets during this iteration.
    pub aspiration_resets: u64,
    /// Whether the PV changed from the previous iteration.
    pub pv_changed: bool,
}

impl std::fmt::Display for IterationInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "depth {} score {:+} nodes {} time {}ms nps {} pv: {}{}",
            self.depth,
            self.score,
            self.nodes,
            self.time_ms,
            self.nps,
            self.pv,
            if self.pv_changed { " (PV changed)" } else { "" },
        )
    }
}

/// Comprehensive search diagnostics collected during an iterative deepening search.
/// Provides detailed telemetry for debugging and analysis.
///
/// Source: implementation_v3.md Section 11 (search diagnostics requirement)
#[derive(Debug, Clone)]
pub struct SearchDiagnostics {
    /// Per-iteration information (one entry per completed iteration).
    pub iterations: Vec<IterationInfo>,
    /// Final search statistics.
    pub final_stats: SearchStats,
    /// TT statistics at end of search.
    pub tt_occupancy: f64,
    /// TT hit rate.
    pub tt_hit_rate: f64,
    /// Whether the search was terminated by time limit.
    pub stopped_by_time: bool,
    /// Whether the search was terminated by node budget.
    pub stopped_by_nodes: bool,
    /// The game phase during which this search was conducted.
    pub search_phase: Phase,
    /// Position volatility score.
    pub volatility: u8,
}

impl std::fmt::Display for SearchDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Search Diagnostics ===")?;
        writeln!(f, "Phase: {:?} | Volatility: {}", self.search_phase, self.volatility)?;
        writeln!(f, "Iterations completed: {}", self.iterations.len())?;
        for info in &self.iterations {
            writeln!(f, "  {}", info)?;
        }
        writeln!(f, "Final: {}", self.final_stats)?;
        writeln!(
            f,
            "TT: occupancy={:.1}% hit_rate={:.1}%",
            self.tt_occupancy * 100.0,
            self.tt_hit_rate * 100.0
        )?;
        if self.stopped_by_time {
            writeln!(f, "Stopped by: time limit")?;
        }
        if self.stopped_by_nodes {
            writeln!(f, "Stopped by: node budget")?;
        }
        Ok(())
    }
}

// ============================================================================
// Phase 6: Iterative Deepening Search
// ============================================================================

/// Full Stockfish-like iterative deepening search engine.
///
/// This is the strongest AI implementation, combining:
/// - **Iterative deepening**: Search depth 1, 2, 3, ..., max_depth
/// - **Aspiration windows**: Narrow search window seeded from previous iteration
/// - **Principal variation tracking**: Best line propagated through the tree
/// - **PV seeding**: Previous iteration's PV seeds move ordering
/// - **Quiescence search**: Continue searching unstable positions at leaf nodes
/// - **Selective extensions**: Extend search for critical moves (charges, fights)
/// - **Transposition table**: Avoids redundant work across iterations
/// - **Killer/history heuristics**: Learned move ordering
/// - **Time management**: Dynamic time allocation with soft/hard limits
///
/// Source: implementation_v3.md Section 11.1-11.8
pub struct IterativeDeepeningSearch {
    /// Position evaluator.
    evaluator: HeuristicEvaluator,
    /// Search configuration.
    config: SearchConfig,
    /// Transposition table.
    tt: TranspositionTable,
    /// Move orderer (killers + history).
    orderer: MoveOrderer,
    /// Cumulative search statistics.
    stats: SearchStats,
    /// Time manager for the current search.
    time_manager: TimeManager,
    /// Best PV from the last completed iteration (used for PV seeding).
    previous_pv: PvLine,
    /// Per-iteration diagnostic information.
    iteration_infos: Vec<IterationInfo>,
    /// External stop signal (for lazy SMP coordination).
    stop_signal: Option<Arc<AtomicBool>>,
}

impl IterativeDeepeningSearch {
    /// Create a new iterative deepening search with default settings.
    pub fn new() -> Self {
        let config = SearchConfig::iterative_deepening(6);
        Self::with_config(config)
    }

    /// Create an iterative deepening search with custom configuration.
    pub fn with_config(config: SearchConfig) -> Self {
        let tt_size = if config.use_tt { config.tt_size_mb } else { 0 };
        Self {
            evaluator: HeuristicEvaluator::with_defaults(),
            tt: if tt_size > 0 {
                TranspositionTable::with_mb(tt_size)
            } else {
                TranspositionTable::new(1024)
            },
            orderer: MoveOrderer::new(config.max_depth as usize + config.max_extensions as usize + config.quiescence_max_depth as usize + 2),
            stats: SearchStats::default(),
            time_manager: TimeManager::new(&config),
            previous_pv: PvLine::new(),
            iteration_infos: Vec::new(),
            stop_signal: None,
            config,
        }
    }

    /// Create an iterative deepening search with custom weights and configuration.
    pub fn with_weights_and_config(weights: HeuristicWeights, config: SearchConfig) -> Self {
        let tt_size = if config.use_tt { config.tt_size_mb } else { 0 };
        Self {
            evaluator: HeuristicEvaluator::new(weights),
            tt: if tt_size > 0 {
                TranspositionTable::with_mb(tt_size)
            } else {
                TranspositionTable::new(1024)
            },
            orderer: MoveOrderer::new(config.max_depth as usize + config.max_extensions as usize + config.quiescence_max_depth as usize + 2),
            stats: SearchStats::default(),
            time_manager: TimeManager::new(&config),
            previous_pv: PvLine::new(),
            iteration_infos: Vec::new(),
            stop_signal: None,
            config,
        }
    }

    /// Set an external stop signal (for lazy SMP thread coordination).
    pub fn set_stop_signal(&mut self, signal: Arc<AtomicBool>) {
        self.stop_signal = Some(signal);
    }

    /// Get the transposition table statistics.
    pub fn tt_stats(&self) -> wh40k_transposition::TTStats {
        self.tt.stats()
    }

    /// Clear the transposition table (call between games).
    pub fn clear_tt(&mut self) {
        self.tt.clear();
    }

    /// Clear all search state (call between games).
    pub fn clear_all(&mut self) {
        self.tt.clear();
        self.orderer.clear_all();
        self.previous_pv.clear();
        self.iteration_infos.clear();
    }

    /// Get the diagnostics from the last search.
    pub fn diagnostics(&self, state: &GameState) -> SearchDiagnostics {
        let tt_stats = self.tt.stats();
        SearchDiagnostics {
            iterations: self.iteration_infos.clone(),
            final_stats: self.stats.clone(),
            tt_occupancy: tt_stats.occupancy,
            tt_hit_rate: tt_stats.hit_rate,
            stopped_by_time: self.time_manager.time_based && self.time_manager.elapsed_ms() >= self.time_manager.hard_limit_ms,
            stopped_by_nodes: self.config.node_budget > 0 && self.stats.nodes_evaluated >= self.config.node_budget,
            search_phase: state.current_phase,
            volatility: position_volatility(state),
        }
    }

    /// Check if any stopping condition has been met.
    fn should_stop(&self) -> bool {
        // External stop signal (lazy SMP)
        if let Some(ref signal) = self.stop_signal {
            if signal.load(Ordering::Relaxed) {
                return true;
            }
        }
        // Time or node budget
        self.time_manager.hard_stop(self.stats.nodes_evaluated)
    }

    /// Compute selective depth extension for a move.
    ///
    /// Extensions give critical moves an extra ply of search depth
    /// to avoid horizon effects. From implementation_v3.md Section 11.8:
    /// - Charge declarations (high-impact decisions)
    /// - Fight order decisions (critical sequencing)
    /// - Reaction window responses (stratagems)
    ///
    /// Returns the number of extra plies to extend (0 or 1).
    fn compute_extension(
        &self,
        action: &MacroAction,
        state: &GameState,
        current_extensions: u8,
    ) -> u8 {
        if !self.config.use_extensions {
            return 0;
        }
        if current_extensions >= self.config.max_extensions {
            return 0;
        }

        match action.intent {
            // Charge declarations are high-impact: extend to see the result
            TacticalIntent::ChargeHighValueTarget
            | TacticalIntent::MultiChargeObjectiveSwing
            | TacticalIntent::ChargeSoftenedTarget => 1,

            // Fight order decisions with maximum damage intent
            TacticalIntent::FightMaxDamage
                if state.current_phase == Phase::Fight => 1,

            // Stratagem usage (both offensive and defensive) can swing the position
            TacticalIntent::DefensiveStratagem
            | TacticalIntent::OffensiveStratagem => 1,

            // Fight order manipulation (Counter-Operative, etc.)
            TacticalIntent::FightOrderManipulation => 1,

            // Movement reactions (Heroic Intervention, Overwatch)
            TacticalIntent::MovementReaction
                if state.has_reaction_window() => 1,

            _ => 0,
        }
    }

    /// Quiescence search: continues searching unstable positions at leaf nodes.
    ///
    /// Instead of returning a static evaluation at depth 0, quiescence
    /// continues searching if the position is "unstable" (mid-combat,
    /// charge resolution, reaction windows, fight sequencing).
    ///
    /// Uses a stand-pat cutoff: if the static evaluation already beats beta,
    /// assume the position is good enough and prune.
    ///
    /// Source: implementation_v3.md Section 11.8
    fn quiescence(
        &mut self,
        state: &GameState,
        mut alpha: Score,
        beta: Score,
        perspective: PlayerId,
        ply: usize,
        qs_depth: u8,
    ) -> Score {
        self.stats.nodes_evaluated += 1;
        self.stats.quiescence_nodes += 1;

        // Track selective depth
        if ply as u8 > self.stats.seldepth {
            self.stats.seldepth = ply as u8;
        }

        // Check stopping conditions
        if self.should_stop() {
            return self.evaluator.evaluate(state, perspective);
        }

        // Terminal state check
        if !state.is_in_progress() {
            return match state.game_outcome {
                GameOutcome::Victory(winner) if winner == perspective => {
                    SCORE_WIN - ply as Score
                }
                GameOutcome::Victory(_) => SCORE_LOSS + ply as Score,
                GameOutcome::Draw => SCORE_DRAW,
                _ => self.evaluator.evaluate(state, perspective),
            };
        }

        // Tabling check
        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + ply as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - ply as Score;
        }

        // Stand-pat score: the static evaluation of the current position.
        // In quiescence, we assume the side to move can at least achieve
        // this score by not making any tactical move.
        let stand_pat = self.evaluator.evaluate(state, perspective);

        // Beta cutoff: position is already too good for the opponent
        if stand_pat >= beta {
            return beta;
        }

        // Improve alpha with stand-pat
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // If position is stable OR max quiescence depth reached, return static eval
        if !is_position_unstable(state) || qs_depth >= self.config.quiescence_max_depth {
            return stand_pat;
        }

        // Generate candidates in the unstable position
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return stand_pat;
        }

        // Order moves (use static ordering in quiescence for speed)
        let ordered = if self.config.use_move_ordering {
            self.orderer.order_moves(&candidates, ply, None)
        } else {
            static_order(&candidates)
        };

        // Search a limited number of candidates in quiescence
        let max_qs_candidates = ordered.len().min(self.config.max_candidates / 2).max(5);

        for ord_move in ordered.iter().take(max_qs_candidates) {
            let action = &actions[ord_move.index];

            // Apply action to cloned state
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            if !applied {
                continue;
            }

            // Determine perspective for child
            let child_perspective = child_state.decision_owner;
            let score = if child_perspective == perspective {
                self.quiescence(
                    &child_state,
                    alpha,
                    beta,
                    perspective,
                    ply + 1,
                    qs_depth + 1,
                )
            } else {
                -self.quiescence(
                    &child_state,
                    -beta,
                    -alpha,
                    child_perspective,
                    ply + 1,
                    qs_depth + 1,
                )
            };

            if score > alpha {
                alpha = score;
            }

            // Beta cutoff
            if alpha >= beta {
                self.stats.beta_cutoffs += 1;
                return beta;
            }

            // Check stopping conditions mid-search
            if self.should_stop() {
                return alpha;
            }
        }

        alpha
    }

    /// Enhanced negamax with PV tracking, quiescence, and selective extensions.
    ///
    /// This is the core search function for iterative deepening.
    /// Unlike the basic NegamaxSearch, this version:
    /// - Tracks the principal variation through pv_line
    /// - Calls quiescence search at leaf nodes instead of static eval
    /// - Applies selective depth extensions for critical moves
    /// - Checks time management for early termination
    ///
    /// Source: implementation_v3.md Section 11.6
    #[allow(clippy::too_many_arguments)]
    fn negamax_pv(
        &mut self,
        state: &GameState,
        depth: u8,
        mut alpha: Score,
        beta: Score,
        perspective: PlayerId,
        ply: usize,
        extensions_used: u8,
        pv_line: &mut PvLine,
    ) -> Score {
        self.stats.nodes_evaluated += 1;
        pv_line.clear();

        // Track selective depth
        if ply as u8 > self.stats.seldepth {
            self.stats.seldepth = ply as u8;
        }

        // Check stopping conditions
        if self.should_stop() {
            return self.evaluator.evaluate(state, perspective);
        }

        // Terminal state check
        if !state.is_in_progress() {
            return match state.game_outcome {
                GameOutcome::Victory(winner) if winner == perspective => {
                    SCORE_WIN - ply as Score
                }
                GameOutcome::Victory(_) => SCORE_LOSS + ply as Score,
                GameOutcome::Draw => SCORE_DRAW,
                _ => self.evaluator.evaluate(state, perspective),
            };
        }

        // Tabling check
        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + ply as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - ply as Score;
        }

        // Leaf node: use quiescence search or static evaluation
        if depth == 0 {
            self.stats.leaf_evaluations += 1;
            if self.config.use_quiescence {
                return self.quiescence(state, alpha, beta, perspective, ply, 0);
            } else {
                return self.evaluator.evaluate(state, perspective);
            }
        }

        // Track max nominal depth reached
        if ply as u8 > self.stats.max_depth_reached {
            self.stats.max_depth_reached = ply as u8;
        }

        // Transposition table probe
        let state_hash = state.compute_hash();
        let mut tt_best_move: Option<u16> = None;

        if self.config.use_tt {
            match self.tt.probe(state_hash, depth) {
                TTProbeResult::Hit(entry) => {
                    match entry.bound {
                        TTBound::Exact => {
                            self.stats.tt_cutoffs += 1;
                            // For PV nodes, we can use the TT score but not the PV
                            return entry.score;
                        }
                        TTBound::LowerBound => {
                            if entry.score >= beta {
                                self.stats.tt_cutoffs += 1;
                                return entry.score;
                            }
                            if entry.score > alpha {
                                alpha = entry.score;
                            }
                        }
                        TTBound::UpperBound => {
                            if entry.score <= alpha {
                                self.stats.tt_cutoffs += 1;
                                return entry.score;
                            }
                        }
                    }
                    tt_best_move = entry.best_move_index.into();
                    if tt_best_move == Some(u16::MAX) {
                        tt_best_move = None;
                    }
                    if tt_best_move.is_some() {
                        self.stats.tt_ordering_hits += 1;
                    }
                }
                TTProbeResult::ShallowHit(entry) => {
                    tt_best_move = if entry.has_best_move() {
                        Some(entry.best_move_index)
                    } else {
                        None
                    };
                    if tt_best_move.is_some() {
                        self.stats.tt_ordering_hits += 1;
                    }
                }
                TTProbeResult::Miss => {}
            }
        }

        // Generate candidates
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            self.stats.leaf_evaluations += 1;
            return self.evaluator.evaluate(state, perspective);
        }

        // PV seeding: check if the previous iteration's PV has a move at this ply.
        // If so, and it matches a candidate, use it as the TT best move for ordering.
        if tt_best_move.is_none() {
            if let Some(pv_move) = self.previous_pv.move_at_ply(ply) {
                for (i, action) in actions.iter().enumerate() {
                    if action.intent == pv_move.intent
                        && action.commands.len() == pv_move.commands.len()
                        && !action.commands.is_empty()
                        && !pv_move.commands.is_empty()
                        && std::mem::discriminant(&action.commands[0])
                            == std::mem::discriminant(&pv_move.commands[0])
                    {
                        tt_best_move = Some(i as u16);
                        break;
                    }
                }
            }
        }

        // Order moves
        let ordered = if self.config.use_move_ordering {
            self.orderer.order_moves(&candidates, ply, tt_best_move)
        } else {
            static_order(&candidates)
        };

        let max_to_search = ordered.len().min(self.config.max_candidates);

        let mut best_score = SCORE_LOSS + ply as Score;
        let mut best_move_index: u16 = u16::MAX;
        let original_alpha = alpha;
        let mut child_pv = PvLine::with_capacity(depth as usize);

        for (move_num, ord_move) in ordered.iter().take(max_to_search).enumerate() {
            let action = &actions[ord_move.index];

            // Apply action to cloned state
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            if !applied {
                continue;
            }

            // Compute selective extension for this move
            let extension = self.compute_extension(action, state, extensions_used);
            if extension > 0 {
                self.stats.extensions += 1;
            }
            let child_depth = depth - 1 + extension;
            let child_extensions = extensions_used + extension;

            // Determine who decides next in the child state
            let child_perspective = child_state.decision_owner;

            child_pv.clear();

            let score = if child_perspective == perspective {
                self.negamax_pv(
                    &child_state,
                    child_depth,
                    alpha,
                    beta,
                    perspective,
                    ply + 1,
                    child_extensions,
                    &mut child_pv,
                )
            } else {
                -self.negamax_pv(
                    &child_state,
                    child_depth,
                    -beta,
                    -alpha,
                    child_perspective,
                    ply + 1,
                    child_extensions,
                    &mut child_pv,
                )
            };

            if score > best_score {
                best_score = score;
                best_move_index = ord_move.index as u16;

                if score > alpha {
                    alpha = score;
                    // Update PV: this move + child PV
                    pv_line.update_from(action, &child_pv);
                }
            }

            // Alpha-beta cutoff
            if self.config.use_alpha_beta && alpha >= beta {
                self.stats.beta_cutoffs += 1;

                if self.config.use_move_ordering {
                    self.orderer.record_cutoff(ply, action);
                    for prev in ordered.iter().take(move_num) {
                        self.orderer.record_searched(&actions[prev.index], ply);
                    }
                }

                break;
            }

            // Check stopping conditions mid-search
            if self.should_stop() {
                break;
            }
        }

        // Store in transposition table
        if self.config.use_tt {
            let bound = if best_score <= original_alpha {
                TTBound::UpperBound
            } else if best_score >= beta {
                TTBound::LowerBound
            } else {
                TTBound::Exact
            };

            self.tt.store(
                state_hash,
                best_score,
                depth,
                bound,
                best_move_index,
            );
        }

        best_score
    }

    /// Run a single iteration of the search at the given depth.
    /// Returns the score and updates the PV.
    ///
    /// If aspiration windows are enabled, searches with a narrow window
    /// first and widens on fail-high/fail-low.
    fn search_iteration(
        &mut self,
        state: &GameState,
        depth: u8,
        perspective: PlayerId,
        candidates: &wh40k_search_abstraction::CandidateSet,
        previous_score: Score,
        root_pv: &mut PvLine,
    ) -> Score {
        let actions = &candidates.candidates;

        // Aspiration window setup
        let (mut alpha, mut beta) = if self.config.use_aspiration_windows && depth > 1 {
            let delta = self.config.aspiration_window_initial;
            (
                (previous_score - delta).max(SCORE_LOSS),
                (previous_score + delta).min(SCORE_WIN),
            )
        } else {
            (SCORE_LOSS, SCORE_WIN)
        };

        let mut aspiration_delta = self.config.aspiration_window_delta;

        loop {
            // Order root moves: use heuristic ordering + PV seeding
            let pv_move_index: Option<u16> = if !self.previous_pv.is_empty() {
                // Find the previous PV root move in the current candidates
                let pv_root = &self.previous_pv.moves[0];
                actions.iter().enumerate().find_map(|(i, action)| {
                    if action.intent == pv_root.intent
                        && action.commands.len() == pv_root.commands.len()
                        && !action.commands.is_empty()
                        && !pv_root.commands.is_empty()
                        && std::mem::discriminant(&action.commands[0])
                            == std::mem::discriminant(&pv_root.commands[0])
                    {
                        Some(i as u16)
                    } else {
                        None
                    }
                })
            } else {
                None
            };

            let ordered = if self.config.heuristic_root_ordering && depth <= 2 {
                // Use heuristic ordering for shallow depths (more expensive but more accurate)
                heuristic_order(candidates, state, perspective, &self.evaluator)
            } else if self.config.use_move_ordering {
                // Use TT/killer/history ordering for deeper searches (faster)
                self.orderer.order_moves(candidates, 0, pv_move_index)
            } else {
                static_order(candidates)
            };

            let max_to_search = ordered.len().min(self.config.max_candidates);

            let mut best_score = SCORE_LOSS;
            let mut best_idx = 0;
            let mut best_pv = PvLine::new();
            let mut child_pv = PvLine::with_capacity(depth as usize);
            let mut local_alpha = alpha;

            for ord_move in ordered.iter().take(max_to_search) {
                let action = &actions[ord_move.index];

                let mut child_state = state.clone();
                let mut applied = true;

                for cmd in &action.commands {
                    if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                        applied = false;
                        break;
                    }
                }

                let score = if applied {
                    let child_perspective = child_state.decision_owner;

                    // Compute root extension
                    let extension = self.compute_extension(action, state, 0);
                    if extension > 0 {
                        self.stats.extensions += 1;
                    }
                    let child_depth = depth - 1 + extension;

                    child_pv.clear();

                    if child_perspective == perspective {
                        self.negamax_pv(
                            &child_state,
                            child_depth,
                            local_alpha,
                            beta,
                            perspective,
                            1,
                            extension,
                            &mut child_pv,
                        )
                    } else {
                        -self.negamax_pv(
                            &child_state,
                            child_depth,
                            -beta,
                            -local_alpha,
                            child_perspective,
                            1,
                            extension,
                            &mut child_pv,
                        )
                    }
                } else {
                    SCORE_LOSS + 1
                };

                if score > best_score {
                    best_score = score;
                    best_idx = ord_move.index;

                    if score > local_alpha {
                        local_alpha = score;
                        // Update root PV
                        best_pv.update_from(action, &child_pv);
                    }
                }

                if self.config.use_alpha_beta && local_alpha >= beta {
                    self.stats.beta_cutoffs += 1;
                    break;
                }

                if self.should_stop() {
                    break;
                }
            }

            // Check aspiration window results
            if self.config.use_aspiration_windows && depth > 1 {
                if best_score <= alpha {
                    // Fail-low: widen alpha
                    alpha = (alpha - aspiration_delta).max(SCORE_LOSS);
                    aspiration_delta *= 2;
                    self.stats.aspiration_resets += 1;

                    // Don't use this result's PV
                    if self.should_stop() {
                        // If we must stop, use whatever we have
                        *root_pv = best_pv;
                        return best_score;
                    }
                    continue; // Re-search with wider window
                }

                if best_score >= beta {
                    // Fail-high: widen beta
                    beta = (beta + aspiration_delta).min(SCORE_WIN);
                    aspiration_delta *= 2;
                    self.stats.aspiration_resets += 1;

                    // The best_pv from fail-high is still useful as a lower bound
                    *root_pv = best_pv;

                    if self.should_stop() {
                        return best_score;
                    }
                    continue; // Re-search with wider window
                }
            }

            // Score is within the aspiration window (or no aspiration)
            let _ = best_idx; // Used via ordered[].index for action lookup
            *root_pv = best_pv;
            return best_score;
        }
    }
}

impl Default for IterativeDeepeningSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl AiWorker for IterativeDeepeningSearch {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        // Reset stats and time manager
        self.stats = SearchStats {
            depth_requested: self.config.max_depth,
            ..Default::default()
        };
        self.time_manager = TimeManager::new(&self.config);
        self.time_manager.reset();
        self.iteration_infos.clear();

        // New TT generation
        if self.config.use_tt {
            self.tt.new_generation();
        }

        // Clear killers for new search
        if self.config.use_move_ordering {
            self.orderer.clear_killers();
        }

        // Generate root candidates
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return None;
        }

        self.stats.root_candidates = actions.len();

        // Adjust time based on position characteristics
        self.time_manager.adjust_for_position(
            state.current_phase,
            false, // No previous PV to compare yet
            0,     // No score gap yet
            actions.len(),
        );

        // === Iterative Deepening Loop ===
        let mut best_result: Option<SearchResult> = None;
        let mut previous_score: Score = 0;
        let mut previous_root_intent: Option<TacticalIntent> = None;
        let mut root_pv = PvLine::new();

        let start_depth: u8 = 1;
        let max_depth = self.config.max_depth;

        for depth in start_depth..=max_depth {
            // Check if we should start a new iteration
            if depth > start_depth && self.time_manager.soft_stop() {
                break;
            }

            // Check hard stop
            if self.should_stop() {
                break;
            }

            let iter_start_nodes = self.stats.nodes_evaluated;
            root_pv.clear();

            // Run search at this depth
            let score = self.search_iteration(
                state,
                depth,
                perspective,
                &candidates,
                previous_score,
                &mut root_pv,
            );

            // Check if search was aborted mid-iteration
            if self.should_stop() && root_pv.is_empty() {
                // Iteration was incomplete; use previous result
                break;
            }

            // Record PV change detection
            let pv_changed = if let Some(first_pv_move) = root_pv.moves.first() {
                match previous_root_intent {
                    Some(prev_intent) => prev_intent != first_pv_move.intent,
                    None => true,
                }
            } else {
                false
            };

            if pv_changed {
                self.stats.pv_changes += 1;
            }

            // Record iteration info
            let elapsed = self.time_manager.elapsed_ms();
            let nodes_this_iter = self.stats.nodes_evaluated - iter_start_nodes;
            let iter_info = IterationInfo {
                depth,
                score,
                pv: root_pv.clone(),
                nodes: nodes_this_iter,
                time_ms: elapsed,
                nps: self.time_manager.nps(self.stats.nodes_evaluated),
                aspiration_resets: self.stats.aspiration_resets,
                pv_changed,
            };
            self.iteration_infos.push(iter_info);
            self.stats.iterations_completed = depth;

            // Update previous PV for next iteration's seeding
            self.previous_pv = root_pv.clone();
            previous_score = score;
            previous_root_intent = root_pv.moves.first().map(|m| m.intent);

            // Adjust time based on PV stability
            if depth > 1 {
                let score_gap = if let Some(ref prev_result) = best_result {
                    (score - prev_result.score).unsigned_abs() as Score
                } else {
                    0
                };

                self.time_manager.adjust_for_position(
                    state.current_phase,
                    !pv_changed,
                    score_gap,
                    actions.len(),
                );
            }

            // Build candidate scores for the result
            // We use the root ordering to get all candidate evaluations
            let mut candidate_scores: Vec<(MacroAction, Score)> = Vec::new();
            for action in actions.iter() {
                let mut child_state = state.clone();
                let mut applied = true;
                for cmd in &action.commands {
                    if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                        applied = false;
                        break;
                    }
                }
                let action_score = if applied {
                    self.evaluator.evaluate(&child_state, perspective)
                } else {
                    SCORE_LOSS + 1
                };
                candidate_scores.push((action.clone(), action_score));
            }

            // Determine best action from PV
            let best_action = if !root_pv.is_empty() {
                root_pv.moves[0].clone()
            } else {
                // Fallback: use heuristic ordering
                let ordered = heuristic_order(&candidates, state, perspective, &self.evaluator);
                if !ordered.is_empty() {
                    actions[ordered[0].index].clone()
                } else {
                    actions[0].clone()
                }
            };

            // Update final stats
            self.stats.time_elapsed_ms = elapsed;
            self.stats.nps = self.time_manager.nps(self.stats.nodes_evaluated);

            best_result = Some(SearchResult {
                best_action,
                score,
                stats: self.stats.clone(),
                pv: root_pv.moves.clone(),
                candidate_scores,
            });

            // Check for early termination: if we found a winning score
            if score >= SCORE_WIN - 100 || score <= SCORE_LOSS + 100 {
                break;
            }
        }

        // Finalize timing stats
        self.stats.time_elapsed_ms = self.time_manager.elapsed_ms();
        self.stats.nps = self.time_manager.nps(self.stats.nodes_evaluated);

        // Update stats in the result
        if let Some(ref mut result) = best_result {
            result.stats = self.stats.clone();
        }

        best_result
    }

    fn name(&self) -> &str {
        "IterativeDeepeningSearch"
    }
}

// ============================================================================
// Phase 6: Lazy SMP Preparation
// ============================================================================

/// Shared state for lazy SMP (Symmetric Multi-Processing) search.
///
/// In lazy SMP, multiple threads run independent iterative deepening
/// searches that share a single transposition table. Each thread searches
/// with slightly different parameters (e.g., different depths or move
/// orderings) so they explore different parts of the tree. Results are
/// shared through the TT, providing effective parallelism without
/// complex synchronization.
///
/// Source: implementation_v3.md Section 11 (search worker threads)
///
/// Note: The TranspositionTable currently uses non-atomic operations.
/// For true multi-threaded use, it would need to be made lock-free
/// (e.g., using atomic u64 pairs with XOR verification, as in Stockfish).
/// This struct provides the architectural framework; actual threading
/// is deferred to a future optimization pass.
pub struct SharedSearchState {
    /// Atomic flag to signal all worker threads to stop.
    /// Set to true when the best thread has found a result or time is up.
    pub stop_flag: Arc<AtomicBool>,
    /// The number of worker threads to use.
    pub thread_count: usize,
}

impl SharedSearchState {
    /// Create a new shared search state.
    pub fn new(thread_count: usize) -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_count: thread_count.max(1),
        }
    }

    /// Signal all threads to stop.
    pub fn signal_stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    /// Reset the stop flag for a new search.
    pub fn reset_stop(&self) {
        self.stop_flag.store(false, Ordering::SeqCst);
    }

    /// Check if the stop signal has been set.
    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }
}

/// Lazy SMP search coordinator.
///
/// Manages multiple `IterativeDeepeningSearch` workers that share a
/// transposition table. Each worker searches independently with the
/// shared TT providing implicit communication.
///
/// ## Design (from Stockfish's approach)
///
/// 1. Main thread starts iterative deepening at depth 1
/// 2. Helper threads start at depth 1 + thread_offset
/// 3. All threads share the TT (lock-free reads/writes)
/// 4. When the main thread's time is up, all threads stop
/// 5. Best result comes from the main thread (deepest completed iteration)
///
/// ## Current Status
///
/// This struct provides the architectural skeleton for lazy SMP.
/// Single-threaded search works fully; multi-threaded execution is
/// prepared but requires making the TT thread-safe (lock-free atomic
/// entries) before enabling.
pub struct LazySmpSearch {
    /// Search configuration (retained for spawning helper thread workers).
    pub config: SearchConfig,
    /// Shared state across all worker threads.
    shared: SharedSearchState,
    /// The main worker (thread 0).
    main_worker: IterativeDeepeningSearch,
}

impl LazySmpSearch {
    /// Create a new lazy SMP search coordinator.
    pub fn new(config: SearchConfig) -> Self {
        let thread_count = config.smp_threads.max(1);
        let shared = SharedSearchState::new(thread_count);

        let mut main_worker = IterativeDeepeningSearch::with_config(config.clone());
        main_worker.set_stop_signal(shared.stop_flag.clone());

        Self {
            config,
            shared,
            main_worker,
        }
    }

    /// Create a lazy SMP search with custom weights.
    pub fn with_weights(weights: HeuristicWeights, config: SearchConfig) -> Self {
        let thread_count = config.smp_threads.max(1);
        let shared = SharedSearchState::new(thread_count);

        let mut main_worker = IterativeDeepeningSearch::with_weights_and_config(
            weights,
            config.clone(),
        );
        main_worker.set_stop_signal(shared.stop_flag.clone());

        Self {
            config,
            shared,
            main_worker,
        }
    }

    /// Get the shared search state (for external stop control).
    pub fn shared_state(&self) -> &SharedSearchState {
        &self.shared
    }

    /// Get the transposition table statistics.
    pub fn tt_stats(&self) -> wh40k_transposition::TTStats {
        self.main_worker.tt_stats()
    }

    /// Clear all search state (call between games).
    pub fn clear_all(&mut self) {
        self.main_worker.clear_all();
    }

    /// Get diagnostics from the main worker's last search.
    pub fn diagnostics(&self, state: &GameState) -> SearchDiagnostics {
        self.main_worker.diagnostics(state)
    }
}

impl AiWorker for LazySmpSearch {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        // Reset stop flag
        self.shared.reset_stop();

        // Currently single-threaded: just run the main worker.
        // When TT is made thread-safe, helper threads would be spawned here,
        // each running their own IterativeDeepeningSearch with shared TT
        // and the shared stop flag.
        //
        // Pseudo-code for future multi-threaded version:
        // ```
        // let handles: Vec<JoinHandle<Option<SearchResult>>> = (1..thread_count)
        //     .map(|i| {
        //         let mut worker = IterativeDeepeningSearch::with_config(config.clone());
        //         worker.set_stop_signal(shared.stop_flag.clone());
        //         // Worker i starts at depth offset to diversify search
        //         std::thread::spawn(move || worker.choose_action(state, perspective))
        //     })
        //     .collect();
        //
        // let main_result = self.main_worker.choose_action(state, perspective);
        // self.shared.signal_stop(); // Tell helpers to stop
        //
        // for handle in handles { handle.join().ok(); }
        // main_result
        // ```

        let result = self.main_worker.choose_action(state, perspective);

        // Signal stop (no-op in single-threaded mode, but correct for API)
        self.shared.signal_stop();

        result
    }

    fn name(&self) -> &str {
        "LazySmpSearch"
    }
}

// ============================================================================
// Search Root (Orchestrator)
// ============================================================================

/// Orchestrates AI search from the current game position.
/// Provides a unified interface for running any AI level.
///
/// # Usage
///
/// ```ignore
/// let mut root = SearchRoot::new(AiLevel::Negamax);
/// if let Some(result) = root.search(&state, player) {
///     for cmd in result.best_commands() {
///         CommandExecutor::execute(&mut state, cmd)?;
///     }
/// }
/// ```
pub struct SearchRoot {
    /// The AI worker implementation.
    worker: Box<dyn AiWorker>,
}

/// AI strength level selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiLevel {
    /// Greedy: depth 0, fastest.
    Greedy,
    /// One-ply: depth 1, moderate.
    OnePly,
    /// Negamax: configurable depth (default 3), strong.
    Negamax,
    /// Negamax with custom depth.
    NegamaxDepth(u8),
    /// Iterative deepening: full Stockfish-like search, strongest.
    /// Uses ID, aspiration windows, PV tracking, quiescence, extensions.
    IterativeDeepening,
    /// Iterative deepening with custom max depth.
    IterativeDeepeningDepth(u8),
    /// Time-limited iterative deepening search.
    /// Searches until the time budget (in ms) is exhausted.
    IterativeDeepeningTimed(u64),
    /// Lazy SMP search (single-threaded for now, multi-thread ready).
    LazySmp,
}

impl SearchRoot {
    /// Create a new search root with the specified AI level.
    pub fn new(level: AiLevel) -> Self {
        let worker: Box<dyn AiWorker> = match level {
            AiLevel::Greedy => Box::new(GreedyAi::new()),
            AiLevel::OnePly => Box::new(OnePlySearch::new()),
            AiLevel::Negamax => Box::new(NegamaxSearch::new()),
            AiLevel::NegamaxDepth(depth) => {
                Box::new(NegamaxSearch::with_config(SearchConfig::negamax(depth)))
            }
            AiLevel::IterativeDeepening => {
                Box::new(IterativeDeepeningSearch::new())
            }
            AiLevel::IterativeDeepeningDepth(depth) => {
                Box::new(IterativeDeepeningSearch::with_config(
                    SearchConfig::iterative_deepening(depth),
                ))
            }
            AiLevel::IterativeDeepeningTimed(time_ms) => {
                Box::new(IterativeDeepeningSearch::with_config(
                    SearchConfig::iterative_deepening_timed(time_ms),
                ))
            }
            AiLevel::LazySmp => {
                Box::new(LazySmpSearch::new(SearchConfig::iterative_deepening(6)))
            }
        };

        Self { worker }
    }

    /// Create a search root with a custom AI worker.
    pub fn with_worker(worker: Box<dyn AiWorker>) -> Self {
        Self { worker }
    }

    /// Run the search and return the best action.
    pub fn search(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        self.worker.choose_action(state, perspective)
    }

    /// Get the name of the current AI worker.
    pub fn worker_name(&self) -> &str {
        self.worker.name()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Quick greedy decision: pick the best immediate action.
/// Fastest AI call, suitable for testing and low-priority decisions.
pub fn greedy_choose(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut ai = GreedyAi::new();
    ai.choose_action(state, perspective)
}

/// One-ply decision: evaluate all candidates after applying them.
/// Moderate speed and strength.
pub fn one_ply_choose(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut ai = OnePlySearch::new();
    ai.choose_action(state, perspective)
}

/// Negamax decision at the given depth.
/// Strong AI, slower execution.
pub fn negamax_choose(
    state: &GameState,
    perspective: PlayerId,
    depth: u8,
) -> Option<SearchResult> {
    let mut ai = NegamaxSearch::with_config(SearchConfig::negamax(depth));
    ai.choose_action(state, perspective)
}

/// Iterative deepening decision at the given max depth.
/// Strongest AI: full ID with aspiration windows, quiescence, extensions.
pub fn id_search_choose(
    state: &GameState,
    perspective: PlayerId,
    max_depth: u8,
) -> Option<SearchResult> {
    let mut ai = IterativeDeepeningSearch::with_config(
        SearchConfig::iterative_deepening(max_depth),
    );
    ai.choose_action(state, perspective)
}

/// Time-limited iterative deepening decision.
/// Searches until the time budget (in ms) is exhausted, returning
/// the best result from the last completed iteration.
pub fn id_search_timed(
    state: &GameState,
    perspective: PlayerId,
    time_ms: u64,
) -> Option<SearchResult> {
    let mut ai = IterativeDeepeningSearch::with_config(
        SearchConfig::iterative_deepening_timed(time_ms),
    );
    ai.choose_action(state, perspective)
}

// ============================================================================
// NNUE Integration
// ============================================================================

/// Greedy AI using NNUE evaluator instead of heuristic.
pub struct GreedyAiNnue {
    evaluator: NnueEvaluator,
}

impl GreedyAiNnue {
    /// Create with a bootstrap NNUE model.
    pub fn bootstrap() -> Self {
        Self {
            evaluator: NnueEvaluator::bootstrap(),
        }
    }

    /// Create with a specific NNUE model.
    pub fn with_model(model: NnueModel) -> Self {
        Self {
            evaluator: NnueEvaluator::new(model),
        }
    }

    /// Create from a model artifact.
    pub fn from_artifact(artifact: &NnueModelArtifact) -> Result<Self, wh40k_eval_nnue::NnueError> {
        Ok(Self {
            evaluator: NnueEvaluator::from_artifact(artifact)?,
        })
    }
}

impl AiWorker for GreedyAiNnue {
    fn choose_action(
        &mut self,
        state: &GameState,
        perspective: PlayerId,
    ) -> Option<SearchResult> {
        let candidates = ActionGenerator::generate(state, perspective);
        let actions = &candidates.candidates;

        if actions.is_empty() {
            return None;
        }

        let mut stats = SearchStats {
            depth_requested: 0,
            root_candidates: actions.len(),
            ..Default::default()
        };

        let mut best_score = SCORE_LOSS;
        let mut best_idx = 0;
        let mut candidate_scores: Vec<(MacroAction, Score)> = Vec::with_capacity(actions.len());

        for (i, action) in actions.iter().enumerate() {
            let mut child_state = state.clone();
            let mut applied = true;

            for cmd in &action.commands {
                if CommandExecutor::execute(&mut child_state, cmd).is_err() {
                    applied = false;
                    break;
                }
            }

            let score = if applied {
                self.evaluator.evaluate_position(&child_state, perspective)
            } else {
                SCORE_LOSS + 1
            };

            stats.nodes_evaluated += 1;
            stats.leaf_evaluations += 1;

            candidate_scores.push((action.clone(), score));

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        let best_action = actions[best_idx].clone();

        Some(SearchResult {
            best_action: best_action.clone(),
            score: best_score,
            stats,
            pv: vec![best_action],
            candidate_scores,
        })
    }

    fn name(&self) -> &str {
        "GreedyAiNnue"
    }
}

/// Choose an action using NNUE greedy evaluation.
pub fn greedy_choose_nnue(state: &GameState, perspective: PlayerId) -> Option<SearchResult> {
    let mut ai = GreedyAiNnue::bootstrap();
    ai.choose_action(state, perspective)
}

/// Choose an action using NNUE greedy evaluation with a specific model.
pub fn greedy_choose_nnue_model(
    state: &GameState,
    perspective: PlayerId,
    model: NnueModel,
) -> Option<SearchResult> {
    let mut ai = GreedyAiNnue::with_model(model);
    ai.choose_action(state, perspective)
}

/// Run an evaluation benchmark comparing heuristic vs NNUE on a position.
/// Evaluates `count` times and returns benchmark results for both evaluators.
pub fn benchmark_heuristic_vs_nnue(
    state: &GameState,
    perspective: PlayerId,
    count: usize,
) -> (BenchmarkResult, BenchmarkResult) {
    let heuristic = HeuristicEvaluator::with_defaults();
    let nnue = NnueEvaluator::bootstrap();
    compare_evaluators(&heuristic, &nnue, state, perspective, count)
}

/// Re-export NNUE types for convenient access from search_core consumers.
pub use wh40k_eval_nnue::{
    AnyEvaluator as EvalAnyEvaluator,
    NnueEvaluator as EvalNnueEvaluator,
    NnueModel as EvalNnueModel,
    NnueModelArtifact as EvalNnueModelArtifact,
    ModelRegistry as EvalModelRegistry,
};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_config_defaults() {
        let config = SearchConfig::default();
        assert_eq!(config.max_depth, 3);
        assert!(config.use_alpha_beta);
        assert!(config.use_tt);
        assert!(config.use_move_ordering);
    }

    #[test]
    fn test_search_config_greedy() {
        let config = SearchConfig::greedy();
        assert_eq!(config.max_depth, 0);
        assert!(!config.use_alpha_beta);
        assert!(!config.use_tt);
    }

    #[test]
    fn test_search_config_one_ply() {
        let config = SearchConfig::one_ply();
        assert_eq!(config.max_depth, 1);
        assert!(config.use_alpha_beta);
    }

    #[test]
    fn test_search_config_negamax() {
        let config = SearchConfig::negamax(3);
        assert_eq!(config.max_depth, 3);
        assert!(config.use_alpha_beta);
        assert!(config.use_tt);
        assert!(config.use_move_ordering);
    }

    #[test]
    fn test_search_stats_display() {
        let stats = SearchStats {
            nodes_evaluated: 1500,
            leaf_evaluations: 800,
            beta_cutoffs: 200,
            tt_cutoffs: 50,
            tt_ordering_hits: 30,
            max_depth_reached: 3,
            root_candidates: 25,
            depth_requested: 3,
            chance_samples: 0,
            ..Default::default()
        };

        let display = format!("{}", stats);
        assert!(display.contains("nodes=1500"));
        assert!(display.contains("depth=3/3"));
        assert!(display.contains("cutoffs=200"));
    }

    #[test]
    fn test_search_result_display() {
        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Test action".to_string(),
            commands: vec![],
            intent: TacticalIntent::HoldObjective,
            actor_units: vec![],
            priority_hint: 50,
        };

        let result = SearchResult {
            best_action: action.clone(),
            score: 250,
            stats: SearchStats::default(),
            pv: vec![action],
            candidate_scores: vec![],
        };

        let display = format!("{}", result);
        assert!(display.contains("Test action"));
        assert!(display.contains("+250"));
    }

    #[test]
    fn test_greedy_ai_creation() {
        let ai = GreedyAi::new();
        assert_eq!(ai.name(), "GreedyAi");
    }

    #[test]
    fn test_one_ply_creation() {
        let ai = OnePlySearch::new();
        assert_eq!(ai.name(), "OnePlySearch");
    }

    #[test]
    fn test_negamax_creation() {
        let ai = NegamaxSearch::new();
        assert_eq!(ai.name(), "NegamaxSearch");
    }

    #[test]
    fn test_negamax_custom_config() {
        let config = SearchConfig {
            max_depth: 5,
            max_candidates: 20,
            tt_size_mb: 32,
            ..SearchConfig::default()
        };
        let ai = NegamaxSearch::with_config(config);
        assert_eq!(ai.config.max_depth, 5);
        assert_eq!(ai.config.max_candidates, 20);
    }

    #[test]
    fn test_search_root_creation() {
        let root = SearchRoot::new(AiLevel::Greedy);
        assert_eq!(root.worker_name(), "GreedyAi");

        let root = SearchRoot::new(AiLevel::OnePly);
        assert_eq!(root.worker_name(), "OnePlySearch");

        let root = SearchRoot::new(AiLevel::Negamax);
        assert_eq!(root.worker_name(), "NegamaxSearch");
    }

    #[test]
    fn test_ai_level_custom_depth() {
        let root = SearchRoot::new(AiLevel::NegamaxDepth(5));
        assert_eq!(root.worker_name(), "NegamaxSearch");
    }

    #[test]
    fn test_negamax_tt_stats() {
        let ai = NegamaxSearch::new();
        let stats = ai.tt_stats();
        assert!(stats.capacity > 0);
        assert_eq!(stats.hit_count, 0);
    }

    #[test]
    fn test_negamax_clear_tt() {
        let mut ai = NegamaxSearch::new();
        ai.clear_tt();
        let stats = ai.tt_stats();
        assert_eq!(stats.stored_count, 0);
    }

    #[test]
    fn test_search_result_best_commands() {
        let cmd = Command::RemainStationary {
            unit_id: wh40k_core_types::UnitId::new(1),
        };
        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Stay".to_string(),
            commands: vec![cmd.clone()],
            intent: TacticalIntent::HoldPosition,
            actor_units: vec![wh40k_core_types::UnitId::new(1)],
            priority_hint: 20,
        };

        let result = SearchResult {
            best_action: action.clone(),
            score: 100,
            stats: SearchStats::default(),
            pv: vec![action],
            candidate_scores: vec![],
        };

        assert_eq!(result.best_commands().len(), 1);
        assert_eq!(result.best_intent(), TacticalIntent::HoldPosition);
    }

    #[test]
    fn test_node_budget() {
        let config = SearchConfig {
            max_depth: 10,
            node_budget: 100,
            ..SearchConfig::default()
        };
        let ai = NegamaxSearch::with_config(config);
        assert_eq!(ai.config.node_budget, 100);
    }

    // ========================================================================
    // Phase 6: Iterative Deepening Tests
    // ========================================================================

    #[test]
    fn test_search_config_iterative_deepening() {
        let config = SearchConfig::iterative_deepening(6);
        assert_eq!(config.max_depth, 6);
        assert!(config.use_iterative_deepening);
        assert!(config.use_aspiration_windows);
        assert!(config.use_quiescence);
        assert!(config.use_extensions);
        assert!(config.use_alpha_beta);
        assert!(config.use_tt);
        assert!(config.use_move_ordering);
        assert_eq!(config.quiescence_max_depth, 4);
        assert_eq!(config.max_extensions, 4);
        assert_eq!(config.aspiration_window_initial, 50);
        assert_eq!(config.time_budget_ms, 0); // depth-limited, not time-limited
    }

    #[test]
    fn test_search_config_iterative_deepening_timed() {
        let config = SearchConfig::iterative_deepening_timed(5000);
        assert_eq!(config.time_budget_ms, 5000);
        assert_eq!(config.max_depth, 64); // Effectively unlimited
        assert!(config.use_iterative_deepening);
        assert!(config.use_aspiration_windows);
        assert!(config.use_quiescence);
        assert!(config.use_extensions);
        assert_eq!(config.soft_time_factor, 600); // 60%
    }

    #[test]
    fn test_search_config_defaults_preserve_behavior() {
        // Verify that default config does NOT enable Phase 6 features
        // This ensures existing code behavior is unchanged
        let config = SearchConfig::default();
        assert!(!config.use_iterative_deepening);
        assert!(!config.use_aspiration_windows);
        assert!(!config.use_quiescence);
        assert!(!config.use_extensions);
        assert_eq!(config.time_budget_ms, 0);
        assert_eq!(config.smp_threads, 1);
    }

    #[test]
    fn test_search_stats_phase6_defaults() {
        let stats = SearchStats::default();
        assert_eq!(stats.iterations_completed, 0);
        assert_eq!(stats.aspiration_resets, 0);
        assert_eq!(stats.quiescence_nodes, 0);
        assert_eq!(stats.extensions, 0);
        assert_eq!(stats.time_elapsed_ms, 0);
        assert_eq!(stats.nps, 0);
        assert_eq!(stats.pv_changes, 0);
        assert_eq!(stats.seldepth, 0);
    }

    #[test]
    fn test_search_stats_phase6_display() {
        let stats = SearchStats {
            nodes_evaluated: 5000,
            leaf_evaluations: 2000,
            beta_cutoffs: 500,
            tt_cutoffs: 100,
            tt_ordering_hits: 80,
            max_depth_reached: 4,
            root_candidates: 15,
            depth_requested: 6,
            chance_samples: 0,
            iterations_completed: 4,
            aspiration_resets: 2,
            quiescence_nodes: 300,
            extensions: 50,
            time_elapsed_ms: 1234,
            nps: 4052,
            pv_changes: 1,
            seldepth: 7,
        };
        let display = format!("{}", stats);
        assert!(display.contains("seldepth=7"));
        assert!(display.contains("iter=4"));
        assert!(display.contains("qs_nodes=300"));
        assert!(display.contains("ext=50"));
        assert!(display.contains("time=1234ms"));
        assert!(display.contains("nps=4052"));
        assert!(display.contains("asp_resets=2"));
        assert!(display.contains("pv_chg=1"));
    }

    // ========================================================================
    // Phase 6: PvLine Tests
    // ========================================================================

    #[test]
    fn test_pv_line_empty() {
        let pv = PvLine::new();
        assert!(pv.is_empty());
        assert_eq!(pv.len(), 0);
        assert!(pv.move_at_ply(0).is_none());
    }

    #[test]
    fn test_pv_line_update_from() {
        let action1 = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Move A".to_string(),
            commands: vec![],
            intent: TacticalIntent::HoldObjective,
            actor_units: vec![],
            priority_hint: 50,
        };
        let action2 = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(1),
            label: "Shoot B".to_string(),
            commands: vec![],
            intent: TacticalIntent::MaximizeKills,
            actor_units: vec![],
            priority_hint: 40,
        };

        let mut child_pv = PvLine::new();
        child_pv.moves.push(action2.clone());

        let mut pv = PvLine::new();
        pv.update_from(&action1, &child_pv);

        assert_eq!(pv.len(), 2);
        assert_eq!(pv.move_at_ply(0).unwrap().label, "Move A");
        assert_eq!(pv.move_at_ply(1).unwrap().label, "Shoot B");
        assert!(pv.move_at_ply(2).is_none());
    }

    #[test]
    fn test_pv_line_display() {
        let action1 = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Move A".to_string(),
            commands: vec![],
            intent: TacticalIntent::HoldObjective,
            actor_units: vec![],
            priority_hint: 0,
        };
        let action2 = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(1),
            label: "Shoot B".to_string(),
            commands: vec![],
            intent: TacticalIntent::MaximizeKills,
            actor_units: vec![],
            priority_hint: 0,
        };

        let mut pv = PvLine::new();
        pv.moves.push(action1);
        pv.moves.push(action2);

        let display = format!("{}", pv);
        assert_eq!(display, "Move A -> Shoot B");

        let empty_pv = PvLine::new();
        assert_eq!(format!("{}", empty_pv), "(empty PV)");
    }

    // ========================================================================
    // Phase 6: TimeManager Tests
    // ========================================================================

    #[test]
    fn test_time_manager_unlimited() {
        let tm = TimeManager::unlimited();
        assert!(!tm.time_based);
        assert!(!tm.hard_stop(0));
        assert!(!tm.soft_stop());
    }

    #[test]
    fn test_time_manager_with_time() {
        let tm = TimeManager::with_time(10000);
        assert!(tm.time_based);
        assert_eq!(tm.hard_limit_ms, 10000);
        assert_eq!(tm.soft_limit_ms, 6000); // 60% of 10000
        assert!(!tm.hard_stop(0)); // Not expired yet
    }

    #[test]
    fn test_time_manager_node_budget() {
        let tm = TimeManager {
            start_time: Instant::now(),
            hard_limit_ms: 0,
            soft_limit_ms: 0,
            node_budget: 100,
            time_based: false,
        };
        assert!(!tm.hard_stop(50));
        assert!(tm.hard_stop(100));
        assert!(tm.hard_stop(200));
    }

    #[test]
    fn test_time_manager_nps() {
        let tm = TimeManager::unlimited();
        // With 0ms elapsed, nps extrapolates
        let nps = tm.nps(1000);
        assert!(nps > 0);
    }

    #[test]
    fn test_time_manager_adjust_for_position() {
        let config = SearchConfig::iterative_deepening_timed(10000);
        let mut tm = TimeManager::new(&config);

        // Adjust for a fight phase with unstable PV
        tm.adjust_for_position(Phase::Fight, false, 50, 15);
        // Fight phase should increase soft limit
        assert!(tm.soft_limit_ms > 0);

        // Adjust for a command phase with stable PV and big score gap
        let mut tm2 = TimeManager::new(&config);
        tm2.adjust_for_position(Phase::Command, true, 500, 3);
        // Command phase with stability should decrease soft limit
        assert!(tm2.soft_limit_ms <= tm.soft_limit_ms);
    }

    // ========================================================================
    // Phase 6: Instability Detection Tests
    // ========================================================================

    #[test]
    fn test_position_volatility_phases() {
        // We can't easily create full GameStates, but we can test the
        // volatility function's structure by checking it doesn't panic
        // with edge cases
        let v = position_volatility as fn(&GameState) -> u8;
        let _ = v; // Just verify the function exists with correct signature
    }

    // ========================================================================
    // Phase 6: IterativeDeepeningSearch Tests
    // ========================================================================

    #[test]
    fn test_id_search_creation() {
        let ai = IterativeDeepeningSearch::new();
        assert_eq!(ai.name(), "IterativeDeepeningSearch");
        assert!(ai.config.use_iterative_deepening);
        assert!(ai.config.use_aspiration_windows);
        assert!(ai.config.use_quiescence);
        assert!(ai.config.use_extensions);
    }

    #[test]
    fn test_id_search_custom_config() {
        let config = SearchConfig::iterative_deepening(4);
        let ai = IterativeDeepeningSearch::with_config(config);
        assert_eq!(ai.config.max_depth, 4);
        assert!(ai.config.use_iterative_deepening);
    }

    #[test]
    fn test_id_search_custom_weights() {
        let weights = HeuristicWeights::aggressive();
        let config = SearchConfig::iterative_deepening(3);
        let ai = IterativeDeepeningSearch::with_weights_and_config(weights, config);
        assert_eq!(ai.config.max_depth, 3);
        assert_eq!(ai.name(), "IterativeDeepeningSearch");
    }

    #[test]
    fn test_id_search_tt_stats() {
        let ai = IterativeDeepeningSearch::new();
        let stats = ai.tt_stats();
        assert!(stats.capacity > 0);
        assert_eq!(stats.hit_count, 0);
    }

    #[test]
    fn test_id_search_clear_all() {
        let mut ai = IterativeDeepeningSearch::new();
        ai.clear_all();
        let stats = ai.tt_stats();
        assert_eq!(stats.stored_count, 0);
        assert!(ai.previous_pv.is_empty());
        assert!(ai.iteration_infos.is_empty());
    }

    #[test]
    fn test_id_search_stop_signal() {
        let mut ai = IterativeDeepeningSearch::new();
        let signal = Arc::new(AtomicBool::new(false));
        ai.set_stop_signal(signal.clone());

        assert!(!ai.should_stop());

        signal.store(true, Ordering::SeqCst);
        assert!(ai.should_stop());
    }

    // ========================================================================
    // Phase 6: SearchRoot with new AiLevel Tests
    // ========================================================================

    #[test]
    fn test_search_root_iterative_deepening() {
        let root = SearchRoot::new(AiLevel::IterativeDeepening);
        assert_eq!(root.worker_name(), "IterativeDeepeningSearch");
    }

    #[test]
    fn test_search_root_id_depth() {
        let root = SearchRoot::new(AiLevel::IterativeDeepeningDepth(4));
        assert_eq!(root.worker_name(), "IterativeDeepeningSearch");
    }

    #[test]
    fn test_search_root_id_timed() {
        let root = SearchRoot::new(AiLevel::IterativeDeepeningTimed(5000));
        assert_eq!(root.worker_name(), "IterativeDeepeningSearch");
    }

    #[test]
    fn test_search_root_lazy_smp() {
        let root = SearchRoot::new(AiLevel::LazySmp);
        assert_eq!(root.worker_name(), "LazySmpSearch");
    }

    // ========================================================================
    // Phase 6: SharedSearchState Tests
    // ========================================================================

    #[test]
    fn test_shared_search_state_creation() {
        let shared = SharedSearchState::new(4);
        assert_eq!(shared.thread_count, 4);
        assert!(!shared.is_stopped());
    }

    #[test]
    fn test_shared_search_state_minimum_threads() {
        let shared = SharedSearchState::new(0);
        assert_eq!(shared.thread_count, 1); // Minimum 1 thread
    }

    #[test]
    fn test_shared_search_state_stop_signal() {
        let shared = SharedSearchState::new(2);
        assert!(!shared.is_stopped());

        shared.signal_stop();
        assert!(shared.is_stopped());

        shared.reset_stop();
        assert!(!shared.is_stopped());
    }

    // ========================================================================
    // Phase 6: LazySmpSearch Tests
    // ========================================================================

    #[test]
    fn test_lazy_smp_creation() {
        let config = SearchConfig::iterative_deepening(4);
        let ai = LazySmpSearch::new(config);
        assert_eq!(ai.name(), "LazySmpSearch");
        assert!(!ai.shared_state().is_stopped());
    }

    #[test]
    fn test_lazy_smp_with_weights() {
        let weights = HeuristicWeights::defensive();
        let config = SearchConfig::iterative_deepening(3);
        let ai = LazySmpSearch::with_weights(weights, config);
        assert_eq!(ai.name(), "LazySmpSearch");
    }

    #[test]
    fn test_lazy_smp_tt_stats() {
        let config = SearchConfig::iterative_deepening(3);
        let ai = LazySmpSearch::new(config);
        let stats = ai.tt_stats();
        assert!(stats.capacity > 0);
    }

    #[test]
    fn test_lazy_smp_clear() {
        let config = SearchConfig::iterative_deepening(3);
        let mut ai = LazySmpSearch::new(config);
        ai.clear_all();
        let stats = ai.tt_stats();
        assert_eq!(stats.stored_count, 0);
    }

    // ========================================================================
    // Phase 6: IterationInfo / SearchDiagnostics Tests
    // ========================================================================

    #[test]
    fn test_iteration_info_display() {
        let info = IterationInfo {
            depth: 3,
            score: 150,
            pv: PvLine::new(),
            nodes: 5000,
            time_ms: 100,
            nps: 50000,
            aspiration_resets: 0,
            pv_changed: false,
        };
        let display = format!("{}", info);
        assert!(display.contains("depth 3"));
        assert!(display.contains("score +150"));
        assert!(display.contains("nodes 5000"));
        assert!(display.contains("nps 50000"));
        assert!(!display.contains("PV changed")); // pv_changed = false
    }

    #[test]
    fn test_iteration_info_pv_changed() {
        let info = IterationInfo {
            depth: 2,
            score: -50,
            pv: PvLine::new(),
            nodes: 1000,
            time_ms: 50,
            nps: 20000,
            aspiration_resets: 1,
            pv_changed: true,
        };
        let display = format!("{}", info);
        assert!(display.contains("(PV changed)"));
    }

    #[test]
    fn test_search_diagnostics_display() {
        let diag = SearchDiagnostics {
            iterations: vec![],
            final_stats: SearchStats::default(),
            tt_occupancy: 0.25,
            tt_hit_rate: 0.65,
            stopped_by_time: true,
            stopped_by_nodes: false,
            search_phase: Phase::Movement,
            volatility: 3,
        };
        let display = format!("{}", diag);
        assert!(display.contains("Search Diagnostics"));
        assert!(display.contains("Movement"));
        assert!(display.contains("Volatility: 3"));
        assert!(display.contains("occupancy=25.0%"));
        assert!(display.contains("hit_rate=65.0%"));
        assert!(display.contains("Stopped by: time limit"));
    }

    // ========================================================================
    // Phase 6: Extension Logic Tests
    // ========================================================================

    #[test]
    fn test_compute_extension_disabled() {
        let config = SearchConfig {
            use_extensions: false,
            ..SearchConfig::iterative_deepening(4)
        };
        let ai = IterativeDeepeningSearch::with_config(config);

        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Charge".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 100,
        };

        // Create a minimal game state for testing
        // (extensions don't depend on full state, just the action and config)
        let state = create_test_game_state();
        assert_eq!(ai.compute_extension(&action, &state, 0), 0);
    }

    #[test]
    fn test_compute_extension_max_reached() {
        let config = SearchConfig {
            use_extensions: true,
            max_extensions: 2,
            ..SearchConfig::iterative_deepening(4)
        };
        let ai = IterativeDeepeningSearch::with_config(config);

        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Charge".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 100,
        };

        let state = create_test_game_state();
        // Max extensions already used
        assert_eq!(ai.compute_extension(&action, &state, 2), 0);
        // Below max: should extend
        assert_eq!(ai.compute_extension(&action, &state, 0), 1);
    }

    #[test]
    fn test_compute_extension_intents() {
        let config = SearchConfig::iterative_deepening(4);
        let ai = IterativeDeepeningSearch::with_config(config);
        let state = create_test_game_state();

        // Charge intents should extend
        let charge = create_test_action(TacticalIntent::ChargeHighValueTarget);
        assert_eq!(ai.compute_extension(&charge, &state, 0), 1);

        let multi_charge = create_test_action(TacticalIntent::MultiChargeObjectiveSwing);
        assert_eq!(ai.compute_extension(&multi_charge, &state, 0), 1);

        // Stratagem intents should extend
        let def_strat = create_test_action(TacticalIntent::DefensiveStratagem);
        assert_eq!(ai.compute_extension(&def_strat, &state, 0), 1);

        let off_strat = create_test_action(TacticalIntent::OffensiveStratagem);
        assert_eq!(ai.compute_extension(&off_strat, &state, 0), 1);

        // Generic intents should NOT extend
        let generic = create_test_action(TacticalIntent::Generic);
        assert_eq!(ai.compute_extension(&generic, &state, 0), 0);

        let hold = create_test_action(TacticalIntent::HoldObjective);
        assert_eq!(ai.compute_extension(&hold, &state, 0), 0);
    }

    // ========================================================================
    // Phase 6: Instability Function Tests
    // ========================================================================

    #[test]
    fn test_is_position_unstable_stable_phases() {
        let mut state = create_test_game_state();
        state.current_subphase = SubPhase::SelectUnitToMove;
        assert!(!is_position_unstable(&state));

        state.current_subphase = SubPhase::SelectUnitToShoot;
        assert!(!is_position_unstable(&state));

        state.current_subphase = SubPhase::CommandPhaseStart;
        assert!(!is_position_unstable(&state));
    }

    #[test]
    fn test_is_position_unstable_combat_resolution() {
        let mut state = create_test_game_state();

        state.current_subphase = SubPhase::ResolveAttacks;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::ResolveMeleeAttacks;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::PileIn;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::Consolidate;
        assert!(is_position_unstable(&state));
    }

    #[test]
    fn test_is_position_unstable_charge_resolution() {
        let mut state = create_test_game_state();

        state.current_subphase = SubPhase::RollChargeDistance;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::MakeChargeMove;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::ResolveOverwatch;
        assert!(is_position_unstable(&state));
    }

    #[test]
    fn test_is_position_unstable_fight_sequencing() {
        let mut state = create_test_game_state();

        state.current_subphase = SubPhase::FightsFirst;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::RemainingCombats;
        assert!(is_position_unstable(&state));
    }

    #[test]
    fn test_is_position_unstable_reaction_windows() {
        let mut state = create_test_game_state();
        state.current_subphase = SubPhase::ReactionWindow;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::StratagemWindow;
        assert!(is_position_unstable(&state));

        state.current_subphase = SubPhase::EffectResolution;
        assert!(is_position_unstable(&state));
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    /// Create a minimal GameState for testing.
    /// This uses the same pattern as other test modules in the project.
    fn create_test_game_state() -> GameState {
        use wh40k_core_types::BattleRound;
        use wh40k_dice::{DiceContext, StreamKind};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::MiscRoll, 0, 0);
        let dice_roller = wh40k_dice::DiceRoller::new(ctx);

        GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::Movement,
            current_subphase: SubPhase::SelectUnitToMove,
            decision_owner: PlayerId::new(0),
            players: [
                wh40k_game_core::state::PlayerState::new(PlayerId::new(0), "Player A".to_string()),
                wh40k_game_core::state::PlayerState::new(PlayerId::new(1), "Player B".to_string()),
            ],
            units: vec![],
            board: wh40k_geometry::Board::combat_patrol(),
            event_bus: wh40k_event_system::EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller,
            active_effects: vec![],
            reaction_windows: vec![],
            turn_flags: wh40k_game_core::state::TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        }
    }

    /// Create a test MacroAction with the given intent.
    fn create_test_action(intent: TacticalIntent) -> MacroAction {
        MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: format!("{:?}", intent),
            commands: vec![],
            intent,
            actor_units: vec![],
            priority_hint: 50,
        }
    }
}
