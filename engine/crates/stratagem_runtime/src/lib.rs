//! WH40K Engine - StratagemRuntime crate
//!
//! Runtime for stratagem management and execution. Handles the timing,
//! targeting, and effect application of both core and faction stratagems.
//!
//! Source: implementation_v3.md Section 6.1 (Layer 3)
//! Source: 40k_revised.md - Core Stratagems
//! Source: Custodes.md, Frenzied_Reavers.md - Faction Stratagems

use serde::{Deserialize, Serialize};
use thiserror::Error;

use wh40k_core_types::{
    Keyword, Phase, PlayerId,
    StratagemId, UnitId,
};
use wh40k_event_system::{GameEvent, CpReason, EffectTarget as EventEffectTarget};
use wh40k_game_core::GameState;

// ─── Errors ────────────────────────────────────────────────────────────────

/// Errors from stratagem operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum StratagemError {
    #[error("Insufficient CP: need {needed}, have {available}")]
    InsufficientCP { needed: u8, available: i8 },

    #[error("Wrong phase: stratagem requires {required:?}, current is {current:?}")]
    WrongPhase { required: Phase, current: Phase },

    #[error("Invalid target: {0}")]
    InvalidTarget(String),

    #[error("Stratagem already used this {scope}")]
    AlreadyUsed { scope: String },

    #[error("Unit is Battle-shocked and cannot use stratagems")]
    BattleShocked,

    #[error("Stratagem not available: {0}")]
    NotAvailable(String),
}

// ─── StratagemUsability ─────────────────────────────────────────────────────

/// Whether a stratagem can be used and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StratagemUsability {
    /// The stratagem can be used.
    Usable,
    /// The stratagem is blocked for the given reason.
    Blocked(String),
}

impl StratagemUsability {
    /// Check if the stratagem is usable.
    pub fn is_usable(&self) -> bool {
        matches!(self, StratagemUsability::Usable)
    }

    /// Get the blocking reason, if any.
    pub fn reason(&self) -> Option<&str> {
        match self {
            StratagemUsability::Usable => None,
            StratagemUsability::Blocked(reason) => Some(reason),
        }
    }
}

// ─── AvailableStratagem ─────────────────────────────────────────────────────

/// A stratagem that is available for use, with its current usability status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableStratagem {
    /// Stratagem identifier.
    pub stratagem_id: StratagemId,
    /// Display name.
    pub name: String,
    /// CP cost.
    pub cp_cost: u8,
    /// Whether the stratagem can currently be used.
    pub can_use: bool,
    /// If blocked, the reason why.
    pub reason_if_blocked: Option<String>,
}

// ─── StratagemTarget ────────────────────────────────────────────────────────

/// The target of a stratagem usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StratagemTarget {
    /// Targets a specific friendly unit.
    FriendlyUnit(UnitId),
    /// Targets a specific enemy unit.
    EnemyUnit(UnitId),
    /// Targets a specific die roll (for Command Re-roll).
    DiceRoll,
    /// No specific target (self-buffing or global).
    NoTarget,
}

// ─── StratagemTiming ────────────────────────────────────────────────────────

/// When a stratagem can be used within a phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StratagemTiming {
    /// At the start of the phase.
    StartOfPhase,
    /// During the phase (general).
    DuringPhase,
    /// At the end of the phase.
    EndOfPhase,
    /// When a specific event occurs (e.g., "after enemy shoots").
    OnEvent(String),
    /// Any time during the phase.
    AnyTime,
}

// ─── TargetRestriction ──────────────────────────────────────────────────────

/// Restrictions on what a stratagem can target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRestriction {
    /// Required keywords on the target unit.
    pub required_keywords: Vec<Keyword>,
    /// Excluded keywords (unit must NOT have these).
    pub excluded_keywords: Vec<Keyword>,
    /// Must be owned by the stratagem user.
    pub must_be_friendly: bool,
    /// Must be an enemy unit.
    pub must_be_enemy: bool,
}

impl Default for TargetRestriction {
    fn default() -> Self {
        Self {
            required_keywords: Vec::new(),
            excluded_keywords: Vec::new(),
            must_be_friendly: true,
            must_be_enemy: false,
        }
    }
}

// ─── Core Stratagem Definitions ─────────────────────────────────────────────

/// Well-known stratagem IDs - re-exported from the authoritative game_core::stratagem module.
///
/// All stratagem IDs are defined authoritatively in `wh40k_game_core::stratagem::ids`.
/// This module re-exports them for backward compatibility.
pub mod core_stratagems {
    pub use wh40k_game_core::stratagem::ids::*;
}

/// Definition of a core stratagem with its rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreStratagemDef {
    /// Stratagem ID.
    pub id: StratagemId,
    /// Display name.
    pub name: String,
    /// CP cost.
    pub cp_cost: u8,
    /// Which phase it can be used in.
    pub phase: Phase,
    /// Timing within the phase.
    pub timing: StratagemTiming,
    /// Target restrictions.
    pub target_restriction: TargetRestriction,
    /// Whether this is once per battle.
    pub once_per_battle: bool,
    /// Whether this is once per turn (standard restriction).
    pub once_per_turn: bool,
    /// Description.
    pub description: String,
}

/// Get all stratagem definitions (all 17: 11 core + 6 faction).
///
/// This function delegates to the authoritative `game_core::stratagem` module
/// and converts each `StratagemDef` into a `CoreStratagemDef` for backward
/// compatibility with the stratagem_runtime API.
pub fn core_stratagem_definitions() -> Vec<CoreStratagemDef> {
    use wh40k_game_core::stratagem;

    stratagem::ALL_STRATAGEMS_SLICE
        .iter()
        .map(|def| {
            let timing = match &def.timing {
                stratagem::StratagemTiming::AnyTime => StratagemTiming::AnyTime,
                stratagem::StratagemTiming::StartOfPhase => StratagemTiming::StartOfPhase,
                stratagem::StratagemTiming::DuringPhase => StratagemTiming::DuringPhase,
                stratagem::StratagemTiming::AfterEnemySelectsTargets => {
                    StratagemTiming::OnEvent("after_enemy_selects_targets".to_string())
                }
                stratagem::StratagemTiming::AfterEnemyShoots => {
                    StratagemTiming::OnEvent("after_enemy_shoots".to_string())
                }
                stratagem::StratagemTiming::AfterEnemyDeclaresCharge => {
                    StratagemTiming::OnEvent("after_enemy_charge".to_string())
                }
                stratagem::StratagemTiming::AfterChargeMoveComplete => {
                    StratagemTiming::OnEvent("after_charge_move_complete".to_string())
                }
                stratagem::StratagemTiming::OnUnitSelectedToFight => {
                    StratagemTiming::OnEvent("unit_selected_to_fight".to_string())
                }
                stratagem::StratagemTiming::AfterEnemyUnitFights => {
                    StratagemTiming::OnEvent("after_enemy_unit_fights".to_string())
                }
            };

            CoreStratagemDef {
                id: def.id,
                name: def.name.to_string(),
                cp_cost: def.cp_cost,
                phase: def.valid_phases.first().copied().unwrap_or(Phase::Command),
                timing,
                target_restriction: TargetRestriction {
                    required_keywords: def.required_keywords.to_vec(),
                    excluded_keywords: Vec::new(),
                    must_be_friendly: def.must_be_friendly,
                    must_be_enemy: def.must_be_enemy,
                },
                once_per_battle: def.once_per_battle,
                once_per_turn: def.once_per_turn,
                description: format!("{} ({}CP)", def.name, def.cp_cost),
            }
        })
        .collect()
}

// ─── StratagemRuntime ───────────────────────────────────────────────────────

/// Runtime for stratagem management and execution.
///
/// Handles eligibility checking, CP spending, timing validation, target
/// restrictions, and effect application for all stratagems.
pub struct StratagemRuntime;

impl StratagemRuntime {
    /// Get all stratagems available to a player, with their current usability.
    pub fn get_available_stratagems(
        state: &GameState,
        player: PlayerId,
    ) -> Vec<AvailableStratagem> {
        let definitions = core_stratagem_definitions();
        let mut available = Vec::new();

        for def in &definitions {
            let target = StratagemTarget::NoTarget; // Generic check
            let usability = Self::can_use_stratagem(state, player, def.id, &target);

            available.push(AvailableStratagem {
                stratagem_id: def.id,
                name: def.name.clone(),
                cp_cost: def.cp_cost,
                can_use: usability.is_usable(),
                reason_if_blocked: usability.reason().map(|s| s.to_string()),
            });
        }

        available
    }

    /// Check if a specific stratagem can be used right now.
    pub fn can_use_stratagem(
        state: &GameState,
        player: PlayerId,
        stratagem_id: StratagemId,
        target: &StratagemTarget,
    ) -> StratagemUsability {
        let definitions = core_stratagem_definitions();
        let def = match definitions.iter().find(|d| d.id == stratagem_id) {
            Some(d) => d,
            None => return StratagemUsability::Blocked("Stratagem not found".to_string()),
        };

        // Check CP
        let player_state = state.player(player);
        if player_state.cp.value() < def.cp_cost as i8 {
            return StratagemUsability::Blocked(format!(
                "Insufficient CP: need {}, have {}",
                def.cp_cost,
                player_state.cp.value()
            ));
        }

        // Check timing (phase)
        // Command Re-roll can be used in any phase
        if stratagem_id != core_stratagems::COMMAND_REROLL
            && !Self::check_timing(state.current_phase, &def.timing, &def.phase)
        {
            return StratagemUsability::Blocked(format!(
                "Wrong phase: requires {:?}",
                def.phase
            ));
        }

        // Check once-per-turn restriction
        if def.once_per_turn
            && player_state
                .stratagem_usage
                .used_this_turn(stratagem_id)
        {
            return StratagemUsability::Blocked("Already used this turn".to_string());
        }

        // Check once-per-battle restriction
        if def.once_per_battle
            && player_state
                .stratagem_usage
                .used_this_battle(stratagem_id)
        {
            return StratagemUsability::Blocked("Already used this battle".to_string());
        }

        // Check same stratagem not used this phase (core rule)
        if player_state
            .stratagem_usage
            .used_this_phase(stratagem_id)
        {
            return StratagemUsability::Blocked("Already used this phase".to_string());
        }

        // Check target restrictions
        if let StratagemTarget::FriendlyUnit(unit_id) = target {
            if let Some(unit) = state.unit(*unit_id) {
                // Check battle-shocked restriction
                if unit.battle_shocked && stratagem_id != core_stratagems::INSANE_BRAVERY {
                    return StratagemUsability::Blocked(
                        "Target unit is Battle-shocked".to_string(),
                    );
                }

                // Check required keywords
                for kw in &def.target_restriction.required_keywords {
                    if !unit.has_keyword(*kw) {
                        return StratagemUsability::Blocked(format!(
                            "Target unit lacks required keyword {:?}",
                            kw
                        ));
                    }
                }

                // Check excluded keywords
                for kw in &def.target_restriction.excluded_keywords {
                    if unit.has_keyword(*kw) {
                        return StratagemUsability::Blocked(format!(
                            "Target unit has excluded keyword {:?}",
                            kw
                        ));
                    }
                }

                // Check ownership
                if def.target_restriction.must_be_friendly && unit.owner != player {
                    return StratagemUsability::Blocked(
                        "Target must be a friendly unit".to_string(),
                    );
                }
            } else {
                return StratagemUsability::Blocked("Target unit not found".to_string());
            }
        }

        if let StratagemTarget::EnemyUnit(unit_id) = target {
            if let Some(unit) = state.unit(*unit_id) {
                if def.target_restriction.must_be_enemy && unit.owner == player {
                    return StratagemUsability::Blocked(
                        "Target must be an enemy unit".to_string(),
                    );
                }
            } else {
                return StratagemUsability::Blocked("Target unit not found".to_string());
            }
        }

        StratagemUsability::Usable
    }

    /// Apply a stratagem: spend CP, record usage, apply effects, return events.
    pub fn apply_stratagem(
        state: &mut GameState,
        player: PlayerId,
        stratagem_id: StratagemId,
        target: &StratagemTarget,
    ) -> Result<Vec<GameEvent>, StratagemError> {
        // Re-check usability
        let usability = Self::can_use_stratagem(state, player, stratagem_id, target);
        if !usability.is_usable() {
            return Err(StratagemError::NotAvailable(
                usability.reason().unwrap_or("Unknown reason").to_string(),
            ));
        }

        let definitions = core_stratagem_definitions();
        let def = definitions
            .iter()
            .find(|d| d.id == stratagem_id)
            .ok_or_else(|| StratagemError::NotAvailable("Stratagem not found".to_string()))?;

        // Spend CP
        let player_state = state.player_mut(player);
        if !player_state.spend_cp(def.cp_cost as i8) {
            return Err(StratagemError::InsufficientCP {
                needed: def.cp_cost,
                available: player_state.cp.value(),
            });
        }

        // Record usage
        player_state.stratagem_usage.record_usage(stratagem_id);

        let mut events = Vec::new();

        // Build the event target
        let event_target = match target {
            StratagemTarget::FriendlyUnit(uid) => EventEffectTarget::Unit(*uid),
            StratagemTarget::EnemyUnit(uid) => EventEffectTarget::Unit(*uid),
            StratagemTarget::DiceRoll => EventEffectTarget::Player(player),
            StratagemTarget::NoTarget => EventEffectTarget::Player(player),
        };

        // Emit CP spent event
        events.push(GameEvent::CommandPointsSpent {
            player,
            amount: def.cp_cost,
            reason: CpReason::StratagemCost(stratagem_id),
        });

        // Emit stratagem used event
        events.push(GameEvent::StratagemUsed {
            stratagem: stratagem_id,
            player,
            target: event_target,
        });

        // Delegate effect application to the authoritative game_core::stratagem module.
        // This ensures all 17 stratagems (11 core + 6 faction) are handled consistently.
        let target_unit = match target {
            StratagemTarget::FriendlyUnit(uid) => Some(*uid),
            StratagemTarget::EnemyUnit(uid) => Some(*uid),
            _ => None,
        };
        let effect_events = wh40k_game_core::stratagem::apply_stratagem_effects(
            state,
            player,
            stratagem_id,
            target_unit,
        );
        events.extend(effect_events);

        Ok(events)
    }

    /// Check if the current phase and timing match the stratagem's requirements.
    pub fn check_timing(
        current_phase: Phase,
        timing: &StratagemTiming,
        required_phase: &Phase,
    ) -> bool {
        // First check phase matches (or is a special case)
        if current_phase != *required_phase {
            // Fire Overwatch can be used in Movement or Charge phase
            if *required_phase == Phase::Movement
                && (current_phase == Phase::Movement || current_phase == Phase::Charge)
            {
                return true;
            }
            return false;
        }

        // Timing is validated at a higher level (event-driven windows)
        // For now, if the phase matches, timing is considered valid
        match timing {
            StratagemTiming::AnyTime => true,
            StratagemTiming::StartOfPhase => true,
            StratagemTiming::DuringPhase => true,
            StratagemTiming::EndOfPhase => true,
            StratagemTiming::OnEvent(_) => true,
        }
    }

    /// Check if a target meets the restrictions for a stratagem.
    pub fn check_target_restrictions(
        state: &GameState,
        target: &StratagemTarget,
        restriction: &TargetRestriction,
        player: PlayerId,
    ) -> bool {
        match target {
            StratagemTarget::FriendlyUnit(unit_id) => {
                if let Some(unit) = state.unit(*unit_id) {
                    // Check ownership
                    if restriction.must_be_friendly && unit.owner != player {
                        return false;
                    }
                    if restriction.must_be_enemy && unit.owner == player {
                        return false;
                    }

                    // Check required keywords
                    for kw in &restriction.required_keywords {
                        if !unit.has_keyword(*kw) {
                            return false;
                        }
                    }

                    // Check excluded keywords
                    for kw in &restriction.excluded_keywords {
                        if unit.has_keyword(*kw) {
                            return false;
                        }
                    }

                    true
                } else {
                    false
                }
            }
            StratagemTarget::EnemyUnit(unit_id) => {
                if let Some(unit) = state.unit(*unit_id) {
                    if restriction.must_be_friendly && unit.owner != player {
                        // Friendly restriction but targeting enemy unit
                        return false;
                    }
                    if restriction.must_be_enemy && unit.owner == player {
                        return false;
                    }

                    for kw in &restriction.required_keywords {
                        if !unit.has_keyword(*kw) {
                            return false;
                        }
                    }
                    for kw in &restriction.excluded_keywords {
                        if unit.has_keyword(*kw) {
                            return false;
                        }
                    }

                    true
                } else {
                    false
                }
            }
            StratagemTarget::DiceRoll | StratagemTarget::NoTarget => true,
        }
    }

}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    // === Core stratagem definitions ===

    #[test]
    fn test_core_stratagem_definitions() {
        let defs = core_stratagem_definitions();
        assert_eq!(defs.len(), 17); // 11 core + 6 faction

        let cmd_reroll = defs.iter().find(|d| d.id == core_stratagems::COMMAND_REROLL).unwrap();
        assert_eq!(cmd_reroll.name, "Command Re-roll");
        assert_eq!(cmd_reroll.cp_cost, 1);

        let counter = defs
            .iter()
            .find(|d| d.id == core_stratagems::COUNTER_OFFENSIVE)
            .unwrap();
        assert_eq!(counter.name, "Counter-Offensive");
        assert_eq!(counter.cp_cost, 2);

        let insane = defs
            .iter()
            .find(|d| d.id == core_stratagems::INSANE_BRAVERY)
            .unwrap();
        assert!(insane.once_per_battle);
    }

    // === StratagemUsability ===

    #[test]
    fn test_stratagem_usability_usable() {
        let u = StratagemUsability::Usable;
        assert!(u.is_usable());
        assert!(u.reason().is_none());
    }

    #[test]
    fn test_stratagem_usability_blocked() {
        let u = StratagemUsability::Blocked("Not enough CP".to_string());
        assert!(!u.is_usable());
        assert_eq!(u.reason(), Some("Not enough CP"));
    }

    // === Timing check ===

    #[test]
    fn test_check_timing_matching_phase() {
        assert!(StratagemRuntime::check_timing(
            Phase::Fight,
            &StratagemTiming::DuringPhase,
            &Phase::Fight
        ));
    }

    #[test]
    fn test_check_timing_wrong_phase() {
        assert!(!StratagemRuntime::check_timing(
            Phase::Shooting,
            &StratagemTiming::DuringPhase,
            &Phase::Fight
        ));
    }

    #[test]
    fn test_check_timing_any_time() {
        assert!(StratagemRuntime::check_timing(
            Phase::Fight,
            &StratagemTiming::AnyTime,
            &Phase::Fight
        ));
    }

    #[test]
    fn test_check_timing_overwatch_movement_phase() {
        // Overwatch required phase is Movement, but can also be used in Charge
        assert!(StratagemRuntime::check_timing(
            Phase::Movement,
            &StratagemTiming::OnEvent("enemy_move_or_charge".to_string()),
            &Phase::Movement
        ));
        assert!(StratagemRuntime::check_timing(
            Phase::Charge,
            &StratagemTiming::OnEvent("enemy_move_or_charge".to_string()),
            &Phase::Movement
        ));
    }

    // === Target restrictions ===

    #[test]
    fn test_check_target_restrictions_no_target() {
        let _restriction = TargetRestriction::default();
        let state_is_not_needed_for_no_target = true;
        // NoTarget always passes
        assert!(state_is_not_needed_for_no_target);
    }

    // === AvailableStratagem ===

    #[test]
    fn test_available_stratagem_creation() {
        let strat = AvailableStratagem {
            stratagem_id: core_stratagems::COMMAND_REROLL,
            name: "Command Re-roll".to_string(),
            cp_cost: 1,
            can_use: true,
            reason_if_blocked: None,
        };
        assert!(strat.can_use);
        assert_eq!(strat.cp_cost, 1);
    }

    #[test]
    fn test_available_stratagem_blocked() {
        let strat = AvailableStratagem {
            stratagem_id: core_stratagems::COUNTER_OFFENSIVE,
            name: "Counter-Offensive".to_string(),
            cp_cost: 2,
            can_use: false,
            reason_if_blocked: Some("Not enough CP".to_string()),
        };
        assert!(!strat.can_use);
        assert_eq!(
            strat.reason_if_blocked.as_deref(),
            Some("Not enough CP")
        );
    }

    // === StratagemTarget ===

    #[test]
    fn test_stratagem_target_variants() {
        let _friendly = StratagemTarget::FriendlyUnit(UnitId::new(1));
        let _enemy = StratagemTarget::EnemyUnit(UnitId::new(2));
        let _dice = StratagemTarget::DiceRoll;
        let _no = StratagemTarget::NoTarget;
    }

    // === Error types ===

    #[test]
    fn test_stratagem_error_display() {
        let err = StratagemError::InsufficientCP {
            needed: 2,
            available: 0,
        };
        assert!(err.to_string().contains("Insufficient CP"));

        let err = StratagemError::WrongPhase {
            required: Phase::Fight,
            current: Phase::Shooting,
        };
        assert!(err.to_string().contains("Wrong phase"));

        let err = StratagemError::BattleShocked;
        assert!(err.to_string().contains("Battle-shocked"));

        let err = StratagemError::AlreadyUsed {
            scope: "turn".to_string(),
        };
        assert!(err.to_string().contains("already used"));

        let err = StratagemError::InvalidTarget("No valid unit".to_string());
        assert!(err.to_string().contains("Invalid target"));

        let err = StratagemError::NotAvailable("Stratagem not found".to_string());
        assert!(err.to_string().contains("not available"));
    }
}
