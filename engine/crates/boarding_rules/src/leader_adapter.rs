//! Leader Non-Attachment & Battlefield Command for Boarding Actions.
//!
//! In Boarding Actions, Leaders do not attach to Bodyguard units. Instead,
//! Leader abilities can be projected via the Battlefield Command stratagem
//! to a nearby eligible unit. Abilities that only work while "being led"
//! do nothing.
//!
//! Source: boarding_actions_complete_v3.md Section 3.7

use wh40k_core_types::{Inches, UnitId};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that prevent a Battlefield Command action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlefieldCommandError {
    /// The designated leader unit is not a CHARACTER/LEADER.
    NotLeader,
    /// The target unit is not a valid bodyguard-type unit the leader could normally join.
    InvalidTarget,
    /// The target unit is not within 6" of the leader.
    OutOfRange,
    /// The target unit has already been targeted by Battlefield Command this command phase.
    AlreadyTargeted,
}

// ---------------------------------------------------------------------------
// Request type
// ---------------------------------------------------------------------------

/// A request to use the Battlefield Command stratagem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattlefieldCommandRequest {
    /// The leader unit performing the command.
    pub leader_unit: UnitId,
    /// The bodyguard-type target unit receiving the ability.
    pub target_unit: UnitId,
    /// The name of the Leader ability being conferred.
    pub ability_name: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// In Boarding Actions, leaders do not attach to bodyguard units.
///
/// Source: boarding_actions_complete_v3.md Section 3.7
/// "Leaders do not attach to Bodyguard units at the start of the battle;
///  they remain separate units all game."
///
/// Always returns `true` in BA mode.
pub fn leaders_do_not_attach() -> bool {
    true
}

/// Validate a Battlefield Command stratagem request.
///
/// Rules (Section 3.7 / Section 4.2):
/// - Leader must be a CHARACTER/LEADER (checked via `leader_is_character`).
/// - Target must be a bodyguard-type unit the leader could normally join
///   (checked via `target_in_can_lead_list`).
/// - Target must be within 6" of the leader (checked via `distance`).
/// - Target must not already have been targeted by Battlefield Command this
///   command phase (checked via `already_targeted_this_phase`).
/// - Costs 1 CP (validated elsewhere by the stratagem system).
///
/// Parameters:
/// - `leader_is_character`: true if the leader unit has the CHARACTER keyword.
/// - `target_in_can_lead_list`: true if the target unit is one the leader
///    could normally attach to (based on the leader's "can lead" list).
/// - `distance`: the distance in Inches between the leader and target unit.
/// - `already_targeted_this_phase`: true if the target unit has already been
///    the target of a Battlefield Command this command phase.
pub fn validate_battlefield_command(
    _request: &BattlefieldCommandRequest,
    leader_is_character: bool,
    target_in_can_lead_list: bool,
    distance: Inches,
    already_targeted_this_phase: bool,
) -> Result<(), BattlefieldCommandError> {
    // Leader must be a CHARACTER
    if !leader_is_character {
        return Err(BattlefieldCommandError::NotLeader);
    }

    // Target must be a unit the leader could normally join
    if !target_in_can_lead_list {
        return Err(BattlefieldCommandError::InvalidTarget);
    }

    // Target must be within 6" (BA_BATTLEFIELD_COMMAND_RANGE)
    if distance > Inches::BA_BATTLEFIELD_COMMAND_RANGE {
        return Err(BattlefieldCommandError::OutOfRange);
    }

    // Target must not already have been targeted this phase
    if already_targeted_this_phase {
        return Err(BattlefieldCommandError::AlreadyTargeted);
    }

    Ok(())
}

/// In Boarding Actions, abilities that only work while a unit is "being led"
/// do nothing.
///
/// Source: boarding_actions_complete_v3.md Section 3.7
/// "Abilities that only work while a unit is being led do nothing in Boarding
///  Actions, even if Battlefield Command is used."
///
/// Always returns `true` to indicate these abilities are disabled.
pub fn led_by_abilities_disabled() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> BattlefieldCommandRequest {
        BattlefieldCommandRequest {
            leader_unit: UnitId::new(0),
            target_unit: UnitId::new(1),
            ability_name: "Inspiring Leader".to_string(),
        }
    }

    #[test]
    fn test_leaders_do_not_attach() {
        assert!(leaders_do_not_attach());
    }

    #[test]
    fn test_led_by_abilities_disabled() {
        assert!(led_by_abilities_disabled());
    }

    #[test]
    fn test_validate_battlefield_command_success() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,  // is character
            true,  // in can_lead list
            Inches::from_inches(4), // within 6"
            false, // not already targeted
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_battlefield_command_not_leader() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            false, // not a character
            true,
            Inches::from_inches(4),
            false,
        );
        assert_eq!(result, Err(BattlefieldCommandError::NotLeader));
    }

    #[test]
    fn test_validate_battlefield_command_invalid_target() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,
            false, // not in can_lead list
            Inches::from_inches(4),
            false,
        );
        assert_eq!(result, Err(BattlefieldCommandError::InvalidTarget));
    }

    #[test]
    fn test_validate_battlefield_command_out_of_range() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,
            true,
            Inches::from_inches(8), // > 6"
            false,
        );
        assert_eq!(result, Err(BattlefieldCommandError::OutOfRange));
    }

    #[test]
    fn test_validate_battlefield_command_at_6_inches() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,
            true,
            Inches::from_inches(6), // exactly 6" — should be valid (within range)
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_battlefield_command_already_targeted() {
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,
            true,
            Inches::from_inches(4),
            true, // already targeted
        );
        assert_eq!(result, Err(BattlefieldCommandError::AlreadyTargeted));
    }

    #[test]
    fn test_validate_checks_order_leader_first() {
        // Verify that NotLeader takes priority over other errors
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            false, // not leader
            false, // also invalid target
            Inches::from_inches(20), // also out of range
            true,  // also already targeted
        );
        assert_eq!(result, Err(BattlefieldCommandError::NotLeader));
    }

    #[test]
    fn test_validate_checks_order_target_second() {
        // Leader is valid, but target is invalid
        let req = make_request();
        let result = validate_battlefield_command(
            &req,
            true,
            false, // invalid target
            Inches::from_inches(20), // also out of range
            true,  // also already targeted
        );
        assert_eq!(result, Err(BattlefieldCommandError::InvalidTarget));
    }
}
