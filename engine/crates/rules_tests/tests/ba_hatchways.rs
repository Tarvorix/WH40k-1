//! Source-linked rules tests for boarding_actions_complete_v3.md Section 2.3 & 3.2:
//! Hatchway States and Hatchway Operations.
//!
//! Tests cover:
//!   S2.3 - Hatchway state machine: Open, Closed, Locked, OneWayOpened
//!   S2.3 - State properties: allows_passage, allows_los, can_operate, toggle
//!   S3.2 - Operating hatchways: range check, engagement block, roll-off
//!   S3.2 - Split unit prevention: cannot close if same unit on both sides
//!   S3.2 - Opening into engagement: enemy within 2" on other side
//!
//! Source: boarding_actions_complete_v3.md Sections 2.3 and 3.2

use std::collections::HashMap;
use wh40k_core_types::{
    BoardDimensions, CompartmentId, HatchwayId, HatchwayOrientation, HatchwayState,
    Inches, Polygon, Position, UnitId,
};
use wh40k_geometry::boarding::{BoardingMap, Compartment, Hatchway, WallSegment};
use wh40k_boarding_rules::hatchway::{
    can_close_hatchway, can_operate_hatchway, check_opening_engagement,
    resolve_hatchway_operation, HatchwayError,
};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a test map with two compartments separated by a wall and connected
/// by a single hatchway at (10, 5). Wall runs along x=10 from y=0..4 and y=6..10.
/// Hatchway is Vertical, width 2", initial state Closed.
fn make_test_map() -> BoardingMap {
    let mut map = BoardingMap::new(BoardDimensions {
        width: Inches::from_inches(20),
        height: Inches::from_inches(10),
    });

    map.compartments.push(Compartment {
        id: CompartmentId::new(0),
        name: "Left".into(),
        boundary: Polygon::new(vec![
            Position::from_inches(0, 0),
            Position::from_inches(10, 0),
            Position::from_inches(10, 10),
            Position::from_inches(0, 10),
        ]),
        tags: vec![],
    });

    map.compartments.push(Compartment {
        id: CompartmentId::new(1),
        name: "Right".into(),
        boundary: Polygon::new(vec![
            Position::from_inches(10, 0),
            Position::from_inches(20, 0),
            Position::from_inches(20, 10),
            Position::from_inches(10, 10),
        ]),
        tags: vec![],
    });

    // Walls along x=10 with gap for hatchway at y=4..6
    map.walls.push(WallSegment {
        id: 0,
        start: Position::from_inches(10, 0),
        end: Position::from_inches(10, 4),
    });
    map.walls.push(WallSegment {
        id: 1,
        start: Position::from_inches(10, 6),
        end: Position::from_inches(10, 10),
    });

    map.hatchways.push(Hatchway {
        id: HatchwayId::new(0),
        position: Position::from_inches(10, 5),
        orientation: HatchwayOrientation::Vertical,
        width: Inches::from_inches(2),
        between: (CompartmentId::new(0), CompartmentId::new(1)),
        initial_state: HatchwayState::Closed,
        tags: vec![],
    });

    map
}

/// Build default hatchway states from the test map (all initial states).
fn default_states(map: &BoardingMap) -> HashMap<HatchwayId, HatchwayState> {
    map.hatchways
        .iter()
        .map(|h| (h.id, h.initial_state))
        .collect()
}

// ===========================================================================
// S2.3 - Hatchway State Properties
// ===========================================================================

/// S2.3: HatchwayState::Open.allows_passage() must be true.
/// An open hatchway permits models to pass through.
#[test]
fn test_open_allows_passage() {
    assert!(
        HatchwayState::Open.allows_passage(),
        "Open hatchway must allow passage"
    );
}

/// S2.3: HatchwayState::Closed.allows_passage() must be false.
/// A closed hatchway blocks all movement through it.
#[test]
fn test_closed_blocks_passage() {
    assert!(
        !HatchwayState::Closed.allows_passage(),
        "Closed hatchway must block passage"
    );
}

/// S2.3: HatchwayState::Locked.allows_passage() must be false.
/// A locked hatchway blocks all movement through it.
#[test]
fn test_locked_blocks_passage() {
    assert!(
        !HatchwayState::Locked.allows_passage(),
        "Locked hatchway must block passage"
    );
}

/// S2.3: HatchwayState::OneWayOpened.allows_passage() must be true.
/// A one-way-opened hatchway allows passage (it has been breached).
#[test]
fn test_one_way_opened_allows_passage() {
    assert!(
        HatchwayState::OneWayOpened.allows_passage(),
        "OneWayOpened hatchway must allow passage"
    );
}

/// S2.3: HatchwayState::Open.allows_los() must be true.
/// An open hatchway does not block visibility.
#[test]
fn test_open_allows_los() {
    assert!(
        HatchwayState::Open.allows_los(),
        "Open hatchway must allow line of sight"
    );
}

/// S2.3: HatchwayState::Closed.allows_los() must be false.
/// A closed hatchway blocks all visibility and measurement.
#[test]
fn test_closed_blocks_los() {
    assert!(
        !HatchwayState::Closed.allows_los(),
        "Closed hatchway must block line of sight"
    );
}

/// S2.3: HatchwayState::Open.can_operate() must be true.
/// Open hatchways can be toggled to Closed.
#[test]
fn test_open_can_operate() {
    assert!(
        HatchwayState::Open.can_operate(),
        "Open hatchway must be operable (can be closed)"
    );
}

/// S2.3: HatchwayState::Closed.can_operate() must be true.
/// Closed hatchways can be toggled to Open.
#[test]
fn test_closed_can_operate() {
    assert!(
        HatchwayState::Closed.can_operate(),
        "Closed hatchway must be operable (can be opened)"
    );
}

/// S2.3: HatchwayState::Locked.can_operate() must be false.
/// Locked hatchways cannot be toggled by normal means.
#[test]
fn test_locked_cannot_operate() {
    assert!(
        !HatchwayState::Locked.can_operate(),
        "Locked hatchway must not be operable"
    );
}

/// S2.3: HatchwayState::OneWayOpened.can_operate() must be false.
/// One-way opened hatchways cannot be toggled back.
#[test]
fn test_one_way_opened_cannot_operate() {
    assert!(
        !HatchwayState::OneWayOpened.can_operate(),
        "OneWayOpened hatchway must not be operable"
    );
}

/// S2.3: toggle() Open -> Some(Closed).
#[test]
fn test_toggle_open_to_closed() {
    assert_eq!(
        HatchwayState::Open.toggle(),
        Some(HatchwayState::Closed),
        "Toggling Open must yield Closed"
    );
}

/// S2.3: toggle() Closed -> Some(Open).
#[test]
fn test_toggle_closed_to_open() {
    assert_eq!(
        HatchwayState::Closed.toggle(),
        Some(HatchwayState::Open),
        "Toggling Closed must yield Open"
    );
}

/// S2.3: toggle() Locked -> None (cannot toggle).
#[test]
fn test_toggle_locked_returns_none() {
    assert_eq!(
        HatchwayState::Locked.toggle(),
        None,
        "Toggling Locked must return None"
    );
}

/// S2.3: toggle() OneWayOpened -> None (cannot toggle).
#[test]
fn test_toggle_one_way_opened_returns_none() {
    assert_eq!(
        HatchwayState::OneWayOpened.toggle(),
        None,
        "Toggling OneWayOpened must return None"
    );
}

// ===========================================================================
// S3.2 - Operating Hatchways: can_operate_hatchway
// ===========================================================================

/// S3.2: A unit within 1" of the hatchway, not engaged, with an operable
/// (Closed) hatchway must be allowed to operate.
#[test]
fn test_can_operate_success_closed_hatchway() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    // Model at (9, 5) -- within 1" of hatchway at (10, 5)
    let unit_positions = vec![Position::from_inches(9, 5)];
    let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
    assert!(result.is_ok(), "Unit within 1\" of closed hatchway should be able to operate");
}

/// S3.2: A unit within engagement range of an enemy cannot operate hatchways.
#[test]
fn test_can_operate_fails_when_engaged() {
    let map = make_test_map();
    let states = default_states(&map);
    let unit_positions = vec![Position::from_inches(9, 5)];
    let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, true, &states);
    assert_eq!(
        result,
        Err(HatchwayError::InEngagementRange),
        "Engaged unit must not be allowed to operate hatchways"
    );
}

/// S3.2: A unit must be within 1" (BA_HATCHWAY_OPERATE_RANGE) of the hatchway.
/// A unit more than 1" away must be rejected.
#[test]
fn test_can_operate_fails_not_in_range() {
    let map = make_test_map();
    let states = default_states(&map);
    // Model at (5, 5) -- 5" from hatchway, well outside 1" range
    let unit_positions = vec![Position::from_inches(5, 5)];
    let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
    assert_eq!(
        result,
        Err(HatchwayError::NotInRange),
        "Unit more than 1\" from hatchway must be rejected"
    );
}

/// S3.2: Locked hatchways cannot be operated.
#[test]
fn test_can_operate_fails_locked() {
    let map = make_test_map();
    let mut states = default_states(&map);
    states.insert(HatchwayId::new(0), HatchwayState::Locked);
    let unit_positions = vec![Position::from_inches(9, 5)];
    let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
    assert_eq!(
        result,
        Err(HatchwayError::NotOperable),
        "Locked hatchway must not be operable"
    );
}

/// S3.2: OneWayOpened hatchways cannot be operated.
#[test]
fn test_can_operate_fails_one_way_opened() {
    let map = make_test_map();
    let mut states = default_states(&map);
    states.insert(HatchwayId::new(0), HatchwayState::OneWayOpened);
    let unit_positions = vec![Position::from_inches(9, 5)];
    let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
    assert_eq!(
        result,
        Err(HatchwayError::NotOperable),
        "OneWayOpened hatchway must not be operable"
    );
}

// ===========================================================================
// S3.2 - Resolving Hatchway Operations: resolve_hatchway_operation
// ===========================================================================

/// S3.2: Uncontested operation on a Closed hatchway toggles to Open.
/// When no opponent is preventing, the state changes automatically.
#[test]
fn test_resolve_uncontested_open() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4,     // operating unit toughness
        false, // no opponent contesting
        0,     // opponent toughness (ignored)
        0,     // operating roll (ignored when uncontested)
        0,     // preventing roll (ignored)
        &[],   // no units for engagement check on side A
        &[],   // no units for engagement check on side B
    );
    assert!(result.success, "Uncontested operation must succeed");
    assert_eq!(
        result.new_state,
        HatchwayState::Open,
        "Closed hatchway must toggle to Open"
    );
}

/// S3.2: Uncontested operation on an Open hatchway toggles to Closed.
#[test]
fn test_resolve_uncontested_close() {
    let map = make_test_map();
    let mut states = default_states(&map);
    states.insert(HatchwayId::new(0), HatchwayState::Open);
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4,     // operating unit toughness
        false, // no opponent contesting
        0, 0, 0,
        &[], &[],
    );
    assert!(result.success, "Uncontested close operation must succeed");
    assert_eq!(
        result.new_state,
        HatchwayState::Closed,
        "Open hatchway must toggle to Closed"
    );
}

/// S3.2: Contested operation where operating player wins the roll-off.
/// Operating: roll 5 + T4 = 9. Preventing: roll 3 + T4 = 7. Operator wins.
#[test]
fn test_resolve_contested_operator_wins() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4,    // operating toughness
        true, // opponent contesting
        4,    // opponent toughness
        5,    // operating roll
        3,    // preventing roll
        &[], &[],
    );
    assert!(result.success, "Operating player with higher total must win");
    assert_eq!(result.new_state, HatchwayState::Open);
}

/// S3.2: Contested operation where preventing player wins the roll-off.
/// Operating: roll 2 + T4 = 6. Preventing: roll 5 + T4 = 9. Defender wins.
#[test]
fn test_resolve_contested_defender_wins() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4,    // operating toughness
        true, // opponent contesting
        4,    // opponent toughness
        2,    // operating roll
        5,    // preventing roll
        &[], &[],
    );
    assert!(!result.success, "Preventing player with higher total must win");
    assert_eq!(
        result.new_state,
        HatchwayState::Closed,
        "State must not change when defender wins"
    );
}

/// S3.2: Operating player wins on ties (roll-off tie goes to the operator).
/// Operating: roll 4 + T4 = 8. Preventing: roll 4 + T4 = 8. Tie => operator wins.
#[test]
fn test_resolve_contested_tie_operator_wins() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4,    // operating toughness
        true, // opponent contesting
        4,    // opponent toughness
        4,    // operating roll
        4,    // preventing roll
        &[], &[],
    );
    assert!(result.success, "Operating player must win on ties");
    assert_eq!(result.new_state, HatchwayState::Open);
}

/// S3.2: Contested with different Toughness values.
/// Operating: roll 3 + T5 = 8. Preventing: roll 4 + T3 = 7. Operator wins.
#[test]
fn test_resolve_contested_different_toughness() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        5,    // operating toughness (higher)
        true, // opponent contesting
        3,    // opponent toughness (lower)
        3,    // operating roll
        4,    // preventing roll
        &[], &[],
    );
    // Operating total: 3 + 5 = 8. Preventing total: 4 + 3 = 7. Operator wins.
    assert!(result.success, "Higher toughness should tip the roll-off");
    assert_eq!(result.new_state, HatchwayState::Open);
}

// ===========================================================================
// S3.2 - Split Unit Prevention: can_close_hatchway
// ===========================================================================

/// S3.2: Closing is allowed when no unit has models on both sides.
#[test]
fn test_can_close_no_split() {
    let map = make_test_map();
    let mut unit_positions = HashMap::new();
    unit_positions.insert(UnitId::new(0), vec![Position::from_inches(5, 5)]);
    assert!(
        can_close_hatchway(&map, HatchwayId::new(0), &unit_positions),
        "Should allow closing when all unit models are on the same side"
    );
}

/// S3.2: Closing is blocked when the SAME unit has models on opposite sides.
/// This prevents stranding part of a unit behind a closed hatchway.
#[test]
fn test_can_close_blocked_by_split_unit() {
    let map = make_test_map();
    let mut unit_positions = HashMap::new();
    // Same unit with models at (5,5) in Left compartment and (15,5) in Right compartment
    unit_positions.insert(
        UnitId::new(0),
        vec![Position::from_inches(5, 5), Position::from_inches(15, 5)],
    );
    assert!(
        !can_close_hatchway(&map, HatchwayId::new(0), &unit_positions),
        "Must not allow closing when same unit has models on both sides"
    );
}

/// S3.2: Different units on opposite sides do NOT block closing.
/// Only the SAME unit straddling counts.
#[test]
fn test_can_close_different_units_opposite_sides_allowed() {
    let map = make_test_map();
    let mut unit_positions = HashMap::new();
    unit_positions.insert(UnitId::new(0), vec![Position::from_inches(5, 5)]);
    unit_positions.insert(UnitId::new(1), vec![Position::from_inches(15, 5)]);
    assert!(
        can_close_hatchway(&map, HatchwayId::new(0), &unit_positions),
        "Different units on opposite sides must not block closing"
    );
}

/// S3.2: A unit with a single model can never straddle, so closing is always allowed.
#[test]
fn test_can_close_single_model_unit_always_allowed() {
    let map = make_test_map();
    let mut unit_positions = HashMap::new();
    // Single model units cannot be on both sides simultaneously
    unit_positions.insert(UnitId::new(0), vec![Position::from_inches(5, 5)]);
    unit_positions.insert(UnitId::new(1), vec![Position::from_inches(15, 5)]);
    assert!(
        can_close_hatchway(&map, HatchwayId::new(0), &unit_positions),
        "Single-model units cannot split across a hatchway"
    );
}

// ===========================================================================
// S3.2 - Opening Into Engagement: check_opening_engagement
// ===========================================================================

/// S3.2: Opening a hatchway triggers engagement if enemy units on opposite
/// sides are within 2" (BA_HATCHWAY_ENGAGEMENT_RANGE) of each other.
#[test]
fn test_opening_engagement_units_within_2_inches() {
    let map = make_test_map();
    // Units 2" apart across the hatchway: (9,5) in Left, (11,5) in Right
    let units_a = vec![(UnitId::new(0), vec![Position::from_inches(9, 5)])];
    let units_b = vec![(UnitId::new(1), vec![Position::from_inches(11, 5)])];
    let pairs = check_opening_engagement(&map, HatchwayId::new(0), &units_a, &units_b);
    assert_eq!(pairs.len(), 1, "Units within 2\" across hatchway must become engaged");
    assert_eq!(pairs[0], (UnitId::new(0), UnitId::new(1)));
}

/// S3.2: Units more than 2" apart across the hatchway are NOT engaged.
#[test]
fn test_opening_engagement_units_too_far() {
    let map = make_test_map();
    // Units 10" apart: (5,5) in Left, (15,5) in Right
    let units_a = vec![(UnitId::new(0), vec![Position::from_inches(5, 5)])];
    let units_b = vec![(UnitId::new(1), vec![Position::from_inches(15, 5)])];
    let pairs = check_opening_engagement(&map, HatchwayId::new(0), &units_a, &units_b);
    assert!(
        pairs.is_empty(),
        "Units more than 2\" apart must not become engaged"
    );
}

/// S3.2: resolve_hatchway_operation detects newly engaged pairs when
/// opening a hatchway with units close to each other on opposite sides.
#[test]
fn test_resolve_with_engagement_detection() {
    let map = make_test_map();
    let states = default_states(&map); // Closed
    let units_a = vec![(UnitId::new(0), vec![Position::from_inches(9, 5)])];
    let units_b = vec![(UnitId::new(1), vec![Position::from_inches(11, 5)])];
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4, false, 0,
        1, 0,
        &units_a, &units_b,
    );
    assert!(result.success);
    assert_eq!(result.new_state, HatchwayState::Open);
    assert_eq!(
        result.newly_engaged_pairs.len(),
        1,
        "Opening hatchway must detect newly engaged pairs"
    );
    assert_eq!(result.newly_engaged_pairs[0], (UnitId::new(0), UnitId::new(1)));
}

/// S3.2: Closing a hatchway does NOT produce newly engaged pairs
/// (engagement detection only applies when OPENING).
#[test]
fn test_resolve_close_no_engagement_detection() {
    let map = make_test_map();
    let mut states = default_states(&map);
    states.insert(HatchwayId::new(0), HatchwayState::Open);
    let units_a = vec![(UnitId::new(0), vec![Position::from_inches(9, 5)])];
    let units_b = vec![(UnitId::new(1), vec![Position::from_inches(11, 5)])];
    let result = resolve_hatchway_operation(
        &map,
        HatchwayId::new(0),
        &states,
        4, false, 0,
        1, 0,
        &units_a, &units_b,
    );
    assert!(result.success);
    assert_eq!(result.new_state, HatchwayState::Closed);
    assert!(
        result.newly_engaged_pairs.is_empty(),
        "Closing a hatchway must not produce newly engaged pairs"
    );
}

// ===========================================================================
// S2.3 - Constants verification
// ===========================================================================

/// S3.2: BA_HATCHWAY_OPERATE_RANGE must be 1".
#[test]
fn test_hatchway_operate_range_constant() {
    assert_eq!(
        Inches::BA_HATCHWAY_OPERATE_RANGE,
        Inches::from_inches(1),
        "BA_HATCHWAY_OPERATE_RANGE must be 1\""
    );
}

/// S3.2: BA_HATCHWAY_ENGAGEMENT_RANGE must be 2".
#[test]
fn test_hatchway_engagement_range_constant() {
    assert_eq!(
        Inches::BA_HATCHWAY_ENGAGEMENT_RANGE,
        Inches::from_inches(2),
        "BA_HATCHWAY_ENGAGEMENT_RANGE must be 2\""
    );
}
