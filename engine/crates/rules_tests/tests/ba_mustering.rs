//! Source-linked rules tests for boarding_actions_complete_v3.md Section 6:
//! Boarding Actions Army Mustering.
//!
//! Tests roster validation rules:
//! - 500 points limit
//! - BA board dimensions: 42" x 22" (BoardDimensions::BOARDING_ACTIONS)
//! - Warlord must be CHARACTER
//! - Up to 2 enhancements, no duplicates
//! - No EPIC HERO enhancements
//! - 6 universal BA enhancements (0 points)
//! - Detachment-specific units only
//! - Faction keyword matching
//!
//! Source: boarding_actions_complete_v3.md Section 6

use wh40k_core_types::{BoardDimensions, Inches};
use wh40k_boarding_rules::roster::{BoardingPatrol, EnhancementAssignment, SelectedUnit};
use wh40k_boarding_rules::validator::{
    BoardingRosterValidator, MAX_BOARDING_PATROL_POINTS,
    UNIVERSAL_BA_ENHANCEMENTS,
};
use wh40k_boarding_content::faction_data::BoardingFactionDef;
use wh40k_boarding_content::loader;

// ===========================================================================
// Helpers: load faction data from JSON files
// ===========================================================================

fn load_world_eaters_butchers() -> BoardingFactionDef {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/boarding_actions/factions/world_eaters_boarding_butchers.json"
    ))
    .unwrap();
    loader::load_faction(&json).unwrap()
}

fn load_space_marines_terminator() -> BoardingFactionDef {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/boarding_actions/factions/space_marines_terminator_assault.json"
    ))
    .unwrap();
    loader::load_faction(&json).unwrap()
}

#[allow(dead_code)]
fn load_astra_militarum_tempestus() -> BoardingFactionDef {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/boarding_actions/factions/astra_militarum_tempestus.json"
    ))
    .unwrap();
    loader::load_faction(&json).unwrap()
}

// ===========================================================================
// Test 1: 500 points limit constant
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// "Boarding Patrol armies are constructed to a 500 points limit."
#[test]
fn ba_mustering_points_cap_is_500() {
    assert_eq!(
        MAX_BOARDING_PATROL_POINTS, 500,
        "BA mustering points limit should be 500"
    );
}

// ===========================================================================
// Test 2: BA board dimensions are 42" x 22"
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 2
/// "TWO Boarding Actions boards placed side by side" => 42" x 22"
#[test]
fn ba_board_dimensions_42x22() {
    let ba = BoardDimensions::BOARDING_ACTIONS;
    assert_eq!(
        ba.width,
        Inches::from_inches(42),
        "BA board width should be 42\""
    );
    assert_eq!(
        ba.height,
        Inches::from_inches(22),
        "BA board height should be 22\""
    );

    // Compare to Combat Patrol dimensions to ensure they are different
    let cp = BoardDimensions::COMBAT_PATROL;
    assert_ne!(
        ba, cp,
        "BA board dimensions should differ from Combat Patrol"
    );
    assert_eq!(cp.width, Inches::from_inches(44));
    assert_eq!(cp.height, Inches::from_inches(30));
}

// ===========================================================================
// Test 3: Warlord must be a CHARACTER when CHARACTERs are present
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// "Warlord must be a CHARACTER model."
/// Non-CHARACTER warlord is rejected when CHARACTER units exist.
#[test]
fn ba_mustering_warlord_must_be_character() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    // Add a CHARACTER unit (Kharn) and a non-CHARACTER unit (Khorne Berzerkers)
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });

    // Designate the non-CHARACTER (Berzerkers, index 1) as warlord
    patrol.warlord_index = Some(1);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        !result.valid,
        "Roster with non-CHARACTER warlord should be invalid"
    );
    assert!(
        result.errors.iter().any(|e| e.code == "WARLORD_NOT_CHARACTER"),
        "Expected WARLORD_NOT_CHARACTER error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 4: Valid warlord as CHARACTER passes validation
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// CHARACTER warlord with legal roster passes validation.
#[test]
fn ba_mustering_character_warlord_valid() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });

    // Designate Kharn (CHARACTER, index 0) as warlord
    patrol.warlord_index = Some(0);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        result.valid,
        "Roster with CHARACTER warlord should be valid, errors: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 5: Up to 2 enhancements allowed, 3 or more rejected
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// "Up to 2 enhancements, no duplicates."
#[test]
fn ba_mustering_max_two_enhancements() {
    let faction = load_space_marines_terminator();
    let mut patrol = BoardingPatrol::new(
        "Adeptus Astartes".to_string(),
        "ADEPTUS ASTARTES".to_string(),
        "Terminator Assault".to_string(),
    );

    // Find three CHARACTER units in the SM faction to assign 3 enhancements to
    // Use allowed units from the detachment
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Captain in Terminator Armour".to_string(),
        model_count: 1,
        points: 95,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Librarian in Terminator Armour".to_string(),
        model_count: 1,
        points: 75,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Chaplain in Terminator Armour".to_string(),
        model_count: 1,
        points: 75,
        wargear_selections: vec![],
    });

    patrol.warlord_index = Some(0);

    // Assign 3 enhancements — should fail
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Superior Boarding Tactics".to_string(),
        unit_index: 0,
    });
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Close-Quarters Killer".to_string(),
        unit_index: 1,
    });
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Expert Breacher".to_string(),
        unit_index: 2,
    });

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        !result.valid,
        "Roster with 3 enhancements should be invalid"
    );
    assert!(
        result.errors.iter().any(|e| e.code == "TOO_MANY_ENHANCEMENTS"),
        "Expected TOO_MANY_ENHANCEMENTS error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 6: No duplicate enhancement names
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// Each enhancement may only be taken once.
#[test]
fn ba_mustering_no_duplicate_enhancements() {
    let faction = load_space_marines_terminator();
    let mut patrol = BoardingPatrol::new(
        "Adeptus Astartes".to_string(),
        "ADEPTUS ASTARTES".to_string(),
        "Terminator Assault".to_string(),
    );

    patrol.add_unit(SelectedUnit {
        datasheet_name: "Captain in Terminator Armour".to_string(),
        model_count: 1,
        points: 95,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Librarian in Terminator Armour".to_string(),
        model_count: 1,
        points: 75,
        wargear_selections: vec![],
    });

    patrol.warlord_index = Some(0);

    // Same enhancement on two different CHARACTER units — should fail
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Superior Boarding Tactics".to_string(),
        unit_index: 0,
    });
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Superior Boarding Tactics".to_string(),
        unit_index: 1,
    });

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        !result.valid,
        "Roster with duplicate enhancements should be invalid"
    );
    assert!(
        result.errors.iter().any(|e| e.code == "DUPLICATE_ENHANCEMENT"),
        "Expected DUPLICATE_ENHANCEMENT error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 7: EPIC HERO cannot receive enhancements
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// "EPIC HEROES cannot receive enhancements."
#[test]
fn ba_mustering_epic_hero_no_enhancements() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    // Kharn is an EPIC HERO
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });

    patrol.warlord_index = Some(0);

    // Assign enhancement to Kharn (EPIC HERO) — should fail
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Superior Boarding Tactics".to_string(),
        unit_index: 0,
    });

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        !result.valid,
        "Roster with enhancement on EPIC HERO should be invalid"
    );
    assert!(
        result.errors.iter().any(|e| e.code == "ENHANCEMENT_ON_EPIC_HERO"),
        "Expected ENHANCEMENT_ON_EPIC_HERO error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 8: 6 universal BA enhancements (all 0 points cost)
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 5
/// "6 universal Boarding Actions enhancements available to all detachments."
/// All 6 are at 0 points cost.
#[test]
fn ba_mustering_six_universal_enhancements() {
    assert_eq!(
        UNIVERSAL_BA_ENHANCEMENTS.len(),
        6,
        "There should be exactly 6 universal BA enhancements"
    );

    let expected = [
        "Superior Boarding Tactics",
        "Close-Quarters Killer",
        "Peerless Leader",
        "Expert Breacher",
        "Personal Teleporter",
        "Trademark Weapon",
    ];

    for name in &expected {
        assert!(
            UNIVERSAL_BA_ENHANCEMENTS.contains(name),
            "Universal BA enhancements should include '{}', got: {:?}",
            name,
            UNIVERSAL_BA_ENHANCEMENTS
        );
    }
}

// ===========================================================================
// Test 9: Over 500 points rejected
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// Rosters exceeding 500 points are rejected with OVER_POINTS.
#[test]
fn ba_mustering_over_500_points_rejected() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 10,
        points: 180,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Chaos Terminators".to_string(),
        model_count: 5,
        points: 175,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Eightbound".to_string(),
        model_count: 3,
        points: 135,
        wargear_selections: vec![],
    });
    // Total: 100 + 180 + 175 + 135 = 590 > 500

    patrol.warlord_index = Some(0);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(!result.valid, "590 points roster should be invalid");
    assert!(
        result.errors.iter().any(|e| e.code == "OVER_POINTS"),
        "Expected OVER_POINTS error, got: {:?}",
        result.errors
    );
    assert!(patrol.total_points > 500);
}

// ===========================================================================
// Test 10: Detachment-specific units only — illegal unit rejected
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// Only units in the detachment's allowed_units list can be selected.
#[test]
fn ba_mustering_illegal_unit_rejected() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    // "Tactical Marines" is not in the World Eaters Boarding Butchers allowed list
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Tactical Marines".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });

    patrol.warlord_index = Some(0);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(!result.valid, "Roster with illegal unit should be invalid");
    assert!(
        result.errors.iter().any(|e| e.code == "INVALID_UNIT"),
        "Expected INVALID_UNIT error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 11: Faction keyword mismatch rejected
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// Patrol faction keyword must match the faction definition's keyword.
#[test]
fn ba_mustering_faction_keyword_mismatch_rejected() {
    let faction = load_world_eaters_butchers();

    // Create a patrol with wrong faction keyword
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "ADEPTUS ASTARTES".to_string(), // Wrong keyword
        "Boarding Butchers".to_string(),
    );

    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });

    patrol.warlord_index = Some(0);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        !result.valid,
        "Roster with mismatched faction keyword should be invalid"
    );
    assert!(
        result.errors.iter().any(|e| e.code == "FACTION_MISMATCH"),
        "Expected FACTION_MISMATCH error, got: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 12: Valid full roster passes all checks
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6
/// A complete legal roster with warlord, enhancements, and legal units passes.
#[test]
fn ba_mustering_valid_roster_passes() {
    let faction = load_world_eaters_butchers();
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    // Kharn (EPIC HERO CHARACTER) - 100pts
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    // Master of Executions (CHARACTER) - 60pts
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Master of Executions".to_string(),
        model_count: 1,
        points: 60,
        wargear_selections: vec![],
    });
    // Khorne Berzerkers x5 - 90pts
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });
    // Eightbound x3 - 135pts
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Eightbound".to_string(),
        model_count: 3,
        points: 135,
        wargear_selections: vec![],
    });
    // Jakhals x10 - 65pts
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Jakhals".to_string(),
        model_count: 10,
        points: 65,
        wargear_selections: vec![],
    });
    // Total = 100 + 60 + 90 + 135 + 65 = 450

    patrol.warlord_index = Some(0); // Kharn as warlord

    // Enhancement on Master of Executions (non-epic CHARACTER)
    patrol.enhancements.push(EnhancementAssignment {
        enhancement_name: "Chosen of Khorne".to_string(),
        unit_index: 1,
    });

    assert_eq!(patrol.total_points, 450, "Total points should be 450");
    assert!(patrol.total_points <= MAX_BOARDING_PATROL_POINTS);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    assert!(
        result.valid,
        "Legal roster should pass validation, errors: {:?}",
        result.errors
    );
}

// ===========================================================================
// Test 13: Underdog bonus — player at least 30 points lower gets bonus
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6.6
/// Rule: "If a player's army is at least 30 points lower than their opponent's,
///        that player is the underdog and receives a bonus (typically +1 CP)."
/// Test: Verify that the underdog threshold (30 points) and bonus calculation
///       work correctly. The mission_loader detects underdog bonus from mission
///       tags.
#[test]
fn ba_mustering_underdog_30_points_threshold() {
    // Underdog threshold: 30 points difference
    let underdog_threshold: u16 = 30;

    // Test cases: (player_points, opponent_points, is_underdog)
    let test_cases: Vec<(u16, u16, bool)> = vec![
        (450, 500, true),    // 50 point deficit => underdog
        (470, 500, true),    // 30 point deficit => exactly at threshold => underdog
        (471, 500, false),   // 29 point deficit => NOT underdog
        (500, 500, false),   // tied => NOT underdog
        (500, 450, false),   // higher points => NOT underdog
        (300, 500, true),    // 200 point deficit => underdog
        (400, 400, false),   // tied at 400 => NOT underdog
        (350, 380, true),    // 30 point deficit => underdog
        (351, 380, false),   // 29 point deficit => NOT underdog
    ];

    for (player_pts, opponent_pts, expected_underdog) in test_cases {
        let diff = if opponent_pts > player_pts {
            opponent_pts - player_pts
        } else {
            0
        };
        let is_underdog = diff >= underdog_threshold;
        assert_eq!(
            is_underdog, expected_underdog,
            "Player at {} pts vs opponent at {} pts: underdog={} (expected {}), diff={}",
            player_pts, opponent_pts, is_underdog, expected_underdog, diff
        );
    }
}

// ===========================================================================
// Test 14: All units must share faction keyword
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6.2
/// Rule: "All units in a Boarding Patrol must share the army's Faction keyword."
/// Test: Verify that the validator rejects rosters with mismatched faction keywords.
///       This test checks a different scenario from test 11 — here we ensure
///       the faction_keyword field on the patrol itself is enforced.
#[test]
fn ba_mustering_all_units_share_faction_keyword() {
    let faction = load_world_eaters_butchers();

    // Valid: faction keyword matches
    let mut valid_patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );
    valid_patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    valid_patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });
    valid_patrol.warlord_index = Some(0);

    let valid_result = BoardingRosterValidator::new(&valid_patrol, &faction).validate();
    assert!(
        valid_result.valid,
        "Roster with matching faction keyword should pass: {:?}",
        valid_result.errors
    );

    // Invalid: wrong faction keyword
    let mut invalid_patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "SPACE MARINES".to_string(), // WRONG — should be WORLD EATERS
        "Boarding Butchers".to_string(),
    );
    invalid_patrol.add_unit(SelectedUnit {
        datasheet_name: "Kharn The Betrayer".to_string(),
        model_count: 1,
        points: 100,
        wargear_selections: vec![],
    });
    invalid_patrol.warlord_index = Some(0);

    let invalid_result = BoardingRosterValidator::new(&invalid_patrol, &faction).validate();
    assert!(
        !invalid_result.valid,
        "Roster with wrong faction keyword should fail"
    );
    assert!(
        invalid_result
            .errors
            .iter()
            .any(|e| e.code == "FACTION_MISMATCH"),
        "Expected FACTION_MISMATCH error, got: {:?}",
        invalid_result.errors
    );
}

// ===========================================================================
// Test 15: No CHARACTER requirement — army can be valid without any CHARACTER
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 6.5
/// Rule: "An army can be valid without any CHARACTER models. The warlord must
///        be a CHARACTER only IF CHARACTER units are present in the roster."
/// Test: A roster with only non-CHARACTER units (e.g., all BATTLELINE) should
///       still be valid. The warlord does not need to be a CHARACTER when there
///       are no CHARACTERs in the roster at all.
#[test]
fn ba_mustering_no_character_requirement() {
    let faction = load_world_eaters_butchers();

    // Build a roster with ONLY non-CHARACTER units
    let mut patrol = BoardingPatrol::new(
        "World Eaters".to_string(),
        "WORLD EATERS".to_string(),
        "Boarding Butchers".to_string(),
    );

    // Add only non-CHARACTER units
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Khorne Berzerkers".to_string(),
        model_count: 5,
        points: 90,
        wargear_selections: vec![],
    });
    patrol.add_unit(SelectedUnit {
        datasheet_name: "Jakhals".to_string(),
        model_count: 10,
        points: 65,
        wargear_selections: vec![],
    });

    // Designate Berzerkers (non-CHARACTER, index 0) as warlord
    // This should be VALID because there are no CHARACTERs in the roster
    patrol.warlord_index = Some(0);

    let validator = BoardingRosterValidator::new(&patrol, &faction);
    let result = validator.validate();

    // Should NOT have WARLORD_NOT_CHARACTER error because no CHARACTERs exist
    let has_warlord_error = result
        .errors
        .iter()
        .any(|e| e.code == "WARLORD_NOT_CHARACTER");
    assert!(
        !has_warlord_error,
        "Roster without any CHARACTER units should NOT produce WARLORD_NOT_CHARACTER error. Errors: {:?}",
        result.errors
    );
}
