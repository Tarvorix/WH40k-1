//! WH40K Engine - Command System crate
//!
//! Defines the command pipeline for the 40K engine. All state changes go
//! through commands. The search engine uses the same command processor as
//! live play.
//!
//! Source: implementation_v3.md Section 6.4 (Layer D - Command system)
//!
//! ## Architecture
//!
//! - [`Command`] - All possible game actions (setup, movement, shooting, etc.)
//! - [`CommandValidationResult`] - Whether a command is legal or illegal
//! - [`CommandRecord`] - A command plus metadata for the history
//! - [`CommandHistory`] - Append-only log with query support
//! - [`DecisionSurface`] - Legal actions at a decision point
//! - [`StratagemTarget`] - What a stratagem targets
//! - [`BlessingAllocation`] - For Blessings of Khorne dice allocation

use serde::{Deserialize, Serialize};
use std::time::Duration;

use wh40k_core_types::{
    BattleRound, CommandId, EnhancementId, FactionId, HatchwayId, ModelId, ObjectiveId, Phase,
    PlayerId, Position, SecondaryObjectiveId, StratagemId, TacticalManoeuvre, UnitId, WeaponId,
};

// Re-export key types at crate root
pub use command::Command;
pub use decision::{DecisionSurface, DecisionType};
pub use history::{CommandHistory, CommandRecord};
pub use stratagem::StratagemTarget;
pub use validation::CommandValidationResult;
pub use blessings::BlessingAllocation;

// ============================================================================
// Command enum
// ============================================================================

pub mod command {
    use super::*;

    /// All possible game actions. Every state change in the engine goes through
    /// a Command.
    ///
    /// Commands are grouped by category:
    /// - Setup: pre-battle configuration
    /// - Phase control: turn/phase progression
    /// - Movement: unit movement actions
    /// - Shooting: ranged attack declaration and resolution
    /// - Charge: charge declaration and resolution
    /// - Fight: melee combat
    /// - Stratagem: stratagem usage
    /// - Scoring: objective scoring
    /// - Blessings of Khorne: faction-specific dice allocation
    /// - Misc: wound allocation, overwatch, pass, concede
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum Command {
        // ===== Setup commands =====
        /// Player selects their faction.
        SelectFaction {
            player: PlayerId,
            faction_id: FactionId,
        },

        /// Player selects a patrol squad variant.
        SelectPatrolSquad {
            player: PlayerId,
            squad_index: u32,
        },

        /// Player selects an enhancement for their warlord.
        SelectEnhancement {
            player: PlayerId,
            enhancement_id: EnhancementId,
        },

        /// Player selects a secondary objective.
        SelectSecondaryObjective {
            player: PlayerId,
            secondary_id: SecondaryObjectiveId,
        },

        /// Place a unit at a specific position during deployment.
        PlaceUnit {
            player: PlayerId,
            unit_id: UnitId,
            position: Position,
        },

        /// Determine which player goes first (after deployment).
        DetermineFirstTurn {
            first_player: PlayerId,
        },

        /// Mark setup as complete; the game can begin.
        SetupComplete,

        // ===== Phase control commands =====
        /// Start a new battle round.
        StartBattleRound {
            round: BattleRound,
        },

        /// Start a specific phase.
        StartPhase {
            phase: Phase,
        },

        /// End the current phase.
        EndPhase {
            phase: Phase,
        },

        /// End the current player's turn.
        EndPlayerTurn {
            player: PlayerId,
        },

        // ===== Movement commands =====
        /// Select a unit to move this phase.
        SelectUnitToMove {
            unit_id: UnitId,
        },

        /// Perform a normal move (up to M characteristic).
        NormalMove {
            unit_id: UnitId,
            destination: Position,
        },

        /// Perform an advance move (M + advance roll).
        AdvanceMove {
            unit_id: UnitId,
            destination: Position,
            advance_roll: u8,
        },

        /// Fall back from engagement range.
        FallBack {
            unit_id: UnitId,
            destination: Position,
        },

        /// Remain stationary (do not move).
        RemainStationary {
            unit_id: UnitId,
        },

        /// Arrive from strategic reserves / deep strike.
        ArriveFromReserves {
            unit_id: UnitId,
            position: Position,
        },

        // ===== Shooting commands =====
        /// Select a unit to shoot this phase.
        SelectUnitToShoot {
            unit_id: UnitId,
        },

        /// Declare which weapons fire at which targets.
        DeclareShootingTargets {
            unit_id: UnitId,
            targets: Vec<(WeaponId, UnitId)>,
        },

        /// Resolve a single ranged attack sequence (weapon vs target).
        ResolveShootingAttack {
            attacker_id: UnitId,
            target_id: UnitId,
            weapon_id: WeaponId,
        },

        // ===== Charge commands =====
        /// Declare a charge against one or more target units.
        DeclareCharge {
            unit_id: UnitId,
            targets: Vec<UnitId>,
        },

        /// Resolve the charge roll (2D6).
        ResolveChargeRoll {
            unit_id: UnitId,
            roll: u8,
        },

        /// Make the charge move to contact.
        MakeChargeMove {
            unit_id: UnitId,
            destination: Position,
        },

        // ===== Fight commands =====
        /// Select a unit to fight this phase.
        SelectUnitToFight {
            unit_id: UnitId,
        },

        /// Choose a Martial Ka'tah stance (Custodes faction ability).
        ChooseKaTahStance {
            unit_id: UnitId,
            stance: String,
        },

        /// Choose a Vaultswords melee profile (Tristraen).
        ChooseVaultswordsProfile {
            model_id: ModelId,
            profile: String,
        },

        /// Pile in: move models up to 3" (or 6") closer to the nearest enemy.
        PileIn {
            unit_id: UnitId,
            positions: Vec<(ModelId, Position)>,
        },

        /// Declare melee weapon targets.
        DeclareMeleeTargets {
            unit_id: UnitId,
            targets: Vec<(WeaponId, UnitId)>,
        },

        /// Resolve a melee attack sequence (weapon vs target).
        ResolveMeleeAttack {
            attacker_id: UnitId,
            target_id: UnitId,
            weapon_id: WeaponId,
        },

        /// Consolidate: move models up to 3" (or 6") closer to the nearest enemy.
        Consolidate {
            unit_id: UnitId,
            positions: Vec<(ModelId, Position)>,
        },

        // ===== Heroic Intervention =====
        /// Execute a Heroic Intervention move (up to 6" towards nearest enemy).
        /// Source: 40k_revised.md - "Heroic Intervention"
        HeroicInterventionMove {
            unit_id: UnitId,
            destination: Position,
        },

        // ===== Stratagem commands =====
        /// Use a stratagem, specifying its target.
        UseStratagem {
            player: PlayerId,
            stratagem_id: StratagemId,
            target: StratagemTarget,
        },

        /// Decline to use a stratagem in the current window.
        DeclineStratagem {
            player: PlayerId,
        },

        // ===== Scoring commands =====
        /// Score an objective (claim victory points).
        ScoreObjective {
            player: PlayerId,
            objective_id: ObjectiveId,
        },

        /// Raze an objective (Scorched Earth mission action).
        RazeObjective {
            player: PlayerId,
            objective_id: ObjectiveId,
        },

        // ===== Blessings of Khorne =====
        /// Allocate Blessings of Khorne dice to blessings.
        AllocateBlessings {
            player: PlayerId,
            allocations: Vec<BlessingAllocation>,
        },

        // ===== Misc commands =====
        /// Allocate a wound to a specific model.
        AllocateWound {
            target_model_id: ModelId,
        },

        /// Assign an overwatch target (for Fire Overwatch stratagem).
        AssignOverwatchTarget {
            unit_id: UnitId,
        },

        /// Pass the current action (do nothing).
        PassAction,

        /// Concede the game.
        Concede {
            player: PlayerId,
        },

        // ===== Boarding Actions commands =====
        /// Operate a hatchway (open or close). Boarding Actions only.
        /// Source: boarding_actions_complete_v3.md Section 3.2
        OperateHatchway {
            player: PlayerId,
            unit_id: UnitId,
            hatchway_id: HatchwayId,
        },

        /// Perform a Tactical Manoeuvre at the start of the Shooting Phase.
        /// Source: boarding_actions_complete_v3.md Section 3.4
        PerformTacticalManoeuvre {
            player: PlayerId,
            unit_id: UnitId,
            manoeuvre: TacticalManoeuvre,
            /// Target objective for SecureSite, None for other manoeuvres
            target_objective: Option<ObjectiveId>,
        },

        /// Use the Battlefield Command stratagem to project a leader ability.
        /// Source: boarding_actions_complete_v3.md Section 3.7, Section 4.2
        UseBattlefieldCommand {
            player: PlayerId,
            leader_unit_id: UnitId,
            target_unit_id: UnitId,
            ability_name: String,
        },

        /// Arrive from an entry zone (Boarding Actions reserves).
        /// Source: boarding_actions_complete_v3.md Section 7.5
        ArriveFromEntryZone {
            player: PlayerId,
            unit_id: UnitId,
            entry_zone_id: u32,
            position: Position,
        },

        /// Perform a mission-specific action (e.g., download data, corrupt objective, change level).
        /// Source: boarding_actions_mission_tags_complete_v3.json
        BoardingMissionAction {
            player: PlayerId,
            action_type: String,
            /// Optional target (objective ID, hatchway ID, region, etc.)
            target: Option<String>,
        },
    }

    impl Command {
        /// Returns the player associated with this command, if any.
        /// Some commands (like SetupComplete, PassAction) are not player-specific.
        pub fn player(&self) -> Option<PlayerId> {
            match self {
                Command::SelectFaction { player, .. } => Some(*player),
                Command::SelectPatrolSquad { player, .. } => Some(*player),
                Command::SelectEnhancement { player, .. } => Some(*player),
                Command::SelectSecondaryObjective { player, .. } => Some(*player),
                Command::PlaceUnit { player, .. } => Some(*player),
                Command::DetermineFirstTurn { first_player } => Some(*first_player),
                Command::EndPlayerTurn { player } => Some(*player),
                Command::UseStratagem { player, .. } => Some(*player),
                Command::DeclineStratagem { player } => Some(*player),
                Command::ScoreObjective { player, .. } => Some(*player),
                Command::RazeObjective { player, .. } => Some(*player),
                Command::AllocateBlessings { player, .. } => Some(*player),
                Command::Concede { player } => Some(*player),
                Command::OperateHatchway { player, .. } => Some(*player),
                Command::PerformTacticalManoeuvre { player, .. } => Some(*player),
                Command::UseBattlefieldCommand { player, .. } => Some(*player),
                Command::ArriveFromEntryZone { player, .. } => Some(*player),
                Command::BoardingMissionAction { player, .. } => Some(*player),
                _ => None,
            }
        }

        /// Returns the unit(s) involved in this command, if any.
        pub fn unit_ids(&self) -> Vec<UnitId> {
            match self {
                Command::PlaceUnit { unit_id, .. } => vec![*unit_id],
                Command::SelectUnitToMove { unit_id } => vec![*unit_id],
                Command::NormalMove { unit_id, .. } => vec![*unit_id],
                Command::AdvanceMove { unit_id, .. } => vec![*unit_id],
                Command::FallBack { unit_id, .. } => vec![*unit_id],
                Command::RemainStationary { unit_id } => vec![*unit_id],
                Command::ArriveFromReserves { unit_id, .. } => vec![*unit_id],
                Command::SelectUnitToShoot { unit_id } => vec![*unit_id],
                Command::DeclareShootingTargets { unit_id, targets } => {
                    let mut ids = vec![*unit_id];
                    for (_, target) in targets {
                        if !ids.contains(target) {
                            ids.push(*target);
                        }
                    }
                    ids
                }
                Command::ResolveShootingAttack {
                    attacker_id,
                    target_id,
                    ..
                } => vec![*attacker_id, *target_id],
                Command::DeclareCharge { unit_id, targets } => {
                    let mut ids = vec![*unit_id];
                    for target in targets {
                        if !ids.contains(target) {
                            ids.push(*target);
                        }
                    }
                    ids
                }
                Command::ResolveChargeRoll { unit_id, .. } => vec![*unit_id],
                Command::MakeChargeMove { unit_id, .. } => vec![*unit_id],
                Command::SelectUnitToFight { unit_id } => vec![*unit_id],
                Command::ChooseKaTahStance { unit_id, .. } => vec![*unit_id],
                Command::PileIn { unit_id, .. } => vec![*unit_id],
                Command::DeclareMeleeTargets { unit_id, targets } => {
                    let mut ids = vec![*unit_id];
                    for (_, target) in targets {
                        if !ids.contains(target) {
                            ids.push(*target);
                        }
                    }
                    ids
                }
                Command::ResolveMeleeAttack {
                    attacker_id,
                    target_id,
                    ..
                } => vec![*attacker_id, *target_id],
                Command::Consolidate { unit_id, .. } => vec![*unit_id],
                Command::HeroicInterventionMove { unit_id, .. } => vec![*unit_id],
                Command::AssignOverwatchTarget { unit_id } => vec![*unit_id],
                Command::OperateHatchway { unit_id, .. } => vec![*unit_id],
                Command::PerformTacticalManoeuvre { unit_id, .. } => vec![*unit_id],
                Command::UseBattlefieldCommand {
                    leader_unit_id,
                    target_unit_id,
                    ..
                } => vec![*leader_unit_id, *target_unit_id],
                Command::ArriveFromEntryZone { unit_id, .. } => vec![*unit_id],
                _ => vec![],
            }
        }

        /// Returns a human-readable category string for this command.
        pub fn category(&self) -> &'static str {
            match self {
                Command::SelectFaction { .. }
                | Command::SelectPatrolSquad { .. }
                | Command::SelectEnhancement { .. }
                | Command::SelectSecondaryObjective { .. }
                | Command::PlaceUnit { .. }
                | Command::DetermineFirstTurn { .. }
                | Command::SetupComplete => "Setup",

                Command::StartBattleRound { .. }
                | Command::StartPhase { .. }
                | Command::EndPhase { .. }
                | Command::EndPlayerTurn { .. } => "PhaseControl",

                Command::SelectUnitToMove { .. }
                | Command::NormalMove { .. }
                | Command::AdvanceMove { .. }
                | Command::FallBack { .. }
                | Command::RemainStationary { .. }
                | Command::ArriveFromReserves { .. } => "Movement",

                Command::SelectUnitToShoot { .. }
                | Command::DeclareShootingTargets { .. }
                | Command::ResolveShootingAttack { .. } => "Shooting",

                Command::DeclareCharge { .. }
                | Command::ResolveChargeRoll { .. }
                | Command::MakeChargeMove { .. }
                | Command::HeroicInterventionMove { .. } => "Charge",

                Command::SelectUnitToFight { .. }
                | Command::ChooseKaTahStance { .. }
                | Command::ChooseVaultswordsProfile { .. }
                | Command::PileIn { .. }
                | Command::DeclareMeleeTargets { .. }
                | Command::ResolveMeleeAttack { .. }
                | Command::Consolidate { .. } => "Fight",

                Command::UseStratagem { .. } | Command::DeclineStratagem { .. } => "Stratagem",

                Command::ScoreObjective { .. } | Command::RazeObjective { .. } => "Scoring",

                Command::AllocateBlessings { .. } => "BlessingsOfKhorne",

                Command::AllocateWound { .. }
                | Command::AssignOverwatchTarget { .. }
                | Command::PassAction
                | Command::Concede { .. } => "Misc",

                Command::OperateHatchway { .. }
                | Command::PerformTacticalManoeuvre { .. }
                | Command::UseBattlefieldCommand { .. }
                | Command::ArriveFromEntryZone { .. }
                | Command::BoardingMissionAction { .. } => "BoardingActions",
            }
        }

        /// Returns true if this is a setup-phase command.
        pub fn is_setup(&self) -> bool {
            self.category() == "Setup"
        }

        /// Returns true if this is a phase-control command.
        pub fn is_phase_control(&self) -> bool {
            self.category() == "PhaseControl"
        }

        /// Returns true if this is a movement command.
        pub fn is_movement(&self) -> bool {
            self.category() == "Movement"
        }

        /// Returns true if this is a shooting command.
        pub fn is_shooting(&self) -> bool {
            self.category() == "Shooting"
        }

        /// Returns true if this is a charge command.
        pub fn is_charge(&self) -> bool {
            self.category() == "Charge"
        }

        /// Returns true if this is a fight command.
        pub fn is_fight(&self) -> bool {
            self.category() == "Fight"
        }

        /// Returns true if this is a stratagem command.
        pub fn is_stratagem(&self) -> bool {
            self.category() == "Stratagem"
        }
    }

    impl std::fmt::Display for Command {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Command::SelectFaction { player, faction_id } => {
                    write!(f, "Player {} selects faction {}", player, faction_id)
                }
                Command::SelectPatrolSquad { player, squad_index } => {
                    write!(
                        f,
                        "Player {} selects patrol squad #{}",
                        player, squad_index
                    )
                }
                Command::SelectEnhancement {
                    player,
                    enhancement_id,
                } => {
                    write!(
                        f,
                        "Player {} selects enhancement {}",
                        player, enhancement_id
                    )
                }
                Command::SelectSecondaryObjective {
                    player,
                    secondary_id,
                } => {
                    write!(
                        f,
                        "Player {} selects secondary objective {}",
                        player, secondary_id
                    )
                }
                Command::PlaceUnit {
                    player,
                    unit_id,
                    position,
                } => {
                    write!(
                        f,
                        "Player {} places unit {} at {}",
                        player, unit_id, position
                    )
                }
                Command::DetermineFirstTurn { first_player } => {
                    write!(f, "Player {} goes first", first_player)
                }
                Command::SetupComplete => write!(f, "Setup complete"),
                Command::StartBattleRound { round } => {
                    write!(f, "Start battle round {}", round.number())
                }
                Command::StartPhase { phase } => write!(f, "Start phase {:?}", phase),
                Command::EndPhase { phase } => write!(f, "End phase {:?}", phase),
                Command::EndPlayerTurn { player } => {
                    write!(f, "Player {} ends turn", player)
                }
                Command::SelectUnitToMove { unit_id } => {
                    write!(f, "Select unit {} to move", unit_id)
                }
                Command::NormalMove {
                    unit_id,
                    destination,
                } => {
                    write!(f, "Normal move unit {} to {}", unit_id, destination)
                }
                Command::AdvanceMove {
                    unit_id,
                    destination,
                    advance_roll,
                } => {
                    write!(
                        f,
                        "Advance unit {} to {} (roll: {})",
                        unit_id, destination, advance_roll
                    )
                }
                Command::FallBack {
                    unit_id,
                    destination,
                } => {
                    write!(f, "Fall back unit {} to {}", unit_id, destination)
                }
                Command::RemainStationary { unit_id } => {
                    write!(f, "Unit {} remains stationary", unit_id)
                }
                Command::ArriveFromReserves { unit_id, position } => {
                    write!(
                        f,
                        "Unit {} arrives from reserves at {}",
                        unit_id, position
                    )
                }
                Command::SelectUnitToShoot { unit_id } => {
                    write!(f, "Select unit {} to shoot", unit_id)
                }
                Command::DeclareShootingTargets { unit_id, targets } => {
                    write!(
                        f,
                        "Unit {} declares {} shooting target(s)",
                        unit_id,
                        targets.len()
                    )
                }
                Command::ResolveShootingAttack {
                    attacker_id,
                    target_id,
                    weapon_id,
                } => {
                    write!(
                        f,
                        "Resolve shooting: unit {} attacks unit {} with weapon {}",
                        attacker_id, target_id, weapon_id
                    )
                }
                Command::DeclareCharge { unit_id, targets } => {
                    write!(
                        f,
                        "Unit {} declares charge against {} target(s)",
                        unit_id,
                        targets.len()
                    )
                }
                Command::ResolveChargeRoll { unit_id, roll } => {
                    write!(f, "Unit {} charge roll: {}", unit_id, roll)
                }
                Command::MakeChargeMove {
                    unit_id,
                    destination,
                } => {
                    write!(f, "Unit {} charge move to {}", unit_id, destination)
                }
                Command::HeroicInterventionMove {
                    unit_id,
                    destination,
                } => {
                    write!(
                        f,
                        "Unit {} heroic intervention move to {}",
                        unit_id, destination
                    )
                }
                Command::SelectUnitToFight { unit_id } => {
                    write!(f, "Select unit {} to fight", unit_id)
                }
                Command::ChooseKaTahStance { unit_id, stance } => {
                    write!(f, "Unit {} chooses Ka'tah stance: {}", unit_id, stance)
                }
                Command::ChooseVaultswordsProfile { model_id, profile } => {
                    write!(
                        f,
                        "Model {} chooses Vaultswords profile: {}",
                        model_id, profile
                    )
                }
                Command::PileIn { unit_id, positions } => {
                    write!(
                        f,
                        "Unit {} pile in ({} models)",
                        unit_id,
                        positions.len()
                    )
                }
                Command::DeclareMeleeTargets { unit_id, targets } => {
                    write!(
                        f,
                        "Unit {} declares {} melee target(s)",
                        unit_id,
                        targets.len()
                    )
                }
                Command::ResolveMeleeAttack {
                    attacker_id,
                    target_id,
                    weapon_id,
                } => {
                    write!(
                        f,
                        "Resolve melee: unit {} attacks unit {} with weapon {}",
                        attacker_id, target_id, weapon_id
                    )
                }
                Command::Consolidate { unit_id, positions } => {
                    write!(
                        f,
                        "Unit {} consolidates ({} models)",
                        unit_id,
                        positions.len()
                    )
                }
                Command::UseStratagem {
                    player,
                    stratagem_id,
                    target,
                } => {
                    write!(
                        f,
                        "Player {} uses stratagem {} on {:?}",
                        player, stratagem_id, target
                    )
                }
                Command::DeclineStratagem { player } => {
                    write!(f, "Player {} declines stratagem", player)
                }
                Command::ScoreObjective {
                    player,
                    objective_id,
                } => {
                    write!(
                        f,
                        "Player {} scores objective {}",
                        player, objective_id
                    )
                }
                Command::RazeObjective {
                    player,
                    objective_id,
                } => {
                    write!(
                        f,
                        "Player {} razes objective {}",
                        player, objective_id
                    )
                }
                Command::AllocateBlessings {
                    player,
                    allocations,
                } => {
                    write!(
                        f,
                        "Player {} allocates {} blessing(s) of Khorne",
                        player,
                        allocations.len()
                    )
                }
                Command::AllocateWound { target_model_id } => {
                    write!(f, "Allocate wound to model {}", target_model_id)
                }
                Command::AssignOverwatchTarget { unit_id } => {
                    write!(f, "Assign overwatch to unit {}", unit_id)
                }
                Command::PassAction => write!(f, "Pass"),
                Command::Concede { player } => write!(f, "Player {} concedes", player),
                Command::OperateHatchway {
                    player,
                    unit_id,
                    hatchway_id,
                } => {
                    write!(
                        f,
                        "Player {} unit {} operates hatchway {}",
                        player, unit_id, hatchway_id
                    )
                }
                Command::PerformTacticalManoeuvre {
                    player,
                    unit_id,
                    manoeuvre,
                    target_objective,
                } => {
                    if let Some(obj) = target_objective {
                        write!(
                            f,
                            "Player {} unit {} performs {:?} on objective {}",
                            player, unit_id, manoeuvre, obj
                        )
                    } else {
                        write!(
                            f,
                            "Player {} unit {} performs {:?}",
                            player, unit_id, manoeuvre
                        )
                    }
                }
                Command::UseBattlefieldCommand {
                    player,
                    leader_unit_id,
                    target_unit_id,
                    ability_name,
                } => {
                    write!(
                        f,
                        "Player {} Battlefield Command: leader {} projects '{}' to unit {}",
                        player, leader_unit_id, ability_name, target_unit_id
                    )
                }
                Command::ArriveFromEntryZone {
                    player,
                    unit_id,
                    entry_zone_id,
                    position,
                } => {
                    write!(
                        f,
                        "Player {} unit {} arrives from entry zone {} at {}",
                        player, unit_id, entry_zone_id, position
                    )
                }
                Command::BoardingMissionAction {
                    player,
                    action_type,
                    target,
                } => {
                    if let Some(t) = target {
                        write!(
                            f,
                            "Player {} boarding mission action '{}' on {}",
                            player, action_type, t
                        )
                    } else {
                        write!(
                            f,
                            "Player {} boarding mission action '{}'",
                            player, action_type
                        )
                    }
                }
            }
        }
    }
}

// ============================================================================
// Validation result
// ============================================================================

pub mod validation {
    use super::*;

    /// Result of validating a command against the current game state.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum CommandValidationResult {
        /// The command is legal in the current game state.
        Legal,
        /// The command is illegal, with a reason and optional rule reference.
        Illegal {
            reason: String,
            rule_reference: Option<String>,
        },
    }

    impl CommandValidationResult {
        /// Returns true if the command is legal.
        pub fn is_legal(&self) -> bool {
            matches!(self, CommandValidationResult::Legal)
        }

        /// Returns true if the command is illegal.
        pub fn is_illegal(&self) -> bool {
            matches!(self, CommandValidationResult::Illegal { .. })
        }

        /// Create a new illegal result with a reason.
        pub fn illegal(reason: impl Into<String>) -> Self {
            CommandValidationResult::Illegal {
                reason: reason.into(),
                rule_reference: None,
            }
        }

        /// Create a new illegal result with a reason and rule reference.
        pub fn illegal_with_ref(
            reason: impl Into<String>,
            rule_reference: impl Into<String>,
        ) -> Self {
            CommandValidationResult::Illegal {
                reason: reason.into(),
                rule_reference: Some(rule_reference.into()),
            }
        }

        /// Get the reason string if illegal, None if legal.
        pub fn reason(&self) -> Option<&str> {
            match self {
                CommandValidationResult::Legal => None,
                CommandValidationResult::Illegal { reason, .. } => Some(reason),
            }
        }

        /// Get the rule reference if illegal, None otherwise.
        pub fn rule_reference(&self) -> Option<&str> {
            match self {
                CommandValidationResult::Legal => None,
                CommandValidationResult::Illegal { rule_reference, .. } => {
                    rule_reference.as_deref()
                }
            }
        }
    }

    impl std::fmt::Display for CommandValidationResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                CommandValidationResult::Legal => write!(f, "Legal"),
                CommandValidationResult::Illegal {
                    reason,
                    rule_reference: Some(rule_ref),
                } => write!(f, "Illegal: {} [{}]", reason, rule_ref),
                CommandValidationResult::Illegal {
                    reason,
                    rule_reference: None,
                } => write!(f, "Illegal: {}", reason),
            }
        }
    }
}

// ============================================================================
// Stratagem target
// ============================================================================

pub mod stratagem {
    use super::*;

    /// What a stratagem targets when used.
    ///
    /// Stratagems can target units, individual models, players, or nothing.
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum StratagemTarget {
        /// Targets a specific unit (e.g., Fire Overwatch, Counter-Offensive).
        Unit(UnitId),
        /// Targets a specific model (e.g., Epic Challenge, Precision).
        Model(ModelId),
        /// Targets a player (e.g., Insane Bravery).
        Player(PlayerId),
        /// No specific target (e.g., Command Re-roll).
        None,
    }

    impl StratagemTarget {
        /// Returns the unit ID if this target is a Unit, None otherwise.
        pub fn unit_id(&self) -> Option<UnitId> {
            match self {
                StratagemTarget::Unit(id) => Some(*id),
                _ => None,
            }
        }

        /// Returns the model ID if this target is a Model, None otherwise.
        pub fn model_id(&self) -> Option<ModelId> {
            match self {
                StratagemTarget::Model(id) => Some(*id),
                _ => None,
            }
        }

        /// Returns the player ID if this target is a Player, None otherwise.
        pub fn player_id(&self) -> Option<PlayerId> {
            match self {
                StratagemTarget::Player(id) => Some(*id),
                _ => None,
            }
        }

        /// Returns true if this target is None.
        pub fn is_none(&self) -> bool {
            matches!(self, StratagemTarget::None)
        }
    }

    impl std::fmt::Display for StratagemTarget {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                StratagemTarget::Unit(id) => write!(f, "Unit({})", id),
                StratagemTarget::Model(id) => write!(f, "Model({})", id),
                StratagemTarget::Player(id) => write!(f, "Player({})", id),
                StratagemTarget::None => write!(f, "None"),
            }
        }
    }
}

// ============================================================================
// Blessings of Khorne
// ============================================================================

pub mod blessings {
    use super::*;

    /// Allocation of dice to a Blessing of Khorne.
    ///
    /// The Frenzied Reavers roll 5D6 at the start of the Fight phase and allocate
    /// dice to blessings. Each blessing requires specific dice values.
    ///
    /// Source: Frenzied_Reavers.md - Blessings of Khorne
    #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct BlessingAllocation {
        /// Name of the blessing (e.g., "Rage-fuelled", "Total Carnage", "Martial Excellence").
        pub blessing_name: String,
        /// Indices into the 5D6 roll that are allocated to this blessing.
        pub dice_indices: Vec<usize>,
    }

    impl BlessingAllocation {
        /// Create a new BlessingAllocation.
        pub fn new(blessing_name: impl Into<String>, dice_indices: Vec<usize>) -> Self {
            Self {
                blessing_name: blessing_name.into(),
                dice_indices,
            }
        }

        /// Returns the number of dice allocated to this blessing.
        pub fn dice_count(&self) -> usize {
            self.dice_indices.len()
        }
    }

    impl std::fmt::Display for BlessingAllocation {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} (dice: {:?})",
                self.blessing_name, self.dice_indices
            )
        }
    }
}

// ============================================================================
// Command record and history
// ============================================================================

pub mod history {
    use super::*;

    /// A command plus metadata for the history log.
    ///
    /// Every command that passes through the engine is recorded with
    /// its validation result, timing information, and state hashes
    /// for determinism verification.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CommandRecord {
        /// Unique identifier for this command in the history.
        pub command_id: CommandId,
        /// The command itself.
        pub command: Command,
        /// The player who issued the command.
        pub player: PlayerId,
        /// Which battle round the command was issued in.
        pub timestamp_round: BattleRound,
        /// Which phase the command was issued in.
        pub timestamp_phase: Phase,
        /// Whether the command was legal or illegal.
        pub validation_result: CommandValidationResult,
        /// Hash of the game state before the command was applied.
        pub state_hash_before: u64,
        /// Hash of the game state after the command was applied.
        /// None if the command was illegal and not applied.
        pub state_hash_after: Option<u64>,
    }

    impl CommandRecord {
        /// Returns true if this command was validated as legal.
        pub fn was_legal(&self) -> bool {
            self.validation_result.is_legal()
        }

        /// Returns true if this command was validated as illegal.
        pub fn was_illegal(&self) -> bool {
            self.validation_result.is_illegal()
        }

        /// Returns the command category string.
        pub fn category(&self) -> &'static str {
            self.command.category()
        }
    }

    impl std::fmt::Display for CommandRecord {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "[{}] R{} {:?} P{}: {} ({})",
                self.command_id,
                self.timestamp_round.number(),
                self.timestamp_phase,
                self.player,
                self.command,
                self.validation_result,
            )
        }
    }

    /// Append-only command history log with query support.
    ///
    /// Records every command that passes through the engine, whether legal
    /// or illegal. Supports querying by player, phase, round, and other
    /// criteria. Serializable for replay and debugging.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CommandHistory {
        /// The ordered list of command records.
        records: Vec<CommandRecord>,
    }

    impl CommandHistory {
        /// Create a new empty command history.
        pub fn new() -> Self {
            Self {
                records: Vec::new(),
            }
        }

        /// Create a new command history with pre-allocated capacity.
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                records: Vec::with_capacity(capacity),
            }
        }

        /// Append a command record to the history.
        pub fn append(&mut self, record: CommandRecord) {
            self.records.push(record);
        }

        /// Returns the total number of records in the history.
        pub fn len(&self) -> usize {
            self.records.len()
        }

        /// Returns true if the history is empty.
        pub fn is_empty(&self) -> bool {
            self.records.is_empty()
        }

        /// Get all records as a slice.
        pub fn records(&self) -> &[CommandRecord] {
            &self.records
        }

        /// Get a specific record by index.
        pub fn get(&self, index: usize) -> Option<&CommandRecord> {
            self.records.get(index)
        }

        /// Get the latest (most recent) record.
        pub fn latest(&self) -> Option<&CommandRecord> {
            self.records.last()
        }

        /// Get the last N records, ordered oldest to newest.
        pub fn latest_n(&self, n: usize) -> &[CommandRecord] {
            let start = self.records.len().saturating_sub(n);
            &self.records[start..]
        }

        /// Query records by player.
        pub fn by_player(&self, player: PlayerId) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| r.player == player)
                .collect()
        }

        /// Query records by phase.
        pub fn by_phase(&self, phase: Phase) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| r.timestamp_phase == phase)
                .collect()
        }

        /// Query records by battle round.
        pub fn by_round(&self, round: BattleRound) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| r.timestamp_round == round)
                .collect()
        }

        /// Query records by player and phase.
        pub fn by_player_and_phase(
            &self,
            player: PlayerId,
            phase: Phase,
        ) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| r.player == player && r.timestamp_phase == phase)
                .collect()
        }

        /// Query records by player, round, and phase.
        pub fn by_player_round_phase(
            &self,
            player: PlayerId,
            round: BattleRound,
            phase: Phase,
        ) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| {
                    r.player == player
                        && r.timestamp_round == round
                        && r.timestamp_phase == phase
                })
                .collect()
        }

        /// Query records by command category.
        pub fn by_category(&self, category: &str) -> Vec<&CommandRecord> {
            self.records
                .iter()
                .filter(|r| r.category() == category)
                .collect()
        }

        /// Query only legal commands.
        pub fn legal_commands(&self) -> Vec<&CommandRecord> {
            self.records.iter().filter(|r| r.was_legal()).collect()
        }

        /// Query only illegal commands.
        pub fn illegal_commands(&self) -> Vec<&CommandRecord> {
            self.records.iter().filter(|r| r.was_illegal()).collect()
        }

        /// Find the record with a specific command ID.
        pub fn find_by_id(&self, id: CommandId) -> Option<&CommandRecord> {
            self.records.iter().find(|r| r.command_id == id)
        }

        /// Get the state hash after the last successfully applied command.
        pub fn last_state_hash(&self) -> Option<u64> {
            self.records
                .iter()
                .rev()
                .find_map(|r| r.state_hash_after)
        }

        /// Set the state_hash_after on the most recent command record.
        ///
        /// Called by the executor after a command has been applied to record
        /// the post-execution state hash.
        pub fn set_last_state_hash_after(&mut self, hash: u64) {
            if let Some(last) = self.records.last_mut() {
                last.state_hash_after = Some(hash);
            }
        }

        /// Serialize the history to JSON.
        pub fn to_json(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(self)
        }

        /// Serialize the history to pretty-printed JSON.
        pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(self)
        }

        /// Deserialize a history from JSON.
        pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
            serde_json::from_str(json)
        }

        /// Iterate over all records.
        pub fn iter(&self) -> impl Iterator<Item = &CommandRecord> {
            self.records.iter()
        }
    }

    impl Default for CommandHistory {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<'a> IntoIterator for &'a CommandHistory {
        type Item = &'a CommandRecord;
        type IntoIter = std::slice::Iter<'a, CommandRecord>;

        fn into_iter(self) -> Self::IntoIter {
            self.records.iter()
        }
    }

    impl IntoIterator for CommandHistory {
        type Item = CommandRecord;
        type IntoIter = std::vec::IntoIter<CommandRecord>;

        fn into_iter(self) -> Self::IntoIter {
            self.records.into_iter()
        }
    }
}

// ============================================================================
// Decision surface
// ============================================================================

pub mod decision {
    use super::*;

    /// Type of decision point the player is facing.
    ///
    /// Used by the AI and UI to understand what kind of choice needs
    /// to be made and present appropriate options.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub enum DecisionType {
        /// Pre-battle setup choices (faction, enhancement, secondary, deployment).
        SetupChoice,
        /// Movement phase action (select unit, choose move type, destination).
        MovementAction,
        /// Shooting phase target selection.
        ShootingTarget,
        /// Charge phase declaration.
        ChargeDeclaration,
        /// Fight phase ordering (which unit fights next).
        FightOrder,
        /// Melee target selection.
        MeleeTarget,
        /// Stratagem window (use or decline a stratagem).
        StratagemWindow,
        /// Overwatch decision (fire overwatch or decline).
        OverwatchDecision,
        /// Wound allocation to a specific model.
        WoundAllocation,
        /// Blessings of Khorne dice allocation.
        BlessingAllocation,
        /// Martial Ka'tah stance selection.
        StanceSelection,
        /// Vaultswords weapon profile selection.
        WeaponProfileSelection,
        /// Scoring action (score or raze objective).
        ScoringAction,
        /// Generic choice that doesn't fit other categories.
        GenericChoice,
        /// Boarding Actions: Tactical Manoeuvre selection.
        BoardingTacticalManoeuvre,
        /// Boarding Actions: Hatchway operation decision.
        BoardingHatchwayOperation,
        /// Boarding Actions: Mission-specific action decision.
        BoardingMissionAction,
    }

    impl DecisionType {
        /// Returns a human-readable name for this decision type.
        pub fn name(&self) -> &'static str {
            match self {
                DecisionType::SetupChoice => "Setup Choice",
                DecisionType::MovementAction => "Movement Action",
                DecisionType::ShootingTarget => "Shooting Target",
                DecisionType::ChargeDeclaration => "Charge Declaration",
                DecisionType::FightOrder => "Fight Order",
                DecisionType::MeleeTarget => "Melee Target",
                DecisionType::StratagemWindow => "Stratagem Window",
                DecisionType::OverwatchDecision => "Overwatch Decision",
                DecisionType::WoundAllocation => "Wound Allocation",
                DecisionType::BlessingAllocation => "Blessing Allocation",
                DecisionType::StanceSelection => "Stance Selection",
                DecisionType::WeaponProfileSelection => "Weapon Profile Selection",
                DecisionType::ScoringAction => "Scoring Action",
                DecisionType::GenericChoice => "Generic Choice",
                DecisionType::BoardingTacticalManoeuvre => "Boarding Tactical Manoeuvre",
                DecisionType::BoardingHatchwayOperation => "Boarding Hatchway Operation",
                DecisionType::BoardingMissionAction => "Boarding Mission Action",
            }
        }

        /// Returns true if this decision type involves combat.
        pub fn is_combat_related(&self) -> bool {
            matches!(
                self,
                DecisionType::ShootingTarget
                    | DecisionType::ChargeDeclaration
                    | DecisionType::FightOrder
                    | DecisionType::MeleeTarget
                    | DecisionType::OverwatchDecision
                    | DecisionType::WoundAllocation
                    | DecisionType::StanceSelection
                    | DecisionType::WeaponProfileSelection
            )
        }
    }

    impl std::fmt::Display for DecisionType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name())
        }
    }

    /// List of all legal actions at the current decision point.
    ///
    /// The DecisionSurface is what the AI searches over and the UI
    /// presents to the player. It contains every legal command the
    /// owner can currently issue.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DecisionSurface {
        /// What kind of decision is being made.
        pub decision_type: DecisionType,
        /// The player who must make this decision.
        pub owner: PlayerId,
        /// All legal commands the owner can choose from.
        pub legal_commands: Vec<Command>,
        /// Whether the player must choose one (true) or can skip/decline (false).
        /// For example, a stratagem window is not mandatory (can decline),
        /// but choosing a unit to move is mandatory if eligible units exist.
        pub is_mandatory: bool,
        /// Optional timeout for this decision (for timed games).
        #[serde(
            serialize_with = "serialize_optional_duration",
            deserialize_with = "deserialize_optional_duration"
        )]
        pub timeout: Option<Duration>,
    }

    impl DecisionSurface {
        /// Create a new decision surface.
        pub fn new(
            decision_type: DecisionType,
            owner: PlayerId,
            legal_commands: Vec<Command>,
            is_mandatory: bool,
        ) -> Self {
            Self {
                decision_type,
                owner,
                legal_commands,
                is_mandatory,
                timeout: None,
            }
        }

        /// Create a new decision surface with a timeout.
        pub fn with_timeout(
            decision_type: DecisionType,
            owner: PlayerId,
            legal_commands: Vec<Command>,
            is_mandatory: bool,
            timeout: Duration,
        ) -> Self {
            Self {
                decision_type,
                owner,
                legal_commands,
                is_mandatory,
                timeout: Some(timeout),
            }
        }

        /// Returns the number of legal commands available.
        pub fn num_options(&self) -> usize {
            self.legal_commands.len()
        }

        /// Returns true if there are no legal commands (shouldn't happen in practice).
        pub fn is_empty(&self) -> bool {
            self.legal_commands.is_empty()
        }

        /// Returns true if there is exactly one legal command (forced move).
        pub fn is_forced(&self) -> bool {
            self.legal_commands.len() == 1 && self.is_mandatory
        }

        /// Get the forced command if there is exactly one mandatory option.
        pub fn forced_command(&self) -> Option<&Command> {
            if self.is_forced() {
                self.legal_commands.first()
            } else {
                None
            }
        }

        /// Check if a specific command is among the legal options.
        pub fn is_legal(&self, command: &Command) -> bool {
            self.legal_commands.contains(command)
        }

        /// Returns true if this decision has a timeout.
        pub fn has_timeout(&self) -> bool {
            self.timeout.is_some()
        }
    }

    impl std::fmt::Display for DecisionSurface {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{} for Player {} ({} option(s), {})",
                self.decision_type,
                self.owner,
                self.num_options(),
                if self.is_mandatory {
                    "mandatory"
                } else {
                    "optional"
                },
            )
        }
    }

    // Custom serialization for Option<Duration> since Duration doesn't
    // implement Serialize/Deserialize by default.
    fn serialize_optional_duration<S>(
        duration: &Option<Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_millis()),
            None => serializer.serialize_none(),
        }
    }

    fn deserialize_optional_duration<'de, D>(
        deserializer: D,
    ) -> Result<Option<Duration>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let millis: Option<u128> = Option::deserialize(deserializer)?;
        Ok(millis.map(|ms| Duration::from_millis(ms as u64)))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Command tests ----

    #[test]
    fn test_command_select_faction() {
        let cmd = Command::SelectFaction {
            player: PlayerId::new(0),
            faction_id: FactionId::new(1),
        };
        assert_eq!(cmd.player(), Some(PlayerId::new(0)));
        assert_eq!(cmd.category(), "Setup");
        assert!(cmd.is_setup());
        assert!(!cmd.is_movement());
    }

    #[test]
    fn test_command_setup_complete() {
        let cmd = Command::SetupComplete;
        assert_eq!(cmd.player(), None);
        assert_eq!(cmd.category(), "Setup");
        assert!(cmd.is_setup());
    }

    #[test]
    fn test_command_normal_move() {
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(5),
            destination: Position::from_inches(10, 15),
        };
        assert_eq!(cmd.player(), None);
        assert_eq!(cmd.category(), "Movement");
        assert!(cmd.is_movement());
        assert_eq!(cmd.unit_ids(), vec![UnitId::new(5)]);
    }

    #[test]
    fn test_command_advance_move() {
        let cmd = Command::AdvanceMove {
            unit_id: UnitId::new(3),
            destination: Position::from_inches(20, 10),
            advance_roll: 4,
        };
        assert!(cmd.is_movement());
        assert_eq!(cmd.unit_ids(), vec![UnitId::new(3)]);
    }

    #[test]
    fn test_command_fall_back() {
        let cmd = Command::FallBack {
            unit_id: UnitId::new(2),
            destination: Position::from_inches(5, 5),
        };
        assert!(cmd.is_movement());
    }

    #[test]
    fn test_command_remain_stationary() {
        let cmd = Command::RemainStationary {
            unit_id: UnitId::new(7),
        };
        assert!(cmd.is_movement());
        assert_eq!(cmd.unit_ids(), vec![UnitId::new(7)]);
    }

    #[test]
    fn test_command_arrive_from_reserves() {
        let cmd = Command::ArriveFromReserves {
            unit_id: UnitId::new(4),
            position: Position::from_inches(22, 15),
        };
        assert!(cmd.is_movement());
    }

    #[test]
    fn test_command_declare_shooting_targets() {
        let cmd = Command::DeclareShootingTargets {
            unit_id: UnitId::new(1),
            targets: vec![
                (WeaponId::new(10), UnitId::new(5)),
                (WeaponId::new(11), UnitId::new(6)),
            ],
        };
        assert!(cmd.is_shooting());
        let ids = cmd.unit_ids();
        assert!(ids.contains(&UnitId::new(1)));
        assert!(ids.contains(&UnitId::new(5)));
        assert!(ids.contains(&UnitId::new(6)));
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_command_resolve_shooting_attack() {
        let cmd = Command::ResolveShootingAttack {
            attacker_id: UnitId::new(1),
            target_id: UnitId::new(5),
            weapon_id: WeaponId::new(10),
        };
        assert!(cmd.is_shooting());
        let ids = cmd.unit_ids();
        assert_eq!(ids, vec![UnitId::new(1), UnitId::new(5)]);
    }

    #[test]
    fn test_command_declare_charge() {
        let cmd = Command::DeclareCharge {
            unit_id: UnitId::new(1),
            targets: vec![UnitId::new(5), UnitId::new(6)],
        };
        assert!(cmd.is_charge());
        let ids = cmd.unit_ids();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn test_command_resolve_charge_roll() {
        let cmd = Command::ResolveChargeRoll {
            unit_id: UnitId::new(1),
            roll: 9,
        };
        assert!(cmd.is_charge());
    }

    #[test]
    fn test_command_fight_commands() {
        let cmd = Command::SelectUnitToFight {
            unit_id: UnitId::new(3),
        };
        assert!(cmd.is_fight());

        let cmd = Command::ChooseKaTahStance {
            unit_id: UnitId::new(3),
            stance: "Dacatarai".to_string(),
        };
        assert!(cmd.is_fight());

        let cmd = Command::ChooseVaultswordsProfile {
            model_id: ModelId::new(10),
            profile: "Hurricanus".to_string(),
        };
        assert!(cmd.is_fight());
    }

    #[test]
    fn test_command_pile_in() {
        let cmd = Command::PileIn {
            unit_id: UnitId::new(1),
            positions: vec![
                (ModelId::new(1), Position::from_inches(10, 10)),
                (ModelId::new(2), Position::from_inches(11, 10)),
            ],
        };
        assert!(cmd.is_fight());
        assert_eq!(cmd.unit_ids(), vec![UnitId::new(1)]);
    }

    #[test]
    fn test_command_consolidate() {
        let cmd = Command::Consolidate {
            unit_id: UnitId::new(1),
            positions: vec![(ModelId::new(1), Position::from_inches(12, 10))],
        };
        assert!(cmd.is_fight());
    }

    #[test]
    fn test_command_declare_melee_targets() {
        let cmd = Command::DeclareMeleeTargets {
            unit_id: UnitId::new(1),
            targets: vec![(WeaponId::new(20), UnitId::new(5))],
        };
        assert!(cmd.is_fight());
        let ids = cmd.unit_ids();
        assert_eq!(ids, vec![UnitId::new(1), UnitId::new(5)]);
    }

    #[test]
    fn test_command_use_stratagem() {
        let cmd = Command::UseStratagem {
            player: PlayerId::new(0),
            stratagem_id: StratagemId::new(3),
            target: StratagemTarget::Unit(UnitId::new(5)),
        };
        assert!(cmd.is_stratagem());
        assert_eq!(cmd.player(), Some(PlayerId::new(0)));
    }

    #[test]
    fn test_command_decline_stratagem() {
        let cmd = Command::DeclineStratagem {
            player: PlayerId::new(1),
        };
        assert!(cmd.is_stratagem());
    }

    #[test]
    fn test_command_scoring() {
        let cmd = Command::ScoreObjective {
            player: PlayerId::new(0),
            objective_id: ObjectiveId::new(1),
        };
        assert_eq!(cmd.category(), "Scoring");
        assert_eq!(cmd.player(), Some(PlayerId::new(0)));

        let cmd = Command::RazeObjective {
            player: PlayerId::new(1),
            objective_id: ObjectiveId::new(2),
        };
        assert_eq!(cmd.category(), "Scoring");
    }

    #[test]
    fn test_command_blessings_of_khorne() {
        let cmd = Command::AllocateBlessings {
            player: PlayerId::new(0),
            allocations: vec![
                BlessingAllocation::new("Rage-fuelled", vec![0, 1]),
                BlessingAllocation::new("Total Carnage", vec![2, 3, 4]),
            ],
        };
        assert_eq!(cmd.category(), "BlessingsOfKhorne");
        assert_eq!(cmd.player(), Some(PlayerId::new(0)));
    }

    #[test]
    fn test_command_misc() {
        let cmd = Command::AllocateWound {
            target_model_id: ModelId::new(5),
        };
        assert_eq!(cmd.category(), "Misc");

        let cmd = Command::AssignOverwatchTarget {
            unit_id: UnitId::new(3),
        };
        assert_eq!(cmd.category(), "Misc");

        let cmd = Command::PassAction;
        assert_eq!(cmd.category(), "Misc");
        assert_eq!(cmd.player(), None);

        let cmd = Command::Concede {
            player: PlayerId::new(1),
        };
        assert_eq!(cmd.category(), "Misc");
        assert_eq!(cmd.player(), Some(PlayerId::new(1)));
    }

    #[test]
    fn test_command_phase_control() {
        let cmd = Command::StartBattleRound {
            round: BattleRound::new(1),
        };
        assert!(cmd.is_phase_control());

        let cmd = Command::StartPhase {
            phase: Phase::Movement,
        };
        assert!(cmd.is_phase_control());

        let cmd = Command::EndPhase {
            phase: Phase::Movement,
        };
        assert!(cmd.is_phase_control());

        let cmd = Command::EndPlayerTurn {
            player: PlayerId::new(0),
        };
        assert!(cmd.is_phase_control());
    }

    #[test]
    fn test_command_display() {
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(3),
            destination: Position::from_inches(10, 15),
        };
        let display = format!("{}", cmd);
        assert!(display.contains("Normal move"));
        assert!(display.contains("3"));

        let cmd = Command::SetupComplete;
        assert_eq!(format!("{}", cmd), "Setup complete");

        let cmd = Command::PassAction;
        assert_eq!(format!("{}", cmd), "Pass");
    }

    #[test]
    fn test_command_serialization() {
        let cmd = Command::NormalMove {
            unit_id: UnitId::new(5),
            destination: Position::from_inches(10, 15),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_command_complex_serialization() {
        let cmd = Command::DeclareShootingTargets {
            unit_id: UnitId::new(1),
            targets: vec![
                (WeaponId::new(10), UnitId::new(5)),
                (WeaponId::new(11), UnitId::new(6)),
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_command_pile_in_serialization() {
        let cmd = Command::PileIn {
            unit_id: UnitId::new(1),
            positions: vec![
                (ModelId::new(1), Position::from_inches(10, 10)),
                (ModelId::new(2), Position::from_inches(11, 10)),
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_command_stratagem_serialization() {
        let cmd = Command::UseStratagem {
            player: PlayerId::new(0),
            stratagem_id: StratagemId::new(3),
            target: StratagemTarget::Unit(UnitId::new(5)),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_command_blessings_serialization() {
        let cmd = Command::AllocateBlessings {
            player: PlayerId::new(0),
            allocations: vec![
                BlessingAllocation::new("Rage-fuelled", vec![0, 1]),
                BlessingAllocation::new("Total Carnage", vec![2, 3, 4]),
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    // ---- Validation result tests ----

    #[test]
    fn test_validation_legal() {
        let result = CommandValidationResult::Legal;
        assert!(result.is_legal());
        assert!(!result.is_illegal());
        assert_eq!(result.reason(), None);
        assert_eq!(result.rule_reference(), None);
        assert_eq!(format!("{}", result), "Legal");
    }

    #[test]
    fn test_validation_illegal() {
        let result = CommandValidationResult::illegal("Unit cannot move in shooting phase");
        assert!(!result.is_legal());
        assert!(result.is_illegal());
        assert_eq!(
            result.reason(),
            Some("Unit cannot move in shooting phase")
        );
        assert_eq!(result.rule_reference(), None);
    }

    #[test]
    fn test_validation_illegal_with_ref() {
        let result = CommandValidationResult::illegal_with_ref(
            "Unit already moved this phase",
            "40k_revised.md - Movement Phase",
        );
        assert!(result.is_illegal());
        assert_eq!(result.reason(), Some("Unit already moved this phase"));
        assert_eq!(
            result.rule_reference(),
            Some("40k_revised.md - Movement Phase")
        );
        let display = format!("{}", result);
        assert!(display.contains("40k_revised.md"));
    }

    #[test]
    fn test_validation_serialization() {
        let legal = CommandValidationResult::Legal;
        let json = serde_json::to_string(&legal).unwrap();
        let deserialized: CommandValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(legal, deserialized);

        let illegal = CommandValidationResult::illegal_with_ref("bad move", "rule 1.2.3");
        let json = serde_json::to_string(&illegal).unwrap();
        let deserialized: CommandValidationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(illegal, deserialized);
    }

    // ---- Stratagem target tests ----

    #[test]
    fn test_stratagem_target_unit() {
        let target = StratagemTarget::Unit(UnitId::new(5));
        assert_eq!(target.unit_id(), Some(UnitId::new(5)));
        assert_eq!(target.model_id(), None);
        assert_eq!(target.player_id(), None);
        assert!(!target.is_none());
    }

    #[test]
    fn test_stratagem_target_model() {
        let target = StratagemTarget::Model(ModelId::new(10));
        assert_eq!(target.unit_id(), None);
        assert_eq!(target.model_id(), Some(ModelId::new(10)));
        assert!(!target.is_none());
    }

    #[test]
    fn test_stratagem_target_player() {
        let target = StratagemTarget::Player(PlayerId::new(0));
        assert_eq!(target.player_id(), Some(PlayerId::new(0)));
        assert!(!target.is_none());
    }

    #[test]
    fn test_stratagem_target_none() {
        let target = StratagemTarget::None;
        assert!(target.is_none());
        assert_eq!(target.unit_id(), None);
        assert_eq!(target.model_id(), None);
        assert_eq!(target.player_id(), None);
    }

    #[test]
    fn test_stratagem_target_display() {
        assert_eq!(
            format!("{}", StratagemTarget::Unit(UnitId::new(3))),
            "Unit(3)"
        );
        assert_eq!(
            format!("{}", StratagemTarget::Model(ModelId::new(7))),
            "Model(7)"
        );
        assert_eq!(
            format!("{}", StratagemTarget::Player(PlayerId::new(1))),
            "Player(1)"
        );
        assert_eq!(format!("{}", StratagemTarget::None), "None");
    }

    #[test]
    fn test_stratagem_target_serialization() {
        let targets = vec![
            StratagemTarget::Unit(UnitId::new(5)),
            StratagemTarget::Model(ModelId::new(10)),
            StratagemTarget::Player(PlayerId::new(0)),
            StratagemTarget::None,
        ];
        for target in &targets {
            let json = serde_json::to_string(target).unwrap();
            let deserialized: StratagemTarget = serde_json::from_str(&json).unwrap();
            assert_eq!(*target, deserialized);
        }
    }

    // ---- Blessing allocation tests ----

    #[test]
    fn test_blessing_allocation() {
        let alloc = BlessingAllocation::new("Rage-fuelled", vec![0, 1]);
        assert_eq!(alloc.blessing_name, "Rage-fuelled");
        assert_eq!(alloc.dice_indices, vec![0, 1]);
        assert_eq!(alloc.dice_count(), 2);
    }

    #[test]
    fn test_blessing_allocation_display() {
        let alloc = BlessingAllocation::new("Total Carnage", vec![2, 3, 4]);
        let display = format!("{}", alloc);
        assert!(display.contains("Total Carnage"));
        assert!(display.contains("2"));
    }

    #[test]
    fn test_blessing_allocation_serialization() {
        let alloc = BlessingAllocation::new("Martial Excellence", vec![0, 1, 2]);
        let json = serde_json::to_string(&alloc).unwrap();
        let deserialized: BlessingAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(alloc, deserialized);
    }

    // ---- Command record tests ----

    fn make_test_record(id: u32, player: u32, round: u8, phase: Phase) -> CommandRecord {
        CommandRecord {
            command_id: CommandId::new(id),
            command: Command::NormalMove {
                unit_id: UnitId::new(1),
                destination: Position::from_inches(10, 10),
            },
            player: PlayerId::new(player),
            timestamp_round: BattleRound::new(round),
            timestamp_phase: phase,
            validation_result: CommandValidationResult::Legal,
            state_hash_before: 1000 + id as u64,
            state_hash_after: Some(2000 + id as u64),
        }
    }

    #[test]
    fn test_command_record_basic() {
        let record = make_test_record(0, 0, 1, Phase::Movement);
        assert!(record.was_legal());
        assert!(!record.was_illegal());
        assert_eq!(record.category(), "Movement");
    }

    #[test]
    fn test_command_record_illegal() {
        let record = CommandRecord {
            command_id: CommandId::new(1),
            command: Command::NormalMove {
                unit_id: UnitId::new(1),
                destination: Position::from_inches(10, 10),
            },
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Shooting,
            validation_result: CommandValidationResult::illegal("Wrong phase"),
            state_hash_before: 1000,
            state_hash_after: None,
        };
        assert!(record.was_illegal());
        assert!(!record.was_legal());
    }

    #[test]
    fn test_command_record_display() {
        let record = make_test_record(0, 0, 1, Phase::Movement);
        let display = format!("{}", record);
        assert!(display.contains("R1"));
        assert!(display.contains("Movement"));
        assert!(display.contains("Legal"));
    }

    #[test]
    fn test_command_record_serialization() {
        let record = make_test_record(42, 1, 3, Phase::Shooting);
        let json = serde_json::to_string(&record).unwrap();
        let deserialized: CommandRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.command_id, CommandId::new(42));
        assert_eq!(deserialized.player, PlayerId::new(1));
        assert_eq!(deserialized.timestamp_round, BattleRound::new(3));
        assert_eq!(deserialized.timestamp_phase, Phase::Shooting);
        assert!(deserialized.was_legal());
    }

    // ---- Command history tests ----

    #[test]
    fn test_history_new() {
        let history = CommandHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
        assert!(history.latest().is_none());
    }

    #[test]
    fn test_history_append_and_len() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());

        history.append(make_test_record(1, 1, 1, Phase::Movement));
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_history_latest() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Shooting));
        history.append(make_test_record(2, 0, 2, Phase::Command));

        let latest = history.latest().unwrap();
        assert_eq!(latest.command_id, CommandId::new(2));
    }

    #[test]
    fn test_history_latest_n() {
        let mut history = CommandHistory::new();
        for i in 0..10 {
            history.append(make_test_record(i, 0, 1, Phase::Movement));
        }

        let last_3 = history.latest_n(3);
        assert_eq!(last_3.len(), 3);
        assert_eq!(last_3[0].command_id, CommandId::new(7));
        assert_eq!(last_3[1].command_id, CommandId::new(8));
        assert_eq!(last_3[2].command_id, CommandId::new(9));

        // Requesting more than available returns all
        let all = history.latest_n(100);
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_history_by_player() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));
        history.append(make_test_record(2, 0, 1, Phase::Shooting));
        history.append(make_test_record(3, 1, 1, Phase::Shooting));

        let p0 = history.by_player(PlayerId::new(0));
        assert_eq!(p0.len(), 2);
        assert_eq!(p0[0].command_id, CommandId::new(0));
        assert_eq!(p0[1].command_id, CommandId::new(2));

        let p1 = history.by_player(PlayerId::new(1));
        assert_eq!(p1.len(), 2);
    }

    #[test]
    fn test_history_by_phase() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 0, 1, Phase::Movement));
        history.append(make_test_record(2, 0, 1, Phase::Shooting));
        history.append(make_test_record(3, 0, 1, Phase::Charge));

        let movement = history.by_phase(Phase::Movement);
        assert_eq!(movement.len(), 2);

        let shooting = history.by_phase(Phase::Shooting);
        assert_eq!(shooting.len(), 1);

        let fight = history.by_phase(Phase::Fight);
        assert_eq!(fight.len(), 0);
    }

    #[test]
    fn test_history_by_round() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 0, 1, Phase::Shooting));
        history.append(make_test_record(2, 0, 2, Phase::Movement));
        history.append(make_test_record(3, 0, 2, Phase::Shooting));
        history.append(make_test_record(4, 0, 3, Phase::Movement));

        let r1 = history.by_round(BattleRound::new(1));
        assert_eq!(r1.len(), 2);

        let r2 = history.by_round(BattleRound::new(2));
        assert_eq!(r2.len(), 2);

        let r3 = history.by_round(BattleRound::new(3));
        assert_eq!(r3.len(), 1);
    }

    #[test]
    fn test_history_by_player_and_phase() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));
        history.append(make_test_record(2, 0, 1, Phase::Shooting));
        history.append(make_test_record(3, 1, 1, Phase::Shooting));

        let p0_move = history.by_player_and_phase(PlayerId::new(0), Phase::Movement);
        assert_eq!(p0_move.len(), 1);
        assert_eq!(p0_move[0].command_id, CommandId::new(0));

        let p1_shoot = history.by_player_and_phase(PlayerId::new(1), Phase::Shooting);
        assert_eq!(p1_shoot.len(), 1);
        assert_eq!(p1_shoot[0].command_id, CommandId::new(3));
    }

    #[test]
    fn test_history_by_player_round_phase() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 0, 1, Phase::Shooting));
        history.append(make_test_record(2, 0, 2, Phase::Movement));
        history.append(make_test_record(3, 1, 1, Phase::Movement));

        let result = history.by_player_round_phase(
            PlayerId::new(0),
            BattleRound::new(1),
            Phase::Movement,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].command_id, CommandId::new(0));
    }

    #[test]
    fn test_history_by_category() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(CommandRecord {
            command_id: CommandId::new(1),
            command: Command::SelectUnitToShoot {
                unit_id: UnitId::new(1),
            },
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Shooting,
            validation_result: CommandValidationResult::Legal,
            state_hash_before: 1001,
            state_hash_after: Some(2001),
        });

        let movement = history.by_category("Movement");
        assert_eq!(movement.len(), 1);

        let shooting = history.by_category("Shooting");
        assert_eq!(shooting.len(), 1);
    }

    #[test]
    fn test_history_legal_and_illegal() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(CommandRecord {
            command_id: CommandId::new(1),
            command: Command::NormalMove {
                unit_id: UnitId::new(2),
                destination: Position::from_inches(50, 50),
            },
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Movement,
            validation_result: CommandValidationResult::illegal("Out of bounds"),
            state_hash_before: 1001,
            state_hash_after: None,
        });

        let legal = history.legal_commands();
        assert_eq!(legal.len(), 1);

        let illegal = history.illegal_commands();
        assert_eq!(illegal.len(), 1);
        assert_eq!(illegal[0].command_id, CommandId::new(1));
    }

    #[test]
    fn test_history_find_by_id() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));
        history.append(make_test_record(2, 0, 1, Phase::Shooting));

        let found = history.find_by_id(CommandId::new(1));
        assert!(found.is_some());
        assert_eq!(found.unwrap().player, PlayerId::new(1));

        let not_found = history.find_by_id(CommandId::new(99));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_history_last_state_hash() {
        let mut history = CommandHistory::new();
        assert_eq!(history.last_state_hash(), None);

        history.append(make_test_record(0, 0, 1, Phase::Movement));
        assert_eq!(history.last_state_hash(), Some(2000));

        // Append an illegal command (no state_hash_after)
        history.append(CommandRecord {
            command_id: CommandId::new(1),
            command: Command::NormalMove {
                unit_id: UnitId::new(2),
                destination: Position::from_inches(50, 50),
            },
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Movement,
            validation_result: CommandValidationResult::illegal("Bad move"),
            state_hash_before: 2000,
            state_hash_after: None,
        });

        // last_state_hash should skip the illegal command and return the last valid hash
        assert_eq!(history.last_state_hash(), Some(2000));
    }

    #[test]
    fn test_history_get_and_records() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));

        assert_eq!(history.records().len(), 2);
        assert_eq!(
            history.get(0).unwrap().command_id,
            CommandId::new(0)
        );
        assert_eq!(
            history.get(1).unwrap().command_id,
            CommandId::new(1)
        );
        assert!(history.get(2).is_none());
    }

    #[test]
    fn test_history_serialization() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Shooting));

        let json = history.to_json().unwrap();
        let deserialized = CommandHistory::from_json(&json).unwrap();
        assert_eq!(deserialized.len(), 2);
        assert_eq!(
            deserialized.get(0).unwrap().command_id,
            CommandId::new(0)
        );
        assert_eq!(
            deserialized.get(1).unwrap().command_id,
            CommandId::new(1)
        );
    }

    #[test]
    fn test_history_pretty_json() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));

        let pretty = history.to_json_pretty().unwrap();
        assert!(pretty.contains('\n')); // Pretty-printed has newlines
        let deserialized = CommandHistory::from_json(&pretty).unwrap();
        assert_eq!(deserialized.len(), 1);
    }

    #[test]
    fn test_history_iteration() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));
        history.append(make_test_record(2, 0, 1, Phase::Shooting));

        let ids: Vec<CommandId> = history.iter().map(|r| r.command_id).collect();
        assert_eq!(
            ids,
            vec![
                CommandId::new(0),
                CommandId::new(1),
                CommandId::new(2),
            ]
        );
    }

    #[test]
    fn test_history_into_iterator_ref() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));

        let mut count = 0;
        for _record in &history {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_history_into_iterator_owned() {
        let mut history = CommandHistory::new();
        history.append(make_test_record(0, 0, 1, Phase::Movement));
        history.append(make_test_record(1, 1, 1, Phase::Movement));

        let records: Vec<CommandRecord> = history.into_iter().collect();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_history_with_capacity() {
        let history = CommandHistory::with_capacity(100);
        assert!(history.is_empty());
    }

    // ---- Decision surface tests ----

    #[test]
    fn test_decision_type_name() {
        assert_eq!(DecisionType::SetupChoice.name(), "Setup Choice");
        assert_eq!(DecisionType::MovementAction.name(), "Movement Action");
        assert_eq!(DecisionType::ShootingTarget.name(), "Shooting Target");
        assert_eq!(DecisionType::StratagemWindow.name(), "Stratagem Window");
        assert_eq!(DecisionType::BlessingAllocation.name(), "Blessing Allocation");
        assert_eq!(DecisionType::StanceSelection.name(), "Stance Selection");
        assert_eq!(
            DecisionType::WeaponProfileSelection.name(),
            "Weapon Profile Selection"
        );
    }

    #[test]
    fn test_decision_type_is_combat_related() {
        assert!(DecisionType::ShootingTarget.is_combat_related());
        assert!(DecisionType::ChargeDeclaration.is_combat_related());
        assert!(DecisionType::FightOrder.is_combat_related());
        assert!(DecisionType::MeleeTarget.is_combat_related());
        assert!(DecisionType::OverwatchDecision.is_combat_related());
        assert!(DecisionType::WoundAllocation.is_combat_related());
        assert!(DecisionType::StanceSelection.is_combat_related());
        assert!(DecisionType::WeaponProfileSelection.is_combat_related());

        assert!(!DecisionType::SetupChoice.is_combat_related());
        assert!(!DecisionType::MovementAction.is_combat_related());
        assert!(!DecisionType::StratagemWindow.is_combat_related());
        assert!(!DecisionType::ScoringAction.is_combat_related());
        assert!(!DecisionType::GenericChoice.is_combat_related());
    }

    #[test]
    fn test_decision_type_display() {
        assert_eq!(format!("{}", DecisionType::MovementAction), "Movement Action");
    }

    #[test]
    fn test_decision_type_serialization() {
        let dt = DecisionType::StanceSelection;
        let json = serde_json::to_string(&dt).unwrap();
        let deserialized: DecisionType = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, deserialized);
    }

    #[test]
    fn test_decision_surface_new() {
        let surface = DecisionSurface::new(
            DecisionType::MovementAction,
            PlayerId::new(0),
            vec![
                Command::NormalMove {
                    unit_id: UnitId::new(1),
                    destination: Position::from_inches(10, 10),
                },
                Command::RemainStationary {
                    unit_id: UnitId::new(1),
                },
            ],
            true,
        );

        assert_eq!(surface.decision_type, DecisionType::MovementAction);
        assert_eq!(surface.owner, PlayerId::new(0));
        assert_eq!(surface.num_options(), 2);
        assert!(surface.is_mandatory);
        assert!(!surface.is_empty());
        assert!(!surface.is_forced());
        assert!(surface.timeout.is_none());
        assert!(!surface.has_timeout());
    }

    #[test]
    fn test_decision_surface_with_timeout() {
        let surface = DecisionSurface::with_timeout(
            DecisionType::StratagemWindow,
            PlayerId::new(1),
            vec![
                Command::UseStratagem {
                    player: PlayerId::new(1),
                    stratagem_id: StratagemId::new(1),
                    target: StratagemTarget::Unit(UnitId::new(3)),
                },
                Command::DeclineStratagem {
                    player: PlayerId::new(1),
                },
            ],
            false,
            Duration::from_secs(30),
        );

        assert!(surface.has_timeout());
        assert_eq!(surface.timeout, Some(Duration::from_secs(30)));
        assert!(!surface.is_mandatory);
    }

    #[test]
    fn test_decision_surface_forced() {
        let surface = DecisionSurface::new(
            DecisionType::WoundAllocation,
            PlayerId::new(0),
            vec![Command::AllocateWound {
                target_model_id: ModelId::new(5),
            }],
            true,
        );

        assert!(surface.is_forced());
        assert_eq!(
            surface.forced_command(),
            Some(&Command::AllocateWound {
                target_model_id: ModelId::new(5),
            })
        );
    }

    #[test]
    fn test_decision_surface_not_forced_when_optional() {
        let surface = DecisionSurface::new(
            DecisionType::StratagemWindow,
            PlayerId::new(0),
            vec![Command::DeclineStratagem {
                player: PlayerId::new(0),
            }],
            false,
        );

        // Only one option but not mandatory => not forced
        assert!(!surface.is_forced());
        assert_eq!(surface.forced_command(), None);
    }

    #[test]
    fn test_decision_surface_is_legal() {
        let move_cmd = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(10, 10),
        };
        let remain_cmd = Command::RemainStationary {
            unit_id: UnitId::new(1),
        };
        let other_cmd = Command::NormalMove {
            unit_id: UnitId::new(2),
            destination: Position::from_inches(20, 20),
        };

        let surface = DecisionSurface::new(
            DecisionType::MovementAction,
            PlayerId::new(0),
            vec![move_cmd.clone(), remain_cmd.clone()],
            true,
        );

        assert!(surface.is_legal(&move_cmd));
        assert!(surface.is_legal(&remain_cmd));
        assert!(!surface.is_legal(&other_cmd));
    }

    #[test]
    fn test_decision_surface_empty() {
        let surface = DecisionSurface::new(
            DecisionType::GenericChoice,
            PlayerId::new(0),
            vec![],
            false,
        );
        assert!(surface.is_empty());
        assert_eq!(surface.num_options(), 0);
    }

    #[test]
    fn test_decision_surface_display() {
        let surface = DecisionSurface::new(
            DecisionType::MovementAction,
            PlayerId::new(0),
            vec![
                Command::NormalMove {
                    unit_id: UnitId::new(1),
                    destination: Position::from_inches(10, 10),
                },
                Command::RemainStationary {
                    unit_id: UnitId::new(1),
                },
            ],
            true,
        );
        let display = format!("{}", surface);
        assert!(display.contains("Movement Action"));
        assert!(display.contains("2 option(s)"));
        assert!(display.contains("mandatory"));
    }

    #[test]
    fn test_decision_surface_serialization() {
        let surface = DecisionSurface::new(
            DecisionType::MovementAction,
            PlayerId::new(0),
            vec![
                Command::NormalMove {
                    unit_id: UnitId::new(1),
                    destination: Position::from_inches(10, 10),
                },
                Command::RemainStationary {
                    unit_id: UnitId::new(1),
                },
            ],
            true,
        );

        let json = serde_json::to_string(&surface).unwrap();
        let deserialized: DecisionSurface = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.decision_type, DecisionType::MovementAction);
        assert_eq!(deserialized.owner, PlayerId::new(0));
        assert_eq!(deserialized.num_options(), 2);
        assert!(deserialized.is_mandatory);
    }

    #[test]
    fn test_decision_surface_with_timeout_serialization() {
        let surface = DecisionSurface::with_timeout(
            DecisionType::StratagemWindow,
            PlayerId::new(1),
            vec![Command::DeclineStratagem {
                player: PlayerId::new(1),
            }],
            false,
            Duration::from_secs(60),
        );

        let json = serde_json::to_string(&surface).unwrap();
        let deserialized: DecisionSurface = serde_json::from_str(&json).unwrap();
        assert!(deserialized.has_timeout());
        // Duration from millis round-trip
        assert_eq!(deserialized.timeout, Some(Duration::from_secs(60)));
    }

    // ---- Integration / cross-module tests ----

    #[test]
    fn test_full_command_lifecycle() {
        // Simulate a mini command lifecycle: create, validate, record, query
        let mut history = CommandHistory::new();

        // Legal command
        let cmd1 = Command::SelectUnitToMove {
            unit_id: UnitId::new(1),
        };
        let record1 = CommandRecord {
            command_id: CommandId::new(0),
            command: cmd1,
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Movement,
            validation_result: CommandValidationResult::Legal,
            state_hash_before: 100,
            state_hash_after: Some(200),
        };
        history.append(record1);

        // Legal move command
        let cmd2 = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(12, 8),
        };
        let record2 = CommandRecord {
            command_id: CommandId::new(1),
            command: cmd2,
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Movement,
            validation_result: CommandValidationResult::Legal,
            state_hash_before: 200,
            state_hash_after: Some(300),
        };
        history.append(record2);

        // Illegal command (try to move again)
        let cmd3 = Command::NormalMove {
            unit_id: UnitId::new(1),
            destination: Position::from_inches(15, 10),
        };
        let record3 = CommandRecord {
            command_id: CommandId::new(2),
            command: cmd3,
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(1),
            timestamp_phase: Phase::Movement,
            validation_result: CommandValidationResult::illegal_with_ref(
                "Unit already moved this phase",
                "40k_revised.md - MOVEMENT PHASE",
            ),
            state_hash_before: 300,
            state_hash_after: None,
        };
        history.append(record3);

        // Verify
        assert_eq!(history.len(), 3);
        assert_eq!(history.legal_commands().len(), 2);
        assert_eq!(history.illegal_commands().len(), 1);
        assert_eq!(history.last_state_hash(), Some(300));

        let movement = history.by_phase(Phase::Movement);
        assert_eq!(movement.len(), 3);

        let p0_r1_move = history.by_player_round_phase(
            PlayerId::new(0),
            BattleRound::new(1),
            Phase::Movement,
        );
        assert_eq!(p0_r1_move.len(), 3);
    }

    #[test]
    fn test_decision_surface_for_stratagem_window() {
        // Test a typical stratagem window decision surface
        let player = PlayerId::new(1);
        let surface = DecisionSurface::new(
            DecisionType::StratagemWindow,
            player,
            vec![
                Command::UseStratagem {
                    player,
                    stratagem_id: StratagemId::new(1), // Fire Overwatch
                    target: StratagemTarget::Unit(UnitId::new(3)),
                },
                Command::UseStratagem {
                    player,
                    stratagem_id: StratagemId::new(2), // Counter-Offensive
                    target: StratagemTarget::Unit(UnitId::new(5)),
                },
                Command::DeclineStratagem { player },
            ],
            false, // stratagems are optional
        );

        assert_eq!(surface.num_options(), 3);
        assert!(!surface.is_mandatory);
        assert!(!surface.is_forced());

        // Player should be able to decline
        assert!(surface.is_legal(&Command::DeclineStratagem { player }));
    }

    #[test]
    fn test_decision_surface_for_fight_order() {
        // Units eligible to fight in order
        let player = PlayerId::new(0);
        let surface = DecisionSurface::new(
            DecisionType::FightOrder,
            player,
            vec![
                Command::SelectUnitToFight {
                    unit_id: UnitId::new(1),
                },
                Command::SelectUnitToFight {
                    unit_id: UnitId::new(3),
                },
                Command::SelectUnitToFight {
                    unit_id: UnitId::new(7),
                },
            ],
            true, // must pick a unit to fight
        );

        assert_eq!(surface.num_options(), 3);
        assert!(surface.is_mandatory);
        assert!(!surface.is_forced());
        assert!(surface.decision_type.is_combat_related());
    }

    #[test]
    fn test_all_command_variants_have_category() {
        // Exhaustive test: every command variant should have a known category
        let commands: Vec<Command> = vec![
            Command::SelectFaction {
                player: PlayerId::new(0),
                faction_id: FactionId::new(0),
            },
            Command::SelectPatrolSquad {
                player: PlayerId::new(0),
                squad_index: 0,
            },
            Command::SelectEnhancement {
                player: PlayerId::new(0),
                enhancement_id: EnhancementId::new(0),
            },
            Command::SelectSecondaryObjective {
                player: PlayerId::new(0),
                secondary_id: SecondaryObjectiveId::new(0),
            },
            Command::PlaceUnit {
                player: PlayerId::new(0),
                unit_id: UnitId::new(0),
                position: Position::from_inches(0, 0),
            },
            Command::DetermineFirstTurn {
                first_player: PlayerId::new(0),
            },
            Command::SetupComplete,
            Command::StartBattleRound {
                round: BattleRound::new(1),
            },
            Command::StartPhase {
                phase: Phase::Command,
            },
            Command::EndPhase {
                phase: Phase::Command,
            },
            Command::EndPlayerTurn {
                player: PlayerId::new(0),
            },
            Command::SelectUnitToMove {
                unit_id: UnitId::new(0),
            },
            Command::NormalMove {
                unit_id: UnitId::new(0),
                destination: Position::from_inches(0, 0),
            },
            Command::AdvanceMove {
                unit_id: UnitId::new(0),
                destination: Position::from_inches(0, 0),
                advance_roll: 3,
            },
            Command::FallBack {
                unit_id: UnitId::new(0),
                destination: Position::from_inches(0, 0),
            },
            Command::RemainStationary {
                unit_id: UnitId::new(0),
            },
            Command::ArriveFromReserves {
                unit_id: UnitId::new(0),
                position: Position::from_inches(0, 0),
            },
            Command::SelectUnitToShoot {
                unit_id: UnitId::new(0),
            },
            Command::DeclareShootingTargets {
                unit_id: UnitId::new(0),
                targets: vec![],
            },
            Command::ResolveShootingAttack {
                attacker_id: UnitId::new(0),
                target_id: UnitId::new(1),
                weapon_id: WeaponId::new(0),
            },
            Command::DeclareCharge {
                unit_id: UnitId::new(0),
                targets: vec![],
            },
            Command::ResolveChargeRoll {
                unit_id: UnitId::new(0),
                roll: 7,
            },
            Command::MakeChargeMove {
                unit_id: UnitId::new(0),
                destination: Position::from_inches(0, 0),
            },
            Command::SelectUnitToFight {
                unit_id: UnitId::new(0),
            },
            Command::ChooseKaTahStance {
                unit_id: UnitId::new(0),
                stance: "Dacatarai".to_string(),
            },
            Command::ChooseVaultswordsProfile {
                model_id: ModelId::new(0),
                profile: "Hurricanus".to_string(),
            },
            Command::PileIn {
                unit_id: UnitId::new(0),
                positions: vec![],
            },
            Command::DeclareMeleeTargets {
                unit_id: UnitId::new(0),
                targets: vec![],
            },
            Command::ResolveMeleeAttack {
                attacker_id: UnitId::new(0),
                target_id: UnitId::new(1),
                weapon_id: WeaponId::new(0),
            },
            Command::Consolidate {
                unit_id: UnitId::new(0),
                positions: vec![],
            },
            Command::UseStratagem {
                player: PlayerId::new(0),
                stratagem_id: StratagemId::new(0),
                target: StratagemTarget::None,
            },
            Command::DeclineStratagem {
                player: PlayerId::new(0),
            },
            Command::ScoreObjective {
                player: PlayerId::new(0),
                objective_id: ObjectiveId::new(0),
            },
            Command::RazeObjective {
                player: PlayerId::new(0),
                objective_id: ObjectiveId::new(0),
            },
            Command::AllocateBlessings {
                player: PlayerId::new(0),
                allocations: vec![],
            },
            Command::AllocateWound {
                target_model_id: ModelId::new(0),
            },
            Command::AssignOverwatchTarget {
                unit_id: UnitId::new(0),
            },
            Command::PassAction,
            Command::Concede {
                player: PlayerId::new(0),
            },
        ];

        let valid_categories = [
            "Setup",
            "PhaseControl",
            "Movement",
            "Shooting",
            "Charge",
            "Fight",
            "Stratagem",
            "Scoring",
            "BlessingsOfKhorne",
            "Misc",
        ];

        for cmd in &commands {
            let cat = cmd.category();
            assert!(
                valid_categories.contains(&cat),
                "Command {:?} has unexpected category: {}",
                cmd,
                cat
            );
        }

        // Also verify display works for all commands
        for cmd in &commands {
            let display = format!("{}", cmd);
            assert!(!display.is_empty(), "Command {:?} has empty display", cmd);
        }
    }

    #[test]
    fn test_all_decision_types_serialization() {
        let types = vec![
            DecisionType::SetupChoice,
            DecisionType::MovementAction,
            DecisionType::ShootingTarget,
            DecisionType::ChargeDeclaration,
            DecisionType::FightOrder,
            DecisionType::MeleeTarget,
            DecisionType::StratagemWindow,
            DecisionType::OverwatchDecision,
            DecisionType::WoundAllocation,
            DecisionType::BlessingAllocation,
            DecisionType::StanceSelection,
            DecisionType::WeaponProfileSelection,
            DecisionType::ScoringAction,
            DecisionType::GenericChoice,
        ];

        for dt in &types {
            let json = serde_json::to_string(dt).unwrap();
            let deserialized: DecisionType = serde_json::from_str(&json).unwrap();
            assert_eq!(*dt, deserialized);
        }
    }
}
