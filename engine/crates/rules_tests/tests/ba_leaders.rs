//! Source-linked rules tests for boarding_actions_complete_v3.md Section 3.7:
//! Leader Non-Attachment & Battlefield Command in Boarding Actions.
//!
//! Tests cover:
//!   - Leaders do NOT attach in BA (attached_leader should be None)
//!   - Leader abilities projected via Battlefield Command stratagem
//!     (6" range = BA_BATTLEFIELD_COMMAND_RANGE)
//!   - Led-by abilities don't function in BA
//!
//! Source: boarding_actions_complete_v3.md Section 3.7

use wh40k_boarding_rules::leader_adapter::{
    led_by_abilities_disabled, leaders_do_not_attach, validate_battlefield_command,
    BattlefieldCommandError, BattlefieldCommandRequest,
};
use wh40k_core_types::{Inches, UnitId};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a standard BattlefieldCommandRequest for testing.
fn make_command_request() -> BattlefieldCommandRequest {
    BattlefieldCommandRequest {
        leader_unit: UnitId::new(0),
        target_unit: UnitId::new(1),
        ability_name: "Inspiring Leader".to_string(),
    }
}

// ===========================================================================
// Section 3.7 -- Leaders do NOT attach in Boarding Actions
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Leaders do not attach to Bodyguard units at the start of the battle;
///        they remain separate units all game."
/// Test: leaders_do_not_attach() returns true in BA mode, indicating that
/// no leader should have an attached_leader reference.
#[test]
fn leaders_do_not_attach_in_boarding_actions() {
    assert!(
        leaders_do_not_attach(),
        "Leaders should NOT attach to bodyguard units in Boarding Actions"
    );
}

// ===========================================================================
// Section 3.7 -- Leader abilities projected via Battlefield Command (6" range)
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.7 / Section 4.2
/// Rule: "Battlefield Command: 1CP, Command phase. Select a CHARACTER leader
///        within 6\" of an eligible bodyguard unit. That unit gains one of the
///        leader's abilities."
/// Test: validate_battlefield_command succeeds when all conditions are met:
/// leader is a CHARACTER, target is in the leader's can-lead list, within 6",
/// and not already targeted this phase.
#[test]
fn battlefield_command_valid_within_six_inches() {
    let req = make_command_request();
    let result = validate_battlefield_command(
        &req,
        true,                      // leader is CHARACTER
        true,                      // target in can_lead list
        Inches::from_inches(4),    // within 6"
        false,                     // not already targeted
    );
    assert!(
        result.is_ok(),
        "Battlefield Command should succeed with valid inputs within 6\""
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Leader must be a CHARACTER/LEADER."
/// Test: validate_battlefield_command fails when the leader is not a CHARACTER.
#[test]
fn battlefield_command_fails_if_not_character() {
    let req = make_command_request();
    let result = validate_battlefield_command(
        &req,
        false, // NOT a character
        true,
        Inches::from_inches(4),
        false,
    );
    assert_eq!(
        result,
        Err(BattlefieldCommandError::NotLeader),
        "Battlefield Command should fail when leader is not a CHARACTER"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Target must be a bodyguard-type unit the leader could normally join."
/// Test: validate_battlefield_command fails when the target is not in the
/// leader's can-lead list.
#[test]
fn battlefield_command_fails_if_invalid_target() {
    let req = make_command_request();
    let result = validate_battlefield_command(
        &req,
        true,
        false, // target NOT in can_lead list
        Inches::from_inches(4),
        false,
    );
    assert_eq!(
        result,
        Err(BattlefieldCommandError::InvalidTarget),
        "Battlefield Command should fail when target is not in the leader's can-lead list"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Target must be within 6\" of the leader."
/// BA_BATTLEFIELD_COMMAND_RANGE = 6 inches.
/// Test: validate_battlefield_command fails when the target is beyond 6".
#[test]
fn battlefield_command_fails_if_out_of_range() {
    let req = make_command_request();
    let result = validate_battlefield_command(
        &req,
        true,
        true,
        Inches::from_inches(8), // > 6"
        false,
    );
    assert_eq!(
        result,
        Err(BattlefieldCommandError::OutOfRange),
        "Battlefield Command should fail when target is beyond 6\" (BA_BATTLEFIELD_COMMAND_RANGE)"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Target must be within 6\"" — exactly 6" is within range.
/// Test: validate_battlefield_command succeeds at exactly 6".
#[test]
fn battlefield_command_succeeds_at_exactly_six_inches() {
    let req = make_command_request();
    let result = validate_battlefield_command(
        &req,
        true,
        true,
        Inches::from_inches(6), // exactly 6" — should be valid
        false,
    );
    assert!(
        result.is_ok(),
        "Battlefield Command at exactly 6\" (BA_BATTLEFIELD_COMMAND_RANGE) should succeed"
    );

    // Also verify the constant itself
    assert_eq!(
        Inches::BA_BATTLEFIELD_COMMAND_RANGE,
        Inches::from_inches(6),
        "BA_BATTLEFIELD_COMMAND_RANGE should be 6 inches"
    );
}

// ===========================================================================
// Section 3.7 -- Led-by abilities don't function in BA
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.7
/// Rule: "Abilities that only work while a unit is being led do nothing in
///        Boarding Actions, even if Battlefield Command is used."
/// Test: led_by_abilities_disabled() returns true, indicating that any ability
/// gated on "while this unit is being led" should not activate.
#[test]
fn led_by_abilities_are_disabled_in_ba() {
    assert!(
        led_by_abilities_disabled(),
        "Led-by abilities should be disabled in Boarding Actions"
    );
}
