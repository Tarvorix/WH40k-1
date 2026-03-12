//! WH40K Engine - CombatPatrolRules crate
//!
//! Combat Patrol format-specific rules overlay on top of base 40K.
//! Enforces CP-specific constraints: board size, CP economy, reserve timing,
//! pre-battle sequence, and objective securing rules.
//!
//! Source: CP_Rules.md - Combat Patrol format rules
//! Source: implementation_v3.md Section 6.1 (Layer 3)

use serde::{Deserialize, Serialize};
use thiserror::Error;

use wh40k_core_types::{
    BattleRound, BoardDimensions, CommandPoints, FactionId, Keyword,
    PlayerId, Position, UnitId, UnitStatus, VictoryPoints,
};
use wh40k_game_core::UnitState;

// ─── Errors ────────────────────────────────────────────────────────────────

/// Errors specific to Combat Patrol rules enforcement.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum CombatPatrolError {
    #[error("Invalid reserve timing: reserves cannot arrive in battle round {0}")]
    InvalidReserveTiming(u8),

    #[error("Reserve must arrive more than 9\" from all enemy models")]
    ReserveTooCloseToEnemy,

    #[error("All reserves must arrive by end of battle round 3")]
    ReservesNotArrivedByRound3,

    #[error("Unit {0} is not eligible for strategic reserves")]
    UnitNotEligibleForReserve(UnitId),

    #[error("Invalid patrol squad choice: faction {faction_id}, index {squad_index}")]
    InvalidPatrolSquadChoice {
        faction_id: FactionId,
        squad_index: usize,
    },

    #[error("Pre-battle step out of order: expected {expected:?}, got {actual:?}")]
    PreBattleStepOutOfOrder {
        expected: PreBattleStep,
        actual: PreBattleStep,
    },

    #[error("Invalid roll-off result: {0}")]
    InvalidRollOff(String),
}

// ─── PreBattleStep ──────────────────────────────────────────────────────────

/// Pre-battle sequence steps in order for Combat Patrol.
/// Source: CP_Rules.md - Pre-Battle Sequence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PreBattleStep {
    /// Step 1: Roll off to determine attacker/defender
    DetermineAttackerDefender,
    /// Step 2: Select enhancement for your warlord
    SelectEnhancement,
    /// Step 3: Select secondary objective
    SelectSecondary,
    /// Step 4: Select patrol squad (if applicable)
    SelectPatrolSquad,
    /// Step 5: Alternating deployment
    Deployment,
    /// Step 6: Roll off to determine first turn (non-deploying player chooses)
    DetermineFirstTurn,
    /// Pre-battle complete
    Complete,
}

impl PreBattleStep {
    /// Returns the next step in the pre-battle sequence.
    pub fn next(self) -> Option<PreBattleStep> {
        match self {
            PreBattleStep::DetermineAttackerDefender => Some(PreBattleStep::SelectEnhancement),
            PreBattleStep::SelectEnhancement => Some(PreBattleStep::SelectSecondary),
            PreBattleStep::SelectSecondary => Some(PreBattleStep::SelectPatrolSquad),
            PreBattleStep::SelectPatrolSquad => Some(PreBattleStep::Deployment),
            PreBattleStep::Deployment => Some(PreBattleStep::DetermineFirstTurn),
            PreBattleStep::DetermineFirstTurn => Some(PreBattleStep::Complete),
            PreBattleStep::Complete => None,
        }
    }
}

// ─── ReserveArrivalRules ────────────────────────────────────────────────────

/// Describes the reserve arrival constraints for Combat Patrol.
/// Source: CP_Rules.md - Reserves
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveArrivalRules {
    /// Reserves cannot arrive in battle round 1.
    pub no_arrival_round_1: bool,
    /// Reserves must arrive by end of this battle round.
    pub must_arrive_by_round: u8,
    /// Minimum distance from enemy models in inches (raw mils).
    pub min_distance_from_enemies_mils: i32,
}

impl Default for ReserveArrivalRules {
    fn default() -> Self {
        Self {
            no_arrival_round_1: true,
            must_arrive_by_round: 3,
            // 9 inches = 9000 mils
            min_distance_from_enemies_mils: 9000,
        }
    }
}

// ─── DeploymentAlternation ──────────────────────────────────────────────────

/// Tracks alternating deployment state for Combat Patrol.
/// Source: CP_Rules.md - Deployment alternation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentAlternation {
    /// Which player deploys first (the attacker).
    pub first_deployer: PlayerId,
    /// Which player deploys next.
    pub current_deployer: PlayerId,
    /// Number of units deployed so far by player 0.
    pub player_0_deployed: usize,
    /// Number of units deployed so far by player 1.
    pub player_1_deployed: usize,
}

impl DeploymentAlternation {
    /// Create a new deployment alternation tracker.
    pub fn new(first_deployer: PlayerId) -> Self {
        Self {
            first_deployer,
            current_deployer: first_deployer,
            player_0_deployed: 0,
            player_1_deployed: 0,
        }
    }

    /// Record a deployment and switch to the next deployer.
    pub fn record_deployment(&mut self, player: PlayerId) {
        if player == PlayerId::new(0) {
            self.player_0_deployed += 1;
        } else {
            self.player_1_deployed += 1;
        }
        // Alternate to the other player
        if self.current_deployer == PlayerId::new(0) {
            self.current_deployer = PlayerId::new(1);
        } else {
            self.current_deployer = PlayerId::new(0);
        }
    }

    /// Get which player should deploy next.
    pub fn next_deployer(&self) -> PlayerId {
        self.current_deployer
    }

    /// Total units deployed by both players.
    pub fn total_deployed(&self) -> usize {
        self.player_0_deployed + self.player_1_deployed
    }
}

// ─── ObjectiveSecuringRules ─────────────────────────────────────────────────

/// Rules for objective securing in Combat Patrol.
/// Source: CP_Rules.md - Objective Securing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveSecuringRules {
    /// BATTLELINE units secure objectives at the end of the Command Phase.
    pub battleline_secures_in_command_phase: bool,
    /// Secured objectives remain controlled even if the unit moves away,
    /// until the opponent controls it.
    pub secured_objectives_persist: bool,
}

impl Default for ObjectiveSecuringRules {
    fn default() -> Self {
        Self {
            battleline_secures_in_command_phase: true,
            secured_objectives_persist: true,
        }
    }
}

// ─── CombatPatrolRules ──────────────────────────────────────────────────────

/// Combat Patrol format-specific rules enforcement.
///
/// Provides static methods for CP-specific constraints on top of base 40K rules.
///
/// Source: CP_Rules.md
pub struct CombatPatrolRules;

impl CombatPatrolRules {
    /// Maximum number of battle rounds in a Combat Patrol game.
    /// Source: CP_Rules.md - "The battle lasts five battle rounds"
    pub fn max_battle_rounds() -> u8 {
        5
    }

    /// Starting Command Points for each player.
    /// Source: CP_Rules.md - "you start the battle with 0 Command points"
    pub fn starting_cp() -> CommandPoints {
        CommandPoints::new(0)
    }

    /// Maximum CP gain per round from the Command Phase (not counting abilities).
    /// Source: CP_Rules.md - "gain 1 Command point at the start of your Command phase"
    pub fn max_cp_gain_per_round() -> CommandPoints {
        CommandPoints::new(1)
    }

    /// Board dimensions for Combat Patrol.
    /// Source: CP_Rules.md - "44\" x 30\" battlefield"
    pub fn board_dimensions() -> BoardDimensions {
        BoardDimensions::COMBAT_PATROL
    }

    /// Returns the reserve arrival rules for Combat Patrol.
    /// Source: CP_Rules.md - Reserves:
    /// - No arrival in BR1
    /// - Must arrive by end of BR3
    /// - More than 9" from all enemy models
    pub fn reserve_arrival_rules() -> ReserveArrivalRules {
        ReserveArrivalRules::default()
    }

    /// Battle Ready bonus: 10VP for having a fully painted army.
    /// Source: CP_Rules.md - "Battle Ready bonus of 10 Victory Points"
    pub fn battle_ready_bonus() -> VictoryPoints {
        VictoryPoints::new(10)
    }

    /// Check if a unit is eligible for strategic reserves in Combat Patrol.
    ///
    /// In Combat Patrol, units that start in reserves must have the ability
    /// on their datasheet (e.g., Deep Strike). Units cannot voluntarily
    /// go into Strategic Reserves unless their datasheet allows it.
    ///
    /// A unit is eligible if:
    /// - It is not destroyed
    /// - It has the reserve_type set (DeepStrike or StrategicReserves)
    /// - It is currently undeployed or already in reserves
    pub fn is_unit_eligible_for_reserve(unit: &UnitState) -> bool {
        // Unit must not be destroyed
        if unit.is_destroyed() {
            return false;
        }

        // Unit must have a reserve type assigned (from its datasheet ability)
        if unit.reserve_type.is_none() {
            return false;
        }

        // Unit must be undeployed or already in reserves
        matches!(
            unit.status,
            UnitStatus::Undeployed | UnitStatus::InStrategicReserves
        )
    }

    /// Check if reserves can arrive in the given battle round.
    ///
    /// Source: CP_Rules.md:
    /// - Cannot arrive in battle round 1
    /// - Must arrive by end of battle round 3
    /// - After round 3, remaining reserves are destroyed
    pub fn check_reserve_timing(round: BattleRound) -> ReserveTimingResult {
        let round_num = round.number();

        if !(1..=5).contains(&round_num) {
            return ReserveTimingResult::InvalidRound;
        }

        if round_num == 1 {
            return ReserveTimingResult::TooEarly;
        }

        if round_num <= 3 {
            return ReserveTimingResult::Allowed;
        }

        // Rounds 4 and 5: reserves that haven't arrived are destroyed
        ReserveTimingResult::TooLate
    }

    /// Check if a patrol squad choice is valid for a given faction.
    ///
    /// Combat Patrols have a fixed roster with one choice slot
    /// (e.g., Custodes choose between Wardens or Allarus).
    /// The squad_index is 0 or 1 (the two options).
    ///
    /// This validates that the squad_index is within the valid range
    /// for the given faction. Actual faction validation requires content
    /// pack data, so this performs a basic range check.
    pub fn is_patrol_squad_choice_valid(
        _faction_id: FactionId,
        squad_index: usize,
    ) -> bool {
        // Combat Patrol typically has 2 choices (index 0 or 1)
        squad_index <= 1
    }

    /// Returns the objective securing rules for Combat Patrol.
    /// Source: CP_Rules.md - BATTLELINE units at end of Command Phase secure objectives
    pub fn objective_securing_rules() -> ObjectiveSecuringRules {
        ObjectiveSecuringRules::default()
    }

    /// Check if a unit can secure an objective (must be BATTLELINE and not battle-shocked).
    /// Source: CP_Rules.md - Only BATTLELINE units can secure objectives
    pub fn can_unit_secure_objective(unit: &UnitState) -> bool {
        unit.has_keyword(Keyword::Battleline)
            && !unit.battle_shocked
            && unit.is_on_battlefield()
            && !unit.is_destroyed()
    }

    /// Check if a position is valid for reserve arrival (more than 9" from all enemies).
    /// Returns true if the position is valid.
    pub fn is_valid_reserve_position(
        position: Position,
        enemy_positions: &[Position],
        board: &BoardDimensions,
    ) -> bool {
        // Must be on the board
        if !board.contains(position) {
            return false;
        }

        // Must be more than 9" from all enemy models
        let min_distance_sq: i64 = 9000i64 * 9000i64; // 9 inches in mils, squared
        for enemy_pos in enemy_positions {
            if position.distance_squared(*enemy_pos) <= min_distance_sq {
                return false;
            }
        }

        true
    }
}

/// Result of checking reserve timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReserveTimingResult {
    /// Reserves can arrive this round.
    Allowed,
    /// Too early (battle round 1).
    TooEarly,
    /// Too late (after battle round 3, reserves are destroyed).
    TooLate,
    /// Invalid battle round number.
    InvalidRound,
}

// ─── CombatPatrolSetup ─────────────────────────────────────────────────────

/// Manages the pre-battle sequence for Combat Patrol.
///
/// Tracks the current step and validates transitions through the setup process.
///
/// Source: CP_Rules.md - Pre-Battle Sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPatrolSetup {
    /// Current step in the pre-battle sequence.
    pub current_step: PreBattleStep,

    /// Result of the attacker/defender roll-off (player IDs).
    pub attacker: Option<PlayerId>,
    pub defender: Option<PlayerId>,

    /// Enhancement choices per player.
    pub enhancement_choices: [Option<usize>; 2],

    /// Secondary objective choices per player.
    pub secondary_choices: [Option<usize>; 2],

    /// Patrol squad choices per player.
    pub patrol_squad_choices: [Option<usize>; 2],

    /// Deployment tracking.
    pub deployment: Option<DeploymentAlternation>,

    /// First turn player (determined by roll-off after deployment).
    pub first_turn_player: Option<PlayerId>,

    /// Roll-off results for auditing.
    pub attacker_defender_rolls: Option<(u8, u8)>,
    pub first_turn_rolls: Option<(u8, u8)>,
}

impl CombatPatrolSetup {
    /// Create a new Combat Patrol setup sequence.
    pub fn new() -> Self {
        Self {
            current_step: PreBattleStep::DetermineAttackerDefender,
            attacker: None,
            defender: None,
            enhancement_choices: [None, None],
            secondary_choices: [None, None],
            patrol_squad_choices: [None, None],
            deployment: None,
            first_turn_player: None,
            attacker_defender_rolls: None,
            first_turn_rolls: None,
        }
    }

    /// Determine attacker and defender via roll-off.
    /// The player who rolls higher chooses to be attacker or defender.
    /// In case of a tie, re-roll (caller should re-roll until different).
    ///
    /// Returns Ok(()) if successful, or an error if the step is out of order
    /// or the rolls are tied.
    pub fn determine_attacker_defender(
        &mut self,
        player_0_roll: u8,
        player_1_roll: u8,
    ) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::DetermineAttackerDefender {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::DetermineAttackerDefender,
                actual: self.current_step,
            });
        }

        if player_0_roll == player_1_roll {
            return Err(CombatPatrolError::InvalidRollOff(
                "Tied roll-off, must re-roll".to_string(),
            ));
        }

        self.attacker_defender_rolls = Some((player_0_roll, player_1_roll));

        // Higher roll wins: the winner is the attacker (deploys first)
        if player_0_roll > player_1_roll {
            self.attacker = Some(PlayerId::new(0));
            self.defender = Some(PlayerId::new(1));
        } else {
            self.attacker = Some(PlayerId::new(1));
            self.defender = Some(PlayerId::new(0));
        }

        self.current_step = PreBattleStep::SelectEnhancement;
        Ok(())
    }

    /// Select enhancement for a player.
    /// enhancement_index is 0 for default, 1 for optional.
    pub fn select_enhancement(
        &mut self,
        player: PlayerId,
        enhancement_index: usize,
    ) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::SelectEnhancement {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::SelectEnhancement,
                actual: self.current_step,
            });
        }

        let player_idx = player.raw() as usize;
        if player_idx > 1 {
            return Err(CombatPatrolError::InvalidRollOff(
                "Invalid player index".to_string(),
            ));
        }

        self.enhancement_choices[player_idx] = Some(enhancement_index);

        // Both players must select before moving on
        if self.enhancement_choices[0].is_some() && self.enhancement_choices[1].is_some() {
            self.current_step = PreBattleStep::SelectSecondary;
        }

        Ok(())
    }

    /// Select secondary objective for a player.
    /// secondary_index is 0 for default, 1 for optional.
    pub fn select_secondary(
        &mut self,
        player: PlayerId,
        secondary_index: usize,
    ) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::SelectSecondary {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::SelectSecondary,
                actual: self.current_step,
            });
        }

        let player_idx = player.raw() as usize;
        if player_idx > 1 {
            return Err(CombatPatrolError::InvalidRollOff(
                "Invalid player index".to_string(),
            ));
        }

        self.secondary_choices[player_idx] = Some(secondary_index);

        // Both players must select before moving on
        if self.secondary_choices[0].is_some() && self.secondary_choices[1].is_some() {
            self.current_step = PreBattleStep::SelectPatrolSquad;
        }

        Ok(())
    }

    /// Select patrol squad choice for a player.
    /// squad_index is 0 or 1 (the two options).
    pub fn select_patrol_squad(
        &mut self,
        player: PlayerId,
        squad_index: usize,
    ) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::SelectPatrolSquad {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::SelectPatrolSquad,
                actual: self.current_step,
            });
        }

        if !CombatPatrolRules::is_patrol_squad_choice_valid(FactionId::new(0), squad_index) {
            return Err(CombatPatrolError::InvalidPatrolSquadChoice {
                faction_id: FactionId::new(0),
                squad_index,
            });
        }

        let player_idx = player.raw() as usize;
        if player_idx > 1 {
            return Err(CombatPatrolError::InvalidRollOff(
                "Invalid player index".to_string(),
            ));
        }

        self.patrol_squad_choices[player_idx] = Some(squad_index);

        // Both players must select before moving on
        if self.patrol_squad_choices[0].is_some() && self.patrol_squad_choices[1].is_some() {
            self.current_step = PreBattleStep::Deployment;
            // Initialize deployment alternation with attacker deploying first
            if let Some(attacker) = self.attacker {
                self.deployment = Some(DeploymentAlternation::new(attacker));
            }
        }

        Ok(())
    }

    /// Begin deployment alternation. The attacker deploys first.
    /// Returns the deployment alternation tracker.
    pub fn deployment_alternation(&self) -> Option<&DeploymentAlternation> {
        self.deployment.as_ref()
    }

    /// Get mutable deployment alternation tracker.
    pub fn deployment_alternation_mut(&mut self) -> Option<&mut DeploymentAlternation> {
        self.deployment.as_mut()
    }

    /// Mark deployment as complete and move to first turn determination.
    pub fn finish_deployment(&mut self) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::Deployment {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::Deployment,
                actual: self.current_step,
            });
        }

        self.current_step = PreBattleStep::DetermineFirstTurn;
        Ok(())
    }

    /// Determine first turn via roll-off.
    /// The player who did NOT finish deploying first chooses who goes first.
    /// In practice, the higher roller chooses.
    ///
    /// Source: CP_Rules.md - "the player who did not deploy first rolls off,
    /// the winner chooses who takes the first turn"
    pub fn determine_first_turn(
        &mut self,
        player_0_roll: u8,
        player_1_roll: u8,
    ) -> Result<(), CombatPatrolError> {
        if self.current_step != PreBattleStep::DetermineFirstTurn {
            return Err(CombatPatrolError::PreBattleStepOutOfOrder {
                expected: PreBattleStep::DetermineFirstTurn,
                actual: self.current_step,
            });
        }

        if player_0_roll == player_1_roll {
            return Err(CombatPatrolError::InvalidRollOff(
                "Tied roll-off for first turn, must re-roll".to_string(),
            ));
        }

        self.first_turn_rolls = Some((player_0_roll, player_1_roll));

        // Higher roller chooses who goes first.
        // For simplicity, the higher roller goes first.
        // In a real game, the winner of the roll-off *chooses* who goes first;
        // this implementation defaults to the winner going first.
        if player_0_roll > player_1_roll {
            self.first_turn_player = Some(PlayerId::new(0));
        } else {
            self.first_turn_player = Some(PlayerId::new(1));
        }

        self.current_step = PreBattleStep::Complete;
        Ok(())
    }

    /// Override the first turn choice (for when the roll-off winner
    /// chooses to let the opponent go first).
    pub fn set_first_turn_player(&mut self, player: PlayerId) {
        self.first_turn_player = Some(player);
    }

    /// Check if the pre-battle sequence is complete.
    pub fn is_complete(&self) -> bool {
        self.current_step == PreBattleStep::Complete
    }

    /// Get the current step.
    pub fn current_step(&self) -> PreBattleStep {
        self.current_step
    }
}

impl Default for CombatPatrolSetup {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        ArmorSave, BaseSize, DatasheetId, Inches, KeywordSet, Leadership,
        ModelId, MoveCharacteristic, ObjectiveControl, Position, Toughness,
        Wounds,
    };
    use wh40k_game_core::{ModelState, UnitState};

    fn make_test_unit(
        id: u32,
        keywords: KeywordSet,
        status: UnitStatus,
        reserve: bool,
    ) -> UnitState {
        let unit_id = UnitId::new(id);
        let model = ModelState::new(
            ModelId::new(id * 100),
            unit_id,
            Wounds::new(3),
            Position::from_inches(10, 10),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        );

        let mut unit = UnitState::new(
            unit_id,
            PlayerId::new(0),
            "Test Unit".to_string(),
            DatasheetId::new(1),
            keywords,
            vec![model],
            MoveCharacteristic::from_inches(6),
            Toughness::new(4),
            ArmorSave::THREE_PLUS,
            None,
            Leadership::new(7),
            ObjectiveControl::new(2),
        );

        unit.status = status;
        if reserve {
            unit.reserve_type = Some(wh40k_game_core::ReserveType::DeepStrike);
        }

        unit
    }

    // === CombatPatrolRules static methods ===

    #[test]
    fn test_max_battle_rounds() {
        assert_eq!(CombatPatrolRules::max_battle_rounds(), 5);
    }

    #[test]
    fn test_starting_cp() {
        assert_eq!(CombatPatrolRules::starting_cp().value(), 0);
    }

    #[test]
    fn test_max_cp_gain_per_round() {
        assert_eq!(CombatPatrolRules::max_cp_gain_per_round().value(), 1);
    }

    #[test]
    fn test_board_dimensions() {
        let dims = CombatPatrolRules::board_dimensions();
        assert_eq!(dims.width, Inches::from_inches(44));
        assert_eq!(dims.height, Inches::from_inches(30));
    }

    #[test]
    fn test_battle_ready_bonus() {
        assert_eq!(CombatPatrolRules::battle_ready_bonus().value(), 10);
    }

    #[test]
    fn test_reserve_arrival_rules() {
        let rules = CombatPatrolRules::reserve_arrival_rules();
        assert!(rules.no_arrival_round_1);
        assert_eq!(rules.must_arrive_by_round, 3);
        assert_eq!(rules.min_distance_from_enemies_mils, 9000);
    }

    #[test]
    fn test_objective_securing_rules() {
        let rules = CombatPatrolRules::objective_securing_rules();
        assert!(rules.battleline_secures_in_command_phase);
        assert!(rules.secured_objectives_persist);
    }

    // === Reserve timing ===

    #[test]
    fn test_reserve_timing_round_1_too_early() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(1)),
            ReserveTimingResult::TooEarly
        );
    }

    #[test]
    fn test_reserve_timing_round_2_allowed() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(2)),
            ReserveTimingResult::Allowed
        );
    }

    #[test]
    fn test_reserve_timing_round_3_allowed() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(3)),
            ReserveTimingResult::Allowed
        );
    }

    #[test]
    fn test_reserve_timing_round_4_too_late() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(4)),
            ReserveTimingResult::TooLate
        );
    }

    #[test]
    fn test_reserve_timing_round_5_too_late() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(5)),
            ReserveTimingResult::TooLate
        );
    }

    #[test]
    fn test_reserve_timing_invalid_round_0() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(0)),
            ReserveTimingResult::InvalidRound
        );
    }

    #[test]
    fn test_reserve_timing_invalid_round_6() {
        assert_eq!(
            CombatPatrolRules::check_reserve_timing(BattleRound::new(6)),
            ReserveTimingResult::InvalidRound
        );
    }

    // === Unit eligibility for reserves ===

    #[test]
    fn test_unit_eligible_for_reserve_with_deep_strike() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            UnitStatus::InStrategicReserves,
            true,
        );
        assert!(CombatPatrolRules::is_unit_eligible_for_reserve(&unit));
    }

    #[test]
    fn test_unit_not_eligible_for_reserve_no_reserve_type() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            UnitStatus::Undeployed,
            false,
        );
        assert!(!CombatPatrolRules::is_unit_eligible_for_reserve(&unit));
    }

    #[test]
    fn test_unit_not_eligible_for_reserve_on_battlefield() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            UnitStatus::OnBattlefield,
            true,
        );
        assert!(!CombatPatrolRules::is_unit_eligible_for_reserve(&unit));
    }

    #[test]
    fn test_unit_not_eligible_for_reserve_destroyed() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            UnitStatus::Destroyed,
            true,
        );
        assert!(!CombatPatrolRules::is_unit_eligible_for_reserve(&unit));
    }

    // === Patrol squad choice ===

    #[test]
    fn test_patrol_squad_choice_valid() {
        assert!(CombatPatrolRules::is_patrol_squad_choice_valid(
            FactionId::new(1),
            0
        ));
        assert!(CombatPatrolRules::is_patrol_squad_choice_valid(
            FactionId::new(1),
            1
        ));
    }

    #[test]
    fn test_patrol_squad_choice_invalid() {
        assert!(!CombatPatrolRules::is_patrol_squad_choice_valid(
            FactionId::new(1),
            2
        ));
    }

    // === Objective securing ===

    #[test]
    fn test_can_secure_objective_battleline() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            UnitStatus::OnBattlefield,
            false,
        );
        assert!(CombatPatrolRules::can_unit_secure_objective(&unit));
    }

    #[test]
    fn test_cannot_secure_objective_not_battleline() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]),
            UnitStatus::OnBattlefield,
            false,
        );
        assert!(!CombatPatrolRules::can_unit_secure_objective(&unit));
    }

    #[test]
    fn test_cannot_secure_objective_battle_shocked() {
        let mut unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            UnitStatus::OnBattlefield,
            false,
        );
        unit.battle_shocked = true;
        assert!(!CombatPatrolRules::can_unit_secure_objective(&unit));
    }

    #[test]
    fn test_cannot_secure_objective_destroyed() {
        let unit = make_test_unit(
            1,
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            UnitStatus::Destroyed,
            false,
        );
        assert!(!CombatPatrolRules::can_unit_secure_objective(&unit));
    }

    // === Reserve position validation ===

    #[test]
    fn test_valid_reserve_position_no_enemies() {
        let board = BoardDimensions::COMBAT_PATROL;
        let pos = Position::from_inches(22, 15);
        assert!(CombatPatrolRules::is_valid_reserve_position(
            pos,
            &[],
            &board
        ));
    }

    #[test]
    fn test_valid_reserve_position_far_from_enemies() {
        let board = BoardDimensions::COMBAT_PATROL;
        let pos = Position::from_inches(22, 15);
        let enemies = vec![Position::from_inches(0, 0)];
        assert!(CombatPatrolRules::is_valid_reserve_position(
            pos,
            &enemies,
            &board
        ));
    }

    #[test]
    fn test_invalid_reserve_position_too_close() {
        let board = BoardDimensions::COMBAT_PATROL;
        let pos = Position::from_inches(10, 10);
        let enemies = vec![Position::from_inches(15, 10)]; // 5" away
        assert!(!CombatPatrolRules::is_valid_reserve_position(
            pos,
            &enemies,
            &board
        ));
    }

    #[test]
    fn test_invalid_reserve_position_off_board() {
        let board = BoardDimensions::COMBAT_PATROL;
        let pos = Position::from_inches(50, 15); // Off the 44" wide board
        assert!(!CombatPatrolRules::is_valid_reserve_position(
            pos,
            &[],
            &board
        ));
    }

    // === CombatPatrolSetup ===

    #[test]
    fn test_setup_initial_state() {
        let setup = CombatPatrolSetup::new();
        assert_eq!(
            setup.current_step(),
            PreBattleStep::DetermineAttackerDefender
        );
        assert!(!setup.is_complete());
    }

    #[test]
    fn test_setup_determine_attacker_defender() {
        let mut setup = CombatPatrolSetup::new();
        setup.determine_attacker_defender(5, 3).unwrap();
        assert_eq!(setup.attacker, Some(PlayerId::new(0)));
        assert_eq!(setup.defender, Some(PlayerId::new(1)));
        assert_eq!(setup.current_step(), PreBattleStep::SelectEnhancement);
    }

    #[test]
    fn test_setup_determine_attacker_defender_player_1_wins() {
        let mut setup = CombatPatrolSetup::new();
        setup.determine_attacker_defender(2, 6).unwrap();
        assert_eq!(setup.attacker, Some(PlayerId::new(1)));
        assert_eq!(setup.defender, Some(PlayerId::new(0)));
    }

    #[test]
    fn test_setup_determine_attacker_defender_tie() {
        let mut setup = CombatPatrolSetup::new();
        let result = setup.determine_attacker_defender(4, 4);
        assert!(result.is_err());
        // Step should not have advanced
        assert_eq!(
            setup.current_step(),
            PreBattleStep::DetermineAttackerDefender
        );
    }

    #[test]
    fn test_setup_full_sequence() {
        let mut setup = CombatPatrolSetup::new();

        // Step 1: Determine attacker/defender
        setup.determine_attacker_defender(5, 3).unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::SelectEnhancement);

        // Step 2: Select enhancements
        setup
            .select_enhancement(PlayerId::new(0), 0)
            .unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::SelectEnhancement); // waiting for p1
        setup
            .select_enhancement(PlayerId::new(1), 1)
            .unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::SelectSecondary);

        // Step 3: Select secondaries
        setup
            .select_secondary(PlayerId::new(0), 0)
            .unwrap();
        setup
            .select_secondary(PlayerId::new(1), 0)
            .unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::SelectPatrolSquad);

        // Step 4: Select patrol squads
        setup
            .select_patrol_squad(PlayerId::new(0), 0)
            .unwrap();
        setup
            .select_patrol_squad(PlayerId::new(1), 1)
            .unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::Deployment);

        // Step 5: Deployment
        assert!(setup.deployment_alternation().is_some());
        let deployer = setup.deployment_alternation().unwrap().next_deployer();
        assert_eq!(deployer, PlayerId::new(0)); // Attacker deploys first

        setup
            .deployment_alternation_mut()
            .unwrap()
            .record_deployment(PlayerId::new(0));
        let next = setup.deployment_alternation().unwrap().next_deployer();
        assert_eq!(next, PlayerId::new(1));

        setup.finish_deployment().unwrap();
        assert_eq!(setup.current_step(), PreBattleStep::DetermineFirstTurn);

        // Step 6: Determine first turn
        setup.determine_first_turn(3, 5).unwrap();
        assert!(setup.is_complete());
        assert_eq!(setup.first_turn_player, Some(PlayerId::new(1)));
    }

    #[test]
    fn test_setup_out_of_order_error() {
        let mut setup = CombatPatrolSetup::new();
        let result = setup.select_enhancement(PlayerId::new(0), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_setup_set_first_turn_override() {
        let mut setup = CombatPatrolSetup::new();
        setup.determine_attacker_defender(5, 3).unwrap();
        setup
            .select_enhancement(PlayerId::new(0), 0)
            .unwrap();
        setup
            .select_enhancement(PlayerId::new(1), 0)
            .unwrap();
        setup
            .select_secondary(PlayerId::new(0), 0)
            .unwrap();
        setup
            .select_secondary(PlayerId::new(1), 0)
            .unwrap();
        setup
            .select_patrol_squad(PlayerId::new(0), 0)
            .unwrap();
        setup
            .select_patrol_squad(PlayerId::new(1), 0)
            .unwrap();
        setup.finish_deployment().unwrap();
        setup.determine_first_turn(6, 2).unwrap();

        // Winner chose to go first by default
        assert_eq!(setup.first_turn_player, Some(PlayerId::new(0)));

        // Override: winner chooses opponent to go first
        setup.set_first_turn_player(PlayerId::new(1));
        assert_eq!(setup.first_turn_player, Some(PlayerId::new(1)));
    }

    // === DeploymentAlternation ===

    #[test]
    fn test_deployment_alternation() {
        let mut dep = DeploymentAlternation::new(PlayerId::new(0));
        assert_eq!(dep.next_deployer(), PlayerId::new(0));
        assert_eq!(dep.total_deployed(), 0);

        dep.record_deployment(PlayerId::new(0));
        assert_eq!(dep.next_deployer(), PlayerId::new(1));
        assert_eq!(dep.player_0_deployed, 1);

        dep.record_deployment(PlayerId::new(1));
        assert_eq!(dep.next_deployer(), PlayerId::new(0));
        assert_eq!(dep.player_1_deployed, 1);
        assert_eq!(dep.total_deployed(), 2);
    }

    // === PreBattleStep ===

    #[test]
    fn test_pre_battle_step_sequence() {
        let step = PreBattleStep::DetermineAttackerDefender;
        assert_eq!(step.next(), Some(PreBattleStep::SelectEnhancement));

        let step = PreBattleStep::SelectEnhancement;
        assert_eq!(step.next(), Some(PreBattleStep::SelectSecondary));

        let step = PreBattleStep::SelectSecondary;
        assert_eq!(step.next(), Some(PreBattleStep::SelectPatrolSquad));

        let step = PreBattleStep::SelectPatrolSquad;
        assert_eq!(step.next(), Some(PreBattleStep::Deployment));

        let step = PreBattleStep::Deployment;
        assert_eq!(step.next(), Some(PreBattleStep::DetermineFirstTurn));

        let step = PreBattleStep::DetermineFirstTurn;
        assert_eq!(step.next(), Some(PreBattleStep::Complete));

        let step = PreBattleStep::Complete;
        assert_eq!(step.next(), None);
    }
}
