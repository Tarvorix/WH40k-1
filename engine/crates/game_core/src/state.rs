//! Game state model: top-level GameState, PlayerState, and supporting types.
//!
//! Source: implementation_v3.md Sections 8.1-8.2
//! Source: 40k_revised.md - Turn structure, Command Points, Victory Points
//! Source: CP_Rules.md - Combat Patrol format specifics

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use wh40k_core_types::{
    BattleRound, CommandPoints, EnhancementId, FactionId, GameOutcome,
    MissionId, Phase, PlayerId, SecondaryObjectiveId, StratagemId, SubPhase, UnitId,
    VictoryPoints,
};
use wh40k_dice::DiceRoller;
use wh40k_event_system::{EventBus, ReactionWindow};
use wh40k_command_system::CommandHistory;
use wh40k_geometry::Board;

use crate::unit::UnitState;
use crate::effect::ActiveEffect;

// ---------------------------------------------------------------------------
// GameState
// ---------------------------------------------------------------------------

/// Top-level game state with ALL fields from implementation_v3.md 8.1.
///
/// This is the complete, authoritative, serializable game state.
/// Every fact about the game is stored here. The GUI is a projection
/// of this state. The search engine clones/mutates this state.
///
/// Source: implementation_v3.md Section 8.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Content version string for determinism/replay verification.
    pub content_version: String,

    /// The scenario (mission) being played, if set.
    pub scenario_id: Option<MissionId>,

    /// Current battle round (1-5 for Combat Patrol).
    pub battle_round: BattleRound,

    /// Which player is currently taking their turn.
    pub active_player: PlayerId,

    /// The current phase of the turn (Command, Movement, Shooting, etc.).
    pub current_phase: Phase,

    /// The current sub-phase within the main phase.
    pub current_subphase: SubPhase,

    /// Which player currently needs to make a decision.
    /// Usually the active player, but can be the opponent during reaction windows.
    pub decision_owner: PlayerId,

    /// The two players in the game. Index 0 = Player A, Index 1 = Player B.
    pub players: [PlayerState; 2],

    /// All units currently in the game (alive, destroyed, or in reserves).
    pub units: Vec<UnitState>,

    /// The game board with terrain and objectives.
    pub board: Board,

    /// The event bus for game event emission and subscription.
    pub event_bus: EventBus,

    /// The command history log.
    pub command_history: CommandHistory,

    /// The deterministic dice roller.
    pub dice_roller: DiceRoller,

    /// Currently active effects (abilities, stratagems, enhancements, etc.).
    pub active_effects: Vec<ActiveEffect>,

    /// Currently open reaction windows awaiting player decisions.
    pub reaction_windows: Vec<ReactionWindow>,

    /// Per-turn tracking flags (cleared at turn start).
    pub turn_flags: TurnFlags,

    /// The outcome of the game (InProgress, Victory, Draw).
    pub game_outcome: GameOutcome,

    /// Monotonically increasing counter for deterministic operations.
    pub deterministic_counter: u64,
}

impl GameState {
    /// Get a player state by PlayerId.
    pub fn player(&self, id: PlayerId) -> &PlayerState {
        if id == self.players[0].id {
            &self.players[0]
        } else {
            &self.players[1]
        }
    }

    /// Get a mutable reference to a player state by PlayerId.
    pub fn player_mut(&mut self, id: PlayerId) -> &mut PlayerState {
        if id == self.players[0].id {
            &mut self.players[0]
        } else {
            &mut self.players[1]
        }
    }

    /// Get the opponent of the given player.
    pub fn opponent_id(&self, id: PlayerId) -> PlayerId {
        if id == self.players[0].id {
            self.players[1].id
        } else {
            self.players[0].id
        }
    }

    /// Get a unit by UnitId.
    pub fn unit(&self, id: UnitId) -> Option<&UnitState> {
        self.units.iter().find(|u| u.id == id)
    }

    /// Get a mutable reference to a unit by UnitId.
    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut UnitState> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    /// Get all units owned by a player.
    pub fn units_for_player(&self, player: PlayerId) -> Vec<&UnitState> {
        self.units.iter().filter(|u| u.owner == player).collect()
    }

    /// Get all alive units owned by a player.
    pub fn alive_units_for_player(&self, player: PlayerId) -> Vec<&UnitState> {
        self.units
            .iter()
            .filter(|u| u.owner == player && !u.is_destroyed())
            .collect()
    }

    /// Check if a player has been tabled (all units destroyed).
    pub fn is_player_tabled(&self, player: PlayerId) -> bool {
        self.units
            .iter()
            .filter(|u| u.owner == player)
            .all(|u| u.is_destroyed())
    }

    /// Check if the game is still in progress.
    pub fn is_in_progress(&self) -> bool {
        matches!(self.game_outcome, GameOutcome::InProgress)
    }

    /// Increment and return the deterministic counter.
    pub fn next_counter(&mut self) -> u64 {
        let val = self.deterministic_counter;
        self.deterministic_counter += 1;
        val
    }

    /// Get the current active reaction window, if any.
    pub fn current_reaction_window(&self) -> Option<&ReactionWindow> {
        self.reaction_windows.last()
    }

    /// Check if there are any open reaction windows.
    pub fn has_reaction_window(&self) -> bool {
        !self.reaction_windows.is_empty()
    }

    /// Compute a deterministic hash of the complete game state.
    ///
    /// Used for replay verification, transposition table keys,
    /// bug localization, and determinism contract enforcement.
    ///
    /// Source: implementation_v3.md Section 18.3
    pub fn compute_hash(&self) -> u64 {
        crate::hasher::compute_state_hash(self)
    }
}

// ---------------------------------------------------------------------------
// PlayerState
// ---------------------------------------------------------------------------

/// Per-player state tracking.
///
/// Source: implementation_v3.md Section 8.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Player identifier (0 or 1).
    pub id: PlayerId,

    /// Display name for this player.
    pub name: String,

    /// The faction this player is using, if selected.
    pub faction_id: Option<FactionId>,

    /// Current command points available.
    pub cp: CommandPoints,

    /// Current victory points accumulated.
    pub vp: VictoryPoints,

    /// Selected enhancement, if any.
    pub enhancement_choice: Option<EnhancementId>,

    /// Selected secondary objective, if any.
    pub secondary_choice: Option<SecondaryObjectiveId>,

    /// Selected patrol squad variant index, if any.
    pub patrol_squad_choice: Option<usize>,

    /// Whether this player goes first in the battle round.
    pub first_turn: bool,

    /// Faction-specific round flags (e.g., Blessings of Khorne tracking).
    pub faction_round_flags: FactionRoundFlags,

    /// Stratagem usage tracking (per-phase, per-turn, per-battle).
    pub stratagem_usage: StratagemUsageTracker,

    /// Mission progress tracking.
    pub mission_progress: MissionProgress,
}

impl PlayerState {
    /// Create a new PlayerState with default values.
    pub fn new(id: PlayerId, name: String) -> Self {
        Self {
            id,
            name,
            faction_id: None,
            cp: CommandPoints::ZERO,
            vp: VictoryPoints::ZERO,
            enhancement_choice: None,
            secondary_choice: None,
            patrol_squad_choice: None,
            first_turn: false,
            faction_round_flags: FactionRoundFlags::default(),
            stratagem_usage: StratagemUsageTracker::default(),
            mission_progress: MissionProgress::default(),
        }
    }

    /// Gain command points.
    pub fn gain_cp(&mut self, amount: i8) {
        self.cp += CommandPoints::new(amount);
    }

    /// Spend command points. Returns true if successful, false if insufficient.
    pub fn spend_cp(&mut self, amount: i8) -> bool {
        if self.cp.value() >= amount {
            self.cp -= CommandPoints::new(amount);
            true
        } else {
            false
        }
    }

    /// Score victory points.
    pub fn score_vp(&mut self, amount: i16) {
        self.vp += VictoryPoints::new(amount);
    }

    /// Check if the player has enough CP for a given cost.
    pub fn can_afford_cp(&self, cost: i8) -> bool {
        self.cp.value() >= cost
    }
}

// ---------------------------------------------------------------------------
// TurnFlags
// ---------------------------------------------------------------------------

/// Per-turn tracking flags. Cleared at the start of each player turn.
///
/// Tracks which units have performed actions this turn for eligibility checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TurnFlags {
    /// Units that have moved this turn.
    pub units_moved: HashSet<UnitId>,

    /// Units that have shot this turn.
    pub units_shot: HashSet<UnitId>,

    /// Units that have charged this turn.
    pub units_charged: HashSet<UnitId>,

    /// Units that have fought this turn.
    pub units_fought: HashSet<UnitId>,

    /// Units that successfully charged THIS turn (for Fights First).
    pub charged_this_turn: HashSet<UnitId>,

    /// Units that fell back this turn (cannot shoot or charge).
    pub fell_back_this_turn: HashSet<UnitId>,

    /// Units that advanced this turn (cannot shoot non-Assault weapons, cannot charge).
    pub advanced_this_turn: HashSet<UnitId>,

    /// Units that remained stationary (for Heavy weapon bonus).
    pub remained_stationary: HashSet<UnitId>,

    /// Whether Overwatch has been used this turn (limited to once per turn normally).
    pub overwatch_used_this_turn: bool,

    /// Stratagems used this phase (for per-phase restriction checking).
    pub stratagems_used_this_phase: Vec<StratagemId>,

    /// Declared charge targets for each unit (set by DeclareCharge, consumed by ResolveChargeRoll).
    /// Source: 40k_revised.md Section 9.2 - charge targets must be tracked for roll validation
    pub declared_charge_targets: HashMap<UnitId, Vec<UnitId>>,

    /// Active Ka'tah stance choices per unit (set by ChooseKaTahStance during fight sequence).
    /// Key: unit_id, Value: stance name (e.g. "Dacatarai", "Rendax").
    /// Source: Custodes.md - Martial Ka'tah: choose stance per unit per fight
    pub ka_tah_stances: HashMap<UnitId, String>,

    /// Active Vaultswords profile choices per model (set by ChooseVaultswordsProfile).
    /// Key: model_id, Value: profile name (e.g. "Behemor", "Hurricanus", "Victus").
    /// Source: Custodes.md - Tristraen's Vaultswords profile selection
    pub vaultswords_profiles: HashMap<wh40k_core_types::ModelId, String>,

    /// Models queued for Fight on Death (Total Carnage blessing).
    /// These models fight back after the attacker finishes, then are removed from play.
    /// Source: Frenzied_Reavers.md - "Total Carnage" blessing
    pub fight_on_death_queue: Vec<wh40k_core_types::ModelId>,
}

impl TurnFlags {
    /// Create empty turn flags.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all turn flags for a new turn.
    pub fn clear(&mut self) {
        self.units_moved.clear();
        self.units_shot.clear();
        self.units_charged.clear();
        self.units_fought.clear();
        self.charged_this_turn.clear();
        self.fell_back_this_turn.clear();
        self.advanced_this_turn.clear();
        self.remained_stationary.clear();
        self.overwatch_used_this_turn = false;
        self.stratagems_used_this_phase.clear();
        self.declared_charge_targets.clear();
        self.ka_tah_stances.clear();
        self.vaultswords_profiles.clear();
        self.fight_on_death_queue.clear();
    }

    /// Clear phase-specific flags for a new phase (stratagems used this phase).
    pub fn clear_phase_flags(&mut self) {
        self.stratagems_used_this_phase.clear();
    }

    /// Record that a unit has moved.
    pub fn mark_moved(&mut self, unit_id: UnitId) {
        self.units_moved.insert(unit_id);
    }

    /// Record that a unit advanced.
    pub fn mark_advanced(&mut self, unit_id: UnitId) {
        self.advanced_this_turn.insert(unit_id);
        self.units_moved.insert(unit_id);
    }

    /// Record that a unit fell back.
    pub fn mark_fell_back(&mut self, unit_id: UnitId) {
        self.fell_back_this_turn.insert(unit_id);
        self.units_moved.insert(unit_id);
    }

    /// Record that a unit remained stationary.
    pub fn mark_remained_stationary(&mut self, unit_id: UnitId) {
        self.remained_stationary.insert(unit_id);
        self.units_moved.insert(unit_id);
    }

    /// Record that a unit has shot.
    pub fn mark_shot(&mut self, unit_id: UnitId) {
        self.units_shot.insert(unit_id);
    }

    /// Record that a unit has charged.
    pub fn mark_charged(&mut self, unit_id: UnitId) {
        self.units_charged.insert(unit_id);
        self.charged_this_turn.insert(unit_id);
    }

    /// Record that a unit has fought.
    pub fn mark_fought(&mut self, unit_id: UnitId) {
        self.units_fought.insert(unit_id);
    }

    /// Check if a unit has already moved this turn.
    pub fn has_moved(&self, unit_id: UnitId) -> bool {
        self.units_moved.contains(&unit_id)
    }

    /// Check if a unit has already shot this turn.
    pub fn has_shot(&self, unit_id: UnitId) -> bool {
        self.units_shot.contains(&unit_id)
    }

    /// Check if a unit has already charged this turn.
    pub fn has_charged(&self, unit_id: UnitId) -> bool {
        self.units_charged.contains(&unit_id)
    }

    /// Check if a unit has already fought this turn.
    pub fn has_fought(&self, unit_id: UnitId) -> bool {
        self.units_fought.contains(&unit_id)
    }

    /// Check if a unit advanced this turn.
    pub fn has_advanced(&self, unit_id: UnitId) -> bool {
        self.advanced_this_turn.contains(&unit_id)
    }

    /// Check if a unit fell back this turn.
    pub fn has_fell_back(&self, unit_id: UnitId) -> bool {
        self.fell_back_this_turn.contains(&unit_id)
    }

    /// Check if a unit remained stationary this turn.
    pub fn is_stationary(&self, unit_id: UnitId) -> bool {
        self.remained_stationary.contains(&unit_id)
    }

    /// Check if a unit charged this turn (for Fights First eligibility).
    pub fn charged_this_turn(&self, unit_id: UnitId) -> bool {
        self.charged_this_turn.contains(&unit_id)
    }

    /// Record declared charge targets for a unit.
    pub fn set_charge_targets(&mut self, unit_id: UnitId, targets: Vec<UnitId>) {
        self.declared_charge_targets.insert(unit_id, targets);
    }

    /// Get the declared charge targets for a unit.
    pub fn get_charge_targets(&self, unit_id: UnitId) -> Option<&Vec<UnitId>> {
        self.declared_charge_targets.get(&unit_id)
    }

    /// Record a Ka'tah stance choice for a unit.
    pub fn set_ka_tah_stance(&mut self, unit_id: UnitId, stance: String) {
        self.ka_tah_stances.insert(unit_id, stance);
    }

    /// Get the Ka'tah stance for a unit, if any.
    pub fn get_ka_tah_stance(&self, unit_id: UnitId) -> Option<&str> {
        self.ka_tah_stances.get(&unit_id).map(|s| s.as_str())
    }

    /// Record a Vaultswords profile choice for a model.
    pub fn set_vaultswords_profile(&mut self, model_id: wh40k_core_types::ModelId, profile: String) {
        self.vaultswords_profiles.insert(model_id, profile);
    }

    /// Get the Vaultswords profile for a model, if any.
    pub fn get_vaultswords_profile(&self, model_id: wh40k_core_types::ModelId) -> Option<&str> {
        self.vaultswords_profiles.get(&model_id).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// FactionRoundFlags
// ---------------------------------------------------------------------------

/// Faction-specific round flags, primarily for Blessings of Khorne tracking.
///
/// Source: Frenzied_Reavers.md - Blessings of Khorne
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionRoundFlags {
    /// Currently active blessings (by name).
    pub active_blessings: Vec<String>,

    /// The dice values rolled for Blessings of Khorne this round.
    pub blessing_dice: Vec<u8>,
}

impl FactionRoundFlags {
    /// Clear faction round flags for a new round.
    pub fn clear(&mut self) {
        self.active_blessings.clear();
        self.blessing_dice.clear();
    }

    /// Check if a specific blessing is active.
    pub fn has_blessing(&self, name: &str) -> bool {
        self.active_blessings.iter().any(|b| b == name)
    }

    /// Add an active blessing.
    pub fn add_blessing(&mut self, name: String) {
        if !self.active_blessings.contains(&name) {
            self.active_blessings.push(name);
        }
    }
}

// ---------------------------------------------------------------------------
// StratagemUsageTracker
// ---------------------------------------------------------------------------

/// Tracks stratagem usage across different time scopes.
///
/// 40K restricts stratagems to at most once per phase, once per turn,
/// and some are once per battle.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StratagemUsageTracker {
    /// Stratagems used this phase.
    pub used_this_phase: HashSet<StratagemId>,

    /// Stratagems used this turn.
    pub used_this_turn: HashSet<StratagemId>,

    /// Stratagems used this battle (for once-per-battle restrictions).
    pub used_this_battle: HashSet<StratagemId>,
}

impl StratagemUsageTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a stratagem was used.
    pub fn record_usage(&mut self, stratagem_id: StratagemId) {
        self.used_this_phase.insert(stratagem_id);
        self.used_this_turn.insert(stratagem_id);
        self.used_this_battle.insert(stratagem_id);
    }

    /// Check if a stratagem was already used this phase.
    pub fn used_this_phase(&self, stratagem_id: StratagemId) -> bool {
        self.used_this_phase.contains(&stratagem_id)
    }

    /// Check if a stratagem was already used this turn.
    pub fn used_this_turn(&self, stratagem_id: StratagemId) -> bool {
        self.used_this_turn.contains(&stratagem_id)
    }

    /// Check if a stratagem was already used this battle.
    pub fn used_this_battle(&self, stratagem_id: StratagemId) -> bool {
        self.used_this_battle.contains(&stratagem_id)
    }

    /// Clear phase-level usage (called at start of each phase).
    pub fn clear_phase(&mut self) {
        self.used_this_phase.clear();
    }

    /// Clear turn-level usage (called at start of each turn).
    pub fn clear_turn(&mut self) {
        self.used_this_phase.clear();
        self.used_this_turn.clear();
    }
}

// ---------------------------------------------------------------------------
// MissionProgress
// ---------------------------------------------------------------------------

/// Tracks mission scoring progress for a player.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MissionProgress {
    /// Victory points from primary objectives.
    pub primary_vp: VictoryPoints,

    /// Victory points from secondary objectives.
    pub secondary_vp: VictoryPoints,

    /// Score history per round: (round number, VP scored that round).
    pub round_scores: Vec<(BattleRound, VictoryPoints)>,
}

impl MissionProgress {
    /// Create a new empty progress tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record VP scored in a round.
    pub fn record_round_score(&mut self, round: BattleRound, vp: VictoryPoints) {
        self.round_scores.push((round, vp));
    }

    /// Total VP (primary + secondary).
    pub fn total_vp(&self) -> VictoryPoints {
        self.primary_vp + self.secondary_vp
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{BattleRound, Phase, SubPhase, GameOutcome};
    use wh40k_dice::{DiceContext, DiceRoller, StreamKind};

    fn make_test_game_state() -> GameState {
        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        GameState {
            content_version: "test-1.0".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "Player A".to_string()),
                PlayerState::new(PlayerId::new(1), "Player B".to_string()),
            ],
            units: Vec::new(),
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        }
    }

    #[test]
    fn test_game_state_creation() {
        let state = make_test_game_state();
        assert_eq!(state.battle_round, BattleRound::new(1));
        assert_eq!(state.active_player, PlayerId::new(0));
        assert_eq!(state.current_phase, Phase::PreBattle);
        assert!(state.is_in_progress());
        assert_eq!(state.units.len(), 0);
    }

    #[test]
    fn test_game_state_serialization() {
        let state = make_test_game_state();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content_version, "test-1.0");
        assert_eq!(deserialized.battle_round, BattleRound::new(1));
        assert_eq!(deserialized.active_player, PlayerId::new(0));
    }

    #[test]
    fn test_player_state_cp() {
        let mut player = PlayerState::new(PlayerId::new(0), "Test".to_string());
        assert_eq!(player.cp.value(), 0);

        player.gain_cp(1);
        assert_eq!(player.cp.value(), 1);

        assert!(player.can_afford_cp(1));
        assert!(!player.can_afford_cp(2));

        assert!(player.spend_cp(1));
        assert_eq!(player.cp.value(), 0);

        assert!(!player.spend_cp(1));
        assert_eq!(player.cp.value(), 0);
    }

    #[test]
    fn test_player_state_vp() {
        let mut player = PlayerState::new(PlayerId::new(0), "Test".to_string());
        player.score_vp(5);
        assert_eq!(player.vp.value(), 5);
        player.score_vp(3);
        assert_eq!(player.vp.value(), 8);
    }

    #[test]
    fn test_turn_flags() {
        let mut flags = TurnFlags::new();
        let unit_a = UnitId::new(0);
        let unit_b = UnitId::new(1);

        assert!(!flags.has_moved(unit_a));
        flags.mark_moved(unit_a);
        assert!(flags.has_moved(unit_a));
        assert!(!flags.has_moved(unit_b));

        flags.mark_advanced(unit_b);
        assert!(flags.has_advanced(unit_b));
        assert!(flags.has_moved(unit_b));
        assert!(!flags.has_advanced(unit_a));

        flags.mark_shot(unit_a);
        assert!(flags.has_shot(unit_a));

        flags.mark_charged(unit_a);
        assert!(flags.has_charged(unit_a));
        assert!(flags.charged_this_turn(unit_a));

        flags.mark_fought(unit_a);
        assert!(flags.has_fought(unit_a));

        // Test clear
        flags.clear();
        assert!(!flags.has_moved(unit_a));
        assert!(!flags.has_advanced(unit_b));
        assert!(!flags.has_shot(unit_a));
        assert!(!flags.has_charged(unit_a));
        assert!(!flags.has_fought(unit_a));
    }

    #[test]
    fn test_turn_flags_fell_back() {
        let mut flags = TurnFlags::new();
        let unit = UnitId::new(5);

        flags.mark_fell_back(unit);
        assert!(flags.has_fell_back(unit));
        assert!(flags.has_moved(unit));
    }

    #[test]
    fn test_turn_flags_stationary() {
        let mut flags = TurnFlags::new();
        let unit = UnitId::new(3);

        flags.mark_remained_stationary(unit);
        assert!(flags.is_stationary(unit));
        assert!(flags.has_moved(unit)); // counted as "moved" for eligibility
    }

    #[test]
    fn test_faction_round_flags() {
        let mut flags = FactionRoundFlags::default();
        assert!(!flags.has_blessing("Total Carnage"));

        flags.add_blessing("Total Carnage".to_string());
        assert!(flags.has_blessing("Total Carnage"));
        assert!(!flags.has_blessing("Warp Blades"));

        flags.clear();
        assert!(!flags.has_blessing("Total Carnage"));
    }

    #[test]
    fn test_stratagem_usage_tracker() {
        let mut tracker = StratagemUsageTracker::new();
        let strat = StratagemId::new(1);

        assert!(!tracker.used_this_phase(strat));
        assert!(!tracker.used_this_turn(strat));
        assert!(!tracker.used_this_battle(strat));

        tracker.record_usage(strat);
        assert!(tracker.used_this_phase(strat));
        assert!(tracker.used_this_turn(strat));
        assert!(tracker.used_this_battle(strat));

        tracker.clear_phase();
        assert!(!tracker.used_this_phase(strat));
        assert!(tracker.used_this_turn(strat));
        assert!(tracker.used_this_battle(strat));

        tracker.clear_turn();
        assert!(!tracker.used_this_phase(strat));
        assert!(!tracker.used_this_turn(strat));
        assert!(tracker.used_this_battle(strat));
    }

    #[test]
    fn test_mission_progress() {
        let mut progress = MissionProgress::new();
        progress.primary_vp = VictoryPoints::new(5);
        progress.secondary_vp = VictoryPoints::new(3);
        assert_eq!(progress.total_vp().value(), 8);

        progress.record_round_score(BattleRound::new(1), VictoryPoints::new(5));
        assert_eq!(progress.round_scores.len(), 1);
        assert_eq!(progress.round_scores[0].0, BattleRound::new(1));
    }

    #[test]
    fn test_game_state_player_lookup() {
        let state = make_test_game_state();
        let p0 = state.player(PlayerId::new(0));
        assert_eq!(p0.name, "Player A");

        let p1 = state.player(PlayerId::new(1));
        assert_eq!(p1.name, "Player B");

        assert_eq!(state.opponent_id(PlayerId::new(0)), PlayerId::new(1));
        assert_eq!(state.opponent_id(PlayerId::new(1)), PlayerId::new(0));
    }

    #[test]
    fn test_game_state_counter() {
        let mut state = make_test_game_state();
        assert_eq!(state.next_counter(), 0);
        assert_eq!(state.next_counter(), 1);
        assert_eq!(state.next_counter(), 2);
    }

    #[test]
    fn test_game_state_no_reaction_windows() {
        let state = make_test_game_state();
        assert!(!state.has_reaction_window());
        assert!(state.current_reaction_window().is_none());
    }

    #[test]
    fn test_game_state_tabled_check() {
        let state = make_test_game_state();
        // No units means tabled check returns true
        assert!(state.is_player_tabled(PlayerId::new(0)));
    }
}
