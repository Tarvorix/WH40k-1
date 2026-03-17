//! Boarding Actions Visibility & Cover rules.
//!
//! Implements LOS checks that account for walls, hatchways, and intervening
//! models from other units. Also provides cover determination, Indirect Fire
//! suppression, Blast visibility counting, and charge visibility requirements.
//!
//! Source: boarding_actions_complete_v3.md Section 3.3

use std::collections::HashMap;
use wh40k_core_types::{CoverType, HatchwayId, HatchwayState, Position, UnitId, Visibility};
use wh40k_geometry::boarding::BoardingMap;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check visibility from one position to another in Boarding Actions.
///
/// A target is NOT visible if the line from observer to target passes through:
/// - A Wall
/// - A closed Hatchway
/// - A model NOT in the target unit (intervening model from another unit)
///
/// An open hatchway door does NOT block visibility.
///
/// `intervening_model_positions` contains positions and unit IDs of all models
/// currently on the battlefield that could potentially block LOS (models from
/// units other than the observer's and target's).
///
/// Source: boarding_actions_complete_v3.md Section 3.3
pub fn check_visibility(
    map: &BoardingMap,
    from: Position,
    to: Position,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    intervening_model_positions: &[(Position, UnitId)],
    target_unit: UnitId,
) -> Visibility {
    // Check walls
    if map.is_wall_between(from, to) {
        return Visibility::NotVisible;
    }

    // Check closed/locked hatchways
    if map.is_closed_hatch_between(from, to, hatchway_states) {
        return Visibility::NotVisible;
    }

    // Check intervening models from other units (not target unit)
    // In Boarding Actions, models from other units block LOS much more strictly.
    // We treat each intervening model as a small circle (using a nominal model
    // base radius) and check if the LOS line passes through it.
    //
    // For simplicity and gameplay accuracy, we check if the LOS line passes
    // within a small radius of each intervening model's position.
    // Standard model base radius is approximately 0.5" (500 mils) for a 25mm base.
    // We use a conservative radius for the blocking check.
    let blocking_radius_sq: i64 = {
        // ~0.5 inches = 500 mils radius for model blocking
        let r: i64 = 500;
        r * r
    };

    for (model_pos, model_unit_id) in intervening_model_positions {
        // Skip models in the target unit (they don't block LOS to their own unit)
        if *model_unit_id == target_unit {
            continue;
        }

        // Check if the LOS line passes within the blocking radius of this model
        let dist_sq = wh40k_geometry::boarding::point_to_line_segment_distance_squared(
            *model_pos, from, to,
        );
        if dist_sq <= blocking_radius_sq {
            return Visibility::NotVisible;
        }
    }

    Visibility::Visible
}

/// Check cover for a target position in Boarding Actions.
///
/// A target has BenefitOfCover UNLESS it is fully visible to at least one
/// attacking model. "Fully visible" means no wall, hatchway, or model
/// partially blocks the line.
///
/// In practice, if ANY attacker model has completely clear LOS (no walls,
/// no closed hatches, no intervening models, and no near-miss on walls/frames),
/// then no cover is granted. Otherwise the target has BenefitOfCover.
///
/// `attacker_positions` are the positions of all models in the attacking unit.
/// `target_pos` is the position of the target model.
/// `hatchway_states` is the current state map.
///
/// Source: boarding_actions_complete_v3.md Section 3.3
pub fn check_cover(
    map: &BoardingMap,
    attacker_positions: &[Position],
    target_pos: Position,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> CoverType {
    for attacker_pos in attacker_positions {
        // Check if this attacker has fully unobstructed LOS
        // First, basic LOS check (walls and hatchways)
        let los = map.check_los_boarding(*attacker_pos, target_pos, hatchway_states);
        if los == Visibility::NotVisible {
            continue;
        }

        // If LOS is clear of walls/hatches, check cover from wall proximity
        let cover = map.check_cover_boarding(*attacker_pos, target_pos, hatchway_states);
        if cover == CoverType::None {
            // This attacker has the target fully visible with no cover
            return CoverType::None;
        }
    }

    // No attacker had fully clear LOS without cover
    CoverType::BenefitOfCover
}

/// In Boarding Actions, weapons lose the Indirect Fire ability.
///
/// Source: boarding_actions_complete_v3.md Section 3.3
/// "While on the battlefield: weapons lose the Indirect Fire ability"
///
/// Always returns `true` to indicate that Indirect Fire is removed.
pub fn indirect_fire_removed() -> bool {
    true
}

/// Count only models visible to the attacker for Blast bonus attacks.
///
/// In Boarding Actions, Blast weapons only count visible models when
/// determining the target unit size for bonus attacks.
///
/// `attacker_positions`: positions of models in the attacking unit.
/// `target_unit_models`: positions of models in the target unit.
/// `target_unit`: the UnitId of the target unit.
/// `intervening_models`: all intervening model positions and their unit IDs.
///
/// Returns the count of target models visible to at least one attacker model.
///
/// Source: boarding_actions_complete_v3.md Section 3.3
pub fn blast_visible_count(
    attacker_positions: &[Position],
    target_unit_models: &[Position],
    map: &BoardingMap,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    intervening_models: &[(Position, UnitId)],
    target_unit: UnitId,
) -> usize {
    let mut visible_count = 0;

    for target_pos in target_unit_models {
        let mut model_visible = false;
        for attacker_pos in attacker_positions {
            let vis = check_visibility(
                map,
                *attacker_pos,
                *target_pos,
                hatchway_states,
                intervening_models,
                target_unit,
            );
            if vis == Visibility::Visible {
                model_visible = true;
                break;
            }
        }
        if model_visible {
            visible_count += 1;
        }
    }

    visible_count
}

/// In Boarding Actions, charge targets must be visible to the charging unit.
///
/// Source: boarding_actions_complete_v3.md Section 3.5
/// "A unit can only be selected as a charge target if it is visible to the
/// charging unit."
///
/// Always returns `true` to indicate this requirement is active.
pub fn charge_target_must_be_visible() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Multi-level cross-board visibility block (BA-16)
// ---------------------------------------------------------------------------

/// Check if two positions are on different board levels (BA-05 Power the Generators).
/// Units on different levels cannot see each other.
/// Source: boarding_actions_missions_complete_v3.md §4.5
pub fn are_on_different_levels(
    pos_a: wh40k_core_types::Position,
    pos_b: wh40k_core_types::Position,
    board_seam_x: i32,
) -> bool {
    // Board 1 is x < seam, Board 2 is x >= seam
    // In multi-level missions, each board is a different level
    let a_board = if pos_a.x.whole_inches() < board_seam_x { 0 } else { 1 };
    let b_board = if pos_b.x.whole_inches() < board_seam_x { 0 } else { 1 };
    a_board != b_board
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        BoardDimensions, CompartmentId, HatchwayOrientation, Inches, Polygon,
    };
    use wh40k_geometry::boarding::{
        BoardingMap, Compartment, Hatchway, WallSegment,
    };

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
            initial_state: HatchwayState::Open,
            tags: vec![],
        });

        map
    }

    fn default_states(map: &BoardingMap) -> HashMap<HatchwayId, HatchwayState> {
        map.hatchways
            .iter()
            .map(|h| (h.id, h.initial_state))
            .collect()
    }

    #[test]
    fn test_visibility_clear_los() {
        let map = make_test_map();
        let states = default_states(&map);
        // Same compartment, no obstacles
        let from = Position::from_inches(2, 5);
        let to = Position::from_inches(7, 5);
        let vis = check_visibility(&map, from, to, &states, &[], UnitId::new(1));
        assert_eq!(vis, Visibility::Visible);
    }

    #[test]
    fn test_visibility_blocked_by_wall() {
        let map = make_test_map();
        let states = default_states(&map);
        // LOS crosses wall at y=2
        let from = Position::from_inches(5, 2);
        let to = Position::from_inches(15, 2);
        let vis = check_visibility(&map, from, to, &states, &[], UnitId::new(1));
        assert_eq!(vis, Visibility::NotVisible);
    }

    #[test]
    fn test_visibility_through_open_hatchway() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);
        let vis = check_visibility(&map, from, to, &states, &[], UnitId::new(1));
        assert_eq!(vis, Visibility::Visible);
    }

    #[test]
    fn test_visibility_blocked_by_closed_hatch() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Closed);
        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);
        let vis = check_visibility(&map, from, to, &states, &[], UnitId::new(1));
        assert_eq!(vis, Visibility::NotVisible);
    }

    #[test]
    fn test_visibility_blocked_by_intervening_model() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(2, 5);
        let to = Position::from_inches(8, 5);
        // Model from another unit blocking the path
        let intervening = vec![(Position::from_inches(5, 5), UnitId::new(2))];
        let vis = check_visibility(&map, from, to, &states, &intervening, UnitId::new(1));
        assert_eq!(vis, Visibility::NotVisible);
    }

    #[test]
    fn test_visibility_target_unit_model_does_not_block() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(2, 5);
        let to = Position::from_inches(8, 5);
        // Model from the target unit on the path — should NOT block
        let intervening = vec![(Position::from_inches(5, 5), UnitId::new(1))];
        let vis = check_visibility(&map, from, to, &states, &intervening, UnitId::new(1));
        assert_eq!(vis, Visibility::Visible);
    }

    #[test]
    fn test_cover_no_cover_when_fully_visible() {
        let map = make_test_map();
        let states = default_states(&map);
        // Attacker and target in the same compartment, far from walls
        let attackers = vec![Position::from_inches(2, 5)];
        let target = Position::from_inches(5, 5);
        let cover = check_cover(&map, &attackers, target, &states);
        assert_eq!(cover, CoverType::None);
    }

    #[test]
    fn test_cover_benefit_near_wall() {
        let map = make_test_map();
        let states = default_states(&map);
        // LOS passes near a wall endpoint at (10, 4)
        let attackers = vec![Position::new(
            Inches::from_inches(5),
            Inches::from_inches_frac(4, 500),
        )];
        let target = Position::new(
            Inches::from_inches(15),
            Inches::from_inches_frac(4, 500),
        );
        let cover = check_cover(&map, &attackers, target, &states);
        assert_eq!(cover, CoverType::BenefitOfCover);
    }

    #[test]
    fn test_cover_multiple_attackers_one_clear() {
        let map = make_test_map();
        let states = default_states(&map);
        // One attacker has clear LOS far from walls, the other doesn't
        let attackers = vec![
            Position::from_inches(5, 5), // clear of walls
            Position::new(
                Inches::from_inches(5),
                Inches::from_inches_frac(4, 500),
            ), // near wall
        ];
        let target = Position::from_inches(7, 5);
        let cover = check_cover(&map, &attackers, target, &states);
        // At least one attacker has fully clear LOS => no cover
        assert_eq!(cover, CoverType::None);
    }

    #[test]
    fn test_indirect_fire_removed() {
        assert!(indirect_fire_removed());
    }

    #[test]
    fn test_charge_target_must_be_visible() {
        assert!(charge_target_must_be_visible());
    }

    #[test]
    fn test_blast_visible_count_all_visible() {
        let map = make_test_map();
        let states = default_states(&map);
        let attackers = vec![Position::from_inches(2, 5)];
        let target_models = vec![
            Position::from_inches(5, 4),
            Position::from_inches(5, 5),
            Position::from_inches(5, 6),
        ];
        let count = blast_visible_count(
            &attackers,
            &target_models,
            &map,
            &states,
            &[],
            UnitId::new(1),
        );
        assert_eq!(count, 3);
    }

    #[test]
    fn test_blast_visible_count_some_hidden() {
        let map = make_test_map();
        let states = default_states(&map);
        let attackers = vec![Position::from_inches(5, 5)];
        // One target behind the wall
        let target_models = vec![
            Position::from_inches(7, 5), // visible (same compartment)
            Position::from_inches(15, 2), // not visible (wall blocks at y=2)
        ];
        let count = blast_visible_count(
            &attackers,
            &target_models,
            &map,
            &states,
            &[],
            UnitId::new(1),
        );
        assert_eq!(count, 1);
    }

    #[test]
    fn test_blast_visible_count_blocked_by_intervening() {
        let map = make_test_map();
        let states = default_states(&map);
        let attackers = vec![Position::from_inches(2, 5)];
        let target_models = vec![
            Position::from_inches(8, 5), // blocked by intervening model
        ];
        let intervening = vec![(Position::from_inches(5, 5), UnitId::new(2))];
        let count = blast_visible_count(
            &attackers,
            &target_models,
            &map,
            &states,
            &intervening,
            UnitId::new(1),
        );
        assert_eq!(count, 0);
    }
}
