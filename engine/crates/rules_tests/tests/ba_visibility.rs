//! Source-linked rules tests for boarding_actions_complete_v3.md Section 3.3:
//! Visibility, Cover, Indirect Fire, and Blast in Boarding Actions.
//!
//! Tests cover:
//!   - Walls block LOS
//!   - Closed Hatchways block LOS
//!   - Open Hatchways allow LOS
//!   - Models from other units block LOS (intervening models)
//!   - Models from the target's own unit do NOT block LOS
//!   - Indirect Fire removed in BA
//!   - Benefit of Cover unless target is FULLY visible
//!   - Blast weapon visible model counting near hatchways
//!   - Charge targets must be visible
//!
//! Source: boarding_actions_complete_v3.md Section 3.3

use std::collections::HashMap;
use wh40k_boarding_rules::visibility::{
    blast_visible_count, charge_target_must_be_visible, check_cover, check_visibility,
    indirect_fire_removed,
};
use wh40k_core_types::{
    BoardDimensions, CompartmentId, CoverType, HatchwayId, HatchwayOrientation, HatchwayState,
    Inches, Polygon, Position, UnitId, Visibility,
};
use wh40k_geometry::boarding::{BoardingMap, Compartment, Hatchway, WallSegment};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a two-compartment test map with a wall separating them at x=10,
/// with a gap for a hatchway between y=4 and y=6.
///
/// Layout:
/// - Left compartment: (0,0) to (10,10)
/// - Right compartment: (10,0) to (20,10)
/// - Wall from (10,0)-(10,4) and (10,6)-(10,10)
/// - Hatchway at (10,5), width 2", initially Open
fn make_two_compartment_map() -> BoardingMap {
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

    // Wall below the hatchway
    map.walls.push(WallSegment {
        id: 0,
        start: Position::from_inches(10, 0),
        end: Position::from_inches(10, 4),
    });

    // Wall above the hatchway
    map.walls.push(WallSegment {
        id: 1,
        start: Position::from_inches(10, 6),
        end: Position::from_inches(10, 10),
    });

    // Hatchway in the gap
    map.hatchways.push(Hatchway {
        id: HatchwayId::new(0),
        position: Position::from_inches(10, 5),
        orientation: HatchwayOrientation::Vertical,
        width: Inches::from_inches(2),
        between: (CompartmentId::new(0), CompartmentId::new(1)),
        initial_state: HatchwayState::Open,
        tags: vec![],
    });

    map
}

/// Build default hatchway states from a map (all at their initial_state).
fn default_hatchway_states(map: &BoardingMap) -> HashMap<HatchwayId, HatchwayState> {
    map.hatchways
        .iter()
        .map(|h| (h.id, h.initial_state))
        .collect()
}

// ===========================================================================
// Section 3.3 -- Walls block LOS
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Walls are impassable and block Line of Sight."
/// Test: LOS from the left compartment to the right compartment through a wall
/// segment (not through the hatchway gap) is blocked.
#[test]
fn walls_block_line_of_sight() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // LOS crosses the wall at y=2, which is within the lower wall segment (y=0..4)
    let observer = Position::from_inches(5, 2);
    let target = Position::from_inches(15, 2);

    let vis = check_visibility(&map, observer, target, &states, &[], UnitId::new(1));
    assert_eq!(
        vis,
        Visibility::NotVisible,
        "LOS through a wall segment should be blocked"
    );
}

// ===========================================================================
// Section 3.3 -- Closed Hatchways block LOS
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Closed Hatchways block Line of Sight."
/// Test: When the hatchway is closed, LOS through the hatchway gap is blocked.
#[test]
fn closed_hatchway_blocks_line_of_sight() {
    let map = make_two_compartment_map();
    let mut states = default_hatchway_states(&map);

    // Close the hatchway
    states.insert(HatchwayId::new(0), HatchwayState::Closed);

    // LOS goes through the hatchway position at (10,5)
    let observer = Position::from_inches(5, 5);
    let target = Position::from_inches(15, 5);

    let vis = check_visibility(&map, observer, target, &states, &[], UnitId::new(1));
    assert_eq!(
        vis,
        Visibility::NotVisible,
        "LOS through a closed hatchway should be blocked"
    );
}

// ===========================================================================
// Section 3.3 -- Open Hatchways allow LOS
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Open Hatchway doors do NOT block visibility."
/// Test: LOS through an open hatchway should pass through.
#[test]
fn open_hatchway_allows_line_of_sight() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map); // hatchway starts Open

    // LOS goes through the open hatchway at (10,5)
    let observer = Position::from_inches(5, 5);
    let target = Position::from_inches(15, 5);

    let vis = check_visibility(&map, observer, target, &states, &[], UnitId::new(1));
    assert_eq!(
        vis,
        Visibility::Visible,
        "LOS through an open hatchway should NOT be blocked"
    );
}

// ===========================================================================
// Section 3.3 -- Intervening models from other units block LOS
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Models block Line of Sight to other models (except their own unit)."
/// Test: An intervening model from a different unit blocks LOS.
#[test]
fn intervening_model_from_other_unit_blocks_los() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let observer = Position::from_inches(2, 5);
    let target = Position::from_inches(8, 5);
    let target_unit = UnitId::new(1);

    // A model from a third unit directly between observer and target
    let intervening = vec![(Position::from_inches(5, 5), UnitId::new(2))];

    let vis = check_visibility(&map, observer, target, &states, &intervening, target_unit);
    assert_eq!(
        vis,
        Visibility::NotVisible,
        "An intervening model from another unit should block LOS"
    );
}

// ===========================================================================
// Section 3.3 -- Models from target's own unit do NOT block LOS
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: Models in the target unit do not block LOS to that unit.
/// Test: A model belonging to the target unit between observer and target
/// does NOT block LOS.
#[test]
fn target_unit_model_does_not_block_los() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let observer = Position::from_inches(2, 5);
    let target = Position::from_inches(8, 5);
    let target_unit = UnitId::new(1);

    // An intervening model from the TARGET unit on the direct path
    let intervening = vec![(Position::from_inches(5, 5), target_unit)];

    let vis = check_visibility(&map, observer, target, &states, &intervening, target_unit);
    assert_eq!(
        vis,
        Visibility::Visible,
        "A model from the target unit should NOT block LOS to that unit"
    );
}

// ===========================================================================
// Section 3.3 -- Indirect Fire removed in Boarding Actions
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "While on the battlefield: weapons lose the Indirect Fire ability."
/// Test: The indirect_fire_removed() function always returns true in BA.
#[test]
fn indirect_fire_is_always_removed_in_ba() {
    assert!(
        indirect_fire_removed(),
        "Indirect Fire should always be removed in Boarding Actions"
    );
}

// ===========================================================================
// Section 3.3 -- Benefit of Cover unless FULLY visible
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "A target has Benefit of Cover unless it is fully visible to at least
///        one attacking model."
/// Test: When the attacker has fully clear LOS (no walls, hatches, etc.), no
/// cover is granted.
#[test]
fn no_cover_when_fully_visible_same_compartment() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // Attacker and target in the same compartment, far from walls
    let attackers = vec![Position::from_inches(2, 5)];
    let target = Position::from_inches(5, 5);

    let cover = check_cover(&map, &attackers, target, &states);
    assert_eq!(
        cover,
        CoverType::None,
        "Target fully visible in the same compartment should NOT have cover"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Benefit of Cover unless FULLY visible."
/// Test: When LOS passes near a wall (partially obstructed), the target gets
/// Benefit of Cover.
#[test]
fn benefit_of_cover_when_near_wall() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // LOS grazes the wall endpoint at (10,4) — target is across the wall boundary
    let attackers = vec![Position::new(
        Inches::from_inches(5),
        Inches::from_inches_frac(4, 500),
    )];
    let target = Position::new(
        Inches::from_inches(15),
        Inches::from_inches_frac(4, 500),
    );

    let cover = check_cover(&map, &attackers, target, &states);
    assert_eq!(
        cover,
        CoverType::BenefitOfCover,
        "Target with LOS grazing a wall should have Benefit of Cover"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "If ANY attacker model has completely clear LOS, no cover is granted."
/// Test: With multiple attackers, if at least one has fully clear LOS, the
/// target gets no cover.
#[test]
fn multiple_attackers_one_clear_los_means_no_cover() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![
        Position::from_inches(5, 5), // clear of walls
        Position::new(
            Inches::from_inches(5),
            Inches::from_inches_frac(4, 500),
        ), // near wall
    ];
    let target = Position::from_inches(7, 5);

    let cover = check_cover(&map, &attackers, target, &states);
    assert_eq!(
        cover,
        CoverType::None,
        "At least one attacker with fully clear LOS should deny cover"
    );
}

// ===========================================================================
// Section 3.3 -- Blast weapon visible model counting
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Blast weapons only count visible models when determining the target
///        unit size for bonus attacks."
/// Test: All target models visible means full count.
#[test]
fn blast_counts_all_visible_models() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![Position::from_inches(2, 5)];
    let target_models = vec![
        Position::from_inches(5, 4),
        Position::from_inches(5, 5),
        Position::from_inches(5, 6),
    ];
    let target_unit = UnitId::new(1);

    let count = blast_visible_count(
        &attackers,
        &target_models,
        &map,
        &states,
        &[],
        target_unit,
    );
    assert_eq!(
        count, 3,
        "All 3 visible target models should be counted for Blast"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Blast weapons only count visible models."
/// Test: A target model behind a wall is not counted for Blast.
#[test]
fn blast_does_not_count_hidden_models() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![Position::from_inches(5, 5)];
    let target_models = vec![
        Position::from_inches(7, 5),  // visible (same compartment)
        Position::from_inches(15, 2), // behind wall at y=2
    ];
    let target_unit = UnitId::new(1);

    let count = blast_visible_count(
        &attackers,
        &target_models,
        &map,
        &states,
        &[],
        target_unit,
    );
    assert_eq!(
        count, 1,
        "Only the visible model should be counted; wall-blocked model excluded"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Blast weapons only count visible models."
/// Test: An intervening model from another unit blocks Blast visibility.
#[test]
fn blast_blocked_by_intervening_model() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![Position::from_inches(2, 5)];
    let target_models = vec![Position::from_inches(8, 5)];
    let target_unit = UnitId::new(1);

    // A model from a third unit directly blocking the path
    let intervening = vec![(Position::from_inches(5, 5), UnitId::new(2))];

    let count = blast_visible_count(
        &attackers,
        &target_models,
        &map,
        &states,
        &intervening,
        target_unit,
    );
    assert_eq!(
        count, 0,
        "An intervening model should block Blast visibility count"
    );
}

// ===========================================================================
// Section 3.5 -- Charge targets must be visible
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.5
/// Rule: "A unit can only be selected as a charge target if it is visible to
///        the charging unit."
/// Test: The charge_target_must_be_visible() flag returns true in BA.
#[test]
fn charge_targets_must_be_visible_in_ba() {
    assert!(
        charge_target_must_be_visible(),
        "Charge targets must be visible in Boarding Actions"
    );
}

// ===========================================================================
// Section 3.3 -- Benefit of Cover: target has cover unless FULLY visible
//                to at least one shooting model
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "A target has Benefit of Cover unless it is FULLY visible to at least
///        one attacking model."
/// Test: When ALL attacker models have partially obstructed LOS (e.g. near a
///       wall edge), the target gets Benefit of Cover because no single model
///       has FULLY clear LOS.
#[test]
fn benefit_of_cover_when_no_attacker_has_fully_clear_los() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // Both attackers positioned so their LOS grazes the wall endpoint at (10,4)
    let attackers = vec![
        Position::new(
            Inches::from_inches(5),
            Inches::from_inches_frac(4, 500),
        ),
        Position::new(
            Inches::from_inches(6),
            Inches::from_inches_frac(4, 500),
        ),
    ];
    let target = Position::new(
        Inches::from_inches(15),
        Inches::from_inches_frac(4, 500),
    );

    let cover = check_cover(&map, &attackers, target, &states);
    assert_eq!(
        cover,
        CoverType::BenefitOfCover,
        "Target should have Benefit of Cover when no attacker model has fully clear LOS"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "A target has Benefit of Cover unless it is FULLY visible to at least
///        one attacking model."
/// Test: When a single attacker model has clear LOS (same compartment, no
///       obstructions), the target does NOT get cover, even if other models
///       in the same unit are blocked.
#[test]
fn no_cover_when_single_attacker_fully_visible_denies_cover() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // One attacker with clear LOS in the same compartment as target
    let attackers = vec![
        Position::from_inches(3, 5), // same compartment, clear LOS
    ];
    let target = Position::from_inches(7, 5);

    let cover = check_cover(&map, &attackers, target, &states);
    assert_eq!(
        cover,
        CoverType::None,
        "One attacker with fully clear LOS should deny Benefit of Cover"
    );
}

// ===========================================================================
// Section 3.3 -- Attack allocation: attacks must be allocated to visible models
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Attacks must be allocated to visible models."
/// Test: blast_visible_count correctly identifies which target models are visible
///       and can receive attack allocations. Models behind walls cannot receive
///       attacks (count = 0 for hidden models).
#[test]
fn attack_allocation_only_visible_models_receive_attacks() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![Position::from_inches(5, 5)];
    // 3 visible models in same compartment, 2 hidden behind wall
    let target_models = vec![
        Position::from_inches(7, 4),   // visible (same compartment)
        Position::from_inches(7, 5),   // visible (same compartment)
        Position::from_inches(7, 6),   // visible (same compartment)
        Position::from_inches(15, 2),  // behind wall (y=2, within wall y=0..4)
        Position::from_inches(15, 8),  // behind wall (y=8, within wall y=6..10)
    ];
    let target_unit = UnitId::new(1);

    let count = blast_visible_count(
        &attackers,
        &target_models,
        &map,
        &states,
        &[],
        target_unit,
    );
    assert_eq!(
        count, 3,
        "Only the 3 visible models should be eligible for attack allocation"
    );
}

/// Source: boarding_actions_complete_v3.md Section 3.3
/// Rule: "Attacks must be allocated to visible models."
/// Test: When an intervening model blocks LOS to some target models but not
///       others, only the unblocked models count as visible for allocation.
#[test]
fn attack_allocation_intervening_model_blocks_some_targets() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let attackers = vec![Position::from_inches(2, 5)];
    let target_unit = UnitId::new(1);

    // Two target models: one directly behind intervening model, one offset
    let target_models = vec![
        Position::from_inches(8, 5),  // directly behind intervening model at (5,5)
        Position::from_inches(8, 8),  // offset, not blocked by model at (5,5)
    ];

    let intervening = vec![(Position::from_inches(5, 5), UnitId::new(2))];

    let count = blast_visible_count(
        &attackers,
        &target_models,
        &map,
        &states,
        &intervening,
        target_unit,
    );
    assert_eq!(
        count, 1,
        "Only the model not blocked by the intervening model should be visible"
    );
}

// ===========================================================================
// Section 3.6 -- Pile-in visibility: cannot end pile-in in ER of unit not
//                visible at start
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.6
/// Rule: "A model making a pile-in move cannot end that move within Engagement
///        Range of a unit that was not visible to it at the start of the Fight
///        phase."
/// Test: A target behind a wall is not visible; check_visibility confirms this,
///       enforcing the pile-in constraint that the game state machine would use.
#[test]
fn pile_in_cannot_end_near_non_visible_unit() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    // Charging model in left compartment, potential pile-in target behind wall
    let charging_model = Position::from_inches(8, 2);
    let target_behind_wall = Position::from_inches(12, 2);

    // The wall at y=0..4 blocks LOS from (8,2) to (12,2) since
    // both are at y=2 and the wall runs along x=10 from y=0 to y=4.
    let vis = check_visibility(
        &map,
        charging_model,
        target_behind_wall,
        &states,
        &[],
        UnitId::new(2),
    );
    assert_eq!(
        vis,
        Visibility::NotVisible,
        "Target behind wall should not be visible; pile-in cannot end in its ER"
    );

    // A visible target through the open hatchway IS valid for pile-in
    let visible_target = Position::from_inches(12, 5);
    let vis_hatch = check_visibility(
        &map,
        Position::from_inches(8, 5),
        visible_target,
        &states,
        &[],
        UnitId::new(2),
    );
    assert_eq!(
        vis_hatch,
        Visibility::Visible,
        "Target visible through open hatchway is a valid pile-in target"
    );
}

// ===========================================================================
// Section 3.6 -- Consolidation: must end closer to closest VISIBLE enemy
// ===========================================================================

/// Source: boarding_actions_complete_v3.md Section 3.6
/// Rule: "When consolidating, a model must end closer to the closest VISIBLE
///        enemy model."
/// Test: Verify that check_visibility correctly determines which enemy models
///       are visible for consolidation targeting. The model must consolidate
///       toward the nearest visible enemy, ignoring non-visible ones.
#[test]
fn consolidation_must_target_closest_visible_enemy() {
    let map = make_two_compartment_map();
    let states = default_hatchway_states(&map);

    let consolidating_model = Position::from_inches(8, 5);
    let _target_unit = UnitId::new(99); // dummy unit, not one of the enemies

    // Enemy A: behind wall at y=2 (closer in straight line but NOT visible)
    let enemy_a = Position::from_inches(12, 2);
    let vis_a = check_visibility(
        &map,
        consolidating_model,
        enemy_a,
        &states,
        &[],
        UnitId::new(10),
    );
    assert_eq!(
        vis_a,
        Visibility::NotVisible,
        "Enemy behind wall should not be visible for consolidation targeting"
    );

    // Enemy B: through open hatchway (further but VISIBLE)
    let enemy_b = Position::from_inches(15, 5);
    let vis_b = check_visibility(
        &map,
        consolidating_model,
        enemy_b,
        &states,
        &[],
        UnitId::new(11),
    );
    assert_eq!(
        vis_b,
        Visibility::Visible,
        "Enemy through open hatchway should be visible for consolidation targeting"
    );

    // The game state machine uses these visibility results to determine
    // that the consolidating model must move toward Enemy B (visible),
    // not Enemy A (not visible, even if closer).
}
