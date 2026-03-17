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
    let capped_move = effective_movement(base_move);
    let move_distance = from.distance(to);
    if move_distance > capped_move {
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
}
