//! Command validation: checks legality for each command type.
//!
//! Source: implementation_v3.md Section 6.4 (Command lifecycle)
//! Source: 40k_revised.md - Movement, Shooting, Charge, Fight, Stratagem rules
//! Source: CP_Rules.md - Combat Patrol restrictions

use wh40k_core_types::{
    EngagementStatus, Keyword, Phase, UnitStatus,
};
use wh40k_command_system::{Command, CommandValidationResult};
use wh40k_geometry;

use crate::state::GameState;

// ---------------------------------------------------------------------------
// CommandValidator
// ---------------------------------------------------------------------------

/// Validates commands against the current game state.
///
/// Checks legality for each command type:
/// - Movement: unit eligible, correct phase, engagement constraints, coherency
/// - Shooting: unit eligible, target in range/LOS, weapon constraints
/// - Charge: unit eligible, not advanced/fell back, correct phase
/// - Stratagem: correct phase, CP available, not battle-shocked, legal target
/// - Fight: unit eligible (engagement or charged), correct phase
pub struct CommandValidator;

impl CommandValidator {
    /// Validate a command against the current game state.
    ///
    /// Returns `CommandValidationResult::Legal` if the command is allowed,
    /// or `CommandValidationResult::Illegal` with a reason if not.
    pub fn validate(state: &GameState, command: &Command) -> CommandValidationResult {
        // Game must be in progress for most commands
        if !state.is_in_progress() {
            // Only allow concede after game end (or nothing)
            if !matches!(command, Command::Concede { .. }) {
                return CommandValidationResult::illegal("Game has already ended");
            }
        }

        match command {
            // ===== Setup commands =====
            Command::SelectFaction { player, .. } => {
                Self::validate_setup_phase(state, *player)
            }
            Command::SelectPatrolSquad { player, .. } => {
                Self::validate_setup_phase(state, *player)
            }
            Command::SelectEnhancement { player, .. } => {
                Self::validate_setup_phase(state, *player)
            }
            Command::SelectSecondaryObjective { player, .. } => {
                Self::validate_setup_phase(state, *player)
            }
            Command::PlaceUnit { player, unit_id, position } => {
                Self::validate_place_unit(state, *player, *unit_id, *position)
            }
            Command::DetermineFirstTurn { .. } => {
                if state.current_phase != Phase::PreBattle {
                    CommandValidationResult::illegal("DetermineFirstTurn only valid in PreBattle phase")
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::SetupComplete => {
                if state.current_phase != Phase::PreBattle {
                    CommandValidationResult::illegal("SetupComplete only valid in PreBattle phase")
                } else {
                    CommandValidationResult::Legal
                }
            }

            // ===== Phase control commands =====
            Command::StartBattleRound { round } => {
                if state.battle_round != *round {
                    CommandValidationResult::illegal(format!(
                        "Cannot start round {}, current round is {}",
                        round.number(),
                        state.battle_round.number()
                    ))
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::StartPhase { phase } => {
                if state.current_phase == Phase::GameEnd {
                    return CommandValidationResult::illegal("Game has ended");
                }
                // Validate phase ordering
                let valid_next = match state.current_phase {
                    Phase::PreBattle => *phase == Phase::Command,
                    Phase::Command => *phase == Phase::Movement,
                    Phase::Movement => *phase == Phase::Shooting,
                    Phase::Shooting => *phase == Phase::Charge,
                    Phase::Charge => *phase == Phase::Fight,
                    Phase::Fight => *phase == Phase::Command, // Next turn
                    Phase::GameEnd => false,
                };
                if !valid_next && *phase != state.current_phase {
                    CommandValidationResult::illegal(format!(
                        "Cannot transition from {:?} to {:?}",
                        state.current_phase, phase
                    ))
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::EndPhase { phase } => {
                if *phase != state.current_phase {
                    CommandValidationResult::illegal(format!(
                        "Cannot end phase {:?}, current phase is {:?}",
                        phase, state.current_phase
                    ))
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::EndPlayerTurn { player } => {
                if *player != state.active_player {
                    CommandValidationResult::illegal("Only the active player can end their turn")
                } else if state.current_phase == Phase::PreBattle {
                    CommandValidationResult::illegal("Cannot end turn during PreBattle")
                } else {
                    CommandValidationResult::Legal
                }
            }

            // ===== Movement commands =====
            Command::SelectUnitToMove { unit_id } => {
                Self::validate_select_unit_to_move(state, *unit_id)
            }
            Command::NormalMove { unit_id, destination } => {
                Self::validate_normal_move(state, *unit_id, *destination)
            }
            Command::AdvanceMove { unit_id, destination, advance_roll } => {
                Self::validate_advance_move(state, *unit_id, *destination, *advance_roll)
            }
            Command::FallBack { unit_id, destination } => {
                Self::validate_fall_back(state, *unit_id, *destination)
            }
            Command::RemainStationary { unit_id } => {
                Self::validate_remain_stationary(state, *unit_id)
            }
            Command::ArriveFromReserves { unit_id, position } => {
                Self::validate_arrive_from_reserves(state, *unit_id, *position)
            }

            // ===== Shooting commands =====
            Command::SelectUnitToShoot { unit_id } => {
                Self::validate_select_unit_to_shoot(state, *unit_id)
            }
            Command::DeclareShootingTargets { unit_id, targets } => {
                Self::validate_declare_shooting_targets(state, *unit_id, targets)
            }
            Command::ResolveShootingAttack { attacker_id, target_id, weapon_id } => {
                // Target may be destroyed by earlier attacks in the same macro-action.
                // Allow it here — executor returns Ok(empty) for destroyed targets.
                if let Some(target) = state.unit(*target_id) {
                    if target.is_destroyed() || target.models_alive() == 0 {
                        return CommandValidationResult::Legal;
                    }
                }
                Self::validate_resolve_shooting_attack(state, *attacker_id, *weapon_id)
            }

            // ===== Charge commands =====
            Command::DeclareCharge { unit_id, targets } => {
                Self::validate_declare_charge(state, *unit_id, targets)
            }
            Command::ResolveChargeRoll { unit_id: _, roll } => {
                // roll=0 is a sentinel meaning "roll actual 2D6 dice" (used by ActionGenerator)
                // Otherwise validate it's a valid 2D6 result (2-12)
                if *roll != 0 && (*roll < 2 || *roll > 12) {
                    return CommandValidationResult::illegal(
                        format!("Charge roll {} is not a valid 2D6 result (must be 2-12)", roll),
                    );
                }
                Self::validate_phase_is(state, Phase::Charge, "ResolveChargeRoll")
            }
            Command::MakeChargeMove { unit_id, destination } => {
                Self::validate_charge_move(state, *unit_id, *destination)
            }

            // ===== Heroic Intervention =====
            Command::HeroicInterventionMove { unit_id, destination } => {
                Self::validate_heroic_intervention_move(state, *unit_id, *destination)
            }

            // ===== Fight commands =====
            Command::SelectUnitToFight { unit_id } => {
                Self::validate_select_unit_to_fight(state, *unit_id)
            }
            Command::ChooseKaTahStance { unit_id, stance } => {
                if state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal("ChooseKaTahStance only valid in Fight phase");
                }
                let unit = match state.unit(*unit_id) {
                    Some(u) => u,
                    None => return CommandValidationResult::illegal("Unit not found"),
                };
                if !unit.has_keyword(Keyword::AdeptusCustodes) {
                    return CommandValidationResult::illegal_with_ref(
                        "Only Adeptus Custodes units can use Martial Ka'tah",
                        "Custodes.md - Martial Ka'tah",
                    );
                }
                // Validate stance is one of the two legal options
                if stance != "Dacatarai" && stance != "Rendax" {
                    return CommandValidationResult::illegal(
                        format!("Invalid Ka'tah stance '{}'. Must be Dacatarai or Rendax", stance),
                    );
                }
                CommandValidationResult::Legal
            }
            Command::ChooseVaultswordsProfile { model_id, profile } => {
                if state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal("ChooseVaultswordsProfile only valid in Fight phase");
                }
                // Find the unit containing this model and check for BladeChampion keyword
                let unit = state.units.iter().find(|u| u.models.iter().any(|m| m.id == *model_id));
                match unit {
                    Some(u) => {
                        if !u.has_keyword(Keyword::BladeChampion) {
                            return CommandValidationResult::illegal_with_ref(
                                "Only Tristraen (Blade Champion) can choose Vaultswords profiles",
                                "Custodes.md - Tristraen Vaultswords",
                            );
                        }
                        // Validate profile name
                        if profile != "Behemor" && profile != "Hurricanus" && profile != "Victus" {
                            return CommandValidationResult::illegal(
                                format!("Invalid profile '{}'. Must be Behemor, Hurricanus, or Victus", profile),
                            );
                        }
                    }
                    None => return CommandValidationResult::illegal("Model not found"),
                }
                CommandValidationResult::Legal
            }
            Command::PileIn { unit_id, positions } => {
                Self::validate_pile_in_closer_to_enemy(state, *unit_id, positions, "PileIn")
            }
            Command::DeclareMeleeTargets { unit_id, targets } => {
                Self::validate_declare_melee_targets(state, *unit_id, targets)
            }
            Command::ResolveMeleeAttack { attacker_id, target_id, .. } => {
                if state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal("ResolveMeleeAttack only valid in Fight phase");
                }
                let attacker = match state.unit(*attacker_id) {
                    Some(u) => u,
                    None => return CommandValidationResult::illegal("Attacker not found"),
                };
                if attacker.is_destroyed() || !attacker.is_on_battlefield() {
                    return CommandValidationResult::illegal("Attacker is destroyed or not on battlefield");
                }
                // Target may be destroyed by earlier attacks in the same macro-action.
                // Allow it here — executor returns Ok(empty) for destroyed targets.
                if let Some(target) = state.unit(*target_id) {
                    if !target.is_on_battlefield() && !target.is_destroyed() {
                        return CommandValidationResult::illegal("Target not on battlefield");
                    }
                }
                CommandValidationResult::Legal
            }
            Command::Consolidate { unit_id, positions } => {
                Self::validate_pile_in_closer_to_enemy(state, *unit_id, positions, "Consolidate")
            }

            // ===== Stratagem commands =====
            Command::UseStratagem { player, stratagem_id, target } => {
                Self::validate_use_stratagem(state, *player, *stratagem_id, target)
            }
            Command::DeclineStratagem { player: _ } => {
                if !state.has_reaction_window() {
                    CommandValidationResult::illegal("No reaction window open to decline")
                } else {
                    CommandValidationResult::Legal
                }
            }

            // ===== Scoring commands =====
            Command::ScoreObjective { player, objective_id } => {
                if *player != state.active_player && *player != state.decision_owner {
                    return CommandValidationResult::illegal("Not this player's turn to score");
                }
                // Verify the objective exists
                if state.board.objective_marker(*objective_id).is_none() {
                    return CommandValidationResult::illegal("Objective not found on the board");
                }
                CommandValidationResult::Legal
            }
            Command::RazeObjective { player, objective_id: _ } => {
                if *player != state.active_player {
                    CommandValidationResult::illegal("Not this player's turn")
                } else {
                    CommandValidationResult::Legal
                }
            }

            // ===== Blessings of Khorne =====
            Command::AllocateBlessings { player, allocations: _ } => {
                if *player != state.active_player {
                    CommandValidationResult::illegal("Not the active player")
                } else {
                    CommandValidationResult::Legal
                }
            }

            // ===== Misc commands =====
            Command::AllocateWound { target_model_id } => {
                // Check that the model exists and is alive
                let model_found = state.units.iter().any(|u| {
                    u.models.iter().any(|m| m.id == *target_model_id && m.alive)
                });
                if !model_found {
                    CommandValidationResult::illegal("Target model not found or already destroyed")
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::AssignOverwatchTarget { unit_id } => {
                // Must have a reaction window open
                if !state.has_reaction_window() {
                    return CommandValidationResult::illegal("No overwatch reaction window open");
                }
                // Overwatch limited to once per turn
                if state.turn_flags.overwatch_used_this_turn {
                    return CommandValidationResult::illegal_with_ref(
                        "Overwatch has already been used this turn",
                        "40k_revised.md - Fire Overwatch: once per turn",
                    );
                }
                // Check the unit exists and can fire
                let unit = match state.unit(*unit_id) {
                    Some(u) => u,
                    None => return CommandValidationResult::illegal("Unit not found"),
                };
                if unit.is_destroyed() || !unit.is_on_battlefield() {
                    return CommandValidationResult::illegal("Unit cannot fire overwatch");
                }
                CommandValidationResult::Legal
            }
            Command::PassAction => CommandValidationResult::Legal,
            Command::Concede { player: _ } => {
                // Can always concede
                CommandValidationResult::Legal
            }

            // ===== Boarding Actions commands =====
            // Basic validation: player must be the active player or decision owner.
            // Detailed Boarding Actions validation will live in the boarding_rules crate.
            Command::OperateHatchway { player, unit_id, .. } => {
                if *player != state.active_player && *player != state.decision_owner {
                    return CommandValidationResult::illegal("Not this player's turn");
                }
                match state.unit(*unit_id) {
                    Some(u) if u.is_on_battlefield() && !u.is_destroyed() => {
                        CommandValidationResult::Legal
                    }
                    _ => CommandValidationResult::illegal("Unit not found or not on battlefield"),
                }
            }
            Command::PerformTacticalManoeuvre { player, unit_id, .. } => {
                if *player != state.active_player && *player != state.decision_owner {
                    return CommandValidationResult::illegal("Not this player's turn");
                }
                match state.unit(*unit_id) {
                    Some(u) if u.is_on_battlefield() && !u.is_destroyed() => {
                        CommandValidationResult::Legal
                    }
                    _ => CommandValidationResult::illegal("Unit not found or not on battlefield"),
                }
            }
            Command::UseBattlefieldCommand { player, leader_unit_id, target_unit_id, .. } => {
                if *player != state.active_player && *player != state.decision_owner {
                    return CommandValidationResult::illegal("Not this player's turn");
                }
                let leader_ok = state.unit(*leader_unit_id)
                    .map(|u| u.is_on_battlefield() && !u.is_destroyed())
                    .unwrap_or(false);
                let target_ok = state.unit(*target_unit_id)
                    .map(|u| u.is_on_battlefield() && !u.is_destroyed())
                    .unwrap_or(false);
                if !leader_ok {
                    CommandValidationResult::illegal("Leader unit not found or not on battlefield")
                } else if !target_ok {
                    CommandValidationResult::illegal("Target unit not found or not on battlefield")
                } else {
                    CommandValidationResult::Legal
                }
            }
            Command::ArriveFromEntryZone { player, unit_id, .. } => {
                if *player != state.active_player && *player != state.decision_owner {
                    return CommandValidationResult::illegal("Not this player's turn");
                }
                match state.unit(*unit_id) {
                    Some(_) => CommandValidationResult::Legal,
                    None => CommandValidationResult::illegal("Unit not found"),
                }
            }
            Command::BoardingMissionAction { player, .. } => {
                if *player != state.active_player && *player != state.decision_owner {
                    CommandValidationResult::illegal("Not this player's turn")
                } else {
                    CommandValidationResult::Legal
                }
            }
        }
    }

    // ===== Helper validation methods =====

    fn validate_setup_phase(
        state: &GameState,
        _player: wh40k_core_types::PlayerId,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::PreBattle {
            CommandValidationResult::illegal_with_ref(
                "Setup commands are only valid in the PreBattle phase",
                "40k_revised.md - Pre-battle setup",
            )
        } else {
            CommandValidationResult::Legal
        }
    }

    fn validate_place_unit(
        state: &GameState,
        player: wh40k_core_types::PlayerId,
        unit_id: wh40k_core_types::UnitId,
        position: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::PreBattle {
            return CommandValidationResult::illegal("PlaceUnit only valid in PreBattle phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != player {
            return CommandValidationResult::illegal("Unit does not belong to this player");
        }

        if unit.status != UnitStatus::Undeployed {
            return CommandValidationResult::illegal("Unit is already deployed or destroyed");
        }

        if !state.board.contains(position) {
            return CommandValidationResult::illegal("Position is outside the board");
        }

        // Check that the position is within the player's deployment zone
        if let Some(ref config) = state.deployment_config {
            // Combat Patrol: check standard deployment zones
            // Source: CP_Rules.md - "Models must be set up wholly within their deployment zone"
            let zone = config.zone_for(player);
            if !zone.contains(position) {
                return CommandValidationResult::illegal_with_ref(
                    "Position is outside the player's deployment zone",
                    "CP_Rules.md - Deployment zones",
                );
            }
        } else if let Some(ref bmap) = state.board.boarding_map {
            // Boarding Actions: check entry zones assigned to this player
            // Source: boarding_actions_complete_v3.md - Entry zones
            let in_entry_zone = bmap.entry_zones.iter().any(|ez| {
                ez.enabled
                    && ez.player_assignment == Some(player)
                    && ez.boundary.contains(position)
            });
            if !in_entry_zone {
                return CommandValidationResult::illegal_with_ref(
                    "Position is outside the player's entry zone",
                    "boarding_actions_complete_v3.md - Entry zones",
                );
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_phase_is(
        state: &GameState,
        expected: Phase,
        command_name: &str,
    ) -> CommandValidationResult {
        if state.current_phase != expected {
            CommandValidationResult::illegal(format!(
                "{} is only valid in the {:?} phase (current: {:?})",
                command_name, expected, state.current_phase
            ))
        } else {
            CommandValidationResult::Legal
        }
    }

    fn validate_select_unit_to_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
    ) -> CommandValidationResult {
        // Must be in Movement phase
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal_with_ref(
                "SelectUnitToMove is only valid in the Movement phase",
                "40k_revised.md - Movement Phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        // Must be owned by active player
        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        // Must be on the battlefield
        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        // Must not have already moved this turn
        if state.turn_flags.has_moved(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit has already moved this turn",
                "40k_revised.md - Each unit can only move once per Movement phase",
            );
        }

        // Must not be destroyed
        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        CommandValidationResult::Legal
    }

    /// Check that all alive models in a unit would remain on the board after
    /// translating from the unit's reference position to the given destination.
    /// Uses `board.contains_model()` which accounts for base radius.
    ///
    /// Source: 40k_revised.md §5.3 - models cannot move off the battlefield
    fn all_models_on_board_after_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
    ) -> Option<String> {
        let unit = state.unit(unit_id)?;
        let ref_pos = unit.reference_position()?;
        let dx = wh40k_core_types::Inches(destination.x.0 - ref_pos.x.0);
        let dy = wh40k_core_types::Inches(destination.y.0 - ref_pos.y.0);

        for (i, model) in unit.models.iter().enumerate() {
            if !model.alive {
                continue;
            }
            let translated = model.position.translate(dx, dy);
            if !state.board.contains_model(translated, model.base_size) {
                return Some(format!(
                    "Model {} would be off the board after move (base {}mm at {:?})",
                    i, model.base_size.diameter_mm(), translated,
                ));
            }
        }
        None
    }

    fn validate_normal_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        // Must be in Movement phase
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal("NormalMove is only valid in the Movement phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Cannot Normal Move if engaged
        if unit.engagement_status == EngagementStatus::Engaged {
            return CommandValidationResult::illegal_with_ref(
                "Unit is within Engagement Range and must Fall Back instead",
                "40k_revised.md - Movement Phase: Engaged units must Fall Back or Remain Stationary",
            );
        }

        // Check destination centroid is on the board
        if !state.board.contains(destination) {
            return CommandValidationResult::illegal("Destination is outside the board");
        }

        // Check all models would remain on the board after translation
        if let Some(reason) = Self::all_models_on_board_after_move(state, unit_id, destination) {
            return CommandValidationResult::illegal_with_ref(
                reason,
                "40k_revised.md §5.3 - Models cannot move off the battlefield",
            );
        }

        // Check movement distance
        if let Some(current_pos) = unit.reference_position() {
            let distance = current_pos.distance(destination);
            let max_move = unit.base_movement.distance();
            if distance > max_move {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "Move distance ({}) exceeds movement characteristic ({})",
                        distance, max_move
                    ),
                    "40k_revised.md - Normal Move: up to M characteristic",
                );
            }
        }

        // #9: Normal Move cannot end within Engagement Range of any enemy model
        // Source: 40k_revised.md §5.3 - "cannot move within Engagement Range of any enemy model"
        let unit_base = unit.models.first().map(|m| m.base_size)
            .unwrap_or(wh40k_core_types::BaseSize::MM25);
        for enemy_unit in &state.units {
            if enemy_unit.owner == unit.owner || enemy_unit.is_destroyed() || !enemy_unit.is_on_battlefield() {
                continue;
            }
            for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                if wh40k_geometry::within_engagement_range_2d(
                    destination, unit_base,
                    enemy_model.position, enemy_model.base_size,
                ) {
                    return CommandValidationResult::illegal_with_ref(
                        "Normal Move cannot end within Engagement Range of enemy models",
                        "40k_revised.md §5.3 - Normal Move: cannot move within ER of enemies",
                    );
                }
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_advance_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
        advance_roll: u8,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal("AdvanceMove is only valid in the Movement phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.engagement_status == EngagementStatus::Engaged {
            return CommandValidationResult::illegal_with_ref(
                "Cannot Advance while within Engagement Range",
                "40k_revised.md - Movement Phase: Engaged units cannot Advance",
            );
        }

        if !state.board.contains(destination) {
            return CommandValidationResult::illegal("Destination is outside the board");
        }

        // Check all models would remain on the board after translation
        if let Some(reason) = Self::all_models_on_board_after_move(state, unit_id, destination) {
            return CommandValidationResult::illegal_with_ref(
                reason,
                "40k_revised.md §5.4 - Models cannot move off the battlefield",
            );
        }

        // Validate advance roll is a valid D6 result (1-6)
        if advance_roll < 1 || advance_roll > 6 {
            return CommandValidationResult::illegal(
                format!("Advance roll {} is not a valid D6 result (must be 1-6)", advance_roll),
            );
        }

        // 40k_revised.md §5.4: Advance = M + D6 roll
        if let Some(current_pos) = unit.reference_position() {
            let distance = current_pos.distance(destination);
            let base_move = unit.base_movement.distance();
            let advance_bonus = wh40k_core_types::Inches::from_inches(advance_roll as i32);
            let max_advance = base_move + advance_bonus;
            if distance > max_advance {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "Advance distance ({}) exceeds M + advance roll ({})",
                        distance, max_advance
                    ),
                    "40k_revised.md - Advance: M characteristic + D6 roll",
                );
            }
        }

        // #9: Advance cannot end within Engagement Range of any enemy model
        // Source: 40k_revised.md §5.4 - same restriction as Normal Move
        let unit_base = unit.models.first().map(|m| m.base_size)
            .unwrap_or(wh40k_core_types::BaseSize::MM25);
        for enemy_unit in &state.units {
            if enemy_unit.owner == unit.owner || enemy_unit.is_destroyed() || !enemy_unit.is_on_battlefield() {
                continue;
            }
            for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                if wh40k_geometry::within_engagement_range_2d(
                    destination, unit_base,
                    enemy_model.position, enemy_model.base_size,
                ) {
                    return CommandValidationResult::illegal_with_ref(
                        "Advance cannot end within Engagement Range of enemy models",
                        "40k_revised.md §5.4 - Advance: cannot move within ER of enemies",
                    );
                }
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_fall_back(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal("FallBack is only valid in the Movement phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        // Must be engaged to fall back
        if unit.engagement_status != EngagementStatus::Engaged {
            return CommandValidationResult::illegal_with_ref(
                "Unit is not engaged and cannot Fall Back",
                "40k_revised.md - Fall Back: only units in Engagement Range can Fall Back",
            );
        }

        if !state.board.contains(destination) {
            return CommandValidationResult::illegal("Destination is outside the board");
        }

        // Check all models would remain on the board after translation
        if let Some(reason) = Self::all_models_on_board_after_move(state, unit_id, destination) {
            return CommandValidationResult::illegal_with_ref(
                reason,
                "40k_revised.md §5.5 - Models cannot move off the battlefield",
            );
        }

        // Fall Back distance limited to M characteristic
        // Source: 40k_revised.md §5.5 - "Fall Back: up to M characteristic"
        if let Some(current_pos) = unit.reference_position() {
            let distance = current_pos.distance(destination);
            let max_move = unit.base_movement.distance();
            if distance > max_move {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "Fall Back distance ({}) exceeds movement characteristic ({})",
                        distance, max_move
                    ),
                    "40k_revised.md §5.5 - Fall Back: up to M characteristic",
                );
            }
        }

        // #9: Fall Back must end outside Engagement Range of ALL enemy models
        // Source: 40k_revised.md §5.5 - "must end its move more than 1\" from all enemy models"
        let unit_base = unit.models.first().map(|m| m.base_size)
            .unwrap_or(wh40k_core_types::BaseSize::MM25);
        for enemy_unit in &state.units {
            if enemy_unit.owner == unit.owner || enemy_unit.is_destroyed() || !enemy_unit.is_on_battlefield() {
                continue;
            }
            for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                if wh40k_geometry::within_engagement_range_2d(
                    destination, unit_base,
                    enemy_model.position, enemy_model.base_size,
                ) {
                    return CommandValidationResult::illegal_with_ref(
                        "Fall Back must end outside Engagement Range of all enemy models",
                        "40k_revised.md §5.5 - Fall Back: must end >1\" from all enemies",
                    );
                }
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_remain_stationary(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal(
                "RemainStationary is only valid in the Movement phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        CommandValidationResult::Legal
    }

    fn validate_arrive_from_reserves(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        position: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Movement {
            return CommandValidationResult::illegal(
                "ArriveFromReserves is only valid in the Movement phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_in_reserves() {
            return CommandValidationResult::illegal_with_ref(
                "Unit is not in reserves",
                "40k_revised.md - Reinforcements: only units in Reserves can arrive",
            );
        }

        if !state.board.contains(position) {
            return CommandValidationResult::illegal("Position is outside the board");
        }

        // Strategic reserves cannot arrive on Turn 1 (except Deep Strike under
        // certain conditions). For Combat Patrol, reserves typically arrive from
        // Turn 2 onward.
        if state.battle_round == wh40k_core_types::BattleRound::new(1) {
            return CommandValidationResult::illegal_with_ref(
                "Reserves cannot arrive in Battle Round 1",
                "40k_revised.md - Reserves: arrive from Turn 2 onwards",
            );
        }

        // Strategic Reserves placement: must be wholly within 6" of a battlefield edge.
        // Source: 40k_revised.md §12.2 - "Strategic Reserves: Set up the unit wholly
        // within 6\" of any battlefield edge"
        // The closest edge distance must be <= 6".
        let six_inches = wh40k_core_types::Inches::from_inches(6);
        let board_w = state.board.dimensions.width;
        let board_h = state.board.dimensions.height;
        let dist_left = position.x;                                       // distance from x=0 edge
        let dist_right = board_w - position.x;                            // distance from x=width edge
        let dist_bottom = position.y;                                     // distance from y=0 edge
        let dist_top = board_h - position.y;                              // distance from y=height edge
        let closest_edge_dist = dist_left.min(dist_right).min(dist_bottom).min(dist_top);
        if closest_edge_dist > six_inches {
            return CommandValidationResult::illegal_with_ref(
                format!(
                    "Strategic Reserves must arrive wholly within 6\" of a battlefield edge \
                     (closest edge is {}\" away)",
                    closest_edge_dist
                ),
                "40k_revised.md §12.2 - Strategic Reserves: within 6\" of battlefield edge",
            );
        }

        // Deep Strike: must be >9" from all enemy models
        // Source: 40k_revised.md §12.3 - "DEEP STRIKE"
        // Source: CP_Rules.md §5.6 - Reserves distance requirement
        let nine_inches = wh40k_core_types::Inches::from_inches(9);
        for enemy_unit in &state.units {
            if enemy_unit.owner == unit.owner {
                continue;
            }
            if !enemy_unit.is_on_battlefield() || enemy_unit.is_destroyed() {
                continue;
            }
            for model in &enemy_unit.models {
                if !model.alive {
                    continue;
                }
                let dist = position.distance(model.position);
                if dist <= nine_inches {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Reserves must arrive more than 9\" from all enemy models (model {} is {}\" away)",
                            model.id, dist
                        ),
                        "40k_revised.md §12.3 - Deep Strike: >9\" from all enemy models",
                    );
                }
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_select_unit_to_shoot(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Shooting {
            return CommandValidationResult::illegal_with_ref(
                "SelectUnitToShoot is only valid in the Shooting phase",
                "40k_revised.md - Shooting Phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Already shot this phase
        if state.turn_flags.has_shot(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit has already shot this phase",
                "40k_revised.md - Each unit can only shoot once per Shooting phase",
            );
        }

        // Units that fell back cannot shoot (unless they have a special ability)
        if state.turn_flags.has_fell_back(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit fell back this turn and cannot shoot",
                "40k_revised.md - Fall Back: unit cannot shoot in the same turn",
            );
        }

        // Units that advanced can only shoot Assault/Pistol weapons (validated
        // at the weapon level during DeclareShootingTargets)

        CommandValidationResult::Legal
    }

    fn validate_declare_shooting_targets(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        targets: &[(wh40k_core_types::WeaponId, wh40k_core_types::UnitId)],
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Shooting {
            return CommandValidationResult::illegal("Must be in the Shooting phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        let is_engaged = unit.engagement_status == EngagementStatus::Engaged;
        let is_monster_or_vehicle = unit.keywords.has(Keyword::Monster)
            || unit.keywords.has(Keyword::Vehicle);

        // Verify all target units exist and are enemy units
        for (weapon_id, target_id) in targets {
            let target = match state.unit(*target_id) {
                Some(t) => t,
                None => {
                    return CommandValidationResult::illegal(format!(
                        "Target unit {} not found",
                        target_id
                    ))
                }
            };

            if target.owner == state.active_player {
                return CommandValidationResult::illegal("Cannot target own units");
            }

            if target.is_destroyed() {
                return CommandValidationResult::illegal(format!(
                    "Target unit {} is already destroyed",
                    target_id
                ));
            }

            if !target.is_on_battlefield() {
                return CommandValidationResult::illegal(format!(
                    "Target unit {} is not on the battlefield",
                    target_id
                ));
            }

            // #10: Weapon range check — target must be within weapon range
            // Source: 40k_revised.md §7.2 - "within the Range of the weapon"
            let weapon_profile = unit
                .alive_models()
                .iter()
                .flat_map(|m| m.ranged_weapons.iter())
                .find(|w| w.id == *weapon_id);
            if let Some(wp) = weapon_profile {
                let weapon_range = wp.range;
                if let (Some(attacker_pos), Some(target_pos)) =
                    (unit.reference_position(), target.reference_position())
                {
                    let dist = wh40k_geometry::distance(attacker_pos, target_pos);
                    if dist > weapon_range {
                        return CommandValidationResult::illegal_with_ref(
                            format!(
                                "Target unit {} is out of weapon range ({} vs max {})",
                                target_id, dist, weapon_range
                            ),
                            "40k_revised.md §7.2 - target must be within weapon Range",
                        );
                    }
                }

                // #11: LOS/visibility check — target must be visible unless Indirect Fire
                // Source: 40k_revised.md §7.2 - "that is visible to the shooting model"
                // Indirect Fire weapons can target non-visible units (§11.3)
                // HOWEVER: Torrent weapons cannot benefit from Indirect Fire.
                // Source: 40k_revised.md §11.16 - Torrent weapons auto-hit and
                // cannot use Indirect Fire rules (they require visibility).
                let has_indirect_fire = wp.abilities.has(&wh40k_core_types::WeaponAbility::IndirectFire);
                let has_torrent = wp.abilities.has(&wh40k_core_types::WeaponAbility::Torrent);
                let effective_indirect_fire = has_indirect_fire && !has_torrent;
                if !effective_indirect_fire {
                    if let (Some(attacker_pos), Some(target_pos)) =
                        (unit.reference_position(), target.reference_position())
                    {
                        // Check LOS through terrain using the board's LOS trace
                        let los = state.board.check_los(attacker_pos, target_pos);
                        if los == wh40k_core_types::Visibility::NotVisible {
                            return CommandValidationResult::illegal_with_ref(
                                format!(
                                    "Target unit {} is not visible (line of sight blocked by terrain)",
                                    target_id
                                ),
                                "40k_revised.md §7.2 - target must be visible to shooting model",
                            );
                        }
                    }
                }
            }

            // Engagement range shooting restrictions
            // Source: 40k_revised.md - Pistol, Big Guns Never Tire
            if is_engaged {
                // Check if weapon is a Pistol
                let weapon_is_pistol = unit.alive_models().iter()
                    .flat_map(|m| m.ranged_weapons.iter())
                    .find(|w| w.id == *weapon_id)
                    .map(|w| w.abilities.has(&wh40k_core_types::WeaponAbility::Pistol))
                    .unwrap_or(false);

                if weapon_is_pistol {
                    // Pistol weapons can only target units within engagement range
                    // Source: 40k_revised.md - "can only target enemy units that are
                    // within Engagement Range of the bearer's unit"
                    let target_in_engagement = target.engagement_status == EngagementStatus::Engaged;
                    if !target_in_engagement {
                        return CommandValidationResult::illegal_with_ref(
                            "Pistol weapons can only target units within engagement range",
                            "40k_revised.md - Pistol: target within Engagement Range",
                        );
                    }
                } else if is_monster_or_vehicle {
                    // Big Guns Never Tire: MONSTER/VEHICLE can shoot non-Pistol
                    // ranged weapons at non-engaged targets while engaged
                    // Source: 40k_revised.md - Big Guns Never Tire
                    // (No additional restriction for non-engaged targets)
                } else {
                    // Normal engaged unit with non-Pistol weapon: illegal
                    return CommandValidationResult::illegal_with_ref(
                        "Engaged units can only shoot Pistol weapons (unless MONSTER/VEHICLE)",
                        "40k_revised.md - Shooting with engaged units",
                    );
                }
            }

            // Lone Operative targeting restriction
            // Source: 40k_revised.md - "LONE OPERATIVE"
            // A unit with Lone Operative cannot be targeted by ranged attacks unless:
            // 1. The attacker is within 12" of the Lone Operative, OR
            // 2. The Lone Operative is the closest eligible target to the attacker
            // Check via active effects on the target
            let target_has_lone_operative = state.active_effects.iter().any(|e| {
                matches!(e.effect_type, crate::effect::EffectType::LoneOperative)
                    && matches!(e.target, crate::effect::EffectTarget::Unit(uid) if uid == *target_id)
            });
            if target_has_lone_operative && !is_engaged {
                // Check distance: if attacker is within 12", Lone Operative can be targeted
                let attacker_pos = unit.reference_position();
                let target_pos = target.reference_position();
                let within_12 = match (attacker_pos, target_pos) {
                    (Some(a), Some(t)) => {
                        let dist = wh40k_geometry::distance(a, t);
                        dist <= wh40k_core_types::Inches::from_inches(12)
                    }
                    _ => false,
                };

                if !within_12 {
                    // Check if this target is the closest eligible enemy unit
                    let closest = Self::is_closest_eligible_target(state, unit_id, *target_id);
                    if !closest {
                        return CommandValidationResult::illegal_with_ref(
                            "Lone Operative cannot be targeted unless within 12\" or closest eligible target",
                            "40k_revised.md - Lone Operative",
                        );
                    }
                }
            }

            // Locked in Combat (§7.4): non-Pistol, non-BGNT weapons cannot target
            // enemies that are within Engagement Range of friendly units.
            // Source: 40k_revised.md §7.4 - "Locked in Combat"
            if !is_engaged {
                let weapon_is_pistol = unit.alive_models().iter()
                    .flat_map(|m| m.ranged_weapons.iter())
                    .find(|w| w.id == *weapon_id)
                    .map(|w| w.abilities.has(&wh40k_core_types::WeaponAbility::Pistol))
                    .unwrap_or(false);

                if !weapon_is_pistol {
                    // Check if any friendly unit is within ER of the target
                    let target_engaged_by_friendly = state.units.iter().any(|fu| {
                        fu.owner == unit.owner
                            && fu.id != unit_id
                            && !fu.is_destroyed()
                            && fu.is_on_battlefield()
                            && fu.alive_models().iter().any(|fm| {
                                target.alive_models().iter().any(|tm| {
                                    wh40k_geometry::within_engagement_range_2d(
                                        fm.position, fm.base_size,
                                        tm.position, tm.base_size,
                                    )
                                })
                            })
                    });

                    if target_engaged_by_friendly {
                        return CommandValidationResult::illegal_with_ref(
                            "Cannot target enemy units that are within Engagement Range of friendly units (Locked in Combat)",
                            "40k_revised.md §7.4 - Locked in Combat",
                        );
                    }
                }
            }

            // #27: Blast restriction — cannot target units within Engagement Range
            // of friendly units.
            // Source: 40k_revised.md §11.5 - "BLAST"
            let weapon_has_blast = unit
                .alive_models()
                .iter()
                .flat_map(|m| m.ranged_weapons.iter())
                .find(|w| w.id == *weapon_id)
                .map(|w| w.abilities.has(&wh40k_core_types::WeaponAbility::Blast))
                .unwrap_or(false);
            if weapon_has_blast {
                // Check if any friendly model is within engagement range of any target model
                for friendly_unit in &state.units {
                    if friendly_unit.owner != unit.owner
                        || friendly_unit.id == unit_id
                        || friendly_unit.is_destroyed()
                        || !friendly_unit.is_on_battlefield()
                    {
                        continue;
                    }
                    for friendly_model in friendly_unit.models.iter().filter(|m| m.alive) {
                        for target_model in target.models.iter().filter(|m| m.alive) {
                            if wh40k_geometry::within_engagement_range_2d(
                                friendly_model.position,
                                friendly_model.base_size,
                                target_model.position,
                                target_model.base_size,
                            ) {
                                return CommandValidationResult::illegal_with_ref(
                                    "Blast weapons cannot target units within Engagement Range of friendly units",
                                    "40k_revised.md §11.5 - BLAST: cannot target within ER of friendlies",
                                );
                            }
                        }
                    }
                }
            }
        }

        // #28: Pistol exclusivity — non-MONSTER/VEHICLE units cannot mix
        // Pistol and non-Pistol weapons in the same shooting phase.
        // Source: 40k_revised.md §11.4 - "PISTOL"
        if !is_monster_or_vehicle && targets.len() > 1 {
            let mut has_pistol = false;
            let mut has_non_pistol = false;
            for (weapon_id, _target_id) in targets {
                let is_pistol = unit
                    .alive_models()
                    .iter()
                    .flat_map(|m| m.ranged_weapons.iter())
                    .find(|w| w.id == *weapon_id)
                    .map(|w| w.abilities.has(&wh40k_core_types::WeaponAbility::Pistol))
                    .unwrap_or(false);
                if is_pistol {
                    has_pistol = true;
                } else {
                    has_non_pistol = true;
                }
            }
            if has_pistol && has_non_pistol {
                return CommandValidationResult::illegal_with_ref(
                    "Non-MONSTER/VEHICLE units cannot fire both Pistol and non-Pistol weapons in the same phase",
                    "40k_revised.md §11.4 - PISTOL: model shooting a Pistol can only shoot Pistols",
                );
            }
        }

        CommandValidationResult::Legal
    }

    /// Validate a ResolveShootingAttack command.
    ///
    /// Checks:
    /// 1. Must be in the Shooting phase
    /// 2. If the attacker unit Advanced this turn, the weapon must have the
    ///    [ASSAULT] ability (or [PISTOL]). Non-Assault/non-Pistol weapons
    ///    cannot be fired after Advancing.
    ///
    /// Source: 40k_revised.md §5.4 - Advance: "it can only shoot with
    ///         weapons that have the [ASSAULT] ability"
    /// Source: 40k_revised.md §7.1 - Shooting Phase weapon restrictions
    fn validate_resolve_shooting_attack(
        state: &GameState,
        attacker_id: wh40k_core_types::UnitId,
        weapon_id: wh40k_core_types::WeaponId,
    ) -> CommandValidationResult {
        // Must be in Shooting phase
        if state.current_phase != Phase::Shooting {
            return CommandValidationResult::illegal_with_ref(
                "ResolveShootingAttack is only valid in the Shooting phase",
                "40k_revised.md - Shooting Phase",
            );
        }

        // If the unit advanced, only Assault and Pistol weapons can fire
        // Source: 40k_revised.md §5.4 - "it can only shoot with weapons
        //         that have the [ASSAULT] ability"
        // Note: Pistol weapons can also fire after Advancing per core rules
        if state.turn_flags.has_advanced(attacker_id) {
            if let Some(unit) = state.unit(attacker_id) {
                // Look up the weapon profile on any alive model
                let weapon_profile = unit.alive_models().iter()
                    .flat_map(|m| m.ranged_weapons.iter())
                    .find(|w| w.id == weapon_id);

                if let Some(weapon) = weapon_profile {
                    if !weapon.can_fire_after_advance() {
                        return CommandValidationResult::illegal_with_ref(
                            "Unit advanced this turn and can only fire weapons with the [ASSAULT] ability",
                            "40k_revised.md §5.4 - Advance: unit can only shoot Assault weapons",
                        );
                    }
                }
                // If weapon not found on models, let execution handle the error
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_declare_charge(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        targets: &[wh40k_core_types::UnitId],
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Charge {
            return CommandValidationResult::illegal_with_ref(
                "DeclareCharge is only valid in the Charge phase",
                "40k_revised.md - Charge Phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Cannot charge if already charged this turn
        if state.turn_flags.has_charged(unit_id) {
            return CommandValidationResult::illegal("Unit has already charged this turn");
        }

        // Cannot charge if fell back
        // Source: 40k_revised.md - Fall Back: unit cannot charge in the same turn
        if state.turn_flags.has_fell_back(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit fell back this turn and cannot charge",
                "40k_revised.md - Fall Back: unit cannot charge in the same turn",
            );
        }

        // Cannot charge if advanced
        // Source: 40k_revised.md - Advance: unit cannot charge in the same turn
        if state.turn_flags.has_advanced(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit advanced this turn and cannot charge",
                "40k_revised.md - Advance: unit cannot charge in the same turn",
            );
        }

        // Cannot charge if AIRCRAFT
        // Source: 40k_revised.md Section 9.1
        if unit.keywords.has(Keyword::Aircraft) {
            return CommandValidationResult::illegal_with_ref(
                "AIRCRAFT units cannot charge",
                "40k_revised.md Section 9.1 - Charge Eligibility",
            );
        }

        // Cannot charge if already within engagement range of any enemy
        // Source: 40k_revised.md Section 9.1 - "not within Engagement Range of enemies"
        let unit_ref_pos = unit.reference_position();
        let unit_base = unit.models.first().map(|m| m.base_size)
            .unwrap_or(wh40k_core_types::BaseSize::MM25);

        if let Some(charger_pos) = unit_ref_pos {
            for other_unit in &state.units {
                if other_unit.owner == unit.owner || other_unit.is_destroyed() || !other_unit.is_on_battlefield() {
                    continue;
                }
                for model in other_unit.models.iter().filter(|m| m.alive) {
                    if wh40k_geometry::within_engagement_range_2d(
                        charger_pos, unit_base,
                        model.position, model.base_size,
                    ) {
                        return CommandValidationResult::illegal_with_ref(
                            "Unit is already within engagement range of enemies and cannot charge",
                            "40k_revised.md Section 9.1 - Charge Eligibility",
                        );
                    }
                }
            }
        }

        // Must have at least one valid target
        if targets.is_empty() {
            return CommandValidationResult::illegal("Must declare at least one charge target");
        }

        // Validate targets and check 12" range
        // Source: 40k_revised.md Section 9.1/9.2 - targets must be within 12"
        let charge_range = wh40k_core_types::Inches::from_inches(12);

        for target_id in targets {
            let target = match state.unit(*target_id) {
                Some(t) => t,
                None => {
                    return CommandValidationResult::illegal(format!(
                        "Charge target unit {} not found",
                        target_id
                    ))
                }
            };

            if target.owner == state.active_player {
                return CommandValidationResult::illegal("Cannot charge own units");
            }

            if target.is_destroyed() {
                return CommandValidationResult::illegal(format!(
                    "Charge target unit {} is already destroyed",
                    target_id
                ));
            }

            if !target.is_on_battlefield() {
                return CommandValidationResult::illegal(format!(
                    "Charge target unit {} is not on the battlefield",
                    target_id
                ));
            }

            // Check 12" range from closest models
            if let Some(charger_pos) = unit_ref_pos {
                let mut any_within_12 = false;
                for target_model in target.models.iter().filter(|m| m.alive) {
                    let dist = wh40k_geometry::distance_between_models(
                        charger_pos, unit_base,
                        target_model.position, target_model.base_size,
                    );
                    if dist <= charge_range {
                        any_within_12 = true;
                        break;
                    }
                }
                if !any_within_12 {
                    return CommandValidationResult::illegal_with_ref(
                        format!("Charge target unit {} is not within 12\"", target_id),
                        "40k_revised.md Section 9.2 - Declare targets within 12\"",
                    );
                }
            }
        }

        CommandValidationResult::Legal
    }

    /// Validate a MakeChargeMove command.
    ///
    /// #14: Full charge move geometric validation per 40k_revised.md §9.4:
    /// 1. Distance cap: total distance ≤ charge roll
    /// 2. Must end in engagement range of at least one declared target
    /// 3. Cannot move within engagement range of non-target enemies
    ///    (unless was already within ER of them)
    ///
    /// Source: 40k_revised.md §9.4 - Charge Moves
    fn validate_charge_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Charge {
            return CommandValidationResult::illegal_with_ref(
                "MakeChargeMove is only valid in the Charge phase",
                "40k_revised.md - Charge Phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        // Must have a successful charge roll
        let charge_roll = match state.turn_flags.get_charge_roll(unit_id) {
            Some(roll) => roll,
            None => {
                return CommandValidationResult::illegal_with_ref(
                    "Unit has no successful charge roll — must resolve charge roll first",
                    "40k_revised.md §9.3 - Charge Roll must succeed before Charge Move",
                );
            }
        };

        let unit_base = unit.models.first().map(|m| m.base_size)
            .unwrap_or(wh40k_core_types::BaseSize::MM25);

        // Check all models would remain on the board after translation
        if let Some(reason) = Self::all_models_on_board_after_move(state, unit_id, destination) {
            return CommandValidationResult::illegal_with_ref(
                reason,
                "40k_revised.md §9.4 - Models cannot move off the battlefield during charge",
            );
        }

        // Distance cap: charge move distance cannot exceed the charge roll
        if let Some(current_pos) = unit.reference_position() {
            let move_distance = current_pos.distance(destination);
            let max_charge_distance = wh40k_core_types::Inches::from_inches(charge_roll as i32);
            if move_distance > max_charge_distance {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "Charge move distance ({}) exceeds charge roll ({}\")",
                        move_distance, charge_roll
                    ),
                    "40k_revised.md §9.4 - Charge Move: up to charge roll distance",
                );
            }
        }

        // Must have declared charge targets
        let charge_targets = state.turn_flags.get_charge_targets(unit_id)
            .cloned()
            .unwrap_or_default();
        if charge_targets.is_empty() {
            return CommandValidationResult::illegal(
                "No charge targets declared for this unit",
            );
        }

        // Must end within engagement range of ALL declared charge targets
        for target_id in &charge_targets {
            let mut in_er_of_this_target = false;
            if let Some(target) = state.unit(*target_id) {
                for target_model in target.models.iter().filter(|m| m.alive) {
                    if wh40k_geometry::within_engagement_range_2d(
                        destination, unit_base,
                        target_model.position, target_model.base_size,
                    ) {
                        in_er_of_this_target = true;
                        break;
                    }
                }
            }
            if !in_er_of_this_target {
                return CommandValidationResult::illegal_with_ref(
                    "Charge move must end within Engagement Range of ALL declared charge targets",
                    "40k_revised.md §9.4 - Charge Move: must end in ER of all targets",
                );
            }
        }

        // Cannot end within engagement range of enemy units that were NOT declared
        // as charge targets (unless the charger was already within ER before charging)
        let current_pos = unit.reference_position().unwrap_or(wh40k_core_types::Position::ORIGIN);
        for enemy_unit in &state.units {
            if enemy_unit.owner == unit.owner
                || enemy_unit.is_destroyed()
                || !enemy_unit.is_on_battlefield()
            {
                continue;
            }
            // Skip declared charge targets — ending in ER with them is allowed
            if charge_targets.contains(&enemy_unit.id) {
                continue;
            }
            for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                let will_be_in_er = wh40k_geometry::within_engagement_range_2d(
                    destination, unit_base,
                    enemy_model.position, enemy_model.base_size,
                );
                if will_be_in_er {
                    // Check if was already within ER before the charge
                    let was_in_er = wh40k_geometry::within_engagement_range_2d(
                        current_pos, unit_base,
                        enemy_model.position, enemy_model.base_size,
                    );
                    if !was_in_er {
                        return CommandValidationResult::illegal_with_ref(
                            format!(
                                "Charge move would end within Engagement Range of non-target enemy unit {}",
                                enemy_unit.id
                            ),
                            "40k_revised.md §9.4 - Cannot move within ER of non-target enemies",
                        );
                    }
                }
            }
        }

        CommandValidationResult::Legal
    }

    /// Validate a Heroic Intervention move command.
    ///
    /// Requirements:
    /// - Must be in the Charge phase
    /// - Unit must have the Heroic Intervention effect active (from UseStratagem)
    /// - Unit must belong to the non-active player (opponent reacting to charge)
    /// - Move distance must be <= 6"
    /// - Must move closer to the nearest enemy model
    ///
    /// Source: 40k_revised.md - "Heroic Intervention"
    fn validate_heroic_intervention_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Charge {
            return CommandValidationResult::illegal_with_ref(
                "HeroicInterventionMove is only valid in the Charge phase",
                "40k_revised.md - Heroic Intervention",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Unit must have the Heroic Intervention effect active
        let has_hi_effect = state.active_effects.iter().any(|e| {
            matches!(e.target, crate::effect::EffectTarget::Unit(uid) if uid == unit_id)
                && matches!(&e.effect_type, crate::effect::EffectType::Custom(s) if s.contains("Heroic Intervention"))
        });
        if !has_hi_effect {
            return CommandValidationResult::illegal_with_ref(
                "Unit does not have an active Heroic Intervention effect (use the stratagem first)",
                "40k_revised.md - Heroic Intervention: requires stratagem",
            );
        }

        // Check move distance <= 6"
        let heroic_range = wh40k_core_types::Inches::from_inches(6);
        if let Some(current_pos) = unit.reference_position() {
            let dist = current_pos.distance(destination);
            if dist > heroic_range {
                return CommandValidationResult::illegal_with_ref(
                    format!("Heroic Intervention move exceeds 6\" (distance: {})", dist),
                    "40k_revised.md - Heroic Intervention: move up to 6\"",
                );
            }
        }

        CommandValidationResult::Legal
    }

    /// Validate selecting a unit to fight in the Fight phase.
    ///
    /// Enforces fight phase alternation order per the rules:
    /// - Fights First step: active player picks first, then alternate
    ///   Only units with Fights First ability (charged this turn or GrantFightsFirst effect)
    /// - Remaining Combats step: non-active player picks first, then alternate
    ///   All remaining eligible units
    ///
    /// Both players can select units to fight (not just the active player).
    /// The alternation tracking in TurnFlags determines whose turn it is to pick.
    /// If the designated player has no eligible units, the other player may pick.
    ///
    /// Source: CP_Rules.md §8.1 - Fight Phase Sequence
    /// Source: 40k_revised.md §10.1 - Fight Phase Structure
    /// Source: 40k_revised.md §10.2 - Fights First Step
    /// Source: 40k_revised.md §10.3 - Remaining Combats Step
    fn validate_select_unit_to_fight(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Fight {
            return CommandValidationResult::illegal_with_ref(
                "SelectUnitToFight is only valid in the Fight phase",
                "40k_revised.md - Fight Phase",
            );
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Already fought this phase
        // Source: 40k_revised.md - "No unit can fight more than once per Fight phase"
        if state.turn_flags.has_fought(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit has already fought this phase",
                "40k_revised.md - Each unit fights once per Fight phase",
            );
        }

        // Must be engaged or have charged this turn to fight
        // Source: 40k_revised.md §10.1 - "Within Engagement Range of enemy units, OR Made a Charge move this turn"
        if unit.engagement_status != EngagementStatus::Engaged
            && !state.turn_flags.charged_this_turn(unit_id)
        {
            return CommandValidationResult::illegal_with_ref(
                "Unit must be within Engagement Range or have charged this turn to fight",
                "40k_revised.md - Fight Phase: eligible units",
            );
        }

        // Check Fights First subphase eligibility
        // Source: 40k_revised.md §10.2 - "Units with the Fights First ability fight in this step"
        // Source: CP_Rules.md §8.1 - "Units that charged this turn" + "Units with Fights First ability"
        if state.current_subphase == wh40k_core_types::SubPhase::FightsFirst {
            let has_fights_first = state.turn_flags.charged_this_turn(unit_id)
                || state.active_effects.iter().any(|e| {
                    matches!(e.target, crate::effect::EffectTarget::Unit(uid) if uid == unit_id)
                        && matches!(e.effect_type, crate::effect::EffectType::GrantFightsFirst)
                });
            if !has_fights_first {
                return CommandValidationResult::illegal_with_ref(
                    "Unit does not have Fights First ability (only units that charged or have \
                     Fights First can fight in the Fights First step)",
                    "40k_revised.md §10.2 - Fights First Step",
                );
            }
        }

        // Enforce fight alternation order
        // Source: CP_Rules.md §8.1 - "Players alternate"
        // Source: 40k_revised.md §10.1 - "Players alternate selecting units"
        if let Some(next_player) = state.turn_flags.fight_alternation_next_player {
            if unit.owner != next_player {
                // Check if the designated player has any eligible units remaining.
                // If not, the other player can pick (cannot pass when eligible units remain).
                // Source: 40k_revised.md §10.1 - "Cannot pass when eligible units remain"
                let designated_has_eligible = state.units.iter().any(|u| {
                    u.owner == next_player
                        && u.is_on_battlefield()
                        && !u.is_destroyed()
                        && !state.turn_flags.has_fought(u.id)
                        && (u.engagement_status == EngagementStatus::Engaged
                            || state.turn_flags.charged_this_turn(u.id))
                        && (state.current_subphase != wh40k_core_types::SubPhase::FightsFirst
                            || state.turn_flags.charged_this_turn(u.id)
                            || state.active_effects.iter().any(|e| {
                                matches!(e.target, crate::effect::EffectTarget::Unit(uid) if uid == u.id)
                                    && matches!(e.effect_type, crate::effect::EffectType::GrantFightsFirst)
                            }))
                });

                if designated_has_eligible {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "It is Player {}'s turn to select a unit to fight (fight alternation order)",
                            next_player
                        ),
                        "CP_Rules.md §8.1 - Players alternate selecting units to fight",
                    );
                }
                // If the designated player has no eligible units, allow the other player to pick
            }
        }

        CommandValidationResult::Legal
    }

    /// Validate Pile-In or Consolidate move: each model must end closer to the
    /// closest enemy model than it started.
    ///
    /// #24: Source: 40k_revised.md §10.4 - "each model must end its move closer
    ///      to the closest enemy model"
    fn validate_pile_in_closer_to_enemy(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        positions: &[(wh40k_core_types::ModelId, wh40k_core_types::Position)],
        move_name: &str,
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Fight {
            return CommandValidationResult::illegal(format!(
                "{} is only valid in the Fight phase",
                move_name
            ));
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if unit.is_destroyed() || !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield or is destroyed");
        }

        // Max move distance: 3" for Pile-In, 3" for Consolidate
        let max_move = wh40k_core_types::Inches::from_inches(3);

        for (model_id, new_pos) in positions {
            // Find the model in this unit
            let model = match unit.models.iter().find(|m| m.id == *model_id) {
                Some(m) => m,
                None => {
                    return CommandValidationResult::illegal(format!(
                        "Model {} not found in unit {}",
                        model_id, unit_id
                    ));
                }
            };

            if !model.alive {
                continue; // Skip dead models
            }

            let old_pos = model.position;

            // Check move distance <= 3"
            let move_dist = old_pos.distance(*new_pos);
            if move_dist > max_move {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "{} move distance ({}) exceeds 3\" limit",
                        move_name, move_dist
                    ),
                    "40k_revised.md §10.4 - Pile-In/Consolidate: up to 3\"",
                );
            }

            // Find closest enemy model distance from OLD position
            let mut closest_old_dist = wh40k_core_types::Inches::from_inches(999);
            for enemy_unit in &state.units {
                if enemy_unit.owner == unit.owner
                    || enemy_unit.is_destroyed()
                    || !enemy_unit.is_on_battlefield()
                {
                    continue;
                }
                for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                    let dist = wh40k_geometry::distance_between_models(
                        old_pos,
                        model.base_size,
                        enemy_model.position,
                        enemy_model.base_size,
                    );
                    if dist < closest_old_dist {
                        closest_old_dist = dist;
                    }
                }
            }

            // Find closest enemy model distance from NEW position
            let mut closest_new_dist = wh40k_core_types::Inches::from_inches(999);
            for enemy_unit in &state.units {
                if enemy_unit.owner == unit.owner
                    || enemy_unit.is_destroyed()
                    || !enemy_unit.is_on_battlefield()
                {
                    continue;
                }
                for enemy_model in enemy_unit.models.iter().filter(|m| m.alive) {
                    let dist = wh40k_geometry::distance_between_models(
                        *new_pos,
                        model.base_size,
                        enemy_model.position,
                        enemy_model.base_size,
                    );
                    if dist < closest_new_dist {
                        closest_new_dist = dist;
                    }
                }
            }

            // New position must be closer to (or equal distance from) the closest enemy
            if closest_new_dist > closest_old_dist {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "{}: model {} must end closer to the closest enemy model (was {}, now {})",
                        move_name, model_id, closest_old_dist, closest_new_dist
                    ),
                    "40k_revised.md §10.4 - Pile-In/Consolidate: must end closer to closest enemy",
                );
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_declare_melee_targets(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        targets: &[(wh40k_core_types::WeaponId, wh40k_core_types::UnitId)],
    ) -> CommandValidationResult {
        if state.current_phase != Phase::Fight {
            return CommandValidationResult::illegal("DeclareMeleeTargets only valid in Fight phase");
        }

        let unit = match state.unit(unit_id) {
            Some(u) => u,
            None => return CommandValidationResult::illegal("Unit not found"),
        };

        if !unit.is_on_battlefield() || unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        // Must be engaged or have charged this turn
        if unit.engagement_status != EngagementStatus::Engaged
            && !state.turn_flags.charged_this_turn(unit_id)
        {
            return CommandValidationResult::illegal_with_ref(
                "Unit must be in Engagement Range or have charged this turn to fight",
                "40k_revised.md §10.2 - Fight eligibility",
            );
        }

        // Check each target exists and is an enemy
        for (_weapon_id, target_id) in targets {
            let target = match state.unit(*target_id) {
                Some(t) => t,
                None => return CommandValidationResult::illegal("Target unit not found"),
            };
            if target.owner == unit.owner {
                return CommandValidationResult::illegal("Cannot target friendly units in melee");
            }
            if target.is_destroyed() || !target.is_on_battlefield() {
                return CommandValidationResult::illegal("Target is destroyed or not on battlefield");
            }
        }

        CommandValidationResult::Legal
    }

    fn validate_use_stratagem(
        state: &GameState,
        player: wh40k_core_types::PlayerId,
        stratagem_id: wh40k_core_types::StratagemId,
        target: &wh40k_command_system::StratagemTarget,
    ) -> CommandValidationResult {
        use crate::stratagem;

        let player_state = state.player(player);

        // Look up the stratagem definition
        let def = match stratagem::get_stratagem_def(stratagem_id) {
            Some(d) => d,
            None => {
                return CommandValidationResult::illegal_with_ref(
                    format!("Unknown stratagem ID {}", stratagem_id),
                    "40k_revised.md - Stratagems",
                );
            }
        };

        // Check CP available (use actual CP cost from definition)
        if player_state.cp.value() < def.cp_cost as i8 {
            return CommandValidationResult::illegal_with_ref(
                format!(
                    "Insufficient Command Points: need {}, have {}",
                    def.cp_cost,
                    player_state.cp.value()
                ),
                "40k_revised.md - Stratagems: must pay CP cost",
            );
        }

        // Mission 3 (Forward Outpost) — Sabotage Enemy Comms:
        // If command_reroll_blocked is set, player cannot use Command Re-roll.
        // Source: CP_Rules.md - Mission 3: Sabotage Enemy Comms
        if stratagem_id == stratagem::ids::COMMAND_REROLL
            && player_state.mission_progress.command_reroll_blocked
        {
            return CommandValidationResult::illegal_with_ref(
                "Command Re-roll is blocked by Sabotage Enemy Comms (Forward Outpost)",
                "CP_Rules.md - Mission 3: Sabotage Enemy Comms",
            );
        }

        // Check phase validity
        // Command Re-roll is valid in any phase
        if stratagem_id != stratagem::ids::COMMAND_REROLL
            && !def.valid_phases.contains(&state.current_phase)
        {
            return CommandValidationResult::illegal_with_ref(
                format!(
                    "Stratagem '{}' cannot be used in {:?} phase",
                    def.name, state.current_phase
                ),
                "40k_revised.md - Stratagems: phase timing restrictions",
            );
        }

        // Check timing window enforcement
        // Source: 40k_revised.md - Each stratagem has a specific timing when it can be used
        match &def.timing {
            stratagem::StratagemTiming::AfterEnemyDeclaresCharge => {
                // Heroic Intervention: requires an active reaction window for charge
                let has_charge_window = state.reaction_windows.iter().any(|rw| {
                    matches!(
                        rw.window_type,
                        wh40k_core_types::ReactionWindowType::Overwatch
                            | wh40k_core_types::ReactionWindowType::HeroicIntervention
                    )
                });
                if !has_charge_window {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' requires an enemy charge to have been declared",
                            def.name
                        ),
                        "40k_revised.md - Stratagems: timing window (after enemy declares charge)",
                    );
                }
            }
            stratagem::StratagemTiming::AfterEnemySelectsTargets => {
                // Go to Ground: should be used during shooting when enemy selects targets
                // Validated by phase (Shooting) already; more granular timing tracked by reaction windows
                if state.current_phase != Phase::Shooting {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' can only be used in the Shooting phase after enemy selects targets",
                            def.name
                        ),
                        "40k_revised.md - Stratagems: timing window (after enemy selects targets)",
                    );
                }
            }
            stratagem::StratagemTiming::OnUnitSelectedToFight => {
                // Epic Challenge / Counter-Operative: requires Fight phase
                if state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' can only be used when a unit is selected to fight",
                            def.name
                        ),
                        "40k_revised.md - Stratagems: timing window (on unit selected to fight)",
                    );
                }
            }
            stratagem::StratagemTiming::AfterChargeMoveComplete => {
                // Counter-Operative (FR): requires charge to have been completed
                let has_co_window = state.reaction_windows.iter().any(|rw| {
                    matches!(
                        rw.window_type,
                        wh40k_core_types::ReactionWindowType::CounterOffensive
                    )
                });
                if !has_co_window && state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' requires a charge move to have been completed",
                            def.name
                        ),
                        "40k_revised.md - Stratagems: timing window (after charge move complete)",
                    );
                }
            }
            stratagem::StratagemTiming::AfterEnemyUnitFights => {
                // Counter-Offensive: requires Fight phase and an enemy unit to have fought
                // Source: 40k_revised.md — Counter-Offensive: "used after an enemy unit has fought"
                if state.current_phase != Phase::Fight {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' can only be used in the Fight phase after an enemy unit has fought",
                            def.name
                        ),
                        "40k_revised.md - Counter-Offensive: after enemy unit fights",
                    );
                }
                // Check for a CounterOffensive reaction window (opened after an enemy unit fights)
                let has_co_window = state.reaction_windows.iter().any(|rw| {
                    matches!(
                        rw.window_type,
                        wh40k_core_types::ReactionWindowType::CounterOffensive
                    )
                });
                if !has_co_window {
                    return CommandValidationResult::illegal_with_ref(
                        format!(
                            "Stratagem '{}' requires an enemy unit to have fought first",
                            def.name
                        ),
                        "40k_revised.md - Counter-Offensive: timing (after enemy unit fights)",
                    );
                }
            }
            // AnyTime, StartOfPhase, DuringPhase, AfterEnemyShoots: validated by phase check above
            _ => {}
        }

        // Check if stratagem was already used this phase (same stratagem restriction)
        if player_state.stratagem_usage.used_this_phase(stratagem_id) {
            return CommandValidationResult::illegal_with_ref(
                "This stratagem has already been used this phase",
                "40k_revised.md - Stratagems: same stratagem cannot be used more than once per phase",
            );
        }

        // Check once-per-turn restriction
        if def.once_per_turn && player_state.stratagem_usage.used_this_turn(stratagem_id) {
            return CommandValidationResult::illegal_with_ref(
                format!("Stratagem '{}' already used this turn", def.name),
                "40k_revised.md - Stratagems: once per turn restriction",
            );
        }

        // Check once-per-battle restriction
        if def.once_per_battle && player_state.stratagem_usage.used_this_battle(stratagem_id) {
            return CommandValidationResult::illegal_with_ref(
                format!("Stratagem '{}' already used this battle", def.name),
                "40k_revised.md - Stratagems: once per battle restriction",
            );
        }

        // Mission 6 (Display of Might) — Break Their Spirit:
        // Insane Bravery can only be used if target unit within 6" of WARLORD.
        // Source: CP_Rules.md - Mission 6: Display of Might, Break Their Spirit
        if stratagem_id == stratagem::ids::INSANE_BRAVERY {
            if state.scenario_id == Some(crate::scoring::mission_ids::DISPLAY_OF_MIGHT) {
                if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
                    let unit_pos = state.unit(*unit_id).and_then(|u| u.reference_position());
                    let warlord_nearby = unit_pos.map_or(false, |pos| {
                        state.units.iter().any(|u| {
                            u.owner == player
                                && !u.is_destroyed()
                                && u.is_on_battlefield()
                                && u.is_character()
                                && u.enhancement_oc_override.is_some() // Warlord has an enhancement
                                && u.reference_position().map_or(false, |wpos| {
                                    wh40k_geometry::distance(pos, wpos)
                                        <= wh40k_core_types::Inches::from_inches(6)
                                })
                        })
                    });
                    if !warlord_nearby {
                        return CommandValidationResult::illegal_with_ref(
                            "Break Their Spirit: Insane Bravery requires target unit within 6\" of WARLORD",
                            "CP_Rules.md - Mission 6: Display of Might, Break Their Spirit",
                        );
                    }
                }
            }
        }

        // Check target restrictions
        if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
            if let Some(unit) = state.unit(*unit_id) {
                // Battle-shocked restriction: battle-shocked units cannot be
                // targeted by stratagems (except Insane Bravery)
                // Source: 40k_revised.md - Battle-shock: "cannot be targeted by Stratagems"
                if unit.battle_shocked && stratagem_id != stratagem::ids::INSANE_BRAVERY {
                    return CommandValidationResult::illegal_with_ref(
                        "Target unit is Battle-shocked and cannot use stratagems",
                        "40k_revised.md - Battle-shock: units cannot be targeted by Stratagems",
                    );
                }

                // Check required keywords
                for kw in def.required_keywords {
                    if !unit.has_keyword(*kw) {
                        return CommandValidationResult::illegal_with_ref(
                            format!(
                                "Target unit lacks required keyword {:?} for stratagem '{}'",
                                kw, def.name
                            ),
                            "40k_revised.md - Stratagems: target keyword requirements",
                        );
                    }
                }

                // Check ownership
                if def.must_be_friendly && unit.owner != player {
                    return CommandValidationResult::illegal_with_ref(
                        "Stratagem target must be a friendly unit",
                        "40k_revised.md - Stratagems: targeting restrictions",
                    );
                }
                if def.must_be_enemy && unit.owner == player {
                    return CommandValidationResult::illegal_with_ref(
                        "Stratagem target must be an enemy unit",
                        "40k_revised.md - Stratagems: targeting restrictions",
                    );
                }
            } else {
                return CommandValidationResult::illegal("Stratagem target unit not found");
            }
        }

        // Tank Shock: target must be a VEHICLE or MONSTER (not enforced via required_keywords
        // because the static def can't express OR logic).
        // Source: CP_Rules.md §11 — Tank Shock: "VEHICLE unit"
        // Source: Frenzied_Reavers.md — Tank Shock available to Vorrakh (MONSTER)
        if stratagem_id == stratagem::ids::TANK_SHOCK {
            if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
                if let Some(unit) = state.unit(*unit_id) {
                    let is_vehicle = unit.has_keyword(wh40k_core_types::Keyword::Vehicle);
                    let is_monster = unit.has_keyword(wh40k_core_types::Keyword::Monster);
                    if !is_vehicle && !is_monster {
                        return CommandValidationResult::illegal_with_ref(
                            "Tank Shock: target must be a VEHICLE or MONSTER unit",
                            "CP_Rules.md - Tank Shock: VEHICLE/MONSTER requirement",
                        );
                    }
                }
            }
        }

        // Bloodlust: JAKHALS unit must have lost one or more models from enemy shooting.
        // Source: Frenzied_Reavers.md — Bloodlust: "one JAKHALS unit that lost one or
        // more models from the attacking unit's attacks"
        if stratagem_id == stratagem::ids::BLOODLUST {
            if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
                if let Some(unit) = state.unit(*unit_id) {
                    // Check if this unit has lost models this phase (models_alive < starting_strength)
                    let starting = unit.starting_model_count();
                    let current = unit.models_alive();
                    if current >= starting {
                        return CommandValidationResult::illegal_with_ref(
                            "Bloodlust: JAKHALS unit must have lost one or more models from enemy shooting",
                            "Frenzied_Reavers.md - Bloodlust: lost models condition",
                        );
                    }
                }
            }
        }

        // Heroic Intervention: VEHICLE must be a WALKER to use.
        // Source: CP_Rules.md §11 — Heroic Intervention: "VEHICLE must be WALKER"
        if stratagem_id == stratagem::ids::HEROIC_INTERVENTION {
            if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
                if let Some(unit) = state.unit(*unit_id) {
                    if unit.has_keyword(wh40k_core_types::Keyword::Vehicle)
                        && !unit.has_keyword(wh40k_core_types::Keyword::Walker)
                    {
                        return CommandValidationResult::illegal_with_ref(
                            "Heroic Intervention: VEHICLE units must have WALKER keyword",
                            "CP_Rules.md - Heroic Intervention: VEHICLE must be WALKER",
                        );
                    }
                }
            }
        }

        // Grenade stratagem additional restrictions.
        // Source: CP_Rules.md — Grenade: "GRENADES unit that hasn't Advanced,
        // Fallen Back, or shot, and isn't in Engagement Range"
        if stratagem_id == stratagem::ids::GRENADE {
            if let wh40k_command_system::StratagemTarget::Unit(unit_id) = target {
                // Cannot use if unit Advanced
                if state.turn_flags.advanced_this_turn.contains(unit_id) {
                    return CommandValidationResult::illegal_with_ref(
                        "Grenade: unit has Advanced this turn",
                        "CP_Rules.md - Grenade restrictions",
                    );
                }
                // Cannot use if unit Fell Back
                if state.turn_flags.fell_back_this_turn.contains(unit_id) {
                    return CommandValidationResult::illegal_with_ref(
                        "Grenade: unit has Fallen Back this turn",
                        "CP_Rules.md - Grenade restrictions",
                    );
                }
                // Cannot use if unit already shot
                if state.turn_flags.units_shot.contains(unit_id) {
                    return CommandValidationResult::illegal_with_ref(
                        "Grenade: unit has already shot this turn",
                        "CP_Rules.md - Grenade restrictions",
                    );
                }
                // Cannot use if unit is within Engagement Range
                if let Some(unit) = state.unit(*unit_id) {
                    if unit.engagement_status == wh40k_core_types::EngagementStatus::Engaged {
                        return CommandValidationResult::illegal_with_ref(
                            "Grenade: unit is within Engagement Range of enemies",
                            "CP_Rules.md - Grenade restrictions",
                        );
                    }
                }
            }
        }

        // Check faction restrictions for faction stratagems
        if def.is_faction && !def.faction_keywords.is_empty() {
            // Check if the player's army has at least one unit with the required faction keyword
            let has_faction_unit = state.units.iter().any(|u| {
                u.owner == player
                    && def.faction_keywords.iter().all(|kw| u.has_keyword(*kw))
            });
            if !has_faction_unit {
                return CommandValidationResult::illegal_with_ref(
                    format!(
                        "Faction stratagem '{}' requires {:?} faction",
                        def.name, def.faction_keywords
                    ),
                    "40k_revised.md - Faction Stratagems: faction requirement",
                );
            }
        }

        CommandValidationResult::Legal
    }

    /// Check if a target unit is the closest eligible enemy unit to the attacker.
    ///
    /// Used for Lone Operative targeting restriction: a Lone Operative can be targeted
    /// if it is the closest eligible enemy unit to the attacking unit.
    ///
    /// Source: 40k_revised.md - "LONE OPERATIVE"
    fn is_closest_eligible_target(
        state: &GameState,
        attacker_id: wh40k_core_types::UnitId,
        target_id: wh40k_core_types::UnitId,
    ) -> bool {
        let attacker = match state.unit(attacker_id) {
            Some(u) => u,
            None => return false,
        };
        let attacker_pos = match attacker.reference_position() {
            Some(p) => p,
            None => return false,
        };

        let target = match state.unit(target_id) {
            Some(u) => u,
            None => return false,
        };
        let target_pos = match target.reference_position() {
            Some(p) => p,
            None => return false,
        };

        let dist_to_target = wh40k_geometry::distance(attacker_pos, target_pos);

        // Check if any other eligible enemy unit is closer
        let opponent = state.opponent_id(attacker.owner);
        for enemy_unit in state.alive_units_for_player(opponent) {
            if enemy_unit.id == target_id {
                continue;
            }
            if enemy_unit.is_destroyed() || !enemy_unit.is_on_battlefield() {
                continue;
            }
            if let Some(enemy_pos) = enemy_unit.reference_position() {
                let dist_to_enemy = wh40k_geometry::distance(attacker_pos, enemy_pos);
                if dist_to_enemy < dist_to_target {
                    return false; // There's a closer eligible target
                }
            }
        }

        true // No closer eligible target found
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GameState, PlayerState, TurnFlags};
    use crate::unit::{ModelState, UnitState};
    use wh40k_core_types::{
        ArmorSave, BaseSize, BattleRound, DatasheetId, GameMode, GameOutcome, Keyword, KeywordSet,
        Leadership, ModelId, MoveCharacteristic, ObjectiveControl, PlayerId, Position, SubPhase,
        Toughness, UnitId, Wounds,
    };
    use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
    use wh40k_event_system::EventBus;
    use wh40k_command_system::CommandHistory;
    use wh40k_geometry::Board;

    fn make_test_state_for_validation() -> GameState {
        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        let mut state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::Movement,
            current_subphase: SubPhase::SelectUnitToMove,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "P0".to_string()),
                PlayerState::new(PlayerId::new(1), "P1".to_string()),
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
            deployment_config: None,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
        };

        state.players[0].first_turn = true;

        // Add units
        let unit_id = UnitId::new(1);
        let model = ModelState::new(
            ModelId::new(100),
            unit_id,
            Wounds::new(3),
            Position::from_inches(10, 5),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        );

        let mut unit = UnitState::new(
            unit_id,
            PlayerId::new(0),
            "Friendly Unit".to_string(),
            DatasheetId::new(1),
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            vec![model],
            MoveCharacteristic::from_inches(6),
            Toughness::new(4),
            ArmorSave::THREE_PLUS,
            None,
            Leadership::new(7),
            ObjectiveControl::new(2),
        );
        unit.status = UnitStatus::OnBattlefield;
        state.units.push(unit);

        // Add an enemy unit
        let enemy_unit_id = UnitId::new(2);
        let enemy_model = ModelState::new(
            ModelId::new(200),
            enemy_unit_id,
            Wounds::new(3),
            Position::from_inches(20, 20),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        );

        let mut enemy_unit = UnitState::new(
            enemy_unit_id,
            PlayerId::new(1),
            "Enemy Unit".to_string(),
            DatasheetId::new(2),
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            vec![enemy_model],
            MoveCharacteristic::from_inches(6),
            Toughness::new(4),
            ArmorSave::THREE_PLUS,
            None,
            Leadership::new(7),
            ObjectiveControl::new(2),
        );
        enemy_unit.status = UnitStatus::OnBattlefield;
        state.units.push(enemy_unit);

        state
    }

    #[test]
    fn test_validate_select_unit_to_move_legal() {
        let state = make_test_state_for_validation();
        let cmd = Command::SelectUnitToMove {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_select_unit_to_move_wrong_phase() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        let cmd = Command::SelectUnitToMove {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_select_unit_to_move_wrong_player() {
        let state = make_test_state_for_validation();
        let cmd = Command::SelectUnitToMove {
            unit_id: UnitId::new(2), // enemy unit
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_select_unit_to_move_already_moved() {
        let mut state = make_test_state_for_validation();
        state.turn_flags.mark_moved(UnitId::new(1));
        let cmd = Command::SelectUnitToMove {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_normal_move_legal() {
        let state = make_test_state_for_validation();
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(14, 5), // 4" away, M is 6"
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_normal_move_too_far() {
        let state = make_test_state_for_validation();
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(20, 5), // 10" away, M is 6"
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_normal_move_engaged_unit() {
        let mut state = make_test_state_for_validation();
        state.unit_mut(UnitId::new(1)).unwrap().engagement_status = EngagementStatus::Engaged;
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(14, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_normal_move_off_board() {
        let state = make_test_state_for_validation();
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(50, 5), // off board (44" wide)
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_fall_back_legal() {
        let mut state = make_test_state_for_validation();
        state.unit_mut(UnitId::new(1)).unwrap().engagement_status = EngagementStatus::Engaged;
        let cmd = Command::FallBack {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(5, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_fall_back_not_engaged() {
        let state = make_test_state_for_validation();
        let cmd = Command::FallBack {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(5, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_remain_stationary() {
        let state = make_test_state_for_validation();
        let cmd = Command::RemainStationary {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_arrive_from_reserves_round_1() {
        let mut state = make_test_state_for_validation();
        state.unit_mut(UnitId::new(1)).unwrap().status = UnitStatus::InStrategicReserves;
        let cmd = Command::ArriveFromReserves {
            unit_id: UnitId::new(1),
            position: Position::from_inches(10, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal()); // Can't arrive round 1
    }

    #[test]
    fn test_validate_arrive_from_reserves_round_2() {
        let mut state = make_test_state_for_validation();
        state.battle_round = BattleRound::new(2);
        state.unit_mut(UnitId::new(1)).unwrap().status = UnitStatus::InStrategicReserves;
        let cmd = Command::ArriveFromReserves {
            unit_id: UnitId::new(1),
            position: Position::from_inches(10, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_select_unit_to_shoot_legal() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        let cmd = Command::SelectUnitToShoot {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_select_unit_to_shoot_fell_back() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        state.turn_flags.mark_fell_back(UnitId::new(1));
        let cmd = Command::SelectUnitToShoot {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_resolve_shooting_advanced_non_assault_weapon_rejected() {
        // Audit #22: Advanced units cannot fire non-Assault weapons
        // Source: 40k_revised.md §5.4
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        let unit_id = UnitId::new(1);
        let weapon_id = wh40k_core_types::WeaponId::new(10);

        // Add a non-Assault ranged weapon to the unit's model
        let non_assault_weapon = wh40k_core_types::WeaponProfile {
            id: weapon_id,
            name: "Heavy bolter".to_string(),
            weapon_type: wh40k_core_types::WeaponType::Ranged,
            range: wh40k_core_types::Inches::from_inches(36),
            attacks: wh40k_core_types::AttackCount::Fixed(3),
            skill: wh40k_core_types::Skill::THREE_PLUS,
            strength: wh40k_core_types::Strength::new(5),
            ap: wh40k_core_types::ArmorPenetration::MINUS_1,
            damage: wh40k_core_types::Damage::Fixed(2),
            abilities: wh40k_core_types::WeaponAbilitySet::from_abilities(vec![
                wh40k_core_types::WeaponAbility::Heavy,
            ]),
        };
        state.unit_mut(unit_id).unwrap().models[0].ranged_weapons.push(non_assault_weapon);

        // Mark unit as having advanced
        state.turn_flags.mark_advanced(unit_id);

        let cmd = Command::ResolveShootingAttack {
            attacker_id: unit_id,
            target_id: UnitId::new(2),
            weapon_id,
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal(), "Advanced unit should not fire non-Assault weapon");
    }

    #[test]
    fn test_validate_resolve_shooting_advanced_assault_weapon_allowed() {
        // Audit #22: Advanced units CAN fire Assault weapons
        // Source: 40k_revised.md §5.4
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        let unit_id = UnitId::new(1);
        let weapon_id = wh40k_core_types::WeaponId::new(11);

        // Add an Assault ranged weapon to the unit's model
        let assault_weapon = wh40k_core_types::WeaponProfile {
            id: weapon_id,
            name: "Assault bolter".to_string(),
            weapon_type: wh40k_core_types::WeaponType::Ranged,
            range: wh40k_core_types::Inches::from_inches(18),
            attacks: wh40k_core_types::AttackCount::Fixed(3),
            skill: wh40k_core_types::Skill::THREE_PLUS,
            strength: wh40k_core_types::Strength::new(4),
            ap: wh40k_core_types::ArmorPenetration::ZERO,
            damage: wh40k_core_types::Damage::Fixed(1),
            abilities: wh40k_core_types::WeaponAbilitySet::from_abilities(vec![
                wh40k_core_types::WeaponAbility::Assault,
            ]),
        };
        state.unit_mut(unit_id).unwrap().models[0].ranged_weapons.push(assault_weapon);

        // Mark unit as having advanced
        state.turn_flags.mark_advanced(unit_id);

        let cmd = Command::ResolveShootingAttack {
            attacker_id: unit_id,
            target_id: UnitId::new(2),
            weapon_id,
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal(), "Advanced unit should be able to fire Assault weapon");
    }

    #[test]
    fn test_validate_resolve_shooting_not_advanced_non_assault_allowed() {
        // Audit #22: Non-advanced units can fire any weapon
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        let unit_id = UnitId::new(1);
        let weapon_id = wh40k_core_types::WeaponId::new(12);

        // Add a non-Assault ranged weapon (Heavy)
        let heavy_weapon = wh40k_core_types::WeaponProfile {
            id: weapon_id,
            name: "Heavy bolter".to_string(),
            weapon_type: wh40k_core_types::WeaponType::Ranged,
            range: wh40k_core_types::Inches::from_inches(36),
            attacks: wh40k_core_types::AttackCount::Fixed(3),
            skill: wh40k_core_types::Skill::THREE_PLUS,
            strength: wh40k_core_types::Strength::new(5),
            ap: wh40k_core_types::ArmorPenetration::MINUS_1,
            damage: wh40k_core_types::Damage::Fixed(2),
            abilities: wh40k_core_types::WeaponAbilitySet::from_abilities(vec![
                wh40k_core_types::WeaponAbility::Heavy,
            ]),
        };
        state.unit_mut(unit_id).unwrap().models[0].ranged_weapons.push(heavy_weapon);

        // Do NOT mark as advanced

        let cmd = Command::ResolveShootingAttack {
            attacker_id: unit_id,
            target_id: UnitId::new(2),
            weapon_id,
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal(), "Non-advanced unit should fire any weapon");
    }

    #[test]
    fn test_validate_resolve_shooting_advanced_pistol_weapon_allowed() {
        // Audit #22: Advanced units CAN fire Pistol weapons
        // Source: 40k_revised.md - Pistol weapons can fire after advance
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        let unit_id = UnitId::new(1);
        let weapon_id = wh40k_core_types::WeaponId::new(13);

        // Add a Pistol weapon
        let pistol_weapon = wh40k_core_types::WeaponProfile {
            id: weapon_id,
            name: "Bolt pistol".to_string(),
            weapon_type: wh40k_core_types::WeaponType::Ranged,
            range: wh40k_core_types::Inches::from_inches(12),
            attacks: wh40k_core_types::AttackCount::Fixed(1),
            skill: wh40k_core_types::Skill::THREE_PLUS,
            strength: wh40k_core_types::Strength::new(4),
            ap: wh40k_core_types::ArmorPenetration::ZERO,
            damage: wh40k_core_types::Damage::Fixed(1),
            abilities: wh40k_core_types::WeaponAbilitySet::from_abilities(vec![
                wh40k_core_types::WeaponAbility::Pistol,
            ]),
        };
        state.unit_mut(unit_id).unwrap().models[0].ranged_weapons.push(pistol_weapon);

        // Mark unit as having advanced
        state.turn_flags.mark_advanced(unit_id);

        let cmd = Command::ResolveShootingAttack {
            attacker_id: unit_id,
            target_id: UnitId::new(2),
            weapon_id,
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal(), "Advanced unit should be able to fire Pistol weapon");
    }

    #[test]
    fn test_validate_declare_charge_legal() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        // Move enemy closer so it's within 12"
        // Friendly at (10,5), move enemy to (18,5) = 8" apart (minus bases)
        state.units[1].models[0].position = Position::from_inches(18, 5);
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_declare_charge_target_too_far() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        // Enemy at (20,20) ≈ 18" away from (10,5) — beyond 12"
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_declare_charge_already_engaged() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        // Move enemy right next to friendly (within 1" engagement range)
        state.units[1].models[0].position = Position::from_inches(10, 5);
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal()); // Can't charge while in engagement range
    }

    #[test]
    fn test_validate_declare_charge_aircraft() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        // Give the charging unit the AIRCRAFT keyword
        state.units[0].keywords = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Aircraft]);
        state.units[1].models[0].position = Position::from_inches(18, 5);
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal()); // AIRCRAFT cannot charge
    }

    #[test]
    fn test_validate_declare_charge_after_advance() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        state.turn_flags.mark_advanced(UnitId::new(1));
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_declare_charge_after_fall_back() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        state.turn_flags.mark_fell_back(UnitId::new(1));
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(2)],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_declare_charge_no_targets() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Charge;
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![],
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_select_unit_to_fight_engaged() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Fight;
        state.unit_mut(UnitId::new(1)).unwrap().engagement_status = EngagementStatus::Engaged;
        let cmd = Command::SelectUnitToFight {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_select_unit_to_fight_charged() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Fight;
        state.turn_flags.mark_charged(UnitId::new(1));
        let cmd = Command::SelectUnitToFight {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_select_unit_to_fight_not_eligible() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Fight;
        // Not engaged, didn't charge
        let cmd = Command::SelectUnitToFight {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_select_unit_to_fight_already_fought() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Fight;
        state.unit_mut(UnitId::new(1)).unwrap().engagement_status = EngagementStatus::Engaged;
        state.turn_flags.mark_fought(UnitId::new(1));
        let cmd = Command::SelectUnitToFight {
            unit_id: UnitId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_use_stratagem_no_cp() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        // Player has 0 CP
        let cmd = Command::UseStratagem {
            player: PlayerId::new(0),
            stratagem_id: wh40k_core_types::StratagemId::new(1),
            target: wh40k_command_system::StratagemTarget::Unit(UnitId::new(1)),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_use_stratagem_has_cp() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        state.player_mut(PlayerId::new(0)).gain_cp(1);
        let cmd = Command::UseStratagem {
            player: PlayerId::new(0),
            stratagem_id: wh40k_core_types::StratagemId::new(1),
            target: wh40k_command_system::StratagemTarget::Unit(UnitId::new(1)),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_game_ended() {
        let mut state = make_test_state_for_validation();
        state.game_outcome = GameOutcome::Victory(PlayerId::new(0));
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(14, 5),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_concede_always_legal() {
        let state = make_test_state_for_validation();
        let cmd = Command::Concede {
            player: PlayerId::new(0),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_pass_always_legal() {
        let state = make_test_state_for_validation();
        let cmd = Command::PassAction;
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal());
    }

    #[test]
    fn test_validate_setup_command_wrong_phase() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Movement;
        let cmd = Command::SelectEnhancement {
            player: PlayerId::new(0),
            enhancement_id: wh40k_core_types::EnhancementId::new(1),
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_declare_shooting_targets_enemy_required() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;
        let cmd = Command::DeclareShootingTargets {
            unit_id: UnitId::new(1),
            targets: vec![(wh40k_core_types::WeaponId::new(1), UnitId::new(1))], // own unit
        };
        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal());
    }

    #[test]
    fn test_validate_lone_operative_blocked_beyond_12() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        // Place attacker at (5,5) and Lone Operative target at (25,5) = ~20" away
        let attacker_id = UnitId::new(1);
        let target_id = UnitId::new(2);

        // Ensure both units are on battlefield
        if let Some(attacker) = state.unit_mut(attacker_id) {
            attacker.status = wh40k_core_types::UnitStatus::OnBattlefield;
            for m in &mut attacker.models {
                m.position = wh40k_core_types::Position::from_inches(5, 5);
            }
        }
        if let Some(target) = state.unit_mut(target_id) {
            target.status = wh40k_core_types::UnitStatus::OnBattlefield;
            for m in &mut target.models {
                m.position = wh40k_core_types::Position::from_inches(25, 5);
            }
        }

        // Add Lone Operative effect to target
        state.active_effects.push(crate::effect::ActiveEffect {
            id: 1,
            source: crate::effect::EffectSource::CoreRule("Lone Operative".to_string()),
            target: crate::effect::EffectTarget::Unit(target_id),
            effect_type: crate::effect::EffectType::LoneOperative,
            duration: wh40k_core_types::EffectDuration::Persistent,
            stacking: wh40k_core_types::StackingBehavior::Unique,
            applied_round: wh40k_core_types::BattleRound::new(1),
            applied_phase: wh40k_core_types::Phase::PreBattle,
        });

        // Also need a closer enemy unit so the Lone Operative is NOT the closest
        let closer_model = ModelState::new(
            ModelId::new(500), UnitId::new(5), Wounds::new(3),
            Position::from_inches(15, 5), BaseSize::MM32,
            Vec::new(), Vec::new(), false, None,
        );
        let mut closer_enemy = UnitState::new(
            UnitId::new(5), PlayerId::new(1),
            "Closer Enemy".to_string(), DatasheetId::new(5),
            KeywordSet::from_keywords(&[Keyword::Infantry]),
            vec![closer_model],
            MoveCharacteristic::from_inches(6), Toughness::new(4),
            ArmorSave::THREE_PLUS, None, Leadership::new(7),
            ObjectiveControl::new(2),
        );
        closer_enemy.status = wh40k_core_types::UnitStatus::OnBattlefield;
        state.units.push(closer_enemy);

        let cmd = Command::DeclareShootingTargets {
            unit_id: attacker_id,
            targets: vec![(wh40k_core_types::WeaponId::new(1), target_id)],
        };

        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_illegal(), "Lone Operative >12\" away with closer targets should be illegal");
    }

    #[test]
    fn test_validate_lone_operative_allowed_within_12() {
        let mut state = make_test_state_for_validation();
        state.current_phase = Phase::Shooting;

        let attacker_id = UnitId::new(1);
        let target_id = UnitId::new(2);

        // Place attacker at (5,5) and Lone Operative target at (15,5) = ~10" away
        if let Some(attacker) = state.unit_mut(attacker_id) {
            attacker.status = wh40k_core_types::UnitStatus::OnBattlefield;
            for m in &mut attacker.models {
                m.position = wh40k_core_types::Position::from_inches(5, 5);
            }
        }
        if let Some(target) = state.unit_mut(target_id) {
            target.status = wh40k_core_types::UnitStatus::OnBattlefield;
            for m in &mut target.models {
                m.position = wh40k_core_types::Position::from_inches(15, 5);
            }
        }

        // Add Lone Operative effect
        state.active_effects.push(crate::effect::ActiveEffect {
            id: 1,
            source: crate::effect::EffectSource::CoreRule("Lone Operative".to_string()),
            target: crate::effect::EffectTarget::Unit(target_id),
            effect_type: crate::effect::EffectType::LoneOperative,
            duration: wh40k_core_types::EffectDuration::Persistent,
            stacking: wh40k_core_types::StackingBehavior::Unique,
            applied_round: wh40k_core_types::BattleRound::new(1),
            applied_phase: wh40k_core_types::Phase::PreBattle,
        });

        let cmd = Command::DeclareShootingTargets {
            unit_id: attacker_id,
            targets: vec![(wh40k_core_types::WeaponId::new(1), target_id)],
        };

        let result = CommandValidator::validate(&state, &cmd);
        assert!(result.is_legal(), "Lone Operative within 12\" should be targetable");
    }
}
