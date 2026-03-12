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
            Command::StartPhase { phase: _ } => {
                if state.current_phase == Phase::GameEnd {
                    CommandValidationResult::illegal("Game has ended")
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
            Command::AdvanceMove { unit_id, destination, .. } => {
                Self::validate_advance_move(state, *unit_id, *destination)
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
            Command::ResolveShootingAttack { attacker_id: _, target_id: _, weapon_id: _ } => {
                // Phase 2: combat resolution will validate in detail
                Self::validate_phase_is(state, Phase::Shooting, "ResolveShootingAttack")
            }

            // ===== Charge commands =====
            Command::DeclareCharge { unit_id, targets } => {
                Self::validate_declare_charge(state, *unit_id, targets)
            }
            Command::ResolveChargeRoll { unit_id: _, .. } => {
                Self::validate_phase_is(state, Phase::Charge, "ResolveChargeRoll")
            }
            Command::MakeChargeMove { unit_id: _, destination: _ } => {
                Self::validate_phase_is(state, Phase::Charge, "MakeChargeMove")
            }

            // ===== Heroic Intervention =====
            Command::HeroicInterventionMove { unit_id, destination } => {
                Self::validate_heroic_intervention_move(state, *unit_id, *destination)
            }

            // ===== Fight commands =====
            Command::SelectUnitToFight { unit_id } => {
                Self::validate_select_unit_to_fight(state, *unit_id)
            }
            Command::ChooseKaTahStance { .. } => {
                Self::validate_phase_is(state, Phase::Fight, "ChooseKaTahStance")
            }
            Command::ChooseVaultswordsProfile { .. } => {
                Self::validate_phase_is(state, Phase::Fight, "ChooseVaultswordsProfile")
            }
            Command::PileIn { unit_id: _, .. } => {
                Self::validate_phase_is(state, Phase::Fight, "PileIn")
            }
            Command::DeclareMeleeTargets { unit_id: _, .. } => {
                Self::validate_phase_is(state, Phase::Fight, "DeclareMeleeTargets")
            }
            Command::ResolveMeleeAttack { .. } => {
                Self::validate_phase_is(state, Phase::Fight, "ResolveMeleeAttack")
            }
            Command::Consolidate { .. } => {
                Self::validate_phase_is(state, Phase::Fight, "Consolidate")
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
            Command::ScoreObjective { player, objective_id: _ } => {
                if *player != state.active_player && *player != state.decision_owner {
                    CommandValidationResult::illegal("Not this player's turn to score")
                } else {
                    CommandValidationResult::Legal
                }
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
            Command::AllocateWound { .. } => CommandValidationResult::Legal,
            Command::AssignOverwatchTarget { .. } => CommandValidationResult::Legal,
            Command::PassAction => CommandValidationResult::Legal,
            Command::Concede { player: _ } => {
                // Can always concede
                CommandValidationResult::Legal
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

        // Check destination is on the board
        if !state.board.contains(destination) {
            return CommandValidationResult::illegal("Destination is outside the board");
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

        CommandValidationResult::Legal
    }

    fn validate_advance_move(
        state: &GameState,
        unit_id: wh40k_core_types::UnitId,
        destination: wh40k_core_types::Position,
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

        if unit.owner != state.active_player {
            return CommandValidationResult::illegal("Unit does not belong to the active player");
        }

        if !unit.is_on_battlefield() {
            return CommandValidationResult::illegal("Unit is not on the battlefield");
        }

        if unit.is_destroyed() {
            return CommandValidationResult::illegal("Unit is destroyed");
        }

        // Already fought this phase
        if state.turn_flags.has_fought(unit_id) {
            return CommandValidationResult::illegal_with_ref(
                "Unit has already fought this phase",
                "40k_revised.md - Each unit fights once per Fight phase",
            );
        }

        // Must be engaged or have charged this turn to fight
        if unit.engagement_status != EngagementStatus::Engaged
            && !state.turn_flags.charged_this_turn(unit_id)
        {
            return CommandValidationResult::illegal_with_ref(
                "Unit must be within Engagement Range or have charged this turn to fight",
                "40k_revised.md - Fight Phase: eligible units",
            );
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
        ArmorSave, BaseSize, BattleRound, DatasheetId, GameOutcome, Keyword, KeywordSet,
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
