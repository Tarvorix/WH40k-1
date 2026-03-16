//! Tactical Manoeuvre Logic for Boarding Actions.
//!
//! Implements eligibility checks and effect application for the three core
//! Tactical Manoeuvres: Secure Site, Set to Defend, and Set Overwatch.
//!
//! Source: boarding_actions_complete_v3.md Section 3.4

use wh40k_core_types::{ObjectiveId, PlayerId, TacticalManoeuvre, UnitId};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that prevent a unit from performing a Tactical Manoeuvre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManoeuvreError {
    /// The unit is Battle-shocked and cannot perform any Tactical Manoeuvre.
    BattleShocked,
    /// The unit is within Engagement Range of an enemy unit.
    InEngagement,
    /// The unit Advanced this turn.
    Advanced,
    /// The unit Fell Back this turn.
    FellBack,
    /// The unit was set up on the battlefield this turn.
    SetUpThisTurn,
    /// The unit is not BATTLELINE (for Secure Site, when not overridden).
    NotBattleline,
}

// ---------------------------------------------------------------------------
// Effect result types
// ---------------------------------------------------------------------------

/// Result of applying Secure Site to an objective.
///
/// The objective is not immediately seized. Instead, at the start of the next
/// Command phase, if conditions are still met, the unit Seizes the objective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecureSiteResult {
    /// The objective marker being secured.
    pub objective_id: ObjectiveId,
    /// The player performing the Secure Site action.
    pub player: PlayerId,
    /// The unit performing the action.
    pub unit_id: UnitId,
    /// This resolves at the start of the next Command phase.
    pub resolves_at_next_command_phase: bool,
}

/// Effect of Set to Defend.
///
/// Grants +1 to Hit for melee attacks until start of next Command Phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetToDefendEffect {
    /// The unit receiving the effect.
    pub unit_id: UnitId,
    /// Bonus to melee hit rolls.
    pub bonus_hit: i8,
    /// Duration description for the effect system.
    pub duration: &'static str,
}

/// Effect of Set Overwatch.
///
/// The unit may fire Overwatch until end of opponent's next turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetOverwatchEffect {
    /// The unit receiving the effect.
    pub unit_id: UnitId,
    /// Description of what triggers Overwatch fire.
    pub triggers: &'static str,
    /// Duration description for the effect system.
    pub duration: &'static str,
    /// Only unmodified 6s hit during Overwatch.
    pub hit_on_unmodified_6_only: bool,
    /// Critical hits only on unmodified 6s during Overwatch.
    pub critical_hit_on_6_only: bool,
    /// Cannot fire Overwatch more than once per turn.
    pub max_once_per_turn: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a unit is eligible to perform any Tactical Manoeuvre.
///
/// Rules (Section 3.4):
/// - NOT Battle-shocked
/// - NOT within Engagement Range of an enemy unit
/// - Did NOT Advance this turn
/// - Did NOT Fall Back this turn
/// - Was NOT set up on the battlefield this turn
///
/// A unit performing a Tactical Manoeuvre:
/// - Is not eligible to shoot this turn
/// - Cannot declare a charge this turn
/// (These consequences are enforced by the game state machine, not here.)
pub fn can_perform_manoeuvre(
    _manoeuvre: TacticalManoeuvre,
    unit_is_battle_shocked: bool,
    unit_in_engagement: bool,
    unit_advanced: bool,
    unit_fell_back: bool,
    unit_set_up_this_turn: bool,
) -> Result<(), ManoeuvreError> {
    if unit_is_battle_shocked {
        return Err(ManoeuvreError::BattleShocked);
    }
    if unit_in_engagement {
        return Err(ManoeuvreError::InEngagement);
    }
    if unit_advanced {
        return Err(ManoeuvreError::Advanced);
    }
    if unit_fell_back {
        return Err(ManoeuvreError::FellBack);
    }
    if unit_set_up_this_turn {
        return Err(ManoeuvreError::SetUpThisTurn);
    }
    Ok(())
}

/// Check whether a unit can perform Secure Site.
///
/// Normally BATTLELINE only, but some missions override this to allow any unit.
///
/// Source: boarding_actions_complete_v3.md Section 3.4
/// "BATTLELINE only unless a mission says otherwise."
///
/// `unit_is_battleline`: whether the unit has the BATTLELINE keyword.
/// `mission_allows_any_unit`: whether the mission overrides the BATTLELINE restriction.
pub fn can_secure_site(unit_is_battleline: bool, mission_allows_any_unit: bool) -> bool {
    unit_is_battleline || mission_allows_any_unit
}

/// Apply the Secure Site Tactical Manoeuvre.
///
/// Procedure (Section 3.4):
/// - Select an objective marker the player controls within range of the unit.
/// - At the start of the next Command phase, if the unit:
///   - is not Battle-shocked
///   - is still within range of that objective
///   - and the player still controls that objective
/// - then the unit Seizes the objective marker.
///
/// This function creates the SecureSiteResult describing the pending action.
/// The actual seizure check happens at the start of the next Command phase.
///
/// Parameters:
/// - `objective_id`: the objective being secured.
/// - `player`: the player performing the action.
/// - `unit_id`: the unit performing the action.
/// - `objective_controlled_by_player`: whether the player currently controls this objective.
/// - `unit_not_battle_shocked`: whether the unit is currently not battle-shocked.
/// - `unit_within_range`: whether the unit is within range of the objective.
///
/// Returns `Some(SecureSiteResult)` if the action can be initiated, `None` if
/// preconditions aren't met.
pub fn apply_secure_site(
    objective_id: ObjectiveId,
    player: PlayerId,
    unit_id: UnitId,
    objective_controlled_by_player: bool,
    unit_not_battle_shocked: bool,
    unit_within_range: bool,
) -> Option<SecureSiteResult> {
    // All preconditions must be met to initiate
    if !objective_controlled_by_player || !unit_not_battle_shocked || !unit_within_range {
        return None;
    }

    Some(SecureSiteResult {
        objective_id,
        player,
        unit_id,
        resolves_at_next_command_phase: true,
    })
}

/// Apply the Set to Defend Tactical Manoeuvre.
///
/// Effect (Section 3.4):
/// Until the start of the next Command phase, each melee attack made by the
/// unit gets +1 to Hit.
pub fn apply_set_to_defend(unit_id: UnitId) -> SetToDefendEffect {
    SetToDefendEffect {
        unit_id,
        bonus_hit: 1,
        duration: "Until the start of your next Command phase",
    }
}

/// Apply the Set Overwatch Tactical Manoeuvre.
///
/// Effect (Section 3.4):
/// Until the end of the opponent's next turn, after an enemy unit:
/// - is set up on the battlefield
/// - ends a Normal/Advance/Fall Back move
/// - declares a charge
/// - or opens a Hatchway
///
/// the unit may fire Overwatch at that enemy unit.
///
/// Overwatch rules in BA:
/// - Shoot as if it were your Shooting phase
/// - Only target the triggering enemy unit
/// - Only unmodified 6s hit
/// - Critical hits only on unmodified 6s
/// - Cannot fire Overwatch more than once per turn
pub fn apply_set_overwatch(unit_id: UnitId) -> SetOverwatchEffect {
    SetOverwatchEffect {
        unit_id,
        triggers: "After enemy: sets up, ends Normal/Advance/Fall Back move, declares charge, or opens a Hatchway",
        duration: "Until the end of opponent's next turn",
        hit_on_unmodified_6_only: true,
        critical_hit_on_6_only: true,
        max_once_per_turn: true,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_perform_manoeuvre_eligible() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SecureSite,
            false, false, false, false, false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_perform_manoeuvre_battle_shocked() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SetToDefend,
            true, false, false, false, false,
        );
        assert_eq!(result, Err(ManoeuvreError::BattleShocked));
    }

    #[test]
    fn test_can_perform_manoeuvre_in_engagement() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SetOverwatch,
            false, true, false, false, false,
        );
        assert_eq!(result, Err(ManoeuvreError::InEngagement));
    }

    #[test]
    fn test_can_perform_manoeuvre_advanced() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SecureSite,
            false, false, true, false, false,
        );
        assert_eq!(result, Err(ManoeuvreError::Advanced));
    }

    #[test]
    fn test_can_perform_manoeuvre_fell_back() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SetToDefend,
            false, false, false, true, false,
        );
        assert_eq!(result, Err(ManoeuvreError::FellBack));
    }

    #[test]
    fn test_can_perform_manoeuvre_set_up_this_turn() {
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SetOverwatch,
            false, false, false, false, true,
        );
        assert_eq!(result, Err(ManoeuvreError::SetUpThisTurn));
    }

    #[test]
    fn test_can_perform_manoeuvre_priority_battle_shocked() {
        // Battle-shocked should take priority over other errors
        let result = can_perform_manoeuvre(
            TacticalManoeuvre::SecureSite,
            true, true, true, true, true,
        );
        assert_eq!(result, Err(ManoeuvreError::BattleShocked));
    }

    #[test]
    fn test_can_secure_site_battleline() {
        assert!(can_secure_site(true, false));
    }

    #[test]
    fn test_can_secure_site_not_battleline() {
        assert!(!can_secure_site(false, false));
    }

    #[test]
    fn test_can_secure_site_mission_override() {
        // Mission allows any unit to Secure Site
        assert!(can_secure_site(false, true));
    }

    #[test]
    fn test_can_secure_site_battleline_with_override() {
        assert!(can_secure_site(true, true));
    }

    #[test]
    fn test_apply_secure_site_success() {
        let result = apply_secure_site(
            ObjectiveId::new(0),
            PlayerId::new(0),
            UnitId::new(1),
            true,  // controlled
            true,  // not battle-shocked
            true,  // within range
        );
        assert!(result.is_some());
        let sr = result.unwrap();
        assert_eq!(sr.objective_id, ObjectiveId::new(0));
        assert_eq!(sr.player, PlayerId::new(0));
        assert_eq!(sr.unit_id, UnitId::new(1));
        assert!(sr.resolves_at_next_command_phase);
    }

    #[test]
    fn test_apply_secure_site_not_controlled() {
        let result = apply_secure_site(
            ObjectiveId::new(0),
            PlayerId::new(0),
            UnitId::new(1),
            false, // not controlled
            true,
            true,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_secure_site_battle_shocked() {
        let result = apply_secure_site(
            ObjectiveId::new(0),
            PlayerId::new(0),
            UnitId::new(1),
            true,
            false, // battle-shocked
            true,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_secure_site_not_within_range() {
        let result = apply_secure_site(
            ObjectiveId::new(0),
            PlayerId::new(0),
            UnitId::new(1),
            true,
            true,
            false, // not within range
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_set_to_defend() {
        let effect = apply_set_to_defend(UnitId::new(5));
        assert_eq!(effect.unit_id, UnitId::new(5));
        assert_eq!(effect.bonus_hit, 1);
        assert_eq!(effect.duration, "Until the start of your next Command phase");
    }

    #[test]
    fn test_apply_set_overwatch() {
        let effect = apply_set_overwatch(UnitId::new(3));
        assert_eq!(effect.unit_id, UnitId::new(3));
        assert!(effect.hit_on_unmodified_6_only);
        assert!(effect.critical_hit_on_6_only);
        assert!(effect.max_once_per_turn);
        assert_eq!(effect.duration, "Until the end of opponent's next turn");
    }

    #[test]
    fn test_set_overwatch_triggers_include_hatchway() {
        let effect = apply_set_overwatch(UnitId::new(0));
        assert!(effect.triggers.contains("Hatchway"));
    }

    #[test]
    fn test_all_manoeuvre_types_eligible() {
        // All three manoeuvre types should pass with clean state
        for manoeuvre in [
            TacticalManoeuvre::SecureSite,
            TacticalManoeuvre::SetToDefend,
            TacticalManoeuvre::SetOverwatch,
        ] {
            let result = can_perform_manoeuvre(manoeuvre, false, false, false, false, false);
            assert!(result.is_ok(), "Manoeuvre {:?} should be eligible", manoeuvre);
        }
    }

    #[test]
    fn test_all_manoeuvre_types_blocked_by_battle_shock() {
        // All three types are blocked by battle-shock
        for manoeuvre in [
            TacticalManoeuvre::SecureSite,
            TacticalManoeuvre::SetToDefend,
            TacticalManoeuvre::SetOverwatch,
        ] {
            let result = can_perform_manoeuvre(manoeuvre, true, false, false, false, false);
            assert_eq!(
                result,
                Err(ManoeuvreError::BattleShocked),
                "Manoeuvre {:?} should be blocked by battle-shock",
                manoeuvre
            );
        }
    }
}
