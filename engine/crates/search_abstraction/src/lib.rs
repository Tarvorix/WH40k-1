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

        // Check for reaction windows first — these take priority over normal phase actions
        if state.has_reaction_window() {
            if let Some(window) = state.current_reaction_window() {
                let window_owner = window.owner;
                // Generate candidates from the reaction window's legal actions
                for action in &window.legal_actions {
                    match action {
                        wh40k_event_system::ReactionAction::FireOverwatch(unit_id) => {
                            // Fire Overwatch: shoot at the charger, hitting only on 6s, costs 1CP
                            // Source: CP_Rules.md - Fire Overwatch stratagem
                            if let Some(unit) = state.unit(*unit_id) {
                                if state.player(window_owner).cp.value() >= 1 {
                                    candidates.add(
                                        Command::AssignOverwatchTarget {
                                            unit_id: *unit_id,
                                        },
                                        TacticalIntent::OffensiveStratagem,
                                        format!("Fire Overwatch with {}", unit.name),
                                    );
                                }
                            }
                        }
                        wh40k_event_system::ReactionAction::HeroicIntervention(unit_id) => {
                            // Heroic Intervention: CHARACTER charges the enemy, no Fights First
                            // Source: CP_Rules.md §11 - Heroic Intervention stratagem, costs 2CP
                            if let Some(unit) = state.unit(*unit_id) {
                                if state.player(window_owner).cp.value() >= 2 {
                                    candidates.add(
                                        Command::UseStratagem {
                                            player: window_owner,
                                            stratagem_id: wh40k_core_types::StratagemId::new(5), // HEROIC_INTERVENTION
                                            target: wh40k_command_system::StratagemTarget::Unit(*unit_id),
                                        },
                                        TacticalIntent::OffensiveStratagem,
                                        format!("{} heroic intervention", unit.name),
                                    );
                                }
                            }
                        }
                        wh40k_event_system::ReactionAction::Decline => {}
                        _ => {}
                    }
                }
            }
            // Always allow declining the reaction
            candidates.add(
                Command::DeclineStratagem { player },
                TacticalIntent::DeclineStratagem,
                "Decline reaction".to_string(),
            );
            Self::pre_validate(&mut candidates, state);
            return candidates;
        }

        // Check for pending charge rolls — units that declared charges but haven't rolled yet
        let pending_charge_units: Vec<UnitId> = state
            .turn_flags
            .declared_charge_targets
            .keys()
            .filter(|uid| !state.turn_flags.charge_roll_results.contains_key(uid))
            .copied()
            .collect();
        if !pending_charge_units.is_empty() {
            // Generate a single charge roll action per pending unit.
            // The roll value 0 is a sentinel — the executor will roll actual 2D6 dice.
            // Source: 40k_revised.md §9.3 - "Roll 2D6 to determine charge distance"
            for unit_id in pending_charge_units {
                if let Some(unit) = state.unit(unit_id) {
                    candidates.add(
                        Command::ResolveChargeRoll { unit_id, roll: 0 },
                        TacticalIntent::ChargeHighValueTarget,
                        format!("{} rolls charge", unit.name),
                    );
                }
            }
            Self::pre_validate(&mut candidates, state);
            return candidates;
        }

        // Check for units with successful charge rolls that need to make charge moves.
        // A unit is in charged_this_turn only on successful rolls (mark_charged is called).
        // We filter to units in charged_this_turn that haven't completed their move yet.
        // A charge move is "completed" when the unit becomes Engaged (set by apply_charge_move).
        let pending_charge_moves: Vec<UnitId> = state
            .turn_flags
            .charged_this_turn
            .iter()
            .filter(|uid| {
                // Must have a charge roll result
                state.turn_flags.charge_roll_results.contains_key(uid)
                    // Must not already be engaged (charge move sets engagement)
                    && state.unit(**uid).map_or(false, |u| {
                        matches!(u.engagement_status, EngagementStatus::NotEngaged)
                    })
            })
            .copied()
            .collect();
        if !pending_charge_moves.is_empty() {
            for unit_id in pending_charge_moves {
                let charge_roll = state.turn_flags.get_charge_roll(unit_id).unwrap_or(0);
                let charge_dist_mils = Inches::from_inches(charge_roll as i32).mils();

                if let Some(unit) = state.unit(unit_id) {
                    if let Some(pos) = unit.reference_position() {
                        let unit_base = unit.models.first()
                            .map(|m| m.base_size)
                            .unwrap_or(wh40k_core_types::BaseSize::MM25);

                        // Get charge targets and compute a destination within roll distance
                        // that also ends in engagement range of the target.
                        // Must also validate: on board, not ending in ER of non-target enemies.
                        if let Some(targets) = state.turn_flags.get_charge_targets(unit_id) {
                            let targets_clone = targets.clone();
                            for &target_id in &targets_clone {
                                if let Some(target) = state.unit(target_id) {
                                    if let Some(target_pos) = target.reference_position() {
                                        // Compute destination: move toward target, clamped to charge roll distance
                                        let dist = distance(pos, target_pos).mils();
                                        let dest = if dist <= charge_dist_mils || dist == 0 {
                                            target_pos
                                        } else {
                                            let ratio = charge_dist_mils as f64 / dist as f64;
                                            let dx = target_pos.x.mils() - pos.x.mils();
                                            let dy = target_pos.y.mils() - pos.y.mils();
                                            Position {
                                                x: Inches::from_mils(pos.x.mils() + (dx as f64 * ratio) as i32),
                                                y: Inches::from_mils(pos.y.mils() + (dy as f64 * ratio) as i32),
                                            }
                                        };

                                        // Check ER against actual target models (not just reference position)
                                        let in_er = target.models.iter()
                                            .filter(|m| m.alive)
                                            .any(|m| wh40k_geometry::within_engagement_range_2d(
                                                dest, unit_base, m.position, m.base_size,
                                            ));

                                        if !in_er {
                                            continue;
                                        }

                                        // Check destination is on the board
                                        if !state.board.contains(dest) {
                                            continue;
                                        }

                                        // Check no non-target enemies would be in ER
                                        // Source: 40k_revised.md §9.4 - cannot end in ER of non-target enemies
                                        let owner = unit.owner;
                                        let ends_in_er_of_non_target = state.units.iter().any(|enemy| {
                                            if enemy.owner == owner || enemy.is_destroyed() || !enemy.is_on_battlefield() {
                                                return false;
                                            }
                                            if targets_clone.contains(&enemy.id) {
                                                return false;
                                            }
                                            enemy.models.iter().filter(|m| m.alive).any(|m| {
                                                let will_be_in_er = wh40k_geometry::within_engagement_range_2d(
                                                    dest, unit_base, m.position, m.base_size,
                                                );
                                                let was_in_er = wh40k_geometry::within_engagement_range_2d(
                                                    pos, unit_base, m.position, m.base_size,
                                                );
                                                will_be_in_er && !was_in_er
                                            })
                                        });

                                        if ends_in_er_of_non_target {
                                            continue;
                                        }

                                        candidates.add(
                                            Command::MakeChargeMove {
                                                unit_id,
                                                destination: dest,
                                            },
                                            TacticalIntent::ChargeHighValueTarget,
                                            format!("{} charges into {}", unit.name, target.name),
                                        );
                                    }
                                }
                            }
                        }

                        // If no valid charge move destination was found, the charge effectively
                        // failed to reach any target — skip this unit (it stays in place).
                        // The EndPhase candidate from generate_phase_control_candidates will
                        // handle advancing past the charge phase.
                    }
                }
            }
            // Only return early if we generated charge move candidates
            if !candidates.candidates.is_empty() {
                Self::pre_validate(&mut candidates, state);
                return candidates;
            }
        }

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

        Self::pre_validate(&mut candidates, state);
        candidates
    }

    /// Remove any candidates whose commands would fail validation.
    /// Called on every exit path from generate() to ensure the UI and AI
    /// never see invalid options.
    fn pre_validate(candidates: &mut CandidateSet, state: &GameState) {
        candidates.candidates.retain(|action| {
            action.commands.iter().all(|cmd| {
                wh40k_game_core::CommandValidator::validate(state, cmd).is_legal()
            })
        });
        candidates.sort_by_priority();
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

                // Generate DeclareShootingTargets with all valid weapon+target pairs
                // Source: 40k_revised.md §7.2 - Each model shoots all its ranged weapons
                let mut weapon_targets: Vec<(wh40k_core_types::WeaponId, wh40k_core_types::UnitId)> = Vec::new();

                for model in unit.models.iter().filter(|m| m.alive) {
                    for weapon in &model.ranged_weapons {
                        // Skip non-pistol weapons if engaged
                        if is_engaged && !weapon.abilities.has_pistol() {
                            continue;
                        }
                        // Check weapon range
                        let weapon_range = weapon.range.mils();
                        if !is_engaged && dist > weapon_range {
                            continue;
                        }
                        // Check LOS (simplified - just check terrain blockage)
                        let los = state.board.check_los(unit_pos, target_pos);
                        if matches!(los, wh40k_core_types::Visibility::NotVisible)
                            && !weapon.abilities.has_indirect_fire() {
                            continue;
                        }
                        weapon_targets.push((weapon.id, target.id));
                    }
                }

                if !weapon_targets.is_empty() {
                    // Generate a multi-command macro:
                    // SelectUnitToShoot + DeclareShootingTargets + ResolveShootingAttack per weapon
                    let mut commands = vec![
                        Command::SelectUnitToShoot { unit_id: unit.id },
                        Command::DeclareShootingTargets {
                            unit_id: unit.id,
                            targets: weapon_targets.clone(),
                        },
                    ];
                    // Add ResolveShootingAttack for each weapon-target pair
                    for (weapon_id, target_id) in &weapon_targets {
                        commands.push(Command::ResolveShootingAttack {
                            attacker_id: unit.id,
                            target_id: *target_id,
                            weapon_id: *weapon_id,
                        });
                    }
                    candidates.add_multi(
                        commands,
                        intent,
                        format!("{} shoots at {} ({} weapons)", unit.name, target.name, weapon_targets.len()),
                        vec![unit.id],
                    );
                }
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
                    && !state.turn_flags.declared_charge_targets.contains_key(&u.id)
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
            for (target, dist) in viable_targets.iter() {
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

            // Find enemy units in engagement range to fight
            let unit_pos = match unit.reference_position() {
                Some(p) => p,
                None => continue,
            };
            let unit_base = unit.models.first()
                .map(|m| m.base_size)
                .unwrap_or(wh40k_core_types::BaseSize::MM25);

            // Collect melee weapon-target pairs for all engaged enemies
            let mut weapon_targets: Vec<(wh40k_core_types::WeaponId, UnitId)> = Vec::new();
            let mut target_names: Vec<String> = Vec::new();

            for enemy in &state.units {
                if enemy.owner == player || enemy.is_destroyed() || !enemy.is_on_battlefield() {
                    continue;
                }
                // Check if any enemy model is in engagement range
                let in_er = enemy.models.iter().filter(|m| m.alive).any(|m| {
                    wh40k_geometry::within_engagement_range_2d(
                        unit_pos, unit_base, m.position, m.base_size,
                    )
                });
                if !in_er {
                    continue;
                }

                // Add melee weapons only for models in engagement range of this enemy.
                // Source: CP_Rules.md §8.3 — only models within ER can attack.
                for model in unit.models.iter().filter(|m| m.alive) {
                    let model_in_er = enemy.models.iter().filter(|em| em.alive).any(|em| {
                        wh40k_geometry::within_engagement_range_2d(
                            model.position, model.base_size,
                            em.position, em.base_size,
                        )
                    });
                    if model_in_er {
                        // Vaultswords profile filtering: if this model has a chosen profile,
                        // only use the selected weapon (not all 3 profiles).
                        // Source: Custodes.md — Tristraen selects one Vaultswords profile per fight.
                        let chosen_profile = state.turn_flags.get_vaultswords_profile(model.id);
                        for weapon in &model.melee_weapons {
                            if let Some(profile) = chosen_profile {
                                // Filter to only the chosen profile's weapon
                                let matches = match profile {
                                    "Behemor" => weapon.id == wh40k_core_types::WeaponId::new(1000),
                                    "Hurricanus" => weapon.id == wh40k_core_types::WeaponId::new(1001),
                                    "Victus" => weapon.id == wh40k_core_types::WeaponId::new(1002),
                                    _ => true, // Unknown profile, allow all
                                };
                                if matches {
                                    weapon_targets.push((weapon.id, enemy.id));
                                }
                            } else {
                                // No profile chosen — include all weapons
                                // (shouldn't happen if ChooseVaultswordsProfile was generated first)
                                weapon_targets.push((weapon.id, enemy.id));
                            }
                        }
                    }
                }
                target_names.push(enemy.name.clone());
            }

            // Compute pile-in positions: each model moves up to 3" toward closest enemy.
            // Source: CP_Rules.md §8.3 — Pile In: up to 3" toward closest enemy model.
            let pile_in_positions: Vec<(wh40k_core_types::ModelId, Position)> = unit.models
                .iter()
                .filter(|m| m.alive)
                .filter_map(|model| {
                    // Find closest enemy model position
                    let closest_enemy_pos = state.units.iter()
                        .filter(|e| e.owner != player && !e.is_destroyed() && e.is_on_battlefield())
                        .flat_map(|e| e.models.iter().filter(|em| em.alive))
                        .map(|em| (em.position, distance(model.position, em.position).mils()))
                        .min_by_key(|(_, d)| *d);

                    if let Some((enemy_pos, dist)) = closest_enemy_pos {
                        if dist <= 0 { return None; } // Already on top
                        let pile_in_dist = 3000i32.min(dist); // 3" in mils, or less
                        let ratio = pile_in_dist as f64 / dist as f64;
                        let dx = enemy_pos.x.mils() - model.position.x.mils();
                        let dy = enemy_pos.y.mils() - model.position.y.mils();
                        let new_pos = Position {
                            x: Inches::from_mils(model.position.x.mils() + (dx as f64 * ratio) as i32),
                            y: Inches::from_mils(model.position.y.mils() + (dy as f64 * ratio) as i32),
                        };
                        Some((model.id, new_pos))
                    } else {
                        None
                    }
                })
                .collect();

            if weapon_targets.is_empty() {
                // No melee weapons or no targets — still generate SelectUnitToFight
                // with pile in (can pile in even without weapons)
                let mut commands: Vec<Command> = vec![
                    Command::SelectUnitToFight { unit_id: unit.id },
                ];
                if !pile_in_positions.is_empty() {
                    commands.push(Command::PileIn {
                        unit_id: unit.id,
                        positions: pile_in_positions.clone(),
                    });
                }
                candidates.add_multi(
                    commands,
                    intent,
                    format!("{} fights", unit.name),
                    vec![unit.id],
                );
            } else {
                // Full fight macro: Select + PileIn + Declare + Resolve + Consolidate
                // Source: CP_Rules.md §8.3 — Pile In → Make Attacks → Consolidate
                let mut commands: Vec<Command> = vec![
                    Command::SelectUnitToFight { unit_id: unit.id },
                ];

                // Pile In (before attacks)
                if !pile_in_positions.is_empty() {
                    commands.push(Command::PileIn {
                        unit_id: unit.id,
                        positions: pile_in_positions.clone(),
                    });
                }

                // Declare targets and resolve attacks
                commands.push(Command::DeclareMeleeTargets {
                    unit_id: unit.id,
                    targets: weapon_targets.clone(),
                });
                for (weapon_id, target_id) in &weapon_targets {
                    commands.push(Command::ResolveMeleeAttack {
                        attacker_id: unit.id,
                        target_id: *target_id,
                        weapon_id: *weapon_id,
                    });
                }

                // Consolidate (after attacks) — reuse same direction logic
                // Source: CP_Rules.md §8.3 — Consolidate: up to 3" toward closest enemy
                if !pile_in_positions.is_empty() {
                    commands.push(Command::Consolidate {
                        unit_id: unit.id,
                        positions: pile_in_positions,
                    });
                }

                let target_str = target_names.join(", ");
                candidates.add_multi(
                    commands,
                    intent,
                    format!("{} fights {}", unit.name, target_str),
                    vec![unit.id],
                );
            }
        }

        // Ka'tah stance choices for Custodes units that haven't chosen a stance yet
        for unit in &eligible_units {
            if unit.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes)
                && state.turn_flags.get_ka_tah_stance(unit.id).is_none()
            {
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

        // Vaultswords profile choices for Tristraen (only if not already chosen)
        for unit in &eligible_units {
            if unit.has_keyword(wh40k_core_types::Keyword::BladeChampion) {
                for model in &unit.models {
                    if model.alive
                        && state.turn_flags.get_vaultswords_profile(model.id).is_none()
                    {
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

        // Generate Command Re-roll stratagem (usable in any phase)
        // Source: 40k_revised.md - Command Re-roll: 1CP, re-roll one dice
        if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(1)) {
            // Check Command Re-roll isn't blocked (Forward Outpost mission rule)
            if !state.player(player).mission_progress.command_reroll_blocked {
                candidates.add(
                    Command::UseStratagem {
                        player,
                        stratagem_id: wh40k_core_types::StratagemId::new(1),
                        target: wh40k_command_system::StratagemTarget::Player(player),
                    },
                    TacticalIntent::DefensiveStratagem,
                    "Command Re-roll (1CP)".to_string(),
                );
            }
        }

        // Generate phase-appropriate stratagems
        match state.current_phase {
            Phase::Shooting => {
                // Go to Ground (1CP): Target gains 6++ invuln for shooting attacks
                for unit in state.units.iter().filter(|u| {
                    u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                        && u.has_keyword(wh40k_core_types::Keyword::Infantry)
                }) {
                    if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(4)) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(4), // GO_TO_GROUND
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::DefensiveStratagem,
                            format!("Go to Ground on {} (1CP)", unit.name),
                        );
                    }
                }

                // Grenade stratagem (1CP): unit with Grenades keyword
                for unit in state.units.iter().filter(|u| {
                    u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                        && u.has_keyword(wh40k_core_types::Keyword::Grenades)
                }) {
                    if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(9)) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(9), // GRENADE
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::OffensiveStratagem,
                            format!("Grenade on {} (1CP)", unit.name),
                        );
                    }
                }
            }

            Phase::Fight => {
                // Counter-offensive (2CP): fight with one of your units next
                if state.player(player).can_afford_cp(2)
                    && !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(2))
                {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && !state.turn_flags.has_fought(u.id)
                            && (u.engagement_status == wh40k_core_types::EngagementStatus::Engaged
                                || state.turn_flags.charged_this_turn(u.id))
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(2), // COUNTER_OFFENSIVE
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::FightOrderManipulation,
                            format!("Counter-offensive on {} (2CP)", unit.name),
                        );
                    }
                }

                // Epic Challenge (1CP): CHARACTER gains Precision in melee
                // Source: 40k_revised.md - Epic Challenge stratagem
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(7)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.is_character()
                            && (u.engagement_status == wh40k_core_types::EngagementStatus::Engaged
                                || state.turn_flags.charged_this_turn(u.id))
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(7), // EPIC_CHALLENGE
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::OffensiveStratagem,
                            format!("Epic Challenge on {} (1CP)", unit.name),
                        );
                    }
                }
            }

            Phase::Movement => {
                // Rapid Ingress (1CP): arrive from Strategic Reserves
                if state.battle_round.number() >= 2
                    && !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(8))
                {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_in_reserves()
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(8), // RAPID_INGRESS
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::MovementReaction,
                            format!("Rapid Ingress for {} (1CP)", unit.name),
                        );
                    }
                }
            }

            Phase::Command => {
                // Insane Bravery (1CP): auto-pass one battle-shock test
                // Source: 40k_revised.md - Insane Bravery stratagem
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(6)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.battle_shocked
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(6), // INSANE_BRAVERY
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::DefensiveStratagem,
                            format!("Insane Bravery on {} (1CP)", unit.name),
                        );
                    }
                }
            }

            _ => {}
        }

        // Faction-specific stratagems
        // Check if player has Custodes units
        let has_custodes = state.units.iter().any(|u| {
            u.owner == player && u.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes)
        });
        let has_world_eaters = state.units.iter().any(|u| {
            u.owner == player && u.has_keyword(wh40k_core_types::Keyword::WorldEaters)
        });

        if has_custodes && state.current_phase == Phase::Fight {
            // Gilded Spear (1CP): 6" pile-in/consolidate for one unit
            if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(200)) {
                for unit in state.units.iter().filter(|u| {
                    u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                        && u.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes)
                }) {
                    candidates.add(
                        Command::UseStratagem {
                            player,
                            stratagem_id: wh40k_core_types::StratagemId::new(200),
                            target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                        },
                        TacticalIntent::OffensiveStratagem,
                        format!("Gilded Spear on {} (1CP)", unit.name),
                    );
                }
            }
        }

        if has_world_eaters {
            if state.current_phase == Phase::Fight {
                // Horrifying Butchery (1CP): enemy unit in ER takes -1Ld modifier
                // Source: Frenzied_Reavers.md - Horrifying Butchery
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(100)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.has_keyword(wh40k_core_types::Keyword::WorldEaters)
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(100),
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::OffensiveStratagem,
                            format!("Horrifying Butchery on {} (1CP)", unit.name),
                        );
                    }
                }

                // Berserk Resilience (1CP): WE CHARACTER gets +2 to Fight-on-Death rolls
                // Source: Frenzied_Reavers.md - Berserk Resilience
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(101)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.is_character()
                            && u.has_keyword(wh40k_core_types::Keyword::WorldEaters)
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(101),
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::DefensiveStratagem,
                            format!("Berserk Resilience on {} (1CP)", unit.name),
                        );
                    }
                }
            }

            if state.current_phase == Phase::Shooting {
                // Bloodlust (1CP): Jakhals unit can shoot after falling back
                // Source: Frenzied_Reavers.md - Bloodlust
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(102)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.name == "Jakhals"
                            && state.turn_flags.fell_back_this_turn.contains(&u.id)
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(102),
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::OffensiveStratagem,
                            format!("Bloodlust on {} (1CP)", unit.name),
                        );
                    }
                }
            }
        }

        if has_custodes {
            if state.current_phase == Phase::Movement {
                // Inescapable Vengeance (1CP): unit can shoot after advancing (all weapons)
                // Source: Custodes.md - Inescapable Vengeance
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(201)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes)
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(201),
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::OffensiveStratagem,
                            format!("Inescapable Vengeance on {} (1CP)", unit.name),
                        );
                    }
                }
            }

            if state.current_phase == Phase::Charge {
                // Overawing Magnificence (1CP): -1 to enemy charge rolls against target
                // Source: Custodes.md - Overawing Magnificence
                if !state.player(player).stratagem_usage.used_this_phase(wh40k_core_types::StratagemId::new(202)) {
                    for unit in state.units.iter().filter(|u| {
                        u.owner == player && u.is_on_battlefield() && !u.is_destroyed()
                            && u.has_keyword(wh40k_core_types::Keyword::AdeptusCustodes)
                    }) {
                        candidates.add(
                            Command::UseStratagem {
                                player,
                                stratagem_id: wh40k_core_types::StratagemId::new(202),
                                target: wh40k_command_system::StratagemTarget::Unit(unit.id),
                            },
                            TacticalIntent::DefensiveStratagem,
                            format!("Overawing Magnificence on {} (1CP)", unit.name),
                        );
                    }
                }
            }
        }
    }

    /// Generate setup/pre-battle candidates.
    ///
    /// During deployment, generate PlaceUnit commands for each undeployed unit
    /// with positions sampled from the player's deployment zone.
    ///
    /// Source: CP_Rules.md - "The Defender sets up their army first, then the Attacker"
    fn generate_setup_candidates(
        state: &GameState,
        player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // Find undeployed units belonging to this player
        let undeployed: Vec<&UnitState> = state
            .units
            .iter()
            .filter(|u| {
                u.owner == player
                    && u.status == wh40k_core_types::UnitStatus::Undeployed
            })
            .collect();

        if undeployed.is_empty() {
            // All units deployed - check if opponent still needs to deploy
            let opponent = state.opponent_id(player);
            let opponent_has_undeployed = state.units.iter().any(|u| {
                u.owner == opponent
                    && u.status == wh40k_core_types::UnitStatus::Undeployed
            });

            if !opponent_has_undeployed {
                // Both players fully deployed, offer SetupComplete
                candidates.add(
                    Command::SetupComplete,
                    TacticalIntent::PhaseControl,
                    "Complete deployment".to_string(),
                );
            }
            return;
        }

        // Get deployment zone for this player
        let zone = match &state.deployment_config {
            Some(config) => config.zone_for(player),
            None => {
                // No deployment config - fall back to pass
                candidates.add(
                    Command::PassAction,
                    TacticalIntent::Generic,
                    "Pass setup action".to_string(),
                );
                return;
            }
        };

        // Generate deployment positions by sampling the zone
        let positions = Self::generate_deployment_positions(state, zone, player);

        for unit in &undeployed {
            for &pos in &positions {
                // Verify the position is within the zone and on the board
                if zone.contains(pos) && state.board.contains(pos) {
                    candidates.add(
                        Command::PlaceUnit {
                            player,
                            unit_id: unit.id,
                            position: pos,
                        },
                        TacticalIntent::HoldObjective,
                        format!("Deploy {} at ({:.0}\", {:.0}\")", unit.name, pos.x.as_f64(), pos.y.as_f64()),
                    );
                }
            }
        }
    }

    /// Generate phase control candidates (end phase, end turn).
    fn generate_phase_control_candidates(
        state: &GameState,
        _player: PlayerId,
        candidates: &mut CandidateSet,
    ) {
        // During PreBattle, don't offer EndPhase if ANY player still has undeployed units.
        // Deployment must be completed before the game can start.
        // Source: CP_Rules.md - "The Defender sets up their army first, then the Attacker"
        if state.current_phase == Phase::PreBattle {
            let any_undeployed = state.units.iter().any(|u| {
                u.status == wh40k_core_types::UnitStatus::Undeployed
            });
            if any_undeployed {
                return;
            }
        }

        // End phase is an option when there are no more units to activate
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
        let board_w = state.board.dimensions.width.mils();
        let board_h = state.board.dimensions.height.mils();

        // Helper: clamp a destination so actual distance from current_pos <= total_move,
        // and position stays within board bounds.
        let clamp_to_range =
            |dest: Position, max_dist: i32| -> Position {
                let dist = distance(current_pos, dest).mils();
                let pos = if dist > max_dist && dist > 0 {
                    // Move along the direction but only as far as max_dist
                    let ratio = max_dist as f64 / dist as f64;
                    let dx = dest.x.mils() - current_pos.x.mils();
                    let dy = dest.y.mils() - current_pos.y.mils();
                    Position {
                        x: Inches::from_mils(current_pos.x.mils() + (dx as f64 * ratio) as i32),
                        y: Inches::from_mils(current_pos.y.mils() + (dy as f64 * ratio) as i32),
                    }
                } else {
                    dest
                };
                // Clamp to board bounds
                Position {
                    x: Inches::from_mils(pos.x.mils().clamp(1000, board_w - 1000)),
                    y: Inches::from_mils(pos.y.mils().clamp(1000, board_h - 1000)),
                }
            };

        // Generate for low/average/high advance rolls to cover D6 range
        for advance_roll in [2u8, 4u8, 6u8] {
            let total_move = move_dist_mils + (advance_roll as i32 * 1000);

            // Advance towards nearest unchosen objective
            for obj in &state.board.objectives {
                let dist = distance(current_pos, obj.position).mils();
                if dist <= total_move && dist > move_dist_mils {
                    let clamped = clamp_to_range(obj.position, total_move);
                    destinations.push((clamped, advance_roll));
                }
            }

            // Advance towards enemy deployment zone
            let target_y = if state.player(player).first_turn {
                board_h - 1000
            } else {
                1000
            };
            let raw_dest = Position {
                x: current_pos.x, // Keep same X to avoid diagonal overshoot
                y: Inches::from_mils(target_y),
            };
            let clamped = clamp_to_range(raw_dest, total_move);
            destinations.push((clamped, advance_roll));
        }

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
        let board_w = state.board.dimensions.width.mils();
        let board_h = state.board.dimensions.height.mils();

        // Fall back towards own deployment zone (straight back)
        let retreat_y = if state.player(player).first_turn {
            (current_pos.y.mils() - move_dist_mils).clamp(1000, board_h - 1000)
        } else {
            (current_pos.y.mils() + move_dist_mils).clamp(1000, board_h - 1000)
        };
        positions.push(Position {
            x: current_pos.x,
            y: Inches::from_mils(retreat_y),
        });

        // Diagonal retreat (back-left)
        let diag_dist = move_dist_mils * 707 / 1000; // ~0.707 * move for 45-degree diagonal
        let diag_y = if state.player(player).first_turn {
            (current_pos.y.mils() - diag_dist).clamp(1000, board_h - 1000)
        } else {
            (current_pos.y.mils() + diag_dist).clamp(1000, board_h - 1000)
        };
        positions.push(Position {
            x: Inches::from_mils((current_pos.x.mils() - diag_dist).clamp(1000, board_w - 1000)),
            y: Inches::from_mils(diag_y),
        });

        // Diagonal retreat (back-right)
        positions.push(Position {
            x: Inches::from_mils((current_pos.x.mils() + diag_dist).clamp(1000, board_w - 1000)),
            y: Inches::from_mils(diag_y),
        });

        // Lateral retreat (left)
        positions.push(Position {
            x: Inches::from_mils((current_pos.x.mils() - move_dist_mils).clamp(1000, board_w - 1000)),
            y: current_pos.y,
        });

        // Lateral retreat (right)
        positions.push(Position {
            x: Inches::from_mils((current_pos.x.mils() + move_dist_mils).clamp(1000, board_w - 1000)),
            y: current_pos.y,
        });

        // Fall back towards nearest friendly objective
        for obj in &state.board.objectives {
            let dist = distance(current_pos, obj.position).mils();
            if dist <= move_dist_mils && dist > 0 {
                positions.push(obj.position);
            }
        }

        // Fall back away from nearest enemy
        let nearest_enemy = state.units.iter()
            .filter(|u| u.owner != player && u.is_on_battlefield() && !u.is_destroyed())
            .filter_map(|u| u.reference_position().map(|p| (p, distance(current_pos, p).mils())))
            .min_by_key(|(_, d)| *d);

        if let Some((enemy_pos, _)) = nearest_enemy {
            let dx = current_pos.x.mils() - enemy_pos.x.mils();
            let dy = current_pos.y.mils() - enemy_pos.y.mils();
            let mag = ((dx as f64).powi(2) + (dy as f64).powi(2)).sqrt() as i32;
            if mag > 0 {
                let flee_x = (current_pos.x.mils() + dx * move_dist_mils / mag).clamp(1000, board_w - 1000);
                let flee_y = (current_pos.y.mils() + dy * move_dist_mils / mag).clamp(1000, board_h - 1000);
                positions.push(Position {
                    x: Inches::from_mils(flee_x),
                    y: Inches::from_mils(flee_y),
                });
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

    /// Generate candidate deployment positions within a deployment zone.
    ///
    /// Samples a grid of positions within the zone's bounding box,
    /// filtered to those actually inside the zone polygon.
    fn generate_deployment_positions(
        state: &GameState,
        zone: &wh40k_geometry::DeploymentZone,
        _player: PlayerId,
    ) -> Vec<Position> {
        let mut positions = Vec::new();

        // Find bounding box of the zone polygon
        let verts = &zone.polygon.vertices;
        if verts.is_empty() {
            return positions;
        }

        let min_x = verts.iter().map(|v| v.x.mils()).min().unwrap_or(0);
        let max_x = verts.iter().map(|v| v.x.mils()).max().unwrap_or(0);
        let min_y = verts.iter().map(|v| v.y.mils()).min().unwrap_or(0);
        let max_y = verts.iter().map(|v| v.y.mils()).max().unwrap_or(0);

        // Sample at 2" intervals within the bounding box, offset 1" from edges
        let step = 2000; // 2 inches in mils
        let margin = 1000; // 1 inch margin from zone edge

        let start_x = min_x + margin;
        let end_x = max_x - margin;
        let start_y = min_y + margin;
        let end_y = max_y - margin;

        let mut x = start_x;
        while x <= end_x {
            let mut y = start_y;
            while y <= end_y {
                let pos = Position {
                    x: Inches::from_mils(x),
                    y: Inches::from_mils(y),
                };
                if zone.contains(pos) && state.board.contains(pos) {
                    // Check not too close to already-deployed units (min 1" apart)
                    let too_close = state.units.iter().any(|u| {
                        u.status == wh40k_core_types::UnitStatus::OnBattlefield
                            && u.reference_position()
                                .map(|upos| distance(pos, upos).mils() < 1000)
                                .unwrap_or(false)
                    });
                    if !too_close {
                        positions.push(pos);
                    }
                }
                y += step;
            }
            x += step;
        }

        // Also add positions near objectives that are within the zone
        for obj in &state.board.objectives {
            if zone.contains(obj.position) && state.board.contains(obj.position) {
                positions.push(obj.position);
            }
        }

        // Limit to reasonable number to avoid explosion
        if positions.len() > 20 {
            // Take evenly spaced subset
            let step = positions.len() / 15;
            positions = positions.into_iter().step_by(step.max(1)).take(15).collect();
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

    #[test]
    fn test_deployment_generates_place_unit_candidates() {
        // Load a real game scenario and verify deployment candidates are generated
        let seed = [42u8; 32];
        let state = wh40k_game_core::scenario::ScenarioLoader::load_scenario(
            wh40k_core_types::FactionId(0), wh40k_core_types::FactionId(1),
            Some(wh40k_core_types::MissionId::new(1)),
            seed,
        );

        assert_eq!(state.current_phase, wh40k_core_types::Phase::PreBattle);
        assert!(state.deployment_config.is_some(), "deployment_config must be set");

        let decision_owner = state.decision_owner;
        let candidates = ActionGenerator::generate(&state, decision_owner);

        // Must have PlaceUnit candidates
        let place_unit_count = candidates.candidates.iter()
            .filter(|c| matches!(&c.commands[0], Command::PlaceUnit { .. }))
            .count();

        eprintln!("Decision owner: {:?}", decision_owner);
        eprintln!("Total candidates: {}", candidates.candidates.len());
        eprintln!("PlaceUnit candidates: {}", place_unit_count);
        for c in candidates.candidates.iter().take(5) {
            eprintln!("  {:?}: {}", c.intent, c.label);
        }

        assert!(place_unit_count > 0, "Must generate PlaceUnit candidates for deployment");
    }

    #[test]
    fn test_full_deployment_flow() {
        use wh40k_game_core::executor::CommandExecutor;

        let seed = [42u8; 32];
        let mut state = wh40k_game_core::scenario::ScenarioLoader::load_scenario(
            wh40k_core_types::FactionId(0), wh40k_core_types::FactionId(1),
            Some(wh40k_core_types::MissionId::new(1)),
            seed,
        );

        let defender = state.decision_owner;
        let attacker = state.opponent_id(defender);

        let defender_count = state.units.iter()
            .filter(|u| u.owner == defender && u.status == wh40k_core_types::UnitStatus::Undeployed)
            .count();
        assert!(defender_count > 0, "Defender should have undeployed units");

        // Deploy all defender units by choosing first PlaceUnit each time
        for _ in 0..50 {
            if state.decision_owner != defender { break; }
            let candidates = ActionGenerator::generate(&state, state.decision_owner);
            let place_cmd = candidates.candidates.iter()
                .find(|c| matches!(&c.commands[0], Command::PlaceUnit { .. }));
            match place_cmd {
                Some(action) => {
                    CommandExecutor::execute(&mut state, &action.commands[0]).unwrap();
                }
                None => break,
            }
        }

        let remaining_defender = state.units.iter()
            .filter(|u| u.owner == defender && u.status == wh40k_core_types::UnitStatus::Undeployed)
            .count();
        assert_eq!(remaining_defender, 0, "All defender units should be deployed");
        assert_eq!(state.decision_owner, attacker, "Decision owner should switch to attacker");

        // Now deploy attacker units
        let attacker_count = state.units.iter()
            .filter(|u| u.owner == attacker && u.status == wh40k_core_types::UnitStatus::Undeployed)
            .count();
        assert!(attacker_count > 0, "Attacker should have undeployed units");

        for _ in 0..50 {
            if state.decision_owner != attacker { break; }
            let candidates = ActionGenerator::generate(&state, state.decision_owner);
            let place_cmd = candidates.candidates.iter()
                .find(|c| matches!(&c.commands[0], Command::PlaceUnit { .. }));
            match place_cmd {
                Some(action) => {
                    CommandExecutor::execute(&mut state, &action.commands[0]).unwrap();
                }
                None => break,
            }
        }

        let remaining_attacker = state.units.iter()
            .filter(|u| u.owner == attacker && u.status == wh40k_core_types::UnitStatus::Undeployed)
            .count();
        assert_eq!(remaining_attacker, 0, "All attacker units should be deployed");

        // All units should now be on battlefield
        let on_battlefield = state.units.iter()
            .filter(|u| u.status == wh40k_core_types::UnitStatus::OnBattlefield)
            .count();
        assert_eq!(on_battlefield, state.units.len(), "All units should be on battlefield");
    }

    #[test]
    fn test_full_game_turn_after_deployment() {
        // Deploy all units, then play through several full AI turns.
        // This reproduces production crashes during the AI turn.
        use wh40k_game_core::executor::CommandExecutor;
        let seed = [42u8; 32];
        let mut state = wh40k_game_core::scenario::ScenarioLoader::load_scenario(
            wh40k_core_types::FactionId(0),
            wh40k_core_types::FactionId(1),
            Some(wh40k_core_types::MissionId::new(1)),
            seed,
        );

        // Deploy all units
        for _ in 0..100 {
            if !state.is_in_progress() { break; }
            let owner = state.decision_owner;
            let candidates = ActionGenerator::generate(&state, owner);
            let place_cmd = candidates.candidates.iter()
                .find(|c| matches!(&c.commands[0], Command::PlaceUnit { .. }));
            match place_cmd {
                Some(action) => {
                    CommandExecutor::execute(&mut state, &action.commands[0]).unwrap();
                }
                None => break,
            }
        }

        // All units should be deployed
        let remaining = state.units.iter()
            .filter(|u| u.status == wh40k_core_types::UnitStatus::Undeployed)
            .count();
        assert_eq!(remaining, 0, "All units should be deployed");

        // Now play through many AI decisions. For each decision point,
        // try each candidate until one succeeds (like the real greedy search).
        // This reproduces any panics that happen during AI turns.
        let mut commands_executed = 0;
        let max_commands = 500;

        while state.is_in_progress() && commands_executed < max_commands {
            let player = state.decision_owner;
            let candidates = ActionGenerator::generate(&state, player);

            assert!(!candidates.candidates.is_empty(),
                "No candidates at cmd #{}, phase={:?}, round={:?}, owner={:?}",
                commands_executed, state.current_phase, state.battle_round, state.decision_owner,
            );

            // Try each candidate on a clone until we find one that works
            let mut applied = false;
            for action in &candidates.candidates {
                let mut clone = state.clone();
                let mut ok = true;
                for cmd in &action.commands {
                    if CommandExecutor::execute(&mut clone, cmd).is_err() {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    // Apply to real state
                    for cmd in &action.commands {
                        CommandExecutor::execute(&mut state, cmd).unwrap();
                        commands_executed += 1;
                    }
                    applied = true;
                    break;
                }
            }

            assert!(applied,
                "All {} candidates failed at cmd #{}, phase={:?}, round={:?}, owner={:?}",
                candidates.candidates.len(),
                commands_executed, state.current_phase, state.battle_round, state.decision_owner,
            );
        }

        // Should have played through at least one full round
        assert!(commands_executed > 10,
            "Should have executed many commands, got {}", commands_executed);
    }

    #[test]
    fn test_full_game_multiple_seeds() {
        // Run full games with multiple seeds to catch edge-case crashes.
        use wh40k_game_core::executor::CommandExecutor;

        for seed_byte in [1u8, 7, 13, 42, 99, 137, 200, 255] {
            let seed = [seed_byte; 32];
            for mission in [1, 2, 3] {
                let mut state = wh40k_game_core::scenario::ScenarioLoader::load_scenario(
                    wh40k_core_types::FactionId(0),
                    wh40k_core_types::FactionId(1),
                    Some(wh40k_core_types::MissionId::new(mission)),
                    seed,
                );

                let mut commands_executed = 0;
                let max_commands = 500;

                while state.is_in_progress() && commands_executed < max_commands {
                    let player = state.decision_owner;
                    let candidates = ActionGenerator::generate(&state, player);
                    if candidates.candidates.is_empty() { break; }

                    let mut applied = false;
                    for action in &candidates.candidates {
                        let mut clone = state.clone();
                        let ok = action.commands.iter()
                            .all(|cmd| CommandExecutor::execute(&mut clone, cmd).is_ok());
                        if ok {
                            for cmd in &action.commands {
                                CommandExecutor::execute(&mut state, cmd).unwrap();
                                commands_executed += 1;
                            }
                            applied = true;
                            break;
                        }
                    }
                    if !applied { break; }
                }

                assert!(commands_executed > 5,
                    "seed={}, mission={}: only {} commands",
                    seed_byte, mission, commands_executed);
            }
        }
    }
}
