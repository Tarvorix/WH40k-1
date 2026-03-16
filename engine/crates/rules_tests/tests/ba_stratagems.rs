//! Source-linked rules tests for boarding_actions_complete_v3.md Section 4:
//! Boarding Actions Universal Stratagems.
//!
//! Tests the 5 universal BA stratagems: Command Re-roll, Battlefield Command,
//! Counter-Offensive, Insane Bravery, and Explosive Clearance.
//!
//! Validates:
//! - Stratagem counts, IDs, names, CP costs, timings
//! - Availability gating by GameMode (BoardingActions vs CombatPatrol)
//! - BA-specific stratagems (Battlefield Command, Explosive Clearance)
//! - Core-like stratagems (Command Re-roll, Counter-Offensive, Insane Bravery)
//! - Enhancement name constants
//!
//! Source: boarding_actions_complete_v3.md Section 4

use wh40k_core_types::GameMode;
use wh40k_boarding_rules::stratagems;

// ===========================================================================
// Test 1: Exactly 5 universal BA stratagems exist
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// "5 universal stratagems replace the normal core stratagem set."
#[test]
fn ba_stratagems_count_is_five() {
    let stratagems = stratagems::get_universal_ba_stratagems();
    assert_eq!(
        stratagems.len(),
        5,
        "There should be exactly 5 universal Boarding Actions stratagems"
    );
}

// ===========================================================================
// Test 2: Command Re-roll — same as core, 1CP
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Command Re-roll (1CP) — mirrors the core Command Re-roll stratagem.
/// Usable in any phase after a qualifying roll.
#[test]
fn ba_command_reroll_properties() {
    let strat = stratagems::get_stratagem_by_id("ba_command_reroll")
        .expect("ba_command_reroll should exist");

    assert_eq!(strat.name, "Command Re-roll");
    assert_eq!(strat.cp_cost, 1, "Command Re-roll costs 1CP");
    assert!(
        strat.timing.contains("Any phase"),
        "Command Re-roll is usable in any phase, got: {}",
        strat.timing
    );
    assert!(
        strat.description.contains("Re-roll"),
        "Description should mention re-rolling, got: {}",
        strat.description
    );
}

// ===========================================================================
// Test 3: Battlefield Command — BA-specific, projects leader ability 6"
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Battlefield Command (1CP) — BA-specific stratagem. Projects a Leader ability
/// to a Bodyguard unit within 6". Used at start or end of any phase.
#[test]
fn ba_battlefield_command_is_ba_specific() {
    let strat = stratagems::get_stratagem_by_id("ba_battlefield_command")
        .expect("ba_battlefield_command should exist");

    assert_eq!(strat.name, "Battlefield Command");
    assert_eq!(strat.cp_cost, 1, "Battlefield Command costs 1CP");
    assert!(
        strat.timing.contains("Start or end of any phase"),
        "Timing should be start or end of any phase, got: {}",
        strat.timing
    );
    assert!(
        strat.description.contains("Leader"),
        "Description should reference Leader unit"
    );
    assert!(
        strat.description.contains("Bodyguard"),
        "Description should reference Bodyguard unit"
    );
    assert!(
        strat.description.contains("6\""),
        "Description should reference 6\" range for projecting leader ability"
    );
}

// ===========================================================================
// Test 4: Counter-Offensive — same as core, 2CP
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Counter-Offensive (2CP) — same as core. Fight phase, after enemy fought.
#[test]
fn ba_counter_offensive_costs_2cp() {
    let strat = stratagems::get_stratagem_by_id("ba_counter_offensive")
        .expect("ba_counter_offensive should exist");

    assert_eq!(strat.name, "Counter-Offensive");
    assert_eq!(strat.cp_cost, 2, "Counter-Offensive costs 2CP");
    assert!(
        strat.timing.contains("Fight phase"),
        "Counter-Offensive is usable in Fight phase, got: {}",
        strat.timing
    );
    assert!(
        strat.description.contains("Engagement Range"),
        "Description should reference Engagement Range"
    );
}

// ===========================================================================
// Test 5: Insane Bravery — same as core, 1CP
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Insane Bravery (1CP) — same as core. Auto-pass a failed Battle-shock test.
#[test]
fn ba_insane_bravery_auto_passes_battleshock() {
    let strat = stratagems::get_stratagem_by_id("ba_insane_bravery")
        .expect("ba_insane_bravery should exist");

    assert_eq!(strat.name, "Insane Bravery");
    assert_eq!(strat.cp_cost, 1, "Insane Bravery costs 1CP");
    assert!(
        strat.timing.contains("Battle-shock"),
        "Timing should reference Battle-shock step, got: {}",
        strat.timing
    );
    assert!(
        strat.description.contains("passed"),
        "Description should mention treating the test as passed"
    );
}

// ===========================================================================
// Test 6: Explosive Clearance — BA-specific, Blast + invisible targeting
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Explosive Clearance (1CP) — BA-specific. Blast counts invisible models,
/// and attacks can be allocated to invisible targets.
#[test]
fn ba_explosive_clearance_invisible_targeting() {
    let strat = stratagems::get_stratagem_by_id("ba_explosive_clearance")
        .expect("ba_explosive_clearance should exist");

    assert_eq!(strat.name, "Explosive Clearance");
    assert_eq!(strat.cp_cost, 1, "Explosive Clearance costs 1CP");
    assert!(
        strat.timing.contains("Shooting phase"),
        "Explosive Clearance is usable in Shooting phase, got: {}",
        strat.timing
    );
    assert!(
        strat.description.contains("Blast"),
        "Description should reference Blast weapons"
    );
    assert!(
        strat.description.contains("NOT visible"),
        "Description should reference models NOT visible to the attacker"
    );
}

// ===========================================================================
// Test 7: BA stratagems are only available in BoardingActions mode
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// "You cannot use normal core/Codex stratagem sets here."
/// BA stratagems are gated to GameMode::BoardingActions.
#[test]
fn ba_stratagems_gated_to_boarding_actions_mode() {
    let all_ids = [
        "ba_command_reroll",
        "ba_battlefield_command",
        "ba_counter_offensive",
        "ba_insane_bravery",
        "ba_explosive_clearance",
    ];

    // All should be available in BoardingActions mode
    for id in &all_ids {
        assert!(
            stratagems::is_ba_stratagem_available(id, GameMode::BoardingActions),
            "Stratagem '{}' should be available in BoardingActions mode",
            id
        );
    }

    // None should be available in CombatPatrol mode
    for id in &all_ids {
        assert!(
            !stratagems::is_ba_stratagem_available(id, GameMode::CombatPatrol),
            "Stratagem '{}' should NOT be available in CombatPatrol mode",
            id
        );
    }

    // Unknown stratagems should never be available
    assert!(
        !stratagems::is_ba_stratagem_available("core_command_reroll", GameMode::BoardingActions),
        "Core stratagems should not be available in BA mode"
    );
    assert!(
        !stratagems::is_ba_stratagem_available("nonexistent", GameMode::BoardingActions),
        "Nonexistent stratagems should not be available"
    );
}

// ===========================================================================
// Test 8: All stratagem IDs are unique and all names are distinct
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 4
/// Each stratagem must have a unique ID and a unique name.
#[test]
fn ba_stratagem_ids_and_names_are_unique() {
    let stratagems = stratagems::get_universal_ba_stratagems();

    // Check ID uniqueness
    let ids: Vec<&str> = stratagems.iter().map(|s| s.id).collect();
    for (i, id) in ids.iter().enumerate() {
        for (j, other) in ids.iter().enumerate() {
            if i != j {
                assert_ne!(id, other, "Duplicate stratagem ID found: {}", id);
            }
        }
    }

    // Check name uniqueness
    let names: Vec<&str> = stratagems.iter().map(|s| s.name).collect();
    for (i, name) in names.iter().enumerate() {
        for (j, other) in names.iter().enumerate() {
            if i != j {
                assert_ne!(name, other, "Duplicate stratagem name found: {}", name);
            }
        }
    }

    // Verify all 5 expected names are present
    let expected_names = [
        "Command Re-roll",
        "Battlefield Command",
        "Counter-Offensive",
        "Insane Bravery",
        "Explosive Clearance",
    ];
    for expected in &expected_names {
        assert!(
            names.contains(expected),
            "Expected stratagem '{}' not found in {:?}",
            expected,
            names
        );
    }
}
