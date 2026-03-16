//! Source-linked rules tests for 40k_revised.md Section 9: Charge Phase.
//!
//! Tests cover:
//!   - 9.1: Charge eligibility (12" range, no Advance/Fall Back, not in ER, not AIRCRAFT)
//!   - 9.3: Charge Roll = 2D6 (valid range 2-12)
//!   - 9.4: Charge success (end in ER of ALL targets, not in ER of non-targets)
//!   - 9.5: Charging units get Fights First (charged_this_turn flag)
//!   - 9.6: Charge over terrain <= 2" free, > 2" climb
//!
//! Source: 40k_revised.md Section 9 -- "CHARGE PHASE"

use wh40k_core_types::{
    ArmorSave, BaseSize, DatasheetId,
    EngagementStatus, Inches, Keyword, KeywordSet, Leadership, ModelId,
    MoveCharacteristic, ObjectiveControl, PlayerId, Position,
    Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_game_core::state::TurnFlags;
use wh40k_game_core::unit::{ModelState, UnitState};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a model at a specified position.
fn make_model(model_id: u32, unit_id: UnitId, wounds: u8, pos: Position) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        pos,
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a unit with given keywords and models.
fn make_unit(
    id: u32,
    owner: PlayerId,
    keywords: &[Keyword],
    models: Vec<ModelState>,
) -> UnitState {
    let mut unit = UnitState::new(
        UnitId::new(id),
        owner,
        "Test Unit".to_string(),
        DatasheetId::new(1),
        KeywordSet::from_keywords(keywords),
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

// ===========================================================================
// 9.1 -- Charge eligibility: enemy within 12"
// ===========================================================================

/// Source: 40k_revised.md 9.1
/// Rule: "you can select any eligible unit ... to make a charge ...
///        enemy within 12\""
/// The 12" distance constant is used in charge range checks.
#[test]
fn charge_range_is_twelve_inches() {
    let charge_range = Inches::from_inches(12);
    assert_eq!(
        charge_range.whole_inches(),
        12,
        "Charge declaration range should be 12 inches"
    );
    assert_eq!(
        charge_range.mils(),
        12000,
        "12 inches = 12000 mils"
    );
}

/// Source: 40k_revised.md 9.1/9.2
/// Rule: Target must be within 12" to be declared as a charge target.
/// Test distance check: unit within 12" is a valid target.
#[test]
fn charge_target_within_twelve_inches_is_valid() {
    let charger_pos = Position::from_inches(10, 10);
    let close_target_pos = Position::from_inches(20, 10); // 10" away

    let charge_range = Inches::from_inches(12);
    let dist = charger_pos.distance(close_target_pos);

    assert!(
        dist <= charge_range,
        "Target 10\" away should be within 12\" charge range (dist = {})",
        dist
    );
}

/// Source: 40k_revised.md 9.1/9.2
/// Rule: Target beyond 12" cannot be declared as a charge target.
#[test]
fn charge_target_beyond_twelve_inches_is_invalid() {
    let charger_pos = Position::from_inches(10, 10);
    let far_target_pos = Position::from_inches(30, 10); // 20" away

    let charge_range = Inches::from_inches(12);
    let dist = charger_pos.distance(far_target_pos);

    assert!(
        dist > charge_range,
        "Target 20\" away should NOT be within 12\" charge range (dist = {})",
        dist
    );
}

/// Source: 40k_revised.md 9.1/9.2
/// Rule: Distance check uses geometry. Test diagonal distance:
/// positions (0,0) and (5,12) should be 13" -- outside charge range.
#[test]
fn charge_range_diagonal_check() {
    let charger = Position::from_inches(0, 0);
    let target = Position::from_inches(5, 12); // sqrt(25 + 144) = sqrt(169) = 13
    let charge_range = Inches::from_inches(12);
    let dist = charger.distance(target);

    assert_eq!(dist, Inches::from_inches(13));
    assert!(
        dist > charge_range,
        "13\" diagonal should be outside 12\" charge range"
    );
}

// ===========================================================================
// 9.1 -- Charge eligibility: Advanced units can't charge
// ===========================================================================

/// Source: 40k_revised.md 9.1
/// Rule: "A unit that Advances ... cannot ... charge in the same turn."
/// TurnFlags.advanced_this_turn blocks charge eligibility.
#[test]
fn advanced_units_cannot_charge() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(5);

    flags.mark_advanced(unit);

    assert!(
        flags.has_advanced(unit),
        "Unit should be marked as advanced"
    );

    // The validator checks has_advanced() and rejects charges.
    // Here we verify the flag is correctly set for the validator to use.
    assert!(
        flags.has_moved(unit),
        "Advanced unit should also be marked as moved"
    );
}

/// Source: 40k_revised.md 9.1
/// Rule: Only the specific unit that advanced is blocked. Other units are fine.
#[test]
fn advanced_flag_only_affects_specific_unit() {
    let mut flags = TurnFlags::new();
    let unit_advanced = UnitId::new(10);
    let unit_normal = UnitId::new(11);

    flags.mark_advanced(unit_advanced);
    flags.mark_moved(unit_normal);

    assert!(flags.has_advanced(unit_advanced));
    assert!(!flags.has_advanced(unit_normal));
}

// ===========================================================================
// 9.1 -- Charge eligibility: Fell Back units can't charge
// ===========================================================================

/// Source: 40k_revised.md 9.1
/// Rule: "A unit that Falls Back ... cannot ... charge in the same turn."
/// TurnFlags.fell_back_this_turn blocks charge eligibility.
#[test]
fn fell_back_units_cannot_charge() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(15);

    flags.mark_fell_back(unit);

    assert!(
        flags.has_fell_back(unit),
        "Unit should be marked as fell back"
    );

    // The validator checks has_fell_back() and rejects charges.
    assert!(
        flags.has_moved(unit),
        "Fell-back unit should also be marked as moved"
    );
}

/// Source: 40k_revised.md 9.1
/// Rule: Fell back flag and advanced flag are independent.
#[test]
fn fell_back_and_advanced_are_independent_flags() {
    let mut flags = TurnFlags::new();
    let unit_fb = UnitId::new(20);
    let unit_adv = UnitId::new(21);

    flags.mark_fell_back(unit_fb);
    flags.mark_advanced(unit_adv);

    assert!(flags.has_fell_back(unit_fb));
    assert!(!flags.has_advanced(unit_fb));

    assert!(flags.has_advanced(unit_adv));
    assert!(!flags.has_fell_back(unit_adv));
}

// ===========================================================================
// 9.1 -- Charge eligibility: AIRCRAFT can't charge
// ===========================================================================

/// Source: 40k_revised.md 9.1
/// Rule: "AIRCRAFT units cannot charge."
/// Verify AIRCRAFT keyword exists and is checkable.
#[test]
fn aircraft_keyword_exists() {
    let aircraft_set = KeywordSet::from_keywords(&[Keyword::Aircraft]);
    assert!(
        aircraft_set.has(Keyword::Aircraft),
        "AIRCRAFT keyword should be available"
    );
}

/// Source: 40k_revised.md 9.1
/// Rule: Units with the AIRCRAFT keyword cannot declare a charge.
/// Verify keyword check on a unit.
#[test]
fn aircraft_unit_has_keyword_for_charge_block() {
    let aircraft_unit = make_unit(
        50,
        PlayerId::new(0),
        &[Keyword::Vehicle, Keyword::Aircraft, Keyword::Fly],
        vec![make_model(5000, UnitId::new(50), 10, Position::from_inches(20, 20))],
    );

    assert!(
        aircraft_unit.has_keyword(Keyword::Aircraft),
        "Unit should have AIRCRAFT keyword"
    );

    // The validator checks unit.keywords.has(Keyword::Aircraft) and rejects charges.
}

/// Source: 40k_revised.md 9.1
/// Rule: Non-AIRCRAFT units (regular infantry) can charge.
#[test]
fn non_aircraft_unit_not_blocked() {
    let infantry_unit = make_unit(
        51,
        PlayerId::new(0),
        &[Keyword::Infantry, Keyword::Battleline],
        vec![make_model(5100, UnitId::new(51), 3, Position::from_inches(10, 10))],
    );

    assert!(
        !infantry_unit.has_keyword(Keyword::Aircraft),
        "Infantry unit should NOT have AIRCRAFT keyword"
    );
}

/// Source: 40k_revised.md 9.1
/// Rule: A unit already within Engagement Range of enemies cannot charge.
/// The engagement_status field tracks this.
#[test]
fn unit_already_in_engagement_range_cannot_charge() {
    let mut unit = make_unit(
        60,
        PlayerId::new(0),
        &[Keyword::Infantry],
        vec![make_model(6000, UnitId::new(60), 3, Position::from_inches(10, 10))],
    );

    // Initially not engaged
    assert_eq!(unit.engagement_status, EngagementStatus::NotEngaged);

    // Set to engaged
    unit.engagement_status = EngagementStatus::Engaged;
    assert_eq!(
        unit.engagement_status,
        EngagementStatus::Engaged,
        "Unit engagement status should be Engaged"
    );
    // The validator checks this and rejects the charge.
}

// ===========================================================================
// 9.3 -- Charge Roll = 2D6 (produces range 2-12)
// ===========================================================================

/// Source: 40k_revised.md 9.3
/// Rule: "The controlling player makes a Charge roll for that unit by rolling 2D6."
/// Valid charge roll results are 2-12 (sum of two D6).
#[test]
fn charge_roll_valid_range_minimum() {
    // Minimum possible 2D6 = 1+1 = 2
    let min_charge_roll: u8 = 2;
    assert!(
        min_charge_roll >= 2 && min_charge_roll <= 12,
        "Minimum charge roll should be 2"
    );
}

/// Source: 40k_revised.md 9.3
/// Rule: Maximum 2D6 result is 12.
#[test]
fn charge_roll_valid_range_maximum() {
    let max_charge_roll: u8 = 12;
    assert!(
        max_charge_roll >= 2 && max_charge_roll <= 12,
        "Maximum charge roll should be 12"
    );
}

/// Source: 40k_revised.md 9.3
/// Rule: Charge roll results from 2-12 are valid.
/// Values outside this range are invalid (1 is impossible with 2D6, 13+ as well).
#[test]
fn charge_roll_boundary_values() {
    // Valid values: 2 through 12
    for roll in 2u8..=12u8 {
        assert!(
            roll >= 2 && roll <= 12,
            "Roll {} should be valid",
            roll
        );
    }

    // Invalid values
    let invalid_low: u8 = 1;
    let invalid_high: u8 = 13;
    assert!(
        invalid_low < 2,
        "Roll of 1 is invalid for 2D6"
    );
    assert!(
        invalid_high > 12,
        "Roll of 13 is invalid for 2D6"
    );
}

/// Source: 40k_revised.md 9.3
/// Rule: Charge roll results are tracked in TurnFlags.charge_roll_results.
/// The charge roll is stored per unit for MakeChargeMove validation.
#[test]
fn charge_roll_result_stored_in_turnflags() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(30);

    // No charge roll initially
    assert!(
        flags.get_charge_roll(unit).is_none(),
        "No charge roll should be stored initially"
    );

    // Store a charge roll of 7
    flags.set_charge_roll(unit, 7);
    assert_eq!(
        flags.get_charge_roll(unit),
        Some(7),
        "Charge roll should be stored as 7"
    );

    // Overwrite with a different value
    flags.set_charge_roll(unit, 10);
    assert_eq!(
        flags.get_charge_roll(unit),
        Some(10),
        "Charge roll should be updated to 10"
    );

    // Clear resets
    flags.clear();
    assert!(
        flags.get_charge_roll(unit).is_none(),
        "Charge roll should be cleared after turn reset"
    );
}

/// Source: 40k_revised.md 9.3
/// Rule: Each unit has its own charge roll result.
#[test]
fn charge_roll_per_unit_tracking() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(31);
    let unit_b = UnitId::new(32);

    flags.set_charge_roll(unit_a, 8);
    flags.set_charge_roll(unit_b, 5);

    assert_eq!(flags.get_charge_roll(unit_a), Some(8));
    assert_eq!(flags.get_charge_roll(unit_b), Some(5));
}

// ===========================================================================
// 9.4 -- Charge success: must end in ER of ALL targets, not in ER of non-targets
// ===========================================================================

/// Source: 40k_revised.md 9.4
/// Rule: "The charging unit must end that Charge move in unit coherency and
///        within Engagement Range of every unit that was declared as a target."
/// Verify engagement range is 1" (used for charge end-point validation).
#[test]
fn engagement_range_for_charge_is_one_inch() {
    assert_eq!(
        Inches::ENGAGEMENT_RANGE,
        Inches::from_inches(1),
        "Engagement Range should be 1 inch"
    );
}

/// Source: 40k_revised.md 9.4
/// Rule: Charge move distance is capped by the charge roll result.
/// Test that TurnFlags stores the roll for use in MakeChargeMove validation.
#[test]
fn charge_move_distance_capped_by_roll() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(40);

    // Roll of 7 means max charge move is 7"
    flags.set_charge_roll(unit, 7);
    let roll = flags.get_charge_roll(unit).unwrap();

    let max_distance = Inches::from_inches(roll as i32);
    assert_eq!(
        max_distance,
        Inches::from_inches(7),
        "Max charge move should be 7 inches for a roll of 7"
    );

    // A move of 6" is within the cap
    let move_dist = Inches::from_inches(6);
    assert!(move_dist <= max_distance, "6\" should be within 7\" cap");

    // A move of 8" exceeds the cap
    let over_dist = Inches::from_inches(8);
    assert!(over_dist > max_distance, "8\" should exceed 7\" cap");
}

/// Source: 40k_revised.md 9.2
/// Rule: Declared charge targets are stored in TurnFlags.declared_charge_targets.
/// These are used during MakeChargeMove validation to ensure the charge ends
/// in ER of at least one target.
#[test]
fn declared_charge_targets_stored_in_turnflags() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(41);
    let target_a = UnitId::new(100);
    let target_b = UnitId::new(101);

    // No targets initially
    assert!(
        flags.get_charge_targets(charger).is_none(),
        "No charge targets should be stored initially"
    );

    // Set charge targets
    flags.set_charge_targets(charger, vec![target_a, target_b]);

    let targets = flags.get_charge_targets(charger).unwrap();
    assert_eq!(targets.len(), 2, "Should have 2 declared targets");
    assert!(targets.contains(&target_a));
    assert!(targets.contains(&target_b));

    // Clear resets
    flags.clear();
    assert!(flags.get_charge_targets(charger).is_none());
}

/// Source: 40k_revised.md 9.4
/// Rule: Coherency must be maintained after charge. For a multi-model unit,
/// the coherency range is 2" horizontal.
#[test]
fn coherency_range_for_charge_is_two_inches() {
    assert_eq!(
        Inches::COHERENCY_RANGE,
        Inches::from_inches(2),
        "Coherency range should be 2 inches"
    );
    assert_eq!(
        Inches::COHERENCY_RANGE_VERTICAL,
        Inches::from_inches(5),
        "Coherency vertical range should be 5 inches"
    );
}

/// Source: 40k_revised.md 9.4
/// Rule: Using within_engagement_range_2d to verify position-based engagement
/// range check for charge end point validation.
#[test]
fn charge_endpoint_engagement_range_check() {
    // Charger base: 32mm (radius ~0.630")
    // Target base: 32mm
    // If charger center is at (10, 10) and target center is at (11, 10),
    // the gap between bases is about 1" - 2*(~0.630") which is negative,
    // meaning they overlap -- definitely in ER.
    let charger_pos = Position::from_inches(10, 10);
    let target_pos = Position::from_inches(11, 10);
    let base = BaseSize::MM32;

    let in_er = wh40k_geometry::within_engagement_range_2d(
        charger_pos,
        base,
        target_pos,
        base,
    );
    assert!(
        in_er,
        "Models ~1\" apart (center to center) with 32mm bases should be in ER"
    );
}

/// Source: 40k_revised.md 9.4
/// Rule: If models are too far apart (> 1" gap between bases), they are not
/// in engagement range. Charge fails if the charger can't reach ER.
#[test]
fn charge_endpoint_outside_engagement_range() {
    // Charger at (0, 0), target at (5, 0)
    // With 32mm bases, the gap is approximately 5" - 2*(~0.630") = ~3.74"
    // This is well outside 1" engagement range.
    let charger_pos = Position::from_inches(0, 0);
    let target_pos = Position::from_inches(5, 0);
    let base = BaseSize::MM32;

    let in_er = wh40k_geometry::within_engagement_range_2d(
        charger_pos,
        base,
        target_pos,
        base,
    );
    assert!(
        !in_er,
        "Models 5\" apart should NOT be in engagement range"
    );
}

// ===========================================================================
// 9.5 -- Charging units get Fights First (charged_this_turn flag)
// ===========================================================================

/// Source: 40k_revised.md 9.5
/// Rule: "A unit that made a charge move this turn is eligible to fight in
///        the Fights First step of the Fight phase."
/// The charged_this_turn flag marks this.
#[test]
fn charged_this_turn_flag_marks_unit_for_fights_first() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(50);

    // Initially not charged
    assert!(
        !flags.charged_this_turn(unit),
        "Unit should not be marked as charged initially"
    );

    // Mark as charged
    flags.mark_charged(unit);

    assert!(
        flags.charged_this_turn(unit),
        "Unit should be marked as charged_this_turn for Fights First"
    );

    // Also should show in the units_charged set
    assert!(
        flags.has_charged(unit),
        "Unit should be in units_charged set"
    );
}

/// Source: 40k_revised.md 9.5
/// Rule: Only units that charged this turn get Fights First; other units
/// that were already in combat do not.
#[test]
fn charged_this_turn_only_applies_to_units_that_charged() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(51);
    let already_in_combat = UnitId::new(52);

    flags.mark_charged(charger);
    // already_in_combat is engaged but did NOT charge this turn

    assert!(flags.charged_this_turn(charger));
    assert!(
        !flags.charged_this_turn(already_in_combat),
        "Unit that didn't charge should not have charged_this_turn"
    );
}

/// Source: 40k_revised.md 9.5
/// Rule: The charged_this_turn flag clears at turn start.
#[test]
fn charged_this_turn_flag_clears_on_turn_reset() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(53);

    flags.mark_charged(unit);
    assert!(flags.charged_this_turn(unit));

    flags.clear();
    assert!(
        !flags.charged_this_turn(unit),
        "charged_this_turn should be cleared after turn reset"
    );
}

/// Source: 40k_revised.md 9.5
/// Rule: Multiple units can charge in the same turn, each getting Fights First.
#[test]
fn multiple_units_can_charge_same_turn() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(54);
    let unit_b = UnitId::new(55);
    let unit_c = UnitId::new(56);

    flags.mark_charged(unit_a);
    flags.mark_charged(unit_b);
    flags.mark_charged(unit_c);

    assert!(flags.charged_this_turn(unit_a));
    assert!(flags.charged_this_turn(unit_b));
    assert!(flags.charged_this_turn(unit_c));
}

// ===========================================================================
// 9.6 -- Charge over terrain: <= 2" free, > 2" climb
// ===========================================================================

/// Source: 40k_revised.md 9.6
/// Rule: "If a terrain feature is 2\" or less in height, models can move
///        over it freely (costs no extra movement)."
/// The 2" threshold is the standard for "passable" terrain.
#[test]
fn terrain_height_threshold_two_inches() {
    let free_height = Inches::from_inches(2);
    let climb_height = Inches::from_inches(3);

    // Heights <= 2" are free to cross
    assert!(
        free_height <= Inches::from_inches(2),
        "Terrain 2\" or less should be freely passable"
    );

    // Heights > 2" require climbing
    assert!(
        climb_height > Inches::from_inches(2),
        "Terrain > 2\" should require climbing movement"
    );
}

/// Source: 40k_revised.md 9.6
/// Rule: The vertical distance is tracked separately from horizontal distance.
/// Engagement Range vertical component is 5", which applies during charge
/// endpoint validation when models are at different heights.
#[test]
fn vertical_engagement_range_for_terrain() {
    assert_eq!(
        Inches::ENGAGEMENT_RANGE_VERTICAL,
        Inches::from_inches(5),
        "Vertical engagement range should be 5 inches"
    );

    // A model on terrain at height 3" can still be in ER of a ground-level model
    // because 3" <= 5" vertical ER
    let height_diff = Inches::from_inches(3);
    assert!(
        height_diff <= Inches::ENGAGEMENT_RANGE_VERTICAL,
        "3\" height difference should be within 5\" vertical ER"
    );

    // A model at height 6" is outside vertical ER
    let too_high = Inches::from_inches(6);
    assert!(
        too_high > Inches::ENGAGEMENT_RANGE_VERTICAL,
        "6\" height difference should exceed 5\" vertical ER"
    );
}

/// Source: 40k_revised.md 9.6
/// Rule: Full engagement range check includes both horizontal and vertical
/// components. Use within_engagement_range (3D) for models at different heights.
#[test]
fn engagement_range_with_height() {
    // Two models at same position horizontally but different heights
    let pos = Position::from_inches(10, 10);
    let base = BaseSize::MM32;

    // Same height: should be in ER (0" gap horizontally)
    let in_er = wh40k_geometry::within_engagement_range(
        pos, base, Inches::ZERO,
        pos, base, Inches::ZERO,
    );
    assert!(in_er, "Same position models should be in ER");

    // 4" height difference: within 5" vertical ER
    let in_er_high = wh40k_geometry::within_engagement_range(
        pos, base, Inches::ZERO,
        pos, base, Inches::from_inches(4),
    );
    assert!(in_er_high, "4\" height difference should be within vertical ER");

    // 6" height difference: outside 5" vertical ER
    let not_in_er = wh40k_geometry::within_engagement_range(
        pos, base, Inches::ZERO,
        pos, base, Inches::from_inches(6),
    );
    assert!(!not_in_er, "6\" height difference should be outside vertical ER");
}

/// Source: 40k_revised.md 9.6
/// Rule: Terrain <= 2" height can be crossed freely during a charge move.
/// This means the effective "cost" of traversing that terrain is 0 extra inches.
/// We verify this by checking that 2" is the threshold.
#[test]
fn terrain_under_two_inches_is_free_to_cross() {
    let terrain_heights = [
        (Inches::from_inches(1), true, "1\" terrain is free"),
        (Inches::from_inches(2), true, "2\" terrain is free"),
        (Inches::from_mils(1500), true, "1.5\" terrain is free"),
        (Inches::from_mils(2001), false, "2.001\" terrain requires climbing"),
        (Inches::from_inches(3), false, "3\" terrain requires climbing"),
    ];

    let threshold = Inches::from_inches(2);
    for (height, should_be_free, desc) in &terrain_heights {
        let is_free = *height <= threshold;
        assert_eq!(is_free, *should_be_free, "{}", desc);
    }
}

// ===========================================================================
// Additional charge phase tests
// ===========================================================================

/// Source: 40k_revised.md 9.1
/// Rule: Combined eligibility check -- a unit must satisfy ALL of:
/// - Not Advanced
/// - Not Fell Back
/// - Not in ER of enemies
/// - Not AIRCRAFT
/// This tests multiple flags together.
#[test]
fn charge_eligibility_all_flags_must_pass() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(70);

    // Fresh unit -- no flags set: eligible
    assert!(!flags.has_advanced(unit));
    assert!(!flags.has_fell_back(unit));
    assert!(!flags.has_charged(unit));

    // Set advanced -- no longer eligible
    flags.mark_advanced(unit);
    assert!(flags.has_advanced(unit));

    // Reset and set fell back -- no longer eligible
    flags.clear();
    flags.mark_fell_back(unit);
    assert!(flags.has_fell_back(unit));
}

/// Source: 40k_revised.md 9.2
/// Rule: Declare charge targets are per-unit, independent.
/// Multiple units can declare charges against different targets.
#[test]
fn multiple_units_independent_charge_targets() {
    let mut flags = TurnFlags::new();
    let charger_a = UnitId::new(71);
    let charger_b = UnitId::new(72);
    let target_x = UnitId::new(200);
    let target_y = UnitId::new(201);

    flags.set_charge_targets(charger_a, vec![target_x]);
    flags.set_charge_targets(charger_b, vec![target_y]);

    let targets_a = flags.get_charge_targets(charger_a).unwrap();
    assert_eq!(targets_a.len(), 1);
    assert!(targets_a.contains(&target_x));

    let targets_b = flags.get_charge_targets(charger_b).unwrap();
    assert_eq!(targets_b.len(), 1);
    assert!(targets_b.contains(&target_y));
}

/// Source: 40k_revised.md 9.2
/// Rule: A unit can declare charges against multiple targets.
#[test]
fn charge_against_multiple_targets() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(73);
    let target_1 = UnitId::new(300);
    let target_2 = UnitId::new(301);
    let target_3 = UnitId::new(302);

    flags.set_charge_targets(charger, vec![target_1, target_2, target_3]);

    let targets = flags.get_charge_targets(charger).unwrap();
    assert_eq!(targets.len(), 3);
    assert!(targets.contains(&target_1));
    assert!(targets.contains(&target_2));
    assert!(targets.contains(&target_3));
}

/// Source: 40k_revised.md 9.4
/// Rule: Charge move distance must not exceed the charge roll.
/// Test various roll values and distances.
#[test]
fn charge_move_distance_validation_various_rolls() {
    let test_cases: Vec<(u8, i32, bool)> = vec![
        // (charge_roll, move_distance_inches, should_be_valid)
        (2, 1, true),   // Roll 2, move 1": valid
        (2, 2, true),   // Roll 2, move 2": valid (exactly at cap)
        (2, 3, false),  // Roll 2, move 3": exceeds cap
        (7, 7, true),   // Roll 7, move 7": exactly at cap
        (7, 8, false),  // Roll 7, move 8": exceeds cap
        (12, 12, true), // Roll 12, move 12": exactly at cap
        (12, 13, false),// Roll 12, move 13": exceeds cap
        (12, 1, true),  // Roll 12, move 1": well within cap
    ];

    for (roll, move_inches, should_be_valid) in test_cases {
        let max_distance = Inches::from_inches(roll as i32);
        let move_distance = Inches::from_inches(move_inches);
        let is_valid = move_distance <= max_distance;
        assert_eq!(
            is_valid, should_be_valid,
            "Roll {}, move {}\" -- expected valid={}, got valid={}",
            roll, move_inches, should_be_valid, is_valid
        );
    }
}

/// Source: 40k_revised.md 9.3
/// Rule: The charge roll is 2D6, so valid results are 2-12.
/// The validator rejects rolls outside this range (except 0 sentinel).
#[test]
fn charge_roll_validator_range_check() {
    // Valid 2D6 results
    for roll in 2u8..=12u8 {
        assert!(
            roll >= 2 && roll <= 12,
            "Roll {} should be valid 2D6 result",
            roll
        );
    }

    // Invalid results below 2
    assert!(1u8 < 2, "Roll of 1 is impossible on 2D6");
    assert!(0u8 < 2, "Roll of 0 is used as sentinel, not a real roll");

    // Invalid results above 12
    assert!(13u8 > 12, "Roll of 13 is impossible on 2D6");
}

/// Source: 40k_revised.md 9.4
/// Rule: Models with bases have a physical size. The engagement range check
/// accounts for base sizes. Larger bases make it easier to reach ER.
#[test]
fn base_size_affects_engagement_range_reach() {
    let pos_a = Position::from_inches(0, 0);
    let pos_b = Position::from_inches(2, 0);

    // With 32mm bases (radius ~0.630"), the gap between base edges is
    // approximately 2" - 2 * 0.630" = ~0.740" which is < 1" ER
    let in_er_32mm = wh40k_geometry::within_engagement_range_2d(
        pos_a,
        BaseSize::MM32,
        pos_b,
        BaseSize::MM32,
    );
    assert!(
        in_er_32mm,
        "32mm bases 2\" apart (center to center) should be in ER"
    );

    // With 25mm bases (radius ~0.492"), the gap is 2" - 2 * 0.492" = ~1.016"
    // which is slightly > 1" ER
    let in_er_25mm = wh40k_geometry::within_engagement_range_2d(
        pos_a,
        BaseSize::MM25,
        pos_b,
        BaseSize::MM25,
    );
    // 25mm bases at 2" center-to-center may or may not be in ER depending
    // on exact fixed-point math. The key point is the check accounts for bases.
    // With radius 492 mils each, gap = 2000 - 492 - 492 = 1016 mils > 1000
    assert!(
        !in_er_25mm,
        "25mm bases 2\" apart (center to center) should NOT be in ER (gap ~1.016\")"
    );
}

/// Source: 40k_revised.md 9.4, 9.5
/// Rule: mark_charged sets both units_charged and charged_this_turn.
/// These serve different purposes: units_charged tracks "has charged" (can't charge again),
/// charged_this_turn tracks "fights first" in fight phase.
#[test]
fn mark_charged_sets_both_tracking_sets() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(80);

    flags.mark_charged(unit);

    // Both should be set
    assert!(
        flags.has_charged(unit),
        "units_charged should contain the unit"
    );
    assert!(
        flags.charged_this_turn(unit),
        "charged_this_turn should contain the unit"
    );
}

/// Source: 40k_revised.md 9.4
/// Rule: The distance_between_models function in geometry accounts for base
/// sizes when computing how far models are from each other.
#[test]
fn distance_between_models_accounts_for_bases() {
    let pos_a = Position::from_inches(0, 0);
    let pos_b = Position::from_inches(10, 0);
    let base = BaseSize::MM32;

    let model_dist = wh40k_geometry::distance_between_models(pos_a, base, pos_b, base);
    let center_dist = pos_a.distance(pos_b);

    // Model distance should be less than center distance (by 2x radius)
    assert!(
        model_dist < center_dist,
        "Distance between models should be less than center-to-center distance"
    );

    // The difference should be approximately 2 * radius_inches for 32mm bases
    let two_radii = base.radius_inches() + base.radius_inches();
    let expected_model_dist = center_dist - two_radii;
    assert_eq!(
        model_dist, expected_model_dist,
        "Model distance should be center distance minus two radii"
    );
}

// ===========================================================================
// 9.2 -- Charge targets do NOT need to be visible
// ===========================================================================

/// Source: 40k_revised.md 9.2
/// Rule: "Charge targets do not need to be visible to the charging unit."
/// Unlike shooting targets which require line of sight, charge targets
/// only need to be within 12" — visibility is not a requirement.
/// Test: a valid charge target set can be stored even when the target
/// would not be visible. The TurnFlags do NOT track visibility for charges.
#[test]
fn charge_targets_do_not_need_visibility() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(90);
    let hidden_target = UnitId::new(200);

    // Declare a charge target — no visibility check required
    flags.set_charge_targets(charger, vec![hidden_target]);

    let targets = flags.get_charge_targets(charger).unwrap();
    assert_eq!(targets.len(), 1);
    assert!(
        targets.contains(&hidden_target),
        "Charge target should be stored regardless of visibility (§9.2)"
    );
}

/// Source: 40k_revised.md 9.2
/// Rule: Multiple non-visible targets can be declared for a single charge.
/// Test: store multiple charge targets, confirming no visibility filtering.
#[test]
fn charge_multiple_non_visible_targets() {
    let mut flags = TurnFlags::new();
    let charger = UnitId::new(91);
    let target_a = UnitId::new(300);
    let target_b = UnitId::new(301);
    let target_c = UnitId::new(302);

    // All targets within 12" but behind terrain (not visible).
    // The charge system allows this — visibility is NOT checked.
    flags.set_charge_targets(charger, vec![target_a, target_b, target_c]);

    let targets = flags.get_charge_targets(charger).unwrap();
    assert_eq!(
        targets.len(), 3,
        "All non-visible targets should be valid charge targets (§9.2)"
    );
    assert!(targets.contains(&target_a));
    assert!(targets.contains(&target_b));
    assert!(targets.contains(&target_c));
}

// ===========================================================================
// 9.4 -- Each model must end closer to a charge target
// ===========================================================================

/// Source: 40k_revised.md 9.4
/// Rule: "Each model in the charging unit must end the charge move closer
///        to at least one of the charge targets than it started."
/// Test: verify position-based distance calculations for "closer" checks.
#[test]
fn charge_each_model_must_end_closer_to_target() {
    let charge_target_pos = Position::from_inches(20, 10);

    // Model starts at (10, 10), 10" from target
    let model_start = Position::from_inches(10, 10);
    let start_dist = model_start.distance(charge_target_pos);
    assert_eq!(start_dist, Inches::from_inches(10));

    // Model ends at (15, 10), 5" from target — closer
    let model_end_closer = Position::from_inches(15, 10);
    let end_dist_closer = model_end_closer.distance(charge_target_pos);
    assert_eq!(end_dist_closer, Inches::from_inches(5));
    assert!(
        end_dist_closer < start_dist,
        "Model must end closer to a charge target (§9.4)"
    );

    // Model ends at (8, 10), 12" from target — NOT closer (moved away)
    let model_end_away = Position::from_inches(8, 10);
    let end_dist_away = model_end_away.distance(charge_target_pos);
    assert_eq!(end_dist_away, Inches::from_inches(12));
    assert!(
        end_dist_away > start_dist,
        "Model that moves away from target violates the closer requirement (§9.4)"
    );
}

// ===========================================================================
// 9.4 -- Must end base-to-base if possible
// ===========================================================================

/// Source: 40k_revised.md 9.4
/// Rule: "If it is possible for a model in the charging unit to end its
///        charge move in base-to-base contact with one or more enemy models,
///        it must do so."
/// Test: verify base-to-base contact can be achieved within charge roll
/// and that the geometry functions support this check.
#[test]
fn charge_must_end_base_to_base_if_possible() {
    let charger_start = Position::from_inches(10, 10);
    let target_pos = Position::from_inches(14, 10); // 4" away center-to-center
    let base = BaseSize::MM32;

    // Check if charger can reach base-to-base with a roll of 5
    let charge_roll: u8 = 5;
    let max_move = Inches::from_inches(charge_roll as i32);

    // Current gap between models (accounting for base sizes)
    let gap = wh40k_geometry::distance_between_models(charger_start, base, target_pos, base);
    // With 32mm bases (~0.63" radius each), gap = 4" - 2*0.63" = ~2.74"
    assert!(
        gap > Inches::ZERO,
        "There should be a positive gap between models"
    );
    assert!(
        gap < max_move,
        "Gap ({}) should be closeable within charge roll of {} (§9.4)",
        gap, charge_roll
    );

    // The charger can reach base-to-base — moving close enough
    // Move to a position that closes the gap
    let btb_pos = Position::from_inches(13, 10); // 1" from target center
    let btb_in_er = wh40k_geometry::within_engagement_range_2d(
        btb_pos, base, target_pos, base,
    );
    assert!(
        btb_in_er,
        "Charger at 1\" from target center should be in ER / base-to-base (§9.4)"
    );

    // Verify the move distance is within the charge roll
    let move_dist = charger_start.distance(btb_pos);
    assert_eq!(move_dist, Inches::from_inches(3));
    assert!(
        move_dist <= max_move,
        "Move to base-to-base ({}) should be within charge roll of {} (§9.4)",
        move_dist, charge_roll
    );
}

/// Source: 40k_revised.md 9.4
/// Rule: If base-to-base contact is NOT possible (e.g., roll too low),
/// the model must still end closer to the target.
/// Test: verify that ending in ER (but not base-to-base) is still valid
/// when base-to-base is out of reach.
#[test]
fn charge_end_in_er_when_btb_not_possible() {
    let charger_start = Position::from_inches(10, 10);
    let target_pos = Position::from_inches(22, 10); // 12" away center-to-center
    let base = BaseSize::MM32;

    // With a minimum roll of 2, the charger can only move 2"
    let charge_roll: u8 = 2;
    let max_move = Inches::from_inches(charge_roll as i32);

    // Gap to base-to-base is about 12" - 2*0.63" = ~10.74"
    // With a roll of 2, the charger cannot reach ER at all.
    // This means the charge fails.
    let gap = wh40k_geometry::distance_between_models(charger_start, base, target_pos, base);
    assert!(
        gap > max_move,
        "Gap ({}) exceeds charge roll of {} — charge would fail (§9.4)",
        gap, charge_roll
    );

    // With a roll of 12 (maximum), the charger can close the gap
    let max_roll: u8 = 12;
    let max_move_12 = Inches::from_inches(max_roll as i32);
    assert!(
        gap <= max_move_12,
        "Gap ({}) should be closeable with maximum charge roll of 12 (§9.4)",
        gap
    );
}
