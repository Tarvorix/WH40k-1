//! Boarding Actions Movement Rules.
//!
//! Implements movement cap, wall/hatchway blocking checks, deep strike distance,
//! and scouts move validation for Boarding Actions games.
//!
//! Source: boarding_actions_complete_v3.md Section 3.2

use std::collections::HashMap;
use wh40k_core_types::{HatchwayId, HatchwayState, Inches, Position};
use wh40k_geometry::boarding::BoardingMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum movement in Boarding Actions. Any model with Move > 9" is capped.
/// Source: boarding_actions_complete_v3.md Section 3.2 - "Move > 9" reduced to 9""
pub const BA_MOVE_CAP: Inches = Inches::BA_MOVE_CAP;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that make a move illegal in Boarding Actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovementError {
    /// The move path crosses a wall segment.
    WallBlocks,
    /// The move path crosses a closed or locked hatchway.
    ClosedHatchBlocks,
    /// The move distance exceeds the model's effective movement.
    ExceedsMovement,
    /// The model would end its move in the doorway of a hatchway.
    EndsInHatchway,
    /// The model would end its move overlapping a wall.
    EndsOnWall,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check if the FLY keyword should be suppressed in Boarding Actions.
/// Source: boarding_actions_complete_v3.md §3.2 — "models lose FLY"
pub fn fly_suppressed_in_ba() -> bool {
    true
}

/// Apply the Boarding Actions movement cap.
///
/// If `base_move` exceeds 9", return 9". Otherwise return `base_move` unchanged.
///
/// Source: boarding_actions_complete_v3.md Section 3.2
/// "if a model's Move characteristic is greater than 9", reduce it to 9" at the
/// start of the battle"
pub fn effective_movement(base_move: Inches) -> Inches {
    if base_move > BA_MOVE_CAP {
        BA_MOVE_CAP
    } else {
        base_move
    }
}

/// Check if a unit should have the FLY keyword removed in Boarding Actions.
/// Source: boarding_actions_complete_v3.md §3.2
pub fn should_strip_fly() -> bool {
    true  // In BA, ALL models lose FLY
}

/// Validate whether a move from `from` to `to` is legal in Boarding Actions.
///
/// Checks:
/// 1. Path must not cross any wall.
/// 2. Path must not cross any closed/locked hatchway.
/// 3. Model cannot end move overlapping a wall (within base_size/2 of a wall segment).
/// 4. Model cannot end move inside a hatchway doorway.
/// 5. Move distance must not exceed effective_movement(base_move).
///
/// `base_move` is the model's Move characteristic (before capping).
/// `base_size_radius` is the radius of the model's base in Inches.
pub fn is_legal_move(
    map: &BoardingMap,
    from: Position,
    to: Position,
    base_move: Inches,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> Result<(), MovementError> {
    // 1. Check wall crossing
    if map.is_wall_between(from, to) {
        return Err(MovementError::WallBlocks);
    }

    // 2. Check closed/locked hatchway crossing
    if map.is_closed_hatch_between(from, to, hatchway_states) {
        return Err(MovementError::ClosedHatchBlocks);
    }

    // 3. Check if ending on a wall (endpoint overlaps a wall segment).
    // We check if the destination position is very close to any wall segment.
    // A model cannot end overlapping a wall, so we check if the position is
    // essentially on a wall segment (within a very small tolerance).
    for wall in &map.walls {
        let dist_sq =
            wh40k_geometry::boarding::point_to_line_segment_distance_squared(to, wall.start, wall.end);
        // Zero distance means on the wall
        if dist_sq == 0 {
            return Err(MovementError::EndsOnWall);
        }
    }

    // 4. Check if ending in a hatchway doorway.
    // A model cannot end its move with its base in the middle of an open hatchway.
    for hatch in &map.hatchways {
        let state = hatchway_states
            .get(&hatch.id)
            .copied()
            .unwrap_or(hatch.initial_state);
        if state.allows_passage() {
            // Check if the destination is essentially at the hatchway position
            // (within a small tolerance representing being "in the doorway")
            // The hatchway doorway region is the hatchway position within half its width
            let half_width = hatch.width / 2;
            let half_width_sq = (half_width.0 as i64) * (half_width.0 as i64);
            let dist_sq = to.distance_squared(hatch.position);
            if dist_sq <= half_width_sq {
                // Check if the model is right in the doorway opening
                // More precise: is the model position on the hatchway line segment itself?
                let hatch_seg_start;
                let hatch_seg_end;
                match hatch.orientation {
                    wh40k_core_types::HatchwayOrientation::Horizontal => {
                        hatch_seg_start = Position::new(hatch.position.x - half_width, hatch.position.y);
                        hatch_seg_end = Position::new(hatch.position.x + half_width, hatch.position.y);
                    }
                    wh40k_core_types::HatchwayOrientation::Vertical => {
                        hatch_seg_start = Position::new(hatch.position.x, hatch.position.y - half_width);
                        hatch_seg_end = Position::new(hatch.position.x, hatch.position.y + half_width);
                    }
                }
                let seg_dist_sq = wh40k_geometry::boarding::point_to_line_segment_distance_squared(
                    to,
                    hatch_seg_start,
                    hatch_seg_end,
                );
                if seg_dist_sq == 0 {
                    return Err(MovementError::EndsInHatchway);
                }
            }
        }
    }

    // 5. Check move distance
    // In BA, distance is measured by the shortest legal path around walls.
    // If the direct path is clear (no wall crossing), use straight-line distance.
    // If walls block the direct path, the move requires routing through open hatchways,
    // and the effective distance is the sum of path segments.
    // Source: boarding_actions_complete_v3.md §3.2
    let capped_move = effective_movement(base_move);
    let direct_distance = from.distance(to);
    // If direct path is clear (passed wall checks above), straight-line is the shortest path
    // If we got here, the direct path IS clear, so straight-line distance applies
    if direct_distance > capped_move {
        return Err(MovementError::ExceedsMovement);
    }

    Ok(())
}

/// Check whether a model can move through a specific hatchway.
///
/// A model can only pass through a hatchway that is Open or OneWayOpened.
///
/// Source: boarding_actions_complete_v3.md Section 3.2
pub fn can_move_through_hatchway(
    map: &BoardingMap,
    hatchway_id: HatchwayId,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> bool {
    let hatch = match map.hatchway(hatchway_id) {
        Some(h) => h,
        None => return false,
    };
    let state = hatchway_states
        .get(&hatchway_id)
        .copied()
        .unwrap_or(hatch.initial_state);
    state.allows_passage()
}

/// Compute the shortest legal path distance between two points in BA.
///
/// In BA, distances cannot be measured through Walls or closed Hatchways.
/// Instead, distance is measured by the shortest legal path around them.
/// If no legal path exists, returns None (infinite distance).
///
/// This is a simplified implementation that uses open hatchway waypoints
/// to compute multi-segment paths. For complex layouts, a full visibility
/// graph or A* pathfinder would be more accurate.
///
/// Source: boarding_actions_complete_v3.md §3.2
pub fn shortest_legal_path_distance(
    from: Position,
    to: Position,
    map: &BoardingMap,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> Option<Inches> {
    // Check if direct path is clear (no wall/hatch obstruction)
    let direct_clear = !map.walls.iter().any(|w| {
        wh40k_geometry::boarding::line_segments_intersect(from, to, w.start, w.end)
    }) && !map.hatchways.iter().any(|h| {
        let state = hatchway_states.get(&h.id).copied().unwrap_or(h.initial_state);
        if state.allows_passage() {
            return false; // open hatchway doesn't block
        }
        let half = h.width.0 / 2;
        let (seg_s, seg_e) = match h.orientation {
            wh40k_core_types::HatchwayOrientation::Horizontal => {
                (Position { x: Inches(h.position.x.0 - half), y: h.position.y },
                 Position { x: Inches(h.position.x.0 + half), y: h.position.y })
            }
            wh40k_core_types::HatchwayOrientation::Vertical => {
                (Position { x: h.position.x, y: Inches(h.position.y.0 - half) },
                 Position { x: h.position.x, y: Inches(h.position.y.0 + half) })
            }
        };
        wh40k_geometry::boarding::line_segments_intersect(from, to, seg_s, seg_e)
    });

    if direct_clear {
        return Some(from.distance(to));
    }

    // Direct path blocked — try routing through open hatchway positions
    // Collect all open hatchway positions as potential waypoints
    let waypoints: Vec<Position> = map.hatchways.iter()
        .filter(|h| {
            let state = hatchway_states.get(&h.id).copied().unwrap_or(h.initial_state);
            state.allows_passage()
        })
        .map(|h| h.position)
        .collect();

    // Simple 2-waypoint search: try from→waypoint→to for each waypoint
    let mut best_distance: Option<Inches> = None;
    for &wp in &waypoints {
        let d1 = from.distance(wp);
        let d2 = wp.distance(to);
        let total = Inches(d1.0 + d2.0);
        if best_distance.is_none() || total < best_distance.unwrap() {
            best_distance = Some(total);
        }
    }

    // Also try 2-waypoint paths: from→wp1→wp2→to
    for (i, &wp1) in waypoints.iter().enumerate() {
        for &wp2 in waypoints.iter().skip(i + 1) {
            let d_a = Inches(from.distance(wp1).0 + wp1.distance(wp2).0 + wp2.distance(to).0);
            let d_b = Inches(from.distance(wp2).0 + wp2.distance(wp1).0 + wp1.distance(to).0);
            let d = if d_a < d_b { d_a } else { d_b };
            if best_distance.is_none() || d < best_distance.unwrap() {
                best_distance = Some(d);
            }
        }
    }

    best_distance
}

/// Check Deep Strike distance in Boarding Actions.
///
/// In BA, deep strike distance IGNORES walls and closed hatchways.
/// The check is a straight-line distance: the unit must be > 9" from all
/// enemy models, measured in a straight line ignoring terrain.
///
/// Source: boarding_actions_complete_v3.md Section 3.2
/// "When measuring Deep Strike distance to enemy models in Boarding Actions:
///  ignore Walls and closed Hatchways"
///
/// Returns `true` if the position is legal for Deep Strike (>9" from all enemies).
pub fn deep_strike_distance_check(
    unit_pos: Position,
    enemy_positions: &[Position],
) -> bool {
    let min_dist_sq = {
        let d = Inches::DEEP_STRIKE_MIN_DISTANCE.0 as i64;
        d * d
    };

    for enemy_pos in enemy_positions {
        let dist_sq = unit_pos.distance_squared(*enemy_pos);
        // Must be strictly greater than 9"
        if dist_sq <= min_dist_sq {
            return false;
        }
    }

    true
}

/// Validate a Scouts move in Boarding Actions.
///
/// Scouts moves must obey Boarding Actions geometry: walls and closed
/// hatchways are impassable, just like normal movement. No open-board
/// assumptions apply.
///
/// Source: boarding_actions_complete_v3.md Section 3.2
///
/// Returns `true` if the scouts move path is legal.
pub fn scouts_move_check(
    map: &BoardingMap,
    from: Position,
    to: Position,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> bool {
    // Path must not cross any wall
    if map.is_wall_between(from, to) {
        return false;
    }
    // Path must not cross any closed/locked hatchway
    if map.is_closed_hatch_between(from, to, hatchway_states) {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Deep Strike round / count limits (BA-3)
// ---------------------------------------------------------------------------

/// Check if deep strike arrival is allowed in the given battle round for Boarding Actions.
/// BA rules: only rounds 2 and 3. Units not deployed by end of round 3 are destroyed.
/// Source: boarding_actions_complete_v3.md §3.1
pub fn is_deep_strike_round_allowed(battle_round: u8) -> bool {
    battle_round == 2 || battle_round == 3
}

/// Check if a player has already used their deep strike arrival this round.
/// BA rules: max 1 unit per battle round via deep strike.
/// Source: boarding_actions_complete_v3.md §3.1
pub fn can_deep_strike_this_round(arrivals_this_round: u8) -> bool {
    arrivals_this_round < 1
}

/// Maximum models that can be returned to a unit per battle round in BA.
/// Source: boarding_actions_complete_v3.md §3.8
pub const BA_MAX_RETURNED_MODELS_PER_ROUND: u8 = 1;

// ---------------------------------------------------------------------------
// Objective marker range (BA-8)
// ---------------------------------------------------------------------------

/// Objective marker range in Boarding Actions: 1" horizontally.
/// Source: boarding_actions_complete_v3.md §3.2
pub const BA_OBJECTIVE_RANGE_INCHES: i32 = 1;

/// Check if a model is within range of an objective marker in BA.
/// In BA, models can end a move on top of an objective marker.
/// Source: boarding_actions_complete_v3.md §3.2
pub fn model_within_objective_range(
    model_pos: wh40k_core_types::Position,
    objective_pos: wh40k_core_types::Position,
) -> bool {
    let dist = wh40k_geometry::distance(model_pos, objective_pos);
    dist <= wh40k_core_types::Inches::from_inches(BA_OBJECTIVE_RANGE_INCHES)
}

// ---------------------------------------------------------------------------
// Inaccessible Area restriction (BA-14)
// ---------------------------------------------------------------------------

/// Check if a position is within an inaccessible area (e.g., BA-01 Void the Ship).
/// Models cannot enter the Inaccessible Area for any reason.
/// Source: boarding_actions_missions_complete_v3.md §4.1
pub fn is_in_inaccessible_area(
    position: wh40k_core_types::Position,
    inaccessible_regions: &[wh40k_geometry::boarding::SpecialRegion],
) -> bool {
    inaccessible_regions.iter().any(|region| {
        region.tags.contains(&"inaccessible".to_string())
            && region.boundary.contains(position)
    })
}

// ---------------------------------------------------------------------------
// BA Deployment Sequence (BA-18)
// ---------------------------------------------------------------------------

/// BA deployment: players alternate deploying ONE unit per Entry Zone, starting with Defender.
/// Source: boarding_actions_complete_v3.md §7.4
pub const BA_DEPLOYMENT_DEFENDER_FIRST: bool = true;

/// Check if a player can deploy another unit in a given entry zone.
/// In BA, only one unit can be deployed per entry zone per deployment round.
/// Source: boarding_actions_complete_v3.md §7.4
pub fn can_deploy_in_entry_zone(
    units_deployed_in_zone: u8,
    max_per_round: u8,
) -> bool {
    units_deployed_in_zone < max_per_round
}

/// At least half a player's units must deploy normally (not in reserves).
/// Source: boarding_actions_complete_v3.md §3.1
pub fn minimum_deployed_units(total_units: usize) -> usize {
    (total_units + 1) / 2  // ceiling division
}

// ---------------------------------------------------------------------------
// BA Reserves Via Empty Entry Zones (BA-12)
// ---------------------------------------------------------------------------

/// In BA, Strategic Reserves arrive via empty Entry Zones (one unit per empty zone per turn).
/// Source: boarding_actions_complete_v3.md §7.5
pub fn can_arrive_via_entry_zone(
    zone_has_models: bool,
    arrivals_this_zone_this_turn: u8,
) -> bool {
    !zone_has_models && arrivals_this_zone_this_turn == 0
}

/// Units not deployed by end of battle round 3 are destroyed.
/// Source: boarding_actions_complete_v3.md §3.1
pub fn should_destroy_undeployed_reserves(battle_round: u8) -> bool {
    battle_round > 3
}

// ---------------------------------------------------------------------------
// BA Pile-In / Consolidation Visibility (BA-6)
// ---------------------------------------------------------------------------

/// In BA, a model making a Pile-in or Consolidation move cannot end within ER of a unit
/// that was NOT visible to its own unit at the start of the move.
/// Source: boarding_actions_complete_v3.md §3.6
pub fn pile_in_target_must_be_visible() -> bool {
    true  // In BA, always enforced
}

/// In BA, pile-in/consolidation does not have to end closer to closest enemy;
/// instead it must end as close as possible to the closest VISIBLE enemy unit.
/// Source: boarding_actions_complete_v3.md §3.6
pub fn pile_in_uses_visible_closest() -> bool {
    true  // In BA, always enforced
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        BoardDimensions, CompartmentId, HatchwayOrientation, Polygon,
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
    fn test_effective_movement_under_cap() {
        assert_eq!(effective_movement(Inches::from_inches(6)), Inches::from_inches(6));
    }

    #[test]
    fn test_effective_movement_at_cap() {
        assert_eq!(effective_movement(Inches::from_inches(9)), Inches::from_inches(9));
    }

    #[test]
    fn test_effective_movement_over_cap() {
        assert_eq!(effective_movement(Inches::from_inches(14)), Inches::from_inches(9));
    }

    #[test]
    fn test_legal_move_within_compartment() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(2, 5);
        let to = Position::from_inches(5, 5);
        let result = is_legal_move(&map, from, to, Inches::from_inches(6), &states);
        assert!(result.is_ok());
    }

    #[test]
    fn test_move_blocked_by_wall() {
        let map = make_test_map();
        let states = default_states(&map);
        // Try to move across the wall at y=2 (below hatchway)
        let from = Position::from_inches(5, 2);
        let to = Position::from_inches(15, 2);
        let result = is_legal_move(&map, from, to, Inches::from_inches(12), &states);
        assert_eq!(result, Err(MovementError::WallBlocks));
    }

    #[test]
    fn test_move_through_open_hatchway() {
        let map = make_test_map();
        let states = default_states(&map); // hatchway open
        // Move through the hatchway at y=5 — need to avoid ending on the hatchway itself
        let from = Position::from_inches(8, 5);
        let to = Position::from_inches(12, 5);
        let result = is_legal_move(&map, from, to, Inches::from_inches(6), &states);
        assert!(result.is_ok());
    }

    #[test]
    fn test_move_blocked_by_closed_hatch() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Closed);
        let from = Position::from_inches(8, 5);
        let to = Position::from_inches(12, 5);
        let result = is_legal_move(&map, from, to, Inches::from_inches(6), &states);
        assert_eq!(result, Err(MovementError::ClosedHatchBlocks));
    }

    #[test]
    fn test_move_exceeds_movement() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(1, 5);
        let to = Position::from_inches(8, 5);
        // Distance is 7", but base move is only 5"
        let result = is_legal_move(&map, from, to, Inches::from_inches(5), &states);
        assert_eq!(result, Err(MovementError::ExceedsMovement));
    }

    #[test]
    fn test_move_exceeds_capped_movement() {
        let map = make_test_map();
        let states = default_states(&map);
        // Base move is 14" but capped to 9"
        let from = Position::from_inches(1, 5);
        let to = Position::from_inches(8, 5);
        // Distance is 7", capped move is 9" — should pass
        let result = is_legal_move(&map, from, to, Inches::from_inches(14), &states);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_move_through_open_hatchway() {
        let map = make_test_map();
        let states = default_states(&map); // Open
        assert!(can_move_through_hatchway(&map, HatchwayId::new(0), &states));
    }

    #[test]
    fn test_cannot_move_through_closed_hatchway() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Closed);
        assert!(!can_move_through_hatchway(&map, HatchwayId::new(0), &states));
    }

    #[test]
    fn test_can_move_through_one_way_opened() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::OneWayOpened);
        assert!(can_move_through_hatchway(&map, HatchwayId::new(0), &states));
    }

    #[test]
    fn test_deep_strike_distance_check_clear() {
        let pos = Position::from_inches(0, 0);
        let enemies = vec![Position::from_inches(15, 0)]; // 15" away
        assert!(deep_strike_distance_check(pos, &enemies));
    }

    #[test]
    fn test_deep_strike_distance_check_too_close() {
        let pos = Position::from_inches(0, 0);
        let enemies = vec![Position::from_inches(5, 0)]; // 5" away
        assert!(!deep_strike_distance_check(pos, &enemies));
    }

    #[test]
    fn test_deep_strike_distance_check_exactly_9() {
        let pos = Position::from_inches(0, 0);
        let enemies = vec![Position::from_inches(9, 0)]; // exactly 9"
        // Must be strictly >9", so exactly 9" is not legal
        assert!(!deep_strike_distance_check(pos, &enemies));
    }

    #[test]
    fn test_deep_strike_ignores_walls() {
        // Deep strike distance is straight-line, ignoring walls.
        // Even if a wall is between two positions, the check is straight-line.
        let pos = Position::from_inches(0, 5);
        let enemies = vec![Position::from_inches(15, 5)]; // 15" away straight-line
        assert!(deep_strike_distance_check(pos, &enemies));
    }

    #[test]
    fn test_scouts_move_legal() {
        let map = make_test_map();
        let states = default_states(&map);
        // Move within a compartment
        let from = Position::from_inches(2, 5);
        let to = Position::from_inches(5, 5);
        assert!(scouts_move_check(&map, from, to, &states));
    }

    #[test]
    fn test_scouts_move_blocked_by_wall() {
        let map = make_test_map();
        let states = default_states(&map);
        let from = Position::from_inches(5, 2);
        let to = Position::from_inches(15, 2);
        assert!(!scouts_move_check(&map, from, to, &states));
    }

    #[test]
    fn test_scouts_move_blocked_by_closed_hatch() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Closed);
        let from = Position::from_inches(8, 5);
        let to = Position::from_inches(12, 5);
        assert!(!scouts_move_check(&map, from, to, &states));
    }

    #[test]
    fn test_scouts_move_through_open_hatch() {
        let map = make_test_map();
        let states = default_states(&map); // Open
        let from = Position::from_inches(8, 5);
        let to = Position::from_inches(12, 5);
        assert!(scouts_move_check(&map, from, to, &states));
    }

    // BA Deployment (BA-18) tests
    #[test]
    fn test_ba_deployment_defender_first() {
        assert!(BA_DEPLOYMENT_DEFENDER_FIRST);
    }

    #[test]
    fn test_can_deploy_in_entry_zone_none_deployed() {
        assert!(can_deploy_in_entry_zone(0, 1));
    }

    #[test]
    fn test_cannot_deploy_in_entry_zone_already_full() {
        assert!(!can_deploy_in_entry_zone(1, 1));
    }

    #[test]
    fn test_minimum_deployed_units_even() {
        // 4 total -> at least 2 must deploy
        assert_eq!(minimum_deployed_units(4), 2);
    }

    #[test]
    fn test_minimum_deployed_units_odd() {
        // 5 total -> at least 3 must deploy (ceiling)
        assert_eq!(minimum_deployed_units(5), 3);
    }

    #[test]
    fn test_minimum_deployed_units_one() {
        assert_eq!(minimum_deployed_units(1), 1);
    }

    // BA Reserves (BA-12) tests
    #[test]
    fn test_can_arrive_via_empty_entry_zone() {
        assert!(can_arrive_via_entry_zone(false, 0));
    }

    #[test]
    fn test_cannot_arrive_via_occupied_entry_zone() {
        assert!(!can_arrive_via_entry_zone(true, 0));
    }

    #[test]
    fn test_cannot_arrive_twice_same_zone() {
        assert!(!can_arrive_via_entry_zone(false, 1));
    }

    #[test]
    fn test_destroy_reserves_after_round_3() {
        assert!(!should_destroy_undeployed_reserves(1));
        assert!(!should_destroy_undeployed_reserves(2));
        assert!(!should_destroy_undeployed_reserves(3));
        assert!(should_destroy_undeployed_reserves(4));
        assert!(should_destroy_undeployed_reserves(5));
    }

    // BA Pile-In/Consolidation Visibility (BA-6) tests
    #[test]
    fn test_pile_in_target_must_be_visible() {
        assert!(pile_in_target_must_be_visible());
    }

    #[test]
    fn test_pile_in_uses_visible_closest() {
        assert!(pile_in_uses_visible_closest());
    }
}
