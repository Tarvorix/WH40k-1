//! Hatchway State Machine for Boarding Actions.
//!
//! Implements hatchway operation rules: opening, closing, roll-offs,
//! split-unit blocking, and opening-into-engagement detection.
//!
//! Source: boarding_actions_complete_v3.md Section 3.2

use std::collections::HashMap;
use wh40k_core_types::{HatchwayId, HatchwayState, Inches, Position, UnitId};
use wh40k_geometry::boarding::BoardingMap;

// ---------------------------------------------------------------------------
// Error & result types
// ---------------------------------------------------------------------------

/// Errors that prevent a hatchway operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HatchwayError {
    /// The unit is within engagement range of an enemy and cannot operate.
    InEngagementRange,
    /// No model in the unit is within 1" of the hatchway.
    NotInRange,
    /// The hatchway cannot be operated (Locked or OneWayOpened).
    NotOperable,
    /// Closing is blocked because models from the same unit are on opposite sides.
    SplitUnitBlocks,
    /// The hatchway ID was not found on the map.
    HatchwayNotFound,
}

/// Result of resolving a hatchway operation (including the roll-off if contested).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HatchwayOperationResult {
    /// Whether the hatchway state actually changed.
    pub success: bool,
    /// The new state of the hatchway after the operation attempt.
    pub new_state: HatchwayState,
    /// Any pairs of units that are now newly engaged through the opened hatchway.
    pub newly_engaged_pairs: Vec<(UnitId, UnitId)>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a unit is allowed to attempt to operate a hatchway.
///
/// Rules (Section 3.2):
/// - Unit must NOT be in engagement range of any enemy.
/// - Hatchway must be within 1" of at least one model in the unit.
/// - Hatchway must be operable (not Locked, not OneWayOpened).
///
/// `unit_positions` are the positions of all models in the operating unit.
/// `engagement_status_engaged` is true if the unit is within engagement range
/// of any enemy model.
pub fn can_operate_hatchway(
    map: &BoardingMap,
    hatchway_id: HatchwayId,
    unit_positions: &[Position],
    engagement_status_engaged: bool,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
) -> Result<(), HatchwayError> {
    // Must not be in engagement range
    if engagement_status_engaged {
        return Err(HatchwayError::InEngagementRange);
    }

    // Look up the hatchway on the map
    let hatch = map.hatchway(hatchway_id).ok_or(HatchwayError::HatchwayNotFound)?;

    // Get current state
    let current_state = hatchway_states
        .get(&hatchway_id)
        .copied()
        .unwrap_or(hatch.initial_state);

    // Hatchway must be operable (only Open and Closed are toggleable)
    if !current_state.can_operate() {
        return Err(HatchwayError::NotOperable);
    }

    // At least one model in the unit must be within 1" of the hatchway
    let any_in_range = unit_positions
        .iter()
        .any(|pos| map.is_within_hatchway_operate_range(*pos, hatchway_id));
    if !any_in_range {
        return Err(HatchwayError::NotInRange);
    }

    Ok(())
}

/// Resolve a hatchway operation, including the roll-off if the opponent contests.
///
/// Rules (Section 3.2):
/// - If opponent has units on the opposite side within 1", they may attempt to prevent.
/// - Roll-off: each player rolls + adds Toughness of one model.
/// - If the operating player wins (or ties — operating player wins ties), state changes.
/// - If the defender declines to contest, state changes automatically.
///
/// Parameters:
/// - `operating_roll`: the operating player's die roll (1-6)
/// - `operating_unit_toughness`: Toughness characteristic of one model in operating unit
/// - `opponent_can_prevent`: whether there is an eligible opponent unit on the other side
/// - `opponent_unit_toughness`: Toughness of one model in the preventing unit (ignored if not contesting)
/// - `preventing_roll`: the preventing player's die roll (0 if not contesting)
/// - `unit_positions_a` / `unit_positions_b`: positions of potentially newly-engaged units
///   on side A (operating player's side) and side B (opponent's side) of the hatchway.
///
/// Returns the result including whether the toggle succeeded and any newly engaged pairs.
pub fn resolve_hatchway_operation(
    map: &BoardingMap,
    hatchway_id: HatchwayId,
    hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    operating_unit_toughness: u8,
    opponent_can_prevent: bool,
    opponent_unit_toughness: u8,
    operating_roll: u8,
    preventing_roll: u8,
    // For engagement detection after opening
    units_side_a: &[(UnitId, Vec<Position>)],
    units_side_b: &[(UnitId, Vec<Position>)],
) -> HatchwayOperationResult {
    let hatch = match map.hatchway(hatchway_id) {
        Some(h) => h,
        None => {
            return HatchwayOperationResult {
                success: false,
                new_state: HatchwayState::Closed,
                newly_engaged_pairs: Vec::new(),
            };
        }
    };

    let current_state = hatchway_states
        .get(&hatchway_id)
        .copied()
        .unwrap_or(hatch.initial_state);

    // Determine if the operation succeeds
    let operation_succeeds = if !opponent_can_prevent {
        // Defender declines or has no unit => auto-success
        true
    } else {
        // Roll-off: operating total vs preventing total
        let operating_total = operating_roll as u16 + operating_unit_toughness as u16;
        let preventing_total = preventing_roll as u16 + opponent_unit_toughness as u16;
        // Operating player wins on tie
        operating_total >= preventing_total
    };

    if !operation_succeeds {
        return HatchwayOperationResult {
            success: false,
            new_state: current_state,
            newly_engaged_pairs: Vec::new(),
        };
    }

    // Toggle the state
    let new_state = match current_state.toggle() {
        Some(s) => s,
        None => {
            // Should not happen if can_operate was checked, but be safe
            return HatchwayOperationResult {
                success: false,
                new_state: current_state,
                newly_engaged_pairs: Vec::new(),
            };
        }
    };

    // Check for newly engaged pairs if opening
    let newly_engaged_pairs = if new_state == HatchwayState::Open {
        check_opening_engagement(map, hatchway_id, units_side_a, units_side_b)
    } else {
        Vec::new()
    };

    HatchwayOperationResult {
        success: true,
        new_state,
        newly_engaged_pairs,
    }
}

/// Check whether a hatchway can be closed, given unit positions.
///
/// A hatchway cannot be closed if models from the SAME unit are on opposite sides.
///
/// `unit_model_positions` maps each unit to its list of model positions.
pub fn can_close_hatchway(
    map: &BoardingMap,
    hatchway_id: HatchwayId,
    unit_model_positions: &HashMap<UnitId, Vec<Position>>,
) -> bool {
    let hatch = match map.hatchway(hatchway_id) {
        Some(h) => h,
        None => return false,
    };

    for (_unit_id, positions) in unit_model_positions {
        if positions.len() < 2 {
            continue;
        }
        // Check if any two models from this unit are on opposite sides
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                let comp_i = map.compartment_containing(positions[i]);
                let comp_j = map.compartment_containing(positions[j]);
                if let (Some(ci), Some(cj)) = (comp_i, comp_j) {
                    if (ci == hatch.between.0 && cj == hatch.between.1)
                        || (ci == hatch.between.1 && cj == hatch.between.0)
                    {
                        // Same unit, opposite sides => cannot close
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Check if opening a hatchway causes units on opposite sides to become engaged.
///
/// In Boarding Actions, engagement range through an open hatchway is 2" (BA_HATCHWAY_ENGAGEMENT_RANGE).
/// If units on opposite sides are within 2" of each other through the newly opened hatchway,
/// they become eligible to fight in the next Fight phase. Neither counts as having charged.
///
/// Returns pairs of (unit_a, unit_b) that are newly engaged.
pub fn check_opening_engagement(
    map: &BoardingMap,
    hatchway_id: HatchwayId,
    units_side_a: &[(UnitId, Vec<Position>)],
    units_side_b: &[(UnitId, Vec<Position>)],
) -> Vec<(UnitId, UnitId)> {
    let engagement_range_sq = {
        let r = Inches::BA_HATCHWAY_ENGAGEMENT_RANGE.0 as i64;
        r * r
    };

    let mut engaged_pairs = Vec::new();

    for (unit_a, positions_a) in units_side_a {
        for (unit_b, positions_b) in units_side_b {
            // Check if any model from unit_a and any model from unit_b are within
            // engagement range through the hatchway
            let mut found = false;
            'outer: for pos_a in positions_a {
                for pos_b in positions_b {
                    // Both must be on opposite sides of this hatchway
                    if map.models_on_opposite_sides_of_hatchway(*pos_a, *pos_b, hatchway_id) {
                        // Check distance
                        let dist_sq = pos_a.distance_squared(*pos_b);
                        if dist_sq <= engagement_range_sq {
                            found = true;
                            break 'outer;
                        }
                    }
                }
            }
            if found {
                engaged_pairs.push((*unit_a, *unit_b));
            }
        }
    }

    engaged_pairs
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

    /// Build a test map with two compartments separated by a wall and connected
    /// by a single hatchway at (10, 5).
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

    fn default_states(map: &BoardingMap) -> HashMap<HatchwayId, HatchwayState> {
        map.hatchways
            .iter()
            .map(|h| (h.id, h.initial_state))
            .collect()
    }

    #[test]
    fn test_can_operate_success() {
        let map = make_test_map();
        let states = default_states(&map);
        // Model at (9, 5) — within 1" of hatchway at (10, 5)
        let unit_positions = vec![Position::from_inches(9, 5)];
        let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_operate_in_engagement() {
        let map = make_test_map();
        let states = default_states(&map);
        let unit_positions = vec![Position::from_inches(9, 5)];
        let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, true, &states);
        assert_eq!(result, Err(HatchwayError::InEngagementRange));
    }

    #[test]
    fn test_can_operate_not_in_range() {
        let map = make_test_map();
        let states = default_states(&map);
        // Model at (5, 5) — more than 1" from hatchway
        let unit_positions = vec![Position::from_inches(5, 5)];
        let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
        assert_eq!(result, Err(HatchwayError::NotInRange));
    }

    #[test]
    fn test_can_operate_locked() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Locked);
        let unit_positions = vec![Position::from_inches(9, 5)];
        let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
        assert_eq!(result, Err(HatchwayError::NotOperable));
    }

    #[test]
    fn test_can_operate_one_way_opened() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::OneWayOpened);
        let unit_positions = vec![Position::from_inches(9, 5)];
        let result = can_operate_hatchway(&map, HatchwayId::new(0), &unit_positions, false, &states);
        assert_eq!(result, Err(HatchwayError::NotOperable));
    }

    #[test]
    fn test_resolve_operation_uncontested() {
        let map = make_test_map();
        let states = default_states(&map); // Closed
        let result = resolve_hatchway_operation(
            &map,
            HatchwayId::new(0),
            &states,
            4, // operating toughness
            false, // no opponent contesting
            0, 0, 0,
            &[], // no units for engagement check on side A
            &[], // no units for engagement check on side B
        );
        assert!(result.success);
        assert_eq!(result.new_state, HatchwayState::Open);
    }

    #[test]
    fn test_resolve_operation_contested_win() {
        let map = make_test_map();
        let states = default_states(&map); // Closed
        // Operating: roll 5 + T4 = 9. Preventing: roll 3 + T4 = 7. Operator wins.
        let result = resolve_hatchway_operation(
            &map,
            HatchwayId::new(0),
            &states,
            4, true, 4,
            5, 3,
            &[], &[],
        );
        assert!(result.success);
        assert_eq!(result.new_state, HatchwayState::Open);
    }

    #[test]
    fn test_resolve_operation_contested_lose() {
        let map = make_test_map();
        let states = default_states(&map); // Closed
        // Operating: roll 2 + T4 = 6. Preventing: roll 5 + T4 = 9. Defender wins.
        let result = resolve_hatchway_operation(
            &map,
            HatchwayId::new(0),
            &states,
            4, true, 4,
            2, 5,
            &[], &[],
        );
        assert!(!result.success);
        assert_eq!(result.new_state, HatchwayState::Closed);
    }

    #[test]
    fn test_resolve_operation_contested_tie() {
        let map = make_test_map();
        let states = default_states(&map); // Closed
        // Tie: operating player wins on tie
        let result = resolve_hatchway_operation(
            &map,
            HatchwayId::new(0),
            &states,
            4, true, 4,
            4, 4,
            &[], &[],
        );
        assert!(result.success);
        assert_eq!(result.new_state, HatchwayState::Open);
    }

    #[test]
    fn test_resolve_opening_closes() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Open);
        let result = resolve_hatchway_operation(
            &map,
            HatchwayId::new(0),
            &states,
            4, false, 0,
            1, 0,
            &[], &[],
        );
        assert!(result.success);
        assert_eq!(result.new_state, HatchwayState::Closed);
    }

    #[test]
    fn test_can_close_hatchway_no_split() {
        let map = make_test_map();
        let mut unit_positions = HashMap::new();
        unit_positions.insert(UnitId::new(0), vec![Position::from_inches(5, 5)]);
        assert!(can_close_hatchway(&map, HatchwayId::new(0), &unit_positions));
    }

    #[test]
    fn test_can_close_hatchway_split_unit_blocks() {
        let map = make_test_map();
        let mut unit_positions = HashMap::new();
        // Same unit has models on both sides
        unit_positions.insert(
            UnitId::new(0),
            vec![Position::from_inches(5, 5), Position::from_inches(15, 5)],
        );
        assert!(!can_close_hatchway(&map, HatchwayId::new(0), &unit_positions));
    }

    #[test]
    fn test_can_close_different_units_opposite_sides() {
        let map = make_test_map();
        let mut unit_positions = HashMap::new();
        // Different units on each side — fine
        unit_positions.insert(UnitId::new(0), vec![Position::from_inches(5, 5)]);
        unit_positions.insert(UnitId::new(1), vec![Position::from_inches(15, 5)]);
        assert!(can_close_hatchway(&map, HatchwayId::new(0), &unit_positions));
    }

    #[test]
    fn test_check_opening_engagement() {
        let map = make_test_map();
        // Units within 2" of each other across the hatchway
        let units_a = vec![(
            UnitId::new(0),
            vec![Position::from_inches(9, 5)],
        )];
        let units_b = vec![(
            UnitId::new(1),
            vec![Position::from_inches(11, 5)],
        )];
        let pairs = check_opening_engagement(&map, HatchwayId::new(0), &units_a, &units_b);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (UnitId::new(0), UnitId::new(1)));
    }

    #[test]
    fn test_check_opening_engagement_too_far() {
        let map = make_test_map();
        // Units more than 2" apart
        let units_a = vec![(
            UnitId::new(0),
            vec![Position::from_inches(5, 5)],
        )];
        let units_b = vec![(
            UnitId::new(1),
            vec![Position::from_inches(15, 5)],
        )];
        let pairs = check_opening_engagement(&map, HatchwayId::new(0), &units_a, &units_b);
        assert!(pairs.is_empty());
    }

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
        assert_eq!(result.newly_engaged_pairs.len(), 1);
        assert_eq!(result.newly_engaged_pairs[0], (UnitId::new(0), UnitId::new(1)));
    }
}
