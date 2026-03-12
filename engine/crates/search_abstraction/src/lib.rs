//! WH40K Engine - SearchAbstraction crate
//!
//! Action abstraction layer for the AI search engine.
//! Bridges raw game engine commands with the search tree.
//!
//! # Key Types
//!
//! - [`MacroAction`] - A tactical action wrapping one or more engine Commands
//! - [`TacticalIntent`] - Classification of an action's tactical purpose
//! - [`CandidateSet`] - Collection of scored candidate actions for a decision point
//! - [`ActionGenerator`] - Generates candidate actions for the current game state
//!
//! # Design Philosophy
//!
//! Instead of searching over every possible atomic command, the AI operates on
//! meaningful tactical abstractions. Movement candidates come from tactical anchors
//! (objectives, cover, charge staging), shooting targets are ranked by expected
//! damage and VP impact, and charges are evaluated by objective swing potential.
//!
//! Source: implementation_v3.md Section 11.4-11.5 (Macro-action model, Candidate generation)

use serde::{Deserialize, Serialize};

use wh40k_core_types::{
    EngagementStatus, Inches, Phase, PlayerId, Position, UnitId,
};
use wh40k_command_system::Command;
use wh40k_game_core::state::GameState;
use wh40k_game_core::unit::UnitState;
use wh40k_geometry::{distance, within_range};

// ============================================================================
// MacroAction ID
// ============================================================================

/// Unique identifier for a macro-action within a candidate set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacroActionId(pub u32);

impl std::fmt::Display for MacroActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MA({})", self.0)
    }
}

// ============================================================================
// Tactical Intent
// ============================================================================

/// Classification of the tactical purpose behind a macro-action.
/// Used for move ordering, search pruning, and diagnostic display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TacticalIntent {
    // === Movement intents ===
    /// Move to hold/control an objective marker.
    HoldObjective,
    /// Move to contest an enemy-held objective.
    ContestObjective,
    /// Move to a position with cover.
    MoveToCover,
    /// Move to a position that enables a charge next phase/turn.
    StageCharge,
    /// Move to screen a lane or protect key units.
    Screen,
    /// Retreat to preserve a damaged or valuable unit.
    Retreat,
    /// Move to gain a shooting position with good LOS.
    LineUpShot,
    /// Move to deny enemy reserve drop zone.
    DenyReserveZone,
    /// Remain stationary (for Heavy weapon bonus or holding position).
    HoldPosition,
    /// Advance to close distance aggressively.
    AdvanceAggressively,
    /// Fall back from engagement to enable shooting.
    FallBackToShoot,
    /// Arrive from reserves at a strategic position.
    DeepStrikeArrive,

    // === Shooting intents ===
    /// Shoot to maximize expected kills.
    MaximizeKills,
    /// Shoot to soften a unit for a subsequent charge.
    SoftenChargeTarget,
    /// Shoot to remove a scoring unit from an objective.
    RemoveScoringUnit,
    /// Shoot to force battle-shock pressure.
    ForceBattleShock,
    /// Shoot to bracket a hard target with multiple weapons.
    BracketHardTarget,

    // === Charge intents ===
    /// Charge the highest-value melee target.
    ChargeHighValueTarget,
    /// Multi-charge for an objective control swing.
    MultiChargeObjectiveSwing,
    /// Charge a softened target for guaranteed destruction.
    ChargeSoftenedTarget,

    // === Fight intents ===
    /// Select fight order for maximum damage.
    FightMaxDamage,
    /// Select fight order to preserve valuable units (fights first).
    FightPreserve,
    /// Choose Ka'tah stance for optimal weapon matchup.
    ChooseStance,
    /// Select Vaultswords profile based on target.
    ChooseWeaponProfile,

    // === Stratagem intents ===
    /// Use a defensive stratagem to preserve a unit.
    DefensiveStratagem,
    /// Use an offensive stratagem to increase damage.
    OffensiveStratagem,
    /// Use a movement-reaction stratagem.
    MovementReaction,
    /// Use a fight-order manipulation stratagem.
    FightOrderManipulation,
    /// Decline to use a stratagem.
    DeclineStratagem,

    // === Misc intents ===
    /// Score/raze an objective.
    ScoreObjective,
    /// Allocate Blessings of Khorne dice.
    AllocateBlessings,
    /// Phase control (end phase, end turn).
    PhaseControl,
    /// Pass or generic action.
    Generic,
}

impl TacticalIntent {
    /// Returns a priority weight for move ordering (higher = search first).
    /// This is used as a rough prior for search efficiency.
    pub fn ordering_priority(self) -> i32 {
        match self {
            // Critical tactical decisions - search first
            TacticalIntent::ChargeHighValueTarget => 100,
            TacticalIntent::MultiChargeObjectiveSwing => 95,
            TacticalIntent::ChargeSoftenedTarget => 90,
            TacticalIntent::FightMaxDamage => 85,
            TacticalIntent::MaximizeKills => 80,
            TacticalIntent::ContestObjective => 75,
            TacticalIntent::HoldObjective => 70,
            TacticalIntent::OffensiveStratagem => 65,
            TacticalIntent::DefensiveStratagem => 60,
            TacticalIntent::SoftenChargeTarget => 55,
            TacticalIntent::RemoveScoringUnit => 50,
            TacticalIntent::ForceBattleShock => 45,
            TacticalIntent::StageCharge => 40,
            TacticalIntent::DeepStrikeArrive => 35,
            TacticalIntent::MoveToCover => 30,
            TacticalIntent::LineUpShot => 28,
            TacticalIntent::Screen => 25,
            TacticalIntent::AdvanceAggressively => 22,
            TacticalIntent::HoldPosition => 20,
            TacticalIntent::Retreat => 18,
            TacticalIntent::FallBackToShoot => 15,
            TacticalIntent::BracketHardTarget => 40,
            TacticalIntent::FightPreserve => 50,
            TacticalIntent::ChooseStance => 30,
            TacticalIntent::ChooseWeaponProfile => 30,
            TacticalIntent::MovementReaction => 55,
            TacticalIntent::FightOrderManipulation => 60,
            TacticalIntent::DenyReserveZone => 20,
            TacticalIntent::ScoreObjective => 90,
            TacticalIntent::AllocateBlessings => 80,
            TacticalIntent::PhaseControl => 10,
            TacticalIntent::DeclineStratagem => 5,
            TacticalIntent::Generic => 0,
        }
    }
}

// ============================================================================
// MacroAction
// ============================================================================

/// A tactical macro-action representing a meaningful decision in the game.
///
/// A MacroAction wraps one or more engine Commands that together represent
/// a single tactical choice. For example, "move unit to contest objective"
/// might contain a SelectUnitToMove + NormalMove command pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroAction {
    /// Unique ID within the candidate set.
    pub id: MacroActionId,
    /// Human-readable label for diagnostics and display.
    pub label: String,
    /// The engine commands that implement this action.
    pub commands: Vec<Command>,
    /// The tactical intent behind this action.
    pub intent: TacticalIntent,
    /// Unit IDs involved in this action.
    pub actor_units: Vec<UnitId>,
    /// Priority hint from the evaluator for move ordering (higher = better).
    pub priority_hint: i32,
}

impl MacroAction {
    /// Create a new MacroAction.
    pub fn new(
        id: MacroActionId,
        label: String,
        commands: Vec<Command>,
        intent: TacticalIntent,
        actor_units: Vec<UnitId>,
    ) -> Self {
        let priority_hint = intent.ordering_priority();
        Self {
            id,
            label,
            commands,
            intent,
            actor_units,
            priority_hint,
        }
    }

    /// Create a MacroAction from a single command with a default label.
    pub fn from_command(id: MacroActionId, command: Command, intent: TacticalIntent) -> Self {
        let units = command.unit_ids();
        let label = format!("{}", command);
        Self::new(id, label, vec![command], intent, units)
    }

    /// Returns the first command in this action.
    pub fn first_command(&self) -> Option<&Command> {
        self.commands.first()
    }

    /// Returns the number of commands in this action.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

// ============================================================================
// Candidate Set
// ============================================================================

/// A collection of candidate macro-actions for a decision point.
/// Generated by the ActionGenerator and consumed by the search engine.
#[derive(Debug, Clone)]
pub struct CandidateSet {
    /// The player who must make this decision.
    pub owner: PlayerId,
    /// The phase in which this decision occurs.
    pub phase: Phase,
    /// The candidate actions to choose from.
    pub candidates: Vec<MacroAction>,
    /// Next ID counter for creating new MacroActions.
    next_id: u32,
}

impl CandidateSet {
    /// Create a new empty candidate set.
    pub fn new(owner: PlayerId, phase: Phase) -> Self {
        Self {
            owner,
            phase,
            candidates: Vec::new(),
            next_id: 0,
        }
    }

    /// Add a candidate action to the set.
    pub fn add(&mut self, command: Command, intent: TacticalIntent, label: String) {
        let id = MacroActionId(self.next_id);
        self.next_id += 1;
        let units = command.unit_ids();
        self.candidates.push(MacroAction::new(
            id,
            label,
            vec![command],
            intent,
            units,
        ));
    }

    /// Add a candidate with multiple commands.
    pub fn add_multi(
        &mut self,
        commands: Vec<Command>,
        intent: TacticalIntent,
        label: String,
        actor_units: Vec<UnitId>,
    ) {
        let id = MacroActionId(self.next_id);
        self.next_id += 1;
        self.candidates.push(MacroAction::new(
            id,
            label,
            commands,
            intent,
            actor_units,
        ));
    }

    /// Add a raw MacroAction directly.
    pub fn add_action(&mut self, mut action: MacroAction) {
        action.id = MacroActionId(self.next_id);
        self.next_id += 1;
        self.candidates.push(action);
    }

    /// Returns the number of candidates.
    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Sort candidates by priority hint (descending).
    pub fn sort_by_priority(&mut self) {
        self.candidates.sort_by(|a, b| b.priority_hint.cmp(&a.priority_hint));
    }
}

// ============================================================================
// Action Generator
// ============================================================================

/// Generates candidate macro-actions from the current game state.
///
/// The generator examines the current phase, subphase, and legal commands
/// to produce a bounded set of tactically meaningful candidates.
pub struct ActionGenerator;

impl ActionGenerator {
    /// Generate candidates for the current decision point.
    ///
    /// This is the main entry point. It examines the game state and produces
    /// a set of candidate actions appropriate for the current phase.
    pub fn generate(state: &GameState, player: PlayerId) -> CandidateSet {
        let mut candidates = CandidateSet::new(player, state.current_phase);

        match state.current_phase {
            Phase::Command => {
                Self::generate_command_phase_candidates(state, player, &mut candidates);
            }
            Phase::Movement => {
                Self::generate_movement_candidates(state, player, &mut candidates);
            }
            Phase::Shooting => {
                Self::generate_shooting_candidates(state, player, &mut candidates);
            }
            Phase::Charge => {
                Self::generate_charge_candidates(state, player, &mut candidates);
            }
            Phase::Fight => {
                Self::generate_fight_candidates(state, player, &mut candidates);
            }
            Phase::PreBattle => {
                Self::generate_setup_candidates(state, player, &mut candidates);
            }
            Phase::GameEnd => {
                // No decisions at game end
            }
        }

        // Always allow phase control actions
        Self::generate_phase_control_candidates(state, player, &mut candidates);

        candidates.sort_by_priority();
        candidates
    }

    /// Generate candidates from a pre-computed legal command list.
    /// This is used when the DecisionSurface already provides legal commands.
    pub fn from_legal_commands(
        legal_commands: &[Command],
        player: PlayerId,
        phase: Phase,
        state: &GameState,
    ) -> CandidateSet {
        let mut candidates = CandidateSet::new(player, phase);

        for cmd in legal_commands {
            let (intent, label) = Self::classify_command(cmd, state, player);
            candidates.add(cmd.clone(), intent, label);
        }

        candidates.sort_by_priority();
        candidates
    }

    // ========================================================================
    // Phase-specific candidate generation
    // ========================================================================

    /// Generate command phase candidates (scoring, blessings, battle-shock).
    fn generate_command_phase_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // Blessings of Khorne allocation (if World Eaters player)
        if !state.player(player).faction_round_flags.blessing_dice.is_empty() {
            // Generate blessing allocation options
            // For simplicity in candidate generation, we produce several common allocations
            let dice = &state.player(player).faction_round_flags.blessing_dice;
            Self::generate_blessing_allocations(dice, player, candidates);
        }

        // End phase action
        candidates.add(
            Command::EndPhase { phase: Phase::Command },
            TacticalIntent::PhaseControl,
            "End Command Phase".to_string(),
        );
    }

    /// Generate movement phase candidates.
    fn generate_movement_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        let units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| {
                u.owner == player
                    && u.is_on_battlefield()
                    && !u.is_destroyed()
                    && !state.turn_flags.units_moved.contains(&u.id)
            })
            .collect();

        for unit in &units {
            let unit_id = unit.id;

            // Option 1: Remain Stationary
            candidates.add(
                Command::RemainStationary { unit_id },
                TacticalIntent::HoldPosition,
                format!("{} remains stationary", unit.name),
            );

            // Skip movement options for engaged units (they can only remain or fall back)
            if matches!(unit.engagement_status, EngagementStatus::Engaged) {
                // Option: Fall Back
                if let Some(pos) = unit.reference_position() {
                    let move_dist = unit.base_movement.distance().mils();
                    // Generate fall-back positions away from enemies
                    let fallback_positions =
                        Self::generate_fallback_positions(state, pos, move_dist, player);
                    for dest in fallback_positions {
                        candidates.add(
                            Command::FallBack {
                                unit_id,
                                destination: dest,
                            },
                            TacticalIntent::FallBackToShoot,
                            format!("{} falls back", unit.name),
                        );
                    }
                }
                continue;
            }

            if let Some(pos) = unit.reference_position() {
                let move_dist = unit.base_movement.distance().mils();

                // Generate movement destinations from tactical anchors
                let destinations =
                    Self::generate_movement_destinations(state, pos, move_dist, player, unit);

                for (dest, intent) in destinations {
                    candidates.add(
                        Command::NormalMove {
                            unit_id,
                            destination: dest,
                        },
                        intent,
                        format!("{} moves to {:?}", unit.name, intent),
                    );
                }

                // Option: Advance (M + D6, average 3.5")
                let advance_destinations = Self::generate_advance_destinations(
                    state, pos, move_dist, player, unit,
                );
                for (dest, roll) in advance_destinations {
                    candidates.add(
                        Command::AdvanceMove {
                            unit_id,
                            destination: dest,
                            advance_roll: roll,
                        },
                        TacticalIntent::AdvanceAggressively,
                        format!("{} advances", unit.name),
                    );
                }
            }
        }

        // Reserve arrivals
        let reserve_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| u.owner == player && u.is_in_reserves())
            .collect();

        if state.battle_round.number() >= 2 {
            for unit in &reserve_units {
                let reserve_positions =
                    Self::generate_reserve_positions(state, player);
                for dest in reserve_positions {
                    candidates.add(
                        Command::ArriveFromReserves {
                            unit_id: unit.id,
                            position: dest,
                        },
                        TacticalIntent::DeepStrikeArrive,
                        format!("{} arrives from reserves", unit.name),
                    );
                }
            }
        }
    }

    /// Generate shooting phase candidates.
    fn generate_shooting_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        let eligible_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| {
                u.owner == player
                    && u.is_on_battlefield()
                    && !u.is_destroyed()
                    && !state.turn_flags.units_shot.contains(&u.id)
                    && !state.turn_flags.fell_back_this_turn.contains(&u.id)
            })
            .collect();

        let enemy_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| u.owner != player && u.is_on_battlefield() && !u.is_destroyed())
            .collect();

        for unit in &eligible_units {
            // Check if unit has ranged weapons
            let has_ranged = unit
                .models
                .iter()
                .any(|m| m.alive && !m.ranged_weapons.is_empty());
            if !has_ranged {
                continue;
            }

            // Find valid targets within weapon range
            let unit_pos = match unit.reference_position() {
                Some(p) => p,
                None => continue,
            };

            // Get max weapon range for this unit
            let max_range = unit
                .models
                .iter()
                .filter(|m| m.alive)
                .flat_map(|m| m.ranged_weapons.iter())
                .map(|w| w.range.mils())
                .max()
                .unwrap_or(0);

            // Check if engaged (can only shoot pistols)
            let is_engaged = matches!(unit.engagement_status, EngagementStatus::Engaged);

            for target in &enemy_units {
                let target_pos = match target.reference_position() {
                    Some(p) => p,
                    None => continue,
                };

                let dist = distance(unit_pos, target_pos).mils();

                // For engaged units, can only use pistol weapons against engaged enemies
                if is_engaged {
                    let has_pistol = unit.models.iter().any(|m| {
                        m.alive && m.ranged_weapons.iter().any(|w| {
                            w.abilities.has_pistol()
                        })
                    });
                    if !has_pistol {
                        continue;
                    }
                    // Pistol target must be within engagement range
                    if dist > Inches::ENGAGEMENT_RANGE.mils() {
                        continue;
                    }
                } else if dist > max_range {
                    continue;
                }

                // Classify shooting intent based on target
                let intent = if target.is_battleline()
                    && target.effective_oc().value() > 0
                {
                    TacticalIntent::RemoveScoringUnit
                } else if target.is_below_half_strength() {
                    TacticalIntent::ForceBattleShock
                } else {
                    TacticalIntent::MaximizeKills
                };

                // Generate a SelectUnitToShoot command for this targeting
                candidates.add(
                    Command::SelectUnitToShoot { unit_id: unit.id },
                    intent,
                    format!("{} shoots at {}", unit.name, target.name),
                );

                // Only generate one "select to shoot" per unit
                // (target selection happens in a subsequent decision)
                break;
            }
        }
    }

    /// Generate charge phase candidates.
    fn generate_charge_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        let eligible_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| {
                u.owner == player
                    && u.is_on_battlefield()
                    && !u.is_destroyed()
                    && !state.turn_flags.charged_this_turn.contains(&u.id)
                    && !state.turn_flags.advanced_this_turn.contains(&u.id)
                    && !state.turn_flags.fell_back_this_turn.contains(&u.id)
                    && matches!(u.engagement_status, EngagementStatus::NotEngaged)
            })
            .collect();

        let enemy_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| u.owner != player && u.is_on_battlefield() && !u.is_destroyed())
            .collect();

        for unit in &eligible_units {
            let unit_pos = match unit.reference_position() {
                Some(p) => p,
                None => continue,
            };

            // Find all enemy units within 12" (charge declaration range)
            let charge_range = Inches::from_inches(12);
            let mut viable_targets: Vec<(&UnitState, i32)> = Vec::new();

            for target in &enemy_units {
                let target_pos = match target.reference_position() {
                    Some(p) => p,
                    None => continue,
                };
                let dist = distance(unit_pos, target_pos).mils();
                if dist <= charge_range.mils() {
                    viable_targets.push((target, dist));
                }
            }

            // Sort targets by distance (closest first)
            viable_targets.sort_by_key(|(_, d)| *d);

            // Generate single-target charges for the top targets
            for (target, dist) in viable_targets.iter().take(3) {
                let intent = if target.is_character() {
                    TacticalIntent::ChargeHighValueTarget
                } else if *dist <= 6000 {
                    // Close targets are easier
                    TacticalIntent::ChargeSoftenedTarget
                } else {
                    TacticalIntent::ChargeHighValueTarget
                };

                candidates.add(
                    Command::DeclareCharge {
                        unit_id: unit.id,
                        targets: vec![target.id],
                    },
                    intent,
                    format!("{} charges {}", unit.name, target.name),
                );
            }

            // Generate multi-charge if multiple targets are close
            if viable_targets.len() >= 2 {
                let multi_targets: Vec<UnitId> = viable_targets
                    .iter()
                    .take(2)
                    .map(|(t, _)| t.id)
                    .collect();
                candidates.add(
                    Command::DeclareCharge {
                        unit_id: unit.id,
                        targets: multi_targets,
                    },
                    TacticalIntent::MultiChargeObjectiveSwing,
                    format!("{} multi-charges", unit.name),
                );
            }
        }
    }

    /// Generate fight phase candidates.
    fn generate_fight_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        let eligible_units: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| {
                u.owner == player
                    && u.is_on_battlefield()
                    && !u.is_destroyed()
                    && !state.turn_flags.units_fought.contains(&u.id)
                    && (matches!(u.engagement_status, EngagementStatus::Engaged)
                        || state.turn_flags.charged_this_turn.contains(&u.id))
            })
            .collect();

        for unit in &eligible_units {
            // Determine if this unit has fights-first (charged this turn)
            let has_fights_first = state.turn_flags.charged_this_turn.contains(&unit.id);
            let intent = if has_fights_first {
                TacticalIntent::FightMaxDamage
            } else {
                TacticalIntent::FightPreserve
            };

            candidates.add(
                Command::SelectUnitToFight { unit_id: unit.id },
                intent,
                format!("{} fights", unit.name),
            );
        }

        // Ka'tah stance choices for Custodes units
        for unit in &eligible_units {
            if unit.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes) {
                candidates.add(
                    Command::ChooseKaTahStance {
                        unit_id: unit.id,
                        stance: "Dacatarai".to_string(),
                    },
                    TacticalIntent::ChooseStance,
                    format!("{} uses Dacatarai stance (Sustained Hits)", unit.name),
                );
                candidates.add(
                    Command::ChooseKaTahStance {
                        unit_id: unit.id,
                        stance: "Rendax".to_string(),
                    },
                    TacticalIntent::ChooseStance,
                    format!("{} uses Rendax stance (Lethal Hits)", unit.name),
                );
            }
        }

        // Vaultswords profile choices for Tristraen
        for unit in &eligible_units {
            if unit.has_keyword(wh40k_core_types::Keyword::BladeChampion) {
                for model in &unit.models {
                    if model.alive {
                        for profile in &["Behemor", "Hurricanus", "Victus"] {
                            candidates.add(
                                Command::ChooseVaultswordsProfile {
                                    model_id: model.id,
                                    profile: profile.to_string(),
                                },
                                TacticalIntent::ChooseWeaponProfile,
                                format!("Tristraen selects {} profile", profile),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Generate stratagem candidates for the current timing window.
    pub fn generate_stratagem_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // Only generate stratagem candidates if the player has CP
        if !state.player(player).can_afford_cp(1) {
            return;
        }

        // Decline stratagem option
        candidates.add(
            Command::DeclineStratagem { player },
            TacticalIntent::DeclineStratagem,
            "Decline stratagem".to_string(),
        );

        // Stratagem generation is complex - for each legal stratagem, we add a candidate.
        // The validator determines legality; we generate reasonable options here.
        // The search engine validates each candidate before applying.

        // Core stratagems are handled by the decision surface in the game engine.
        // Here we generate the most common/valuable options.
    }

    /// Generate setup/pre-battle candidates.
    fn generate_setup_candidates(
        _state: &GameState,
        _player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // Setup phase candidates are typically driven by the DecisionSurface.
        // The AI can choose enhancements, secondaries, and deployment positions.
        // These are handled via from_legal_commands when the engine provides options.

        candidates.add(
            Command::PassAction,
            TacticalIntent::Generic,
            "Pass setup action".to_string(),
        );
    }

    /// Generate phase control candidates (end phase, end turn).
    fn generate_phase_control_candidates(
        state: &GameState,
        _player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // End phase is always an option (when there are no more units to activate)
        candidates.add(
            Command::EndPhase {
                phase: state.current_phase,
            },
            TacticalIntent::PhaseControl,
            format!("End {:?} phase", state.current_phase),
        );
    }

    // ========================================================================
    // Position generation helpers
    // ========================================================================

    /// Generate tactical movement destinations for a unit.
    /// Returns (position, intent) pairs.
    fn generate_movement_destinations(
        state: &GameState,
        current_pos: Position,
        move_dist_mils: i32,
        player: PlayerId,
        unit: &UnitState,
    ) -> Vec<(Position, TacticalIntent)> {
        let mut destinations = Vec::new();

        // 1. Objective-anchored positions
        for obj in &state.board.objectives {
            let dist = distance(current_pos, obj.position).mils();
            if dist <= move_dist_mils {
                // Can reach the objective this move
                destinations.push((obj.position, TacticalIntent::HoldObjective));
            } else if dist <= move_dist_mils * 2 {
                // Move towards the objective
                let ratio = move_dist_mils as f64 / dist as f64;
                let dx = obj.position.x.mils() - current_pos.x.mils();
                let dy = obj.position.y.mils() - current_pos.y.mils();
                let dest = Position {
                    x: Inches::from_mils(current_pos.x.mils() + (dx as f64 * ratio) as i32),
                    y: Inches::from_mils(current_pos.y.mils() + (dy as f64 * ratio) as i32),
                };
                destinations.push((dest, TacticalIntent::ContestObjective));
            }
        }

        // 2. Charge staging positions (move within 9-12" of enemies)
        let opponent = state.opponent_id(player);
        for enemy_unit in state.units.iter().filter(|u| {
            u.owner == opponent && u.is_on_battlefield() && !u.is_destroyed()
        }) {
            if let Some(enemy_pos) = enemy_unit.reference_position() {
                let dist = distance(current_pos, enemy_pos).mils();
                // Stage at charge range (9-12") if we can get close enough
                let target_dist = 10_000; // 10" - sweet spot for charge staging
                if dist > target_dist && dist - target_dist <= move_dist_mils {
                    let ratio = (dist - target_dist) as f64 / dist as f64;
                    let dx = enemy_pos.x.mils() - current_pos.x.mils();
                    let dy = enemy_pos.y.mils() - current_pos.y.mils();
                    let dest = Position {
                        x: Inches::from_mils(
                            current_pos.x.mils() + (dx as f64 * ratio) as i32,
                        ),
                        y: Inches::from_mils(
                            current_pos.y.mils() + (dy as f64 * ratio) as i32,
                        ),
                    };
                    destinations.push((dest, TacticalIntent::StageCharge));
                }
            }
        }

        // 3. Screening positions (between enemy and own valuable units/objectives)
        // Only for non-character, non-warlord units
        if !unit.is_character() && unit.effective_oc().value() <= 2 {
            // Find center of board
            let board_center = Position {
                x: Inches::from_mils(state.board.dimensions.width.mils() / 2),
                y: Inches::from_mils(state.board.dimensions.height.mils() / 2),
            };
            let dist_to_center = distance(current_pos, board_center).mils();
            if dist_to_center <= move_dist_mils {
                destinations.push((board_center, TacticalIntent::Screen));
            }
        }

        // Clamp all destinations to board bounds
        let board_w = state.board.dimensions.width.mils();
        let board_h = state.board.dimensions.height.mils();
        destinations
            .into_iter()
            .map(|(pos, intent)| {
                let clamped = Position {
                    x: Inches::from_mils(pos.x.mils().clamp(1000, board_w - 1000)),
                    y: Inches::from_mils(pos.y.mils().clamp(1000, board_h - 1000)),
                };
                (clamped, intent)
            })
            .collect()
    }

    /// Generate advance move destinations (M + D6).
    /// Returns (position, advance_roll) pairs.
    fn generate_advance_destinations(
        state: &GameState,
        current_pos: Position,
        move_dist_mils: i32,
        player: PlayerId,
        _unit: &UnitState,
    ) -> Vec<(Position, u8)> {
        let mut destinations = Vec::new();

        // Average advance roll is 3-4, generate with average roll of 4
        let advance_roll = 4u8;
        let total_move = move_dist_mils + (advance_roll as i32 * 1000);

        // Advance towards nearest unchosen objective
        for obj in &state.board.objectives {
            let dist = distance(current_pos, obj.position).mils();
            if dist <= total_move && dist > move_dist_mils {
                destinations.push((obj.position, advance_roll));
            }
        }

        // Advance towards enemy deployment zone
        let board_h = state.board.dimensions.height.mils();
        let mid_x = state.board.dimensions.width.mils() / 2;
        let target_y = if state.player(player).first_turn {
            // Advance towards enemy (top) side
            current_pos.y.mils() + total_move
        } else {
            // Advance towards enemy (bottom) side
            current_pos.y.mils() - total_move
        };
        let clamped_y = target_y.clamp(1000, board_h - 1000);
        destinations.push((
            Position {
                x: Inches::from_mils(mid_x),
                y: Inches::from_mils(clamped_y),
            },
            advance_roll,
        ));

        destinations
    }

    /// Generate fall-back positions for a unit falling back from engagement.
    fn generate_fallback_positions(
        state: &GameState,
        current_pos: Position,
        move_dist_mils: i32,
        player: PlayerId,
    ) -> Vec<Position> {
        let mut positions = Vec::new();
        let _board_w = state.board.dimensions.width.mils();
        let board_h = state.board.dimensions.height.mils();

        // Fall back towards own deployment zone
        let retreat_y = if state.player(player).first_turn {
            (current_pos.y.mils() - move_dist_mils).clamp(1000, board_h - 1000)
        } else {
            (current_pos.y.mils() + move_dist_mils).clamp(1000, board_h - 1000)
        };

        positions.push(Position {
            x: current_pos.x,
            y: Inches::from_mils(retreat_y),
        });

        // Also fall back towards nearest friendly objective
        for obj in &state.board.objectives {
            let dist = distance(current_pos, obj.position).mils();
            if dist <= move_dist_mils && dist > 0 {
                positions.push(obj.position);
            }
        }

        positions
    }

    /// Generate reserve arrival positions (Deep Strike: >9" from all enemies).
    fn generate_reserve_positions(state: &GameState, player: PlayerId) -> Vec<Position> {
        let mut positions = Vec::new();
        let board_w = state.board.dimensions.width.mils();
        let board_h = state.board.dimensions.height.mils();
        let min_dist = Inches::from_inches(9);

        // Generate positions near objectives
        for obj in &state.board.objectives {
            // Try positions offset from objectives
            let offsets = [
                (10_000, 0),
                (-10_000, 0),
                (0, 10_000),
                (0, -10_000),
                (7_000, 7_000),
                (-7_000, 7_000),
                (7_000, -7_000),
                (-7_000, -7_000),
            ];

            for (dx, dy) in &offsets {
                let x = (obj.position.x.mils() + dx).clamp(1000, board_w - 1000);
                let y = (obj.position.y.mils() + dy).clamp(1000, board_h - 1000);
                let pos = Position {
                    x: Inches::from_mils(x),
                    y: Inches::from_mils(y),
                };

                // Check >9" from all enemies
                let too_close = state.units.iter().any(|u| {
                    u.owner != player
                        && u.is_on_battlefield()
                        && u.reference_position()
                            .map(|ep| distance(pos, ep).mils() < min_dist.mils())
                            .unwrap_or(false)
                });

                if !too_close {
                    positions.push(pos);
                }
            }
        }

        // If no good positions near objectives, try board center regions
        if positions.is_empty() {
            let mid_x = board_w / 2;
            let mid_y = board_h / 2;
            let center = Position {
                x: Inches::from_mils(mid_x),
                y: Inches::from_mils(mid_y),
            };
            let too_close = state.units.iter().any(|u| {
                u.owner != player
                    && u.is_on_battlefield()
                    && u.reference_position()
                        .map(|ep| distance(center, ep).mils() < min_dist.mils())
                        .unwrap_or(false)
            });
            if !too_close {
                positions.push(center);
            }
        }

        positions
    }

    /// Generate Blessings of Khorne allocation options.
    fn generate_blessing_allocations(
        dice: &[u8],
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        use wh40k_command_system::BlessingAllocation;

        // Try to allocate common blessing combinations
        // Each blessing needs a pair of dice meeting certain criteria

        // Rage-fuelled Invigoration: Double 2+ (any two dice showing 2+)
        // Total Carnage: Double 2+ (same requirement)
        // Martial Excellence: Double 4+ OR Triple (any value)

        let dice_values: Vec<(usize, u8)> = dice.iter().enumerate().map(|(i, &v)| (i, v)).collect();
        let twos_plus: Vec<usize> = dice_values
            .iter()
            .filter(|(_, v)| *v >= 2)
            .map(|(i, _)| *i)
            .collect();
        let fours_plus: Vec<usize> = dice_values
            .iter()
            .filter(|(_, v)| *v >= 4)
            .map(|(i, _)| *i)
            .collect();

        // Single blessing: Rage-fuelled Invigoration
        if twos_plus.len() >= 2 {
            candidates.add(
                Command::AllocateBlessings {
                    player,
                    allocations: vec![BlessingAllocation {
                        blessing_name: "Rage-fuelled Invigoration".to_string(),
                        dice_indices: vec![twos_plus[0], twos_plus[1]],
                    }],
                },
                TacticalIntent::AllocateBlessings,
                "Activate Rage-fuelled Invigoration".to_string(),
            );
        }

        // Single blessing: Total Carnage
        if twos_plus.len() >= 2 {
            candidates.add(
                Command::AllocateBlessings {
                    player,
                    allocations: vec![BlessingAllocation {
                        blessing_name: "Total Carnage".to_string(),
                        dice_indices: vec![twos_plus[0], twos_plus[1]],
                    }],
                },
                TacticalIntent::AllocateBlessings,
                "Activate Total Carnage".to_string(),
            );
        }

        // Single blessing: Martial Excellence (Double 4+)
        if fours_plus.len() >= 2 {
            candidates.add(
                Command::AllocateBlessings {
                    player,
                    allocations: vec![BlessingAllocation {
                        blessing_name: "Martial Excellence".to_string(),
                        dice_indices: vec![fours_plus[0], fours_plus[1]],
                    }],
                },
                TacticalIntent::AllocateBlessings,
                "Activate Martial Excellence".to_string(),
            );
        }

        // Two blessings: Martial Excellence + Total Carnage
        if fours_plus.len() >= 2 && twos_plus.len() >= 4 {
            // Use fours_plus pair for ME, and remaining twos for TC
            let me_dice: Vec<usize> = vec![fours_plus[0], fours_plus[1]];
            let remaining: Vec<usize> = twos_plus
                .iter()
                .filter(|i| !me_dice.contains(i))
                .copied()
                .collect();
            if remaining.len() >= 2 {
                candidates.add(
                    Command::AllocateBlessings {
                        player,
                        allocations: vec![
                            BlessingAllocation {
                                blessing_name: "Martial Excellence".to_string(),
                                dice_indices: me_dice,
                            },
                            BlessingAllocation {
                                blessing_name: "Total Carnage".to_string(),
                                dice_indices: vec![remaining[0], remaining[1]],
                            },
                        ],
                    },
                    TacticalIntent::AllocateBlessings,
                    "Activate Martial Excellence + Total Carnage".to_string(),
                );
            }
        }

        // Two blessings: Total Carnage + Rage-fuelled
        if twos_plus.len() >= 4 {
            candidates.add(
                Command::AllocateBlessings {
                    player,
                    allocations: vec![
                        BlessingAllocation {
                            blessing_name: "Total Carnage".to_string(),
                            dice_indices: vec![twos_plus[0], twos_plus[1]],
                        },
                        BlessingAllocation {
                            blessing_name: "Rage-fuelled Invigoration".to_string(),
                            dice_indices: vec![twos_plus[2], twos_plus[3]],
                        },
                    ],
                },
                TacticalIntent::AllocateBlessings,
                "Activate Total Carnage + Rage-fuelled Invigoration".to_string(),
            );
        }

        // Empty allocation (skip blessings)
        candidates.add(
            Command::AllocateBlessings {
                player,
                allocations: vec![],
            },
            TacticalIntent::AllocateBlessings,
            "Skip Blessings of Khorne".to_string(),
        );
    }

    /// Classify a command into a tactical intent and generate a label.
    fn classify_command(
        cmd: &Command,
        state: &GameState,
        _player: PlayerId,
    ) -> (TacticalIntent, String) {
        match cmd {
            Command::RemainStationary { unit_id } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::HoldPosition, format!("{} remains stationary", name))
            }
            Command::NormalMove { unit_id, destination } => {
                let name = unit_name(state, *unit_id);
                let intent = classify_move_intent(state, *unit_id, *destination);
                (intent, format!("{} moves", name))
            }
            Command::AdvanceMove { unit_id, .. } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::AdvanceAggressively, format!("{} advances", name))
            }
            Command::FallBack { unit_id, .. } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::FallBackToShoot, format!("{} falls back", name))
            }
            Command::SelectUnitToShoot { unit_id } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::MaximizeKills, format!("{} shoots", name))
            }
            Command::DeclareCharge { unit_id, targets } => {
                let name = unit_name(state, *unit_id);
                if targets.len() > 1 {
                    (TacticalIntent::MultiChargeObjectiveSwing, format!("{} multi-charges", name))
                } else {
                    (TacticalIntent::ChargeHighValueTarget, format!("{} charges", name))
                }
            }
            Command::SelectUnitToFight { unit_id } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::FightMaxDamage, format!("{} fights", name))
            }
            Command::ChooseKaTahStance { stance, .. } => {
                (TacticalIntent::ChooseStance, format!("Choose {} stance", stance))
            }
            Command::ChooseVaultswordsProfile { profile, .. } => {
                (TacticalIntent::ChooseWeaponProfile, format!("Choose {} profile", profile))
            }
            Command::UseStratagem { stratagem_id, .. } => {
                (TacticalIntent::OffensiveStratagem, format!("Use stratagem {}", stratagem_id))
            }
            Command::DeclineStratagem { .. } => {
                (TacticalIntent::DeclineStratagem, "Decline stratagem".to_string())
            }
            Command::EndPhase { phase } => {
                (TacticalIntent::PhaseControl, format!("End {:?} phase", phase))
            }
            Command::EndPlayerTurn { .. } => {
                (TacticalIntent::PhaseControl, "End turn".to_string())
            }
            Command::PassAction => {
                (TacticalIntent::Generic, "Pass".to_string())
            }
            Command::ArriveFromReserves { unit_id, .. } => {
                let name = unit_name(state, *unit_id);
                (TacticalIntent::DeepStrikeArrive, format!("{} arrives from reserves", name))
            }
            Command::AllocateBlessings { .. } => {
                (TacticalIntent::AllocateBlessings, "Allocate Blessings".to_string())
            }
            Command::ScoreObjective { .. } => {
                (TacticalIntent::ScoreObjective, "Score objective".to_string())
            }
            Command::RazeObjective { .. } => {
                (TacticalIntent::ScoreObjective, "Raze objective".to_string())
            }
            _ => {
                (TacticalIntent::Generic, format!("{}", cmd))
            }
        }
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Get a unit's name from the game state.
fn unit_name(state: &GameState, unit_id: UnitId) -> String {
    state
        .unit(unit_id)
        .map(|u| u.name.clone())
        .unwrap_or_else(|| format!("Unit({})", unit_id))
}

/// Classify a movement destination into a tactical intent.
fn classify_move_intent(
    state: &GameState,
    unit_id: UnitId,
    destination: Position,
) -> TacticalIntent {
    // Check if moving towards an objective
    for obj in &state.board.objectives {
        if within_range(destination, obj.position, Inches::from_inches(3)) {
            return TacticalIntent::HoldObjective;
        }
    }

    // Check if moving towards charge range of an enemy
    let unit = match state.unit(unit_id) {
        Some(u) => u,
        None => return TacticalIntent::Generic,
    };

    let opponent = state.opponent_id(unit.owner);
    for enemy in state.units.iter().filter(|u| {
        u.owner == opponent && u.is_on_battlefield()
    }) {
        if let Some(enemy_pos) = enemy.reference_position() {
            let dist = distance(destination, enemy_pos).mils();
            if (9_000..=12_000).contains(&dist) {
                return TacticalIntent::StageCharge;
            }
        }
    }

    TacticalIntent::Generic
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_action_id_display() {
        let id = MacroActionId(42);
        assert_eq!(format!("{}", id), "MA(42)");
    }

    #[test]
    fn test_tactical_intent_ordering_priority() {
        assert!(
            TacticalIntent::ChargeHighValueTarget.ordering_priority()
                > TacticalIntent::HoldPosition.ordering_priority()
        );
        assert!(
            TacticalIntent::MaximizeKills.ordering_priority()
                > TacticalIntent::PhaseControl.ordering_priority()
        );
        assert!(
            TacticalIntent::ContestObjective.ordering_priority()
                > TacticalIntent::Retreat.ordering_priority()
        );
    }

    #[test]
    fn test_candidate_set_basic() {
        let player = PlayerId::new(1);
        let mut cs = CandidateSet::new(player, Phase::Movement);
        assert!(cs.is_empty());
        assert_eq!(cs.len(), 0);

        cs.add(
            Command::PassAction,
            TacticalIntent::Generic,
            "test".to_string(),
        );
        assert_eq!(cs.len(), 1);
        assert!(!cs.is_empty());
    }

    #[test]
    fn test_candidate_set_sorting() {
        let player = PlayerId::new(1);
        let mut cs = CandidateSet::new(player, Phase::Movement);

        cs.add(
            Command::PassAction,
            TacticalIntent::Generic,
            "low priority".to_string(),
        );
        cs.add(
            Command::PassAction,
            TacticalIntent::ChargeHighValueTarget,
            "high priority".to_string(),
        );
        cs.add(
            Command::PassAction,
            TacticalIntent::HoldObjective,
            "medium priority".to_string(),
        );

        cs.sort_by_priority();

        assert_eq!(cs.candidates[0].intent, TacticalIntent::ChargeHighValueTarget);
        assert_eq!(cs.candidates[2].intent, TacticalIntent::Generic);
    }

    #[test]
    fn test_macro_action_from_command() {
        let id = MacroActionId(0);
        let cmd = Command::PassAction;
        let action = MacroAction::from_command(id, cmd, TacticalIntent::Generic);

        assert_eq!(action.command_count(), 1);
        assert!(action.first_command().is_some());
        assert_eq!(action.intent, TacticalIntent::Generic);
    }

    #[test]
    fn test_candidate_set_auto_ids() {
        let player = PlayerId::new(1);
        let mut cs = CandidateSet::new(player, Phase::Movement);

        cs.add(Command::PassAction, TacticalIntent::Generic, "a".to_string());
        cs.add(Command::PassAction, TacticalIntent::Generic, "b".to_string());
        cs.add(Command::PassAction, TacticalIntent::Generic, "c".to_string());

        assert_eq!(cs.candidates[0].id, MacroActionId(0));
        assert_eq!(cs.candidates[1].id, MacroActionId(1));
        assert_eq!(cs.candidates[2].id, MacroActionId(2));
    }
}
