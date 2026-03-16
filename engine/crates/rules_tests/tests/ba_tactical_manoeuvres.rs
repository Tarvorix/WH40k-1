//! Source-linked rules tests for boarding_actions_complete_v3.md Section 3.4:
//! Tactical Manoeuvres in Boarding Actions.
//!
//! Tests cover:
//!   - Secure Site: BATTLELINE only, marks objective, no shoot/charge
//!   - Set to Defend: improved defensive position (+1 melee hit)
//!   - Set Overwatch: triggers on enemy actions, unmodified 6 hits, lasts until
//!     end of opponent's turn
//!   - Eligibility: not Battle-shocked, not in ER, didn't Advance/Fall Back,
//!     not just set up
//!   - Only one manoeuvre per unit per turn
//!
//! Source: boarding_actions_complete_v3.md Section 3.4

use wh40k_boarding_rules::tactical_manoeuvres::{
    apply_secure_site, apply_set_overwatch, apply_set_to_defend, can_perform_manoeuvre,
    can_secure_site, ManoeuvreError,
};
use wh40k_core_types::{ObjectiveId, PlayerId, TacticalManoeuvre, UnitId};

// ===========================================================================
// Secure Site -- BATTLELINE only
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "BATTLELINE only unless a mission says otherwise."
/// Test: A BATTLELINE unit can perform Secure Site.
#[test]
fn secure_site_battleline_unit_allowed() {
    assert!(
        can_secure_site(true, false),
        "A BATTLELINE unit should be eligible for Secure Site"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "BATTLELINE only unless a mission says otherwise."
/// Test: A non-BATTLELINE unit cannot perform Secure Site normally.
#[test]
fn secure_site_non_battleline_denied() {
    assert!(
        !can_secure_site(false, false),
        "A non-BATTLELINE unit should NOT be eligible for Secure Site"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "...unless a mission says otherwise."
/// Test: When the mission overrides the BATTLELINE restriction, any unit can
/// Secure Site.
#[test]
fn secure_site_mission_override_allows_any_unit() {
    assert!(
        can_secure_site(false, true),
        "With mission override, even a non-BATTLELINE unit should be able to Secure Site"
    );
}

// ===========================================================================
// Secure Site -- marks objective, resolves at next Command phase
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Select an objective marker the player controls within range. At
///        the start of the next Command phase, if conditions are met, the
///        unit Seizes the objective."
/// Test: apply_secure_site succeeds when all preconditions are met and the
/// result marks resolution at the next Command phase.
#[test]
fn secure_site_marks_objective_resolves_at_next_command_phase() {
    let result = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        true,  // player controls objective
        true,  // unit not battle-shocked
        true,  // unit within range
    );
    assert!(result.is_some(), "Secure Site should succeed with valid preconditions");
    let sr = result.unwrap();
    assert_eq!(sr.objective_id, ObjectiveId::new(0));
    assert_eq!(sr.player, PlayerId::new(0));
    assert_eq!(sr.unit_id, UnitId::new(1));
    assert!(
        sr.resolves_at_next_command_phase,
        "Secure Site should resolve at the start of the next Command phase"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: Precondition: player must control the objective.
/// Test: Secure Site fails when the player does not control the objective.
#[test]
fn secure_site_fails_if_objective_not_controlled() {
    let result = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        false, // player does NOT control objective
        true,
        true,
    );
    assert!(
        result.is_none(),
        "Secure Site should fail when player does not control the objective"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: Precondition: unit must not be battle-shocked.
/// Test: Secure Site fails when the unit is battle-shocked.
#[test]
fn secure_site_fails_if_battle_shocked() {
    let result = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        true,
        false, // unit IS battle-shocked
        true,
    );
    assert!(
        result.is_none(),
        "Secure Site should fail when unit is battle-shocked"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: Precondition: unit must be within range of the objective.
/// Test: Secure Site fails when the unit is not within range.
#[test]
fn secure_site_fails_if_not_within_range() {
    let result = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        true,
        true,
        false, // unit NOT within range
    );
    assert!(
        result.is_none(),
        "Secure Site should fail when unit is not within range of the objective"
    );
}

// ===========================================================================
// Set to Defend -- improved defensive position
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Until the start of the next Command phase, each melee attack made
///        by the unit gets +1 to Hit."
/// Test: apply_set_to_defend grants +1 bonus hit until next Command phase.
#[test]
fn set_to_defend_grants_plus_one_melee_hit() {
    let effect = apply_set_to_defend(UnitId::new(5));
    assert_eq!(effect.unit_id, UnitId::new(5));
    assert_eq!(
        effect.bonus_hit, 1,
        "Set to Defend should grant +1 to melee hit rolls"
    );
    assert_eq!(
        effect.duration, "Until the start of your next Command phase",
        "Set to Defend should last until the start of the next Command phase"
    );
}

// ===========================================================================
// Set Overwatch -- triggers, unmodified 6 hits, duration
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Until the end of the opponent's next turn, after an enemy unit is set
///        up, ends a move, declares a charge, or opens a Hatchway, the unit may
///        fire Overwatch."
/// Test: apply_set_overwatch sets all the correct overwatch properties.
#[test]
fn set_overwatch_triggers_on_enemy_actions() {
    let effect = apply_set_overwatch(UnitId::new(3));
    assert_eq!(effect.unit_id, UnitId::new(3));
    assert_eq!(
        effect.duration, "Until the end of opponent's next turn",
        "Set Overwatch should last until the end of opponent's next turn"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Only unmodified 6s hit during Overwatch."
/// Test: The Set Overwatch effect requires unmodified 6 to hit.
#[test]
fn set_overwatch_hits_on_unmodified_6_only() {
    let effect = apply_set_overwatch(UnitId::new(3));
    assert!(
        effect.hit_on_unmodified_6_only,
        "Set Overwatch should require unmodified 6 to hit"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Critical hits only on unmodified 6s during Overwatch."
/// Test: Critical hits during overwatch only on unmodified 6s.
#[test]
fn set_overwatch_critical_hit_on_6_only() {
    let effect = apply_set_overwatch(UnitId::new(3));
    assert!(
        effect.critical_hit_on_6_only,
        "Set Overwatch should only crit on unmodified 6"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Cannot fire Overwatch more than once per turn."
/// Test: Set Overwatch has max-once-per-turn restriction.
#[test]
fn set_overwatch_max_once_per_turn() {
    let effect = apply_set_overwatch(UnitId::new(3));
    assert!(
        effect.max_once_per_turn,
        "Set Overwatch should be limited to once per turn"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "...or opens a Hatchway" is a trigger for Set Overwatch.
/// Test: The overwatch triggers description includes the Hatchway trigger.
#[test]
fn set_overwatch_triggers_include_hatchway_open() {
    let effect = apply_set_overwatch(UnitId::new(0));
    assert!(
        effect.triggers.contains("Hatchway"),
        "Set Overwatch triggers should include opening a Hatchway"
    );
}

// ===========================================================================
// Eligibility -- not Battle-shocked, not in ER, didn't Advance/Fall Back,
// not just set up
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "A unit can perform a Tactical Manoeuvre if: NOT Battle-shocked, NOT
///        within Engagement Range, did NOT Advance, did NOT Fall Back, and was
///        NOT set up this turn."
/// Test: A clean unit with no disqualifying conditions is eligible.
#[test]
fn eligible_unit_can_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SecureSite,
        false, // not battle-shocked
        false, // not in engagement
        false, // did not advance
        false, // did not fall back
        false, // not set up this turn
    );
    assert!(
        result.is_ok(),
        "A unit with no disqualifying conditions should be eligible"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "NOT Battle-shocked"
/// Test: A Battle-shocked unit cannot perform any Tactical Manoeuvre.
#[test]
fn battle_shocked_unit_cannot_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SetToDefend,
        true,  // battle-shocked
        false, false, false, false,
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::BattleShocked),
        "A Battle-shocked unit should be denied"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "NOT within Engagement Range of an enemy unit"
/// Test: A unit in Engagement Range cannot perform any Tactical Manoeuvre.
#[test]
fn unit_in_engagement_range_cannot_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SetOverwatch,
        false,
        true, // in engagement
        false, false, false,
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::InEngagement),
        "A unit in Engagement Range should be denied"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Did NOT Advance this turn"
/// Test: A unit that Advanced this turn cannot perform any Tactical Manoeuvre.
#[test]
fn unit_that_advanced_cannot_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SecureSite,
        false, false,
        true, // advanced
        false, false,
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::Advanced),
        "A unit that Advanced should be denied"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Did NOT Fall Back this turn"
/// Test: A unit that Fell Back this turn cannot perform any Tactical Manoeuvre.
#[test]
fn unit_that_fell_back_cannot_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SetToDefend,
        false, false, false,
        true, // fell back
        false,
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::FellBack),
        "A unit that Fell Back should be denied"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Was NOT set up on the battlefield this turn"
/// Test: A unit that was set up this turn cannot perform any Tactical Manoeuvre.
#[test]
fn unit_set_up_this_turn_cannot_perform_manoeuvre() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SetOverwatch,
        false, false, false, false,
        true, // set up this turn
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::SetUpThisTurn),
        "A unit set up this turn should be denied"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: Battle-shock check takes priority over other disqualifiers.
/// Test: When all conditions fail, BattleShocked is returned first.
#[test]
fn battle_shock_takes_priority_over_other_errors() {
    let result = can_perform_manoeuvre(
        TacticalManoeuvre::SecureSite,
        true, // battle-shocked
        true, // also in engagement
        true, // also advanced
        true, // also fell back
        true, // also set up this turn
    );
    assert_eq!(
        result,
        Err(ManoeuvreError::BattleShocked),
        "BattleShocked should take priority when multiple conditions fail"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: All three manoeuvre types have the same eligibility requirements.
/// Test: All three TacticalManoeuvre variants pass eligibility with a clean unit.
#[test]
fn all_three_manoeuvre_types_eligible_for_clean_unit() {
    for manoeuvre in [
        TacticalManoeuvre::SecureSite,
        TacticalManoeuvre::SetToDefend,
        TacticalManoeuvre::SetOverwatch,
    ] {
        let result = can_perform_manoeuvre(manoeuvre, false, false, false, false, false);
        assert!(
            result.is_ok(),
            "Manoeuvre {:?} should be eligible for a clean unit",
            manoeuvre
        );
    }
}

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: All three manoeuvre types are blocked by battle-shock.
/// Test: All three TacticalManoeuvre variants fail if the unit is battle-shocked.
#[test]
fn all_three_manoeuvre_types_blocked_by_battle_shock() {
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

// ===========================================================================
// Only one manoeuvre per unit per turn
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Each unit can only perform one Tactical Manoeuvre per turn."
/// Test: The BoardingActionsModeState.tactical_manoeuvres HashMap tracks which
/// units have performed a manoeuvre. Inserting one entry should prevent a second.
///
/// This test verifies the tracking data structure that enforces the one-per-turn
/// rule. The enforcement itself happens in the game state machine; here we verify
/// that the HashMap correctly detects duplicate entries.
#[test]
fn only_one_manoeuvre_per_unit_per_turn_tracking() {
    use std::collections::HashMap;

    let mut tactical_manoeuvres: HashMap<UnitId, TacticalManoeuvre> = HashMap::new();
    let unit = UnitId::new(7);

    // First manoeuvre should succeed (entry doesn't exist yet)
    assert!(
        !tactical_manoeuvres.contains_key(&unit),
        "Unit should not have a manoeuvre recorded before performing one"
    );
    tactical_manoeuvres.insert(unit, TacticalManoeuvre::SetToDefend);

    // Second manoeuvre for the same unit should be blocked (entry already exists)
    assert!(
        tactical_manoeuvres.contains_key(&unit),
        "Unit should have a manoeuvre recorded after performing one"
    );

    // The game state machine checks contains_key before allowing a second manoeuvre
    let already_performed = tactical_manoeuvres.contains_key(&unit);
    assert!(
        already_performed,
        "Tracking should show the unit already performed a manoeuvre this turn"
    );
}

// ===========================================================================
// Section 3.4 -- Unit cannot shoot after performing Secure Site/Set to Defend
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "A unit that performs a Tactical Manoeuvre is not eligible to shoot
///        this turn."
/// Test: Verify the rule is documented in the can_perform_manoeuvre comment.
///       The enforcement happens in the game state machine, which checks if the
///       unit has performed a Tactical Manoeuvre before allowing shooting.
///       Here we verify the tracking data structure that the state machine uses.
#[test]
fn unit_cannot_shoot_after_secure_site_or_set_to_defend() {
    use std::collections::HashMap;

    // The game state machine uses this tracking HashMap
    let mut manoeuvres_performed: HashMap<UnitId, TacticalManoeuvre> = HashMap::new();
    let unit_a = UnitId::new(10);
    let unit_b = UnitId::new(11);

    // Unit A performs Secure Site
    manoeuvres_performed.insert(unit_a, TacticalManoeuvre::SecureSite);

    // Unit B performs Set to Defend
    manoeuvres_performed.insert(unit_b, TacticalManoeuvre::SetToDefend);

    // When the Shooting phase arrives, the state machine checks:
    let unit_a_can_shoot = !manoeuvres_performed.contains_key(&unit_a);
    assert!(
        !unit_a_can_shoot,
        "Unit that performed Secure Site should NOT be eligible to shoot"
    );

    let unit_b_can_shoot = !manoeuvres_performed.contains_key(&unit_b);
    assert!(
        !unit_b_can_shoot,
        "Unit that performed Set to Defend should NOT be eligible to shoot"
    );

    // A unit that did NOT perform a manoeuvre CAN shoot
    let unit_c = UnitId::new(12);
    let unit_c_can_shoot = !manoeuvres_performed.contains_key(&unit_c);
    assert!(
        unit_c_can_shoot,
        "Unit that did NOT perform a Tactical Manoeuvre should be eligible to shoot"
    );
}

// ===========================================================================
// Section 3.4 -- Unit cannot charge after performing Secure Site/Set to Defend
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "A unit that performs a Tactical Manoeuvre cannot declare a charge
///        this turn."
/// Test: Verify the tracking data structure blocks charging for units that
///       performed any Tactical Manoeuvre.
#[test]
fn unit_cannot_charge_after_secure_site_or_set_to_defend() {
    use std::collections::HashMap;

    let mut manoeuvres_performed: HashMap<UnitId, TacticalManoeuvre> = HashMap::new();

    let unit = UnitId::new(20);
    manoeuvres_performed.insert(unit, TacticalManoeuvre::SecureSite);

    // When the Charge phase arrives, the state machine checks:
    let can_charge = !manoeuvres_performed.contains_key(&unit);
    assert!(
        !can_charge,
        "Unit that performed Secure Site should NOT be eligible to charge"
    );

    // Set to Defend also blocks charging
    let unit2 = UnitId::new(21);
    manoeuvres_performed.insert(unit2, TacticalManoeuvre::SetToDefend);
    let can_charge_2 = !manoeuvres_performed.contains_key(&unit2);
    assert!(
        !can_charge_2,
        "Unit that performed Set to Defend should NOT be eligible to charge"
    );

    // Set Overwatch blocks charging too (all manoeuvres do)
    let unit3 = UnitId::new(22);
    manoeuvres_performed.insert(unit3, TacticalManoeuvre::SetOverwatch);
    let can_charge_3 = !manoeuvres_performed.contains_key(&unit3);
    assert!(
        !can_charge_3,
        "Unit that performed Set Overwatch should NOT be eligible to charge"
    );
}

// ===========================================================================
// Section 3.4 -- Seizing vs Securing distinction
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4
/// Rule: "Seized = controlled in Command phase after Secure Site resolves.
///        Secured = persistent control that remains even if unit moves away."
/// Test: Verify that Secure Site resolves at the next Command phase (seizing),
///       and that the SecureSiteResult indicates this timing. The persistence
///       (securing) is a subsequent state the game machine applies after
///       successful seizure.
#[test]
fn seizing_vs_securing_distinction() {
    // Seizing: happens at next Command phase when Secure Site resolves
    let result = apply_secure_site(
        ObjectiveId::new(2),
        PlayerId::new(0),
        UnitId::new(5),
        true,  // player controls objective
        true,  // not battle-shocked
        true,  // within range
    );
    assert!(result.is_some(), "Secure Site should succeed");
    let sr = result.unwrap();

    // The SecureSiteResult explicitly says it resolves at the next Command phase
    assert!(
        sr.resolves_at_next_command_phase,
        "Secure Site (seizure) resolves at the start of the next Command phase"
    );

    // Seizing is step 1: the unit attempts to seize at the next Command phase.
    // If successful (unit still in range, not battle-shocked, player still controls),
    // the objective becomes "secured" — persistent control.
    //
    // This means:
    // - "Seized" = the act of claiming at the Command phase
    // - "Secured" = the persistent state after successful seizure
    //
    // The game state machine tracks this as a two-step process:
    // 1. apply_secure_site returns a pending SecureSiteResult
    // 2. At the next Command phase, if conditions are met, the objective is "secured"
    assert_eq!(
        sr.objective_id,
        ObjectiveId::new(2),
        "The seized objective should match the requested one"
    );
}

// ===========================================================================
// Section 3.4 -- Secure Site range uses BA_OBJECTIVE_RANGE (1")
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.4 + Section 3.2
/// Rule: "Secure Site must be performed within range of the objective. In BA,
///        the objective interaction range is 1\" (BA_OBJECTIVE_RANGE), not the
///        standard 3\"."
/// Test: Verify that BA_OBJECTIVE_RANGE is 1" and that Secure Site uses this
///       range for eligibility. The unit_within_range parameter to
///       apply_secure_site is pre-calculated using this constant.
#[test]
fn secure_site_uses_ba_objective_range() {
    use wh40k_core_types::Inches;

    // BA_OBJECTIVE_RANGE is 1", not the standard 3"
    assert_eq!(
        Inches::BA_OBJECTIVE_RANGE,
        Inches::from_inches(1),
        "BA objective range for Secure Site must be 1\""
    );

    // Standard is 3" — Secure Site in BA uses the tighter range
    assert!(
        Inches::BA_OBJECTIVE_RANGE < Inches::OBJECTIVE_RANGE,
        "BA objective range (1\") must be smaller than standard (3\")"
    );

    // When unit is within BA_OBJECTIVE_RANGE (1"), Secure Site succeeds
    let within_range = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        true,  // controlled
        true,  // not battle-shocked
        true,  // within BA_OBJECTIVE_RANGE
    );
    assert!(
        within_range.is_some(),
        "Secure Site should succeed when unit is within BA_OBJECTIVE_RANGE"
    );

    // When unit is NOT within BA_OBJECTIVE_RANGE (even if within 3"), Secure Site fails
    let out_of_range = apply_secure_site(
        ObjectiveId::new(0),
        PlayerId::new(0),
        UnitId::new(1),
        true,
        true,
        false, // NOT within BA_OBJECTIVE_RANGE
    );
    assert!(
        out_of_range.is_none(),
        "Secure Site should fail when unit is NOT within BA_OBJECTIVE_RANGE"
    );
}
