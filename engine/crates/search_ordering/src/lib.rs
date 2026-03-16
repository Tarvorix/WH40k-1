//! WH40K Engine - SearchOrdering crate
//!
//! Move ordering heuristics for alpha-beta search efficiency.
//! Good move ordering dramatically improves alpha-beta pruning by
//! searching the best moves first, causing more beta cutoffs.
//!
//! # Ordering Strategies
//!
//! 1. **TT Move**: Best move from transposition table (highest priority)
//! 2. **Killer Moves**: Moves that caused beta cutoffs at the same depth
//! 3. **Tactical Intent Priority**: Strategic importance of the action type
//! 4. **Heuristic Prior**: Quick evaluation score for the resulting position
//! 5. **History Heuristic**: Moves that have been good in past searches
//!
//! Source: implementation_v3.md Section 11.1 (killer/history heuristics)

use wh40k_core_types::PlayerId;
use wh40k_game_core::state::GameState;
use wh40k_eval_heuristic::HeuristicEvaluator;
use wh40k_search_abstraction::{CandidateSet, MacroAction, TacticalIntent};

// ============================================================================
// Move Ordering Score
// ============================================================================

/// A scored move candidate for ordering.
/// Associates a macro-action index with its ordering score.
#[derive(Debug, Clone, Copy)]
pub struct OrderedMove {
    /// Index into the candidate set's actions array.
    pub index: usize,
    /// Ordering score (higher = search first).
    pub score: i32,
}

impl OrderedMove {
    /// Create a new ordered move.
    pub fn new(index: usize, score: i32) -> Self {
        Self { index, score }
    }
}

// ============================================================================
// Killer Table
// ============================================================================

/// Killer moves: moves that caused a beta cutoff at a given depth.
/// Killer moves are often good in sibling nodes at the same depth,
/// so they should be searched early.
///
/// We store two killer slots per depth (common in chess engines).
/// The killer is identified by the TacticalIntent + actor unit pattern,
/// since exact command matching across different positions is unreliable.
#[derive(Debug, Clone)]
pub struct KillerTable {
    /// Killer entries indexed by depth. Each depth has two slots.
    /// Stored as (intent, priority_hint) pairs for matching.
    entries: Vec<[Option<KillerEntry>; 2]>,
    /// Maximum depth tracked.
    max_depth: usize,
}

/// A single killer move entry.
#[derive(Debug, Clone, Copy)]
pub struct KillerEntry {
    /// The tactical intent of the killer move.
    pub intent: TacticalIntent,
    /// The priority hint from the original action (for tie-breaking).
    pub priority_hint: i32,
}

impl KillerTable {
    /// Create a new killer table for the given maximum depth.
    pub fn new(max_depth: usize) -> Self {
        Self {
            entries: vec![[None; 2]; max_depth + 1],
            max_depth,
        }
    }

    /// Record a killer move at the given depth.
    /// Uses a two-slot replacement scheme: new killer goes to slot 0,
    /// old slot 0 moves to slot 1.
    pub fn record(&mut self, depth: usize, intent: TacticalIntent, priority_hint: i32) {
        if depth > self.max_depth {
            return;
        }

        let entry = KillerEntry {
            intent,
            priority_hint,
        };

        // Don't record the same killer twice
        if let Some(existing) = &self.entries[depth][0] {
            if existing.intent == intent {
                return;
            }
        }

        // Shift slot 0 to slot 1, put new entry in slot 0
        self.entries[depth][1] = self.entries[depth][0];
        self.entries[depth][0] = Some(entry);
    }

    /// Check if a move matches a killer at the given depth.
    /// Returns the killer bonus score if matched (higher for slot 0).
    pub fn probe(&self, depth: usize, intent: TacticalIntent) -> Option<i32> {
        if depth > self.max_depth {
            return None;
        }

        // Slot 0 killer (most recent) gets higher bonus
        if let Some(entry) = &self.entries[depth][0] {
            if entry.intent == intent {
                return Some(KILLER_BONUS_PRIMARY);
            }
        }

        // Slot 1 killer gets lower bonus
        if let Some(entry) = &self.entries[depth][1] {
            if entry.intent == intent {
                return Some(KILLER_BONUS_SECONDARY);
            }
        }

        None
    }

    /// Clear all killer entries (call at the start of a new search).
    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            *entry = [None; 2];
        }
    }
}

/// Bonus score for primary killer move (slot 0).
const KILLER_BONUS_PRIMARY: i32 = 9000;
/// Bonus score for secondary killer move (slot 1).
const KILLER_BONUS_SECONDARY: i32 = 8000;

// ============================================================================
// History Table
// ============================================================================

/// History heuristic: tracks which TacticalIntent types have been
/// successful across the entire search tree. Moves that frequently
/// cause cutoffs get higher ordering priority.
///
/// Indexed by TacticalIntent variant for simplicity.
/// In a more sophisticated engine, this could also incorporate
/// the specific game state context.
#[derive(Debug, Clone)]
pub struct HistoryTable {
    /// Score accumulated for each TacticalIntent variant.
    /// Indexed by intent as usize (there are 40 variants including BA intents).
    scores: Vec<i64>,
    /// Maximum score to prevent overflow. Scores are halved when any exceeds this.
    max_score: i64,
}

impl HistoryTable {
    /// Create a new empty history table.
    pub fn new() -> Self {
        Self {
            scores: vec![0i64; 64], // Generous size for all TacticalIntent variants
            max_score: 1_000_000,
        }
    }

    /// Record a successful move (caused a cutoff or improved alpha).
    /// The bonus is proportional to the depth^2 (deeper cutoffs are more valuable).
    pub fn record_success(&mut self, intent: TacticalIntent, depth: u8) {
        let idx = intent_index(intent);
        let bonus = (depth as i64 + 1) * (depth as i64 + 1);
        self.scores[idx] += bonus;

        // Age all scores if any exceeds the maximum
        if self.scores[idx] > self.max_score {
            for score in self.scores.iter_mut() {
                *score /= 2;
            }
        }
    }

    /// Record a failed move (searched but didn't cause cutoff).
    /// Gives a mild penalty to reduce its priority.
    pub fn record_failure(&mut self, intent: TacticalIntent, depth: u8) {
        let idx = intent_index(intent);
        let penalty = depth as i64 + 1;
        self.scores[idx] = (self.scores[idx] - penalty).max(0);
    }

    /// Get the history score for a move's tactical intent.
    /// Returns a value suitable for move ordering (0 = no history).
    pub fn probe(&self, intent: TacticalIntent) -> i32 {
        let idx = intent_index(intent);
        // Scale to a reasonable ordering range (0-5000)
        let raw = self.scores[idx];
        if raw == 0 {
            return 0;
        }
        // Logarithmic scaling to prevent history from dominating
        let scaled = ((raw as f64).ln() * 500.0) as i32;
        scaled.clamp(0, 5000)
    }

    /// Clear all history scores (call between games, not between searches).
    pub fn clear(&mut self) {
        for score in self.scores.iter_mut() {
            *score = 0;
        }
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a TacticalIntent to an index for the history table.
fn intent_index(intent: TacticalIntent) -> usize {
    // Use a match to ensure exhaustiveness and stable mapping
    match intent {
        TacticalIntent::HoldObjective => 0,
        TacticalIntent::ContestObjective => 1,
        TacticalIntent::MoveToCover => 2,
        TacticalIntent::StageCharge => 3,
        TacticalIntent::Screen => 4,
        TacticalIntent::Retreat => 5,
        TacticalIntent::LineUpShot => 6,
        TacticalIntent::DenyReserveZone => 7,
        TacticalIntent::HoldPosition => 8,
        TacticalIntent::AdvanceAggressively => 9,
        TacticalIntent::FallBackToShoot => 10,
        TacticalIntent::DeepStrikeArrive => 11,
        TacticalIntent::MaximizeKills => 12,
        TacticalIntent::SoftenChargeTarget => 13,
        TacticalIntent::RemoveScoringUnit => 14,
        TacticalIntent::ForceBattleShock => 15,
        TacticalIntent::BracketHardTarget => 16,
        TacticalIntent::ChargeHighValueTarget => 17,
        TacticalIntent::MultiChargeObjectiveSwing => 18,
        TacticalIntent::ChargeSoftenedTarget => 19,
        TacticalIntent::FightMaxDamage => 20,
        TacticalIntent::FightPreserve => 21,
        TacticalIntent::ChooseStance => 22,
        TacticalIntent::ChooseWeaponProfile => 23,
        TacticalIntent::DefensiveStratagem => 24,
        TacticalIntent::OffensiveStratagem => 25,
        TacticalIntent::MovementReaction => 26,
        TacticalIntent::FightOrderManipulation => 27,
        TacticalIntent::DeclineStratagem => 28,
        // Boarding Actions intents
        TacticalIntent::OperateHatch => 29,
        TacticalIntent::SecureObjective => 30,
        TacticalIntent::DefendPosition => 31,
        TacticalIntent::ControlChokepoint => 32,
        TacticalIntent::FlankThroughHatch => 33,
        TacticalIntent::ProjectLeaderAbility => 34,
        TacticalIntent::EnterFromReserves => 35,
        // Misc intents
        TacticalIntent::ScoreObjective => 36,
        TacticalIntent::AllocateBlessings => 37,
        TacticalIntent::PhaseControl => 38,
        TacticalIntent::Generic => 39,
    }
}

// ============================================================================
// Move Orderer
// ============================================================================

/// The main move ordering system that combines all ordering heuristics
/// to sort candidate actions for optimal alpha-beta pruning.
///
/// Ordering priority (highest to lowest):
/// 1. TT best move (score: 20000)
/// 2. Killer moves (score: 8000-9000)
/// 3. History heuristic (score: 0-5000)
/// 4. Tactical intent priority (score: 0-100 from TacticalIntent)
/// 5. Action priority hint (from candidate generation)
pub struct MoveOrderer {
    /// Killer move table.
    pub killers: KillerTable,
    /// History heuristic table.
    pub history: HistoryTable,
}

/// Bonus for the TT best move (always searched first).
const TT_MOVE_BONUS: i32 = 20000;

impl MoveOrderer {
    /// Create a new move orderer with the given maximum search depth.
    pub fn new(max_depth: usize) -> Self {
        Self {
            killers: KillerTable::new(max_depth),
            history: HistoryTable::new(),
        }
    }

    /// Order the candidates in a CandidateSet for search.
    ///
    /// Returns indices into the candidate set sorted by ordering score
    /// (highest first = most promising to search first).
    ///
    /// # Arguments
    /// * `candidates` - The candidate actions to order
    /// * `depth` - Current search depth (for killer move lookup)
    /// * `tt_best_move` - Index of the TT best move, if any
    pub fn order_moves(
        &self,
        candidates: &CandidateSet,
        depth: usize,
        tt_best_move: Option<u16>,
    ) -> Vec<OrderedMove> {
        let actions = &candidates.candidates;
        let mut ordered: Vec<OrderedMove> = Vec::with_capacity(actions.len());

        for (i, action) in actions.iter().enumerate() {
            let mut score: i32 = 0;

            // 1. TT best move gets highest priority
            if let Some(tt_idx) = tt_best_move {
                if i == tt_idx as usize {
                    score += TT_MOVE_BONUS;
                }
            }

            // 2. Killer move bonus
            if let Some(killer_bonus) = self.killers.probe(depth, action.intent) {
                score += killer_bonus;
            }

            // 3. History heuristic bonus
            score += self.history.probe(action.intent);

            // 4. Tactical intent ordering priority
            score += action.intent.ordering_priority();

            // 5. Action's own priority hint from candidate generation
            score += action.priority_hint;

            ordered.push(OrderedMove::new(i, score));
        }

        // Sort descending by score (best moves first)
        ordered.sort_unstable_by(|a, b| b.score.cmp(&a.score));

        ordered
    }

    /// Record a beta cutoff for move ordering improvement.
    /// Updates both killer and history tables.
    pub fn record_cutoff(
        &mut self,
        depth: usize,
        action: &MacroAction,
    ) {
        // Record killer
        self.killers
            .record(depth, action.intent, action.priority_hint);

        // Record history success
        self.history
            .record_success(action.intent, depth as u8);
    }

    /// Record a move that was searched but did not cause a cutoff.
    pub fn record_searched(&mut self, action: &MacroAction, depth: usize) {
        self.history.record_failure(action.intent, depth as u8);
    }

    /// Clear killer table (call at start of new root search).
    pub fn clear_killers(&mut self) {
        self.killers.clear();
    }

    /// Clear all ordering data (call between games).
    pub fn clear_all(&mut self) {
        self.killers.clear();
        self.history.clear();
    }

    /// Get the number of ordered moves that would be generated for
    /// the given candidate set (utility for capacity planning).
    pub fn count_candidates(&self, candidates: &CandidateSet) -> usize {
        candidates.candidates.len()
    }
}

// ============================================================================
// Static Move Ordering (no search state needed)
// ============================================================================

/// Quick static ordering of candidates without any search-dependent heuristics.
/// Uses only tactical intent priority and action priority hints.
/// Useful for the root node or when no search state is available.
pub fn static_order(candidates: &CandidateSet) -> Vec<OrderedMove> {
    let actions = &candidates.candidates;
    let mut ordered: Vec<OrderedMove> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let score = action.intent.ordering_priority() + action.priority_hint;
            OrderedMove::new(i, score)
        })
        .collect();

    ordered.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    ordered
}

/// Order candidates with an additional heuristic evaluation score.
/// This is more expensive than static ordering but gives better results
/// for the root node where move quality matters most.
pub fn heuristic_order(
    candidates: &CandidateSet,
    state: &GameState,
    perspective: PlayerId,
    evaluator: &HeuristicEvaluator,
) -> Vec<OrderedMove> {
    let actions = &candidates.candidates;
    let mut ordered: Vec<OrderedMove> = Vec::with_capacity(actions.len());

    // Get a base evaluation for comparison
    let base_score = evaluator.move_ordering_score(state, perspective);

    for (i, action) in actions.iter().enumerate() {
        let mut score: i32 = 0;

        // Tactical intent priority
        score += action.intent.ordering_priority() * 10;

        // Action priority hint
        score += action.priority_hint;

        // Try to evaluate the position after this action
        // This is expensive but worthwhile at the root
        let mut child_state = state.clone();
        let mut commands_applied = true;
        for cmd in &action.commands {
            if wh40k_game_core::CommandExecutor::execute(&mut child_state, cmd).is_err() {
                commands_applied = false;
                break;
            }
        }

        if commands_applied {
            let child_score = evaluator.move_ordering_score(&child_state, perspective);
            // Positive delta means this move improved our position
            let delta = child_score - base_score;
            score += delta;
        }

        ordered.push(OrderedMove::new(i, score));
    }

    ordered.sort_unstable_by(|a, b| b.score.cmp(&a.score));
    ordered
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_killer_table_basic() {
        let mut killers = KillerTable::new(10);

        // Record a killer at depth 3
        killers.record(3, TacticalIntent::ChargeHighValueTarget, 100);

        // Should find it
        let bonus = killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, Some(KILLER_BONUS_PRIMARY));

        // Should not find it at a different depth
        let bonus = killers.probe(4, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, None);

        // Should not find a different intent at same depth
        let bonus = killers.probe(3, TacticalIntent::MaximizeKills);
        assert_eq!(bonus, None);
    }

    #[test]
    fn test_killer_table_two_slots() {
        let mut killers = KillerTable::new(10);

        // Record two different killers at depth 3
        killers.record(3, TacticalIntent::ChargeHighValueTarget, 100);
        killers.record(3, TacticalIntent::MaximizeKills, 80);

        // First recorded should now be in slot 1
        let bonus = killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, Some(KILLER_BONUS_SECONDARY));

        // Most recent should be in slot 0
        let bonus = killers.probe(3, TacticalIntent::MaximizeKills);
        assert_eq!(bonus, Some(KILLER_BONUS_PRIMARY));
    }

    #[test]
    fn test_killer_table_no_duplicate() {
        let mut killers = KillerTable::new(10);

        killers.record(3, TacticalIntent::ChargeHighValueTarget, 100);
        // Recording same intent again should not change anything
        killers.record(3, TacticalIntent::ChargeHighValueTarget, 100);

        // Should still be in slot 0
        let bonus = killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, Some(KILLER_BONUS_PRIMARY));
    }

    #[test]
    fn test_killer_table_clear() {
        let mut killers = KillerTable::new(10);
        killers.record(3, TacticalIntent::ChargeHighValueTarget, 100);
        killers.clear();

        let bonus = killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, None);
    }

    #[test]
    fn test_history_table_basic() {
        let mut history = HistoryTable::new();

        // Record some successes
        history.record_success(TacticalIntent::ChargeHighValueTarget, 5);
        history.record_success(TacticalIntent::ChargeHighValueTarget, 5);
        history.record_success(TacticalIntent::ChargeHighValueTarget, 5);

        // Should have a positive score
        let score = history.probe(TacticalIntent::ChargeHighValueTarget);
        assert!(score > 0, "Expected positive history score, got {}", score);

        // Unknown intents should have zero
        let score = history.probe(TacticalIntent::DenyReserveZone);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_history_table_failure_reduces() {
        let mut history = HistoryTable::new();

        // Build up score
        history.record_success(TacticalIntent::MaximizeKills, 5);
        let score_before = history.probe(TacticalIntent::MaximizeKills);

        // Apply failures
        history.record_failure(TacticalIntent::MaximizeKills, 5);
        history.record_failure(TacticalIntent::MaximizeKills, 5);
        let score_after = history.probe(TacticalIntent::MaximizeKills);

        assert!(
            score_after <= score_before,
            "Failures should reduce score: before={}, after={}",
            score_before,
            score_after
        );
    }

    #[test]
    fn test_history_table_clear() {
        let mut history = HistoryTable::new();
        history.record_success(TacticalIntent::ChargeHighValueTarget, 5);
        history.clear();

        let score = history.probe(TacticalIntent::ChargeHighValueTarget);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_move_orderer_creation() {
        let orderer = MoveOrderer::new(20);
        assert_eq!(orderer.killers.max_depth, 20);
    }

    #[test]
    fn test_move_orderer_tt_move_highest() {
        use wh40k_core_types::Phase;

        let orderer = MoveOrderer::new(10);

        // Create a candidate set with multiple actions
        let mut candidates = CandidateSet::new(
            wh40k_core_types::PlayerId::new(0),
            Phase::Movement,
        );
        candidates.add_action(MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Low priority".to_string(),
            commands: vec![],
            intent: TacticalIntent::Generic,
            actor_units: vec![],
            priority_hint: 0,
        });
        candidates.add_action(MacroAction {
            id: wh40k_search_abstraction::MacroActionId(1),
            label: "High priority".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 100,
        });

        // Order with TT best move = index 0 (the low priority one)
        let ordered = orderer.order_moves(&candidates, 0, Some(0));

        // TT move should be first despite low tactical priority
        assert_eq!(ordered[0].index, 0, "TT move should be ordered first");
        assert!(
            ordered[0].score > ordered[1].score,
            "TT move should have highest score"
        );
    }

    #[test]
    fn test_intent_index_unique() {
        // Ensure all intents map to unique indices
        let intents = vec![
            TacticalIntent::HoldObjective,
            TacticalIntent::ContestObjective,
            TacticalIntent::MoveToCover,
            TacticalIntent::StageCharge,
            TacticalIntent::Screen,
            TacticalIntent::Retreat,
            TacticalIntent::LineUpShot,
            TacticalIntent::DenyReserveZone,
            TacticalIntent::HoldPosition,
            TacticalIntent::AdvanceAggressively,
            TacticalIntent::FallBackToShoot,
            TacticalIntent::DeepStrikeArrive,
            TacticalIntent::MaximizeKills,
            TacticalIntent::SoftenChargeTarget,
            TacticalIntent::RemoveScoringUnit,
            TacticalIntent::ForceBattleShock,
            TacticalIntent::BracketHardTarget,
            TacticalIntent::ChargeHighValueTarget,
            TacticalIntent::MultiChargeObjectiveSwing,
            TacticalIntent::ChargeSoftenedTarget,
            TacticalIntent::FightMaxDamage,
            TacticalIntent::FightPreserve,
            TacticalIntent::ChooseStance,
            TacticalIntent::ChooseWeaponProfile,
            TacticalIntent::DefensiveStratagem,
            TacticalIntent::OffensiveStratagem,
            TacticalIntent::MovementReaction,
            TacticalIntent::FightOrderManipulation,
            TacticalIntent::DeclineStratagem,
            TacticalIntent::ScoreObjective,
            TacticalIntent::AllocateBlessings,
            TacticalIntent::PhaseControl,
            TacticalIntent::Generic,
        ];

        let mut indices: Vec<usize> = intents.iter().map(|i| intent_index(*i)).collect();
        let original_len = indices.len();
        indices.sort();
        indices.dedup();
        assert_eq!(
            indices.len(),
            original_len,
            "All intents should have unique indices"
        );
    }

    #[test]
    fn test_static_order_by_priority() {
        use wh40k_core_types::Phase;

        let mut candidates = CandidateSet::new(
            wh40k_core_types::PlayerId::new(0),
            Phase::Movement,
        );

        // Add actions with different priorities
        candidates.add_action(MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Generic".to_string(),
            commands: vec![],
            intent: TacticalIntent::Generic,
            actor_units: vec![],
            priority_hint: 0,
        });
        candidates.add_action(MacroAction {
            id: wh40k_search_abstraction::MacroActionId(1),
            label: "Charge".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 50,
        });
        candidates.add_action(MacroAction {
            id: wh40k_search_abstraction::MacroActionId(2),
            label: "Hold".to_string(),
            commands: vec![],
            intent: TacticalIntent::HoldObjective,
            actor_units: vec![],
            priority_hint: 20,
        });

        let ordered = static_order(&candidates);

        // Charge (priority 100 + 50 = 150) should be first
        assert_eq!(ordered[0].index, 1, "Charge should be first");
        // Hold (priority 70 + 20 = 90) should be second
        assert_eq!(ordered[1].index, 2, "Hold should be second");
        // Generic (priority 0 + 0 = 0) should be last
        assert_eq!(ordered[2].index, 0, "Generic should be last");
    }

    #[test]
    fn test_move_orderer_record_cutoff() {
        let mut orderer = MoveOrderer::new(10);

        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Charge".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 50,
        };

        // Record cutoff at depth 3
        orderer.record_cutoff(3, &action);

        // Killer should be set
        let bonus = orderer.killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, Some(KILLER_BONUS_PRIMARY));

        // History should be updated
        let history_score = orderer.history.probe(TacticalIntent::ChargeHighValueTarget);
        assert!(history_score > 0);
    }

    #[test]
    fn test_move_orderer_clear() {
        let mut orderer = MoveOrderer::new(10);

        let action = MacroAction {
            id: wh40k_search_abstraction::MacroActionId(0),
            label: "Charge".to_string(),
            commands: vec![],
            intent: TacticalIntent::ChargeHighValueTarget,
            actor_units: vec![],
            priority_hint: 50,
        };

        orderer.record_cutoff(3, &action);
        orderer.clear_killers();

        // Killers should be cleared
        let bonus = orderer.killers.probe(3, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(bonus, None);

        // History should still be intact
        let history_score = orderer.history.probe(TacticalIntent::ChargeHighValueTarget);
        assert!(history_score > 0);

        // Clear all
        orderer.clear_all();
        let history_score = orderer.history.probe(TacticalIntent::ChargeHighValueTarget);
        assert_eq!(history_score, 0);
    }
}
