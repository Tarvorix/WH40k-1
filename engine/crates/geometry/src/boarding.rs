//! Boarding Actions geometry module.
//!
//! Provides the compartmentalized board representation, hatchway management,
//! wall segment collision, line-of-sight, cover checks, and pathfinding
//! for Boarding Actions games.
//!
//! All distance calculations use fixed-point `Inches` from core_types for
//! deterministic behavior across platforms. No floating-point in game logic.
//!
//! Source: boarding_actions_complete_v3.md
//! Source: boarding_actions_maps_complete_v3.json
//! Source: boarding_actions_integration_rust.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wh40k_core_types::{
    BoardDimensions, CompartmentId, CoverType, EntryZoneRole, HatchwayId, HatchwayOrientation,
    HatchwayState, Inches, ObjectiveId, PlayerId, Polygon, Position, RegionId, Visibility,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The top-level Boarding Actions map, holding all spatial data for a
/// compartmentalized ship interior.
///
/// Source: boarding_actions_complete_v3.md Section 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingMap {
    pub dimensions: BoardDimensions,
    pub compartments: Vec<Compartment>,
    pub walls: Vec<WallSegment>,
    pub hatchways: Vec<Hatchway>,
    pub entry_zones: Vec<EntryZone>,
    pub objectives: Vec<BoardingObjectiveMarker>,
    pub special_regions: Vec<SpecialRegion>,
}

/// A compartment (room / zone) on the Boarding Actions map.
///
/// Source: boarding_actions_complete_v3.md Section 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compartment {
    pub id: CompartmentId,
    pub name: String,
    pub boundary: Polygon,
    pub tags: Vec<String>,
}

/// A line segment representing an impassable, LOS-blocking wall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallSegment {
    pub id: u32,
    pub start: Position,
    pub end: Position,
}

/// A doorway between two compartments with open/closed state.
///
/// Source: boarding_actions_complete_v3.md Section 2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hatchway {
    pub id: HatchwayId,
    pub position: Position,
    pub orientation: HatchwayOrientation,
    pub width: Inches,
    pub between: (CompartmentId, CompartmentId),
    pub initial_state: HatchwayState,
    pub tags: Vec<String>,
}

/// Deployment / reserve entry area.
///
/// Source: boarding_actions_complete_v3.md Section 7
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryZone {
    pub id: u32,
    pub name: String,
    pub role: EntryZoneRole,
    pub boundary: Polygon,
    pub player_assignment: Option<PlayerId>,
    pub enabled: bool,
}

/// An objective marker on a Boarding Actions map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingObjectiveMarker {
    pub id: ObjectiveId,
    pub position: Position,
    pub label: String,
    pub tags: Vec<String>,
}

/// Mission-specific region (lighting area, furnace, sector, access zone, etc.)
///
/// Source: boarding_actions_complete_v3.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialRegion {
    pub id: RegionId,
    pub name: String,
    pub boundary: Polygon,
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helper Functions (free functions)
// ---------------------------------------------------------------------------

/// Do two line segments (a1-a2) and (b1-b2) intersect?
///
/// Uses the integer cross-product orientation test for full determinism.
/// Returns `true` if the segments share any interior or boundary point.
pub fn line_segments_intersect(a1: Position, a2: Position, b1: Position, b2: Position) -> bool {
    let d1 = cross(a1, a2, b1);
    let d2 = cross(a1, a2, b2);
    let d3 = cross(b1, b2, a1);
    let d4 = cross(b1, b2, a2);

    // General case: segments straddle each other
    if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0))
        && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0))
    {
        return true;
    }

    // Collinear / endpoint-on-segment cases
    if d1 == 0 && on_segment(a1, a2, b1) {
        return true;
    }
    if d2 == 0 && on_segment(a1, a2, b2) {
        return true;
    }
    if d3 == 0 && on_segment(b1, b2, a1) {
        return true;
    }
    if d4 == 0 && on_segment(b1, b2, a2) {
        return true;
    }

    false
}

/// Squared distance from a point to the nearest point on a line segment.
/// Uses integer arithmetic for determinism. Result is in mils^2.
pub fn point_to_line_segment_distance_squared(
    point: Position,
    seg_start: Position,
    seg_end: Position,
) -> i64 {
    let ab_x = (seg_end.x.0 - seg_start.x.0) as i64;
    let ab_y = (seg_end.y.0 - seg_start.y.0) as i64;
    let ap_x = (point.x.0 - seg_start.x.0) as i64;
    let ap_y = (point.y.0 - seg_start.y.0) as i64;

    let ab_sq = ab_x * ab_x + ab_y * ab_y;
    if ab_sq == 0 {
        // Degenerate segment (single point)
        return ap_x * ap_x + ap_y * ap_y;
    }

    let t_num = ap_x * ab_x + ap_y * ab_y;

    if t_num <= 0 {
        // Closest to seg_start
        ap_x * ap_x + ap_y * ap_y
    } else if t_num >= ab_sq {
        // Closest to seg_end
        let bp_x = (point.x.0 - seg_end.x.0) as i64;
        let bp_y = (point.y.0 - seg_end.y.0) as i64;
        bp_x * bp_x + bp_y * bp_y
    } else {
        // Closest point is interior to the segment
        // dist^2 = |ap|^2 - (ap . ab)^2 / |ab|^2
        let ap_sq = ap_x * ap_x + ap_y * ap_y;
        ap_sq - (t_num * t_num) / ab_sq
    }
}

/// Does a line segment pass through or touch a circle?
///
/// The circle is defined by `center` and `radius`. Uses integer squared
/// comparisons (no sqrt) for determinism.
pub fn line_intersects_circle(
    line_start: Position,
    line_end: Position,
    center: Position,
    radius: Inches,
) -> bool {
    let dist_sq = point_to_line_segment_distance_squared(center, line_start, line_end);
    let radius_sq = (radius.0 as i64) * (radius.0 as i64);
    dist_sq <= radius_sq
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Cross product of vectors (p2 - p1) x (p3 - p1).
/// Positive => counter-clockwise, negative => clockwise, zero => collinear.
fn cross(p1: Position, p2: Position, p3: Position) -> i64 {
    let a_x = (p2.x.0 - p1.x.0) as i64;
    let a_y = (p2.y.0 - p1.y.0) as i64;
    let b_x = (p3.x.0 - p1.x.0) as i64;
    let b_y = (p3.y.0 - p1.y.0) as i64;
    a_x * b_y - a_y * b_x
}

/// Given collinear points p1, p2, q, check whether q lies on segment p1-p2.
fn on_segment(p1: Position, p2: Position, q: Position) -> bool {
    let qx = q.x.0 as i64;
    let qy = q.y.0 as i64;
    let min_x = (p1.x.0 as i64).min(p2.x.0 as i64);
    let max_x = (p1.x.0 as i64).max(p2.x.0 as i64);
    let min_y = (p1.y.0 as i64).min(p2.y.0 as i64);
    let max_y = (p1.y.0 as i64).max(p2.y.0 as i64);
    qx >= min_x && qx <= max_x && qy >= min_y && qy <= max_y
}

/// Integer square root (Newton's method) — deterministic across platforms.
/// Retained for potential future use within this module (e.g., manual distance
/// calculations that bypass `Position::distance`).
#[allow(dead_code)]
fn isqrt(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Hatchway segment endpoints for a hatchway at `pos` with given orientation
/// and width.  The segment is centred on `pos`.
fn hatchway_segment(hatchway: &Hatchway) -> (Position, Position) {
    let half = hatchway.width / 2;
    match hatchway.orientation {
        HatchwayOrientation::Horizontal => {
            let start = Position::new(hatchway.position.x - half, hatchway.position.y);
            let end = Position::new(hatchway.position.x + half, hatchway.position.y);
            (start, end)
        }
        HatchwayOrientation::Vertical => {
            let start = Position::new(hatchway.position.x, hatchway.position.y - half);
            let end = Position::new(hatchway.position.x, hatchway.position.y + half);
            (start, end)
        }
    }
}

// ---------------------------------------------------------------------------
// BoardingMap implementation
// ---------------------------------------------------------------------------

impl BoardingMap {
    // ----- 1. Constructor ---------------------------------------------------

    /// Create an empty Boarding Actions map with the given dimensions.
    pub fn new(dimensions: BoardDimensions) -> Self {
        Self {
            dimensions,
            compartments: Vec::new(),
            walls: Vec::new(),
            hatchways: Vec::new(),
            entry_zones: Vec::new(),
            objectives: Vec::new(),
            special_regions: Vec::new(),
        }
    }

    // ----- 2. compartment_containing ----------------------------------------

    /// Which compartment contains the given position?
    /// Returns the first matching compartment ID, or `None` if the position
    /// is outside all compartments.
    pub fn compartment_containing(&self, pos: Position) -> Option<CompartmentId> {
        for comp in &self.compartments {
            if comp.boundary.contains(pos) {
                return Some(comp.id);
            }
        }
        None
    }

    // ----- 3. compartment ---------------------------------------------------

    /// Get a compartment by its ID.
    pub fn compartment(&self, id: CompartmentId) -> Option<&Compartment> {
        self.compartments.iter().find(|c| c.id == id)
    }

    // ----- 4. hatchway ------------------------------------------------------

    /// Get a hatchway by its ID.
    pub fn hatchway(&self, id: HatchwayId) -> Option<&Hatchway> {
        self.hatchways.iter().find(|h| h.id == id)
    }

    // ----- 5. hatchways_between ---------------------------------------------

    /// All hatchways connecting two compartments (order-independent).
    pub fn hatchways_between(&self, a: CompartmentId, b: CompartmentId) -> Vec<&Hatchway> {
        self.hatchways
            .iter()
            .filter(|h| {
                (h.between.0 == a && h.between.1 == b)
                    || (h.between.0 == b && h.between.1 == a)
            })
            .collect()
    }

    // ----- 6. hatchways_for_compartment -------------------------------------

    /// All hatchways touching a given compartment.
    pub fn hatchways_for_compartment(&self, compartment: CompartmentId) -> Vec<&Hatchway> {
        self.hatchways
            .iter()
            .filter(|h| h.between.0 == compartment || h.between.1 == compartment)
            .collect()
    }

    // ----- 7. is_wall_between -----------------------------------------------

    /// Does any wall segment intersect the line between two positions?
    pub fn is_wall_between(&self, from: Position, to: Position) -> bool {
        for wall in &self.walls {
            if line_segments_intersect(from, to, wall.start, wall.end) {
                return true;
            }
        }
        false
    }

    // ----- 8. is_closed_hatch_between ---------------------------------------

    /// Does any closed or locked hatchway block the line between two positions?
    ///
    /// `hatchway_states` carries the *current* state of each hatchway (states
    /// change during gameplay). If a hatchway is not present in the map it is
    /// silently ignored (shouldn't happen in a well-formed game).
    pub fn is_closed_hatch_between(
        &self,
        from: Position,
        to: Position,
        hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    ) -> bool {
        for hatch in &self.hatchways {
            let state = hatchway_states
                .get(&hatch.id)
                .copied()
                .unwrap_or(hatch.initial_state);

            if !state.allows_los() {
                // Closed or locked — check if the LOS line crosses the hatchway segment
                let (h_start, h_end) = hatchway_segment(hatch);
                if line_segments_intersect(from, to, h_start, h_end) {
                    return true;
                }
            }
        }
        false
    }

    // ----- 9. check_los_boarding --------------------------------------------

    /// Boarding Actions LOS check.
    ///
    /// A model is NOT visible if the line from one position to another passes
    /// through a wall or a closed/locked hatchway.
    ///
    /// Source: boarding_actions_complete_v3.md Section 3.3
    pub fn check_los_boarding(
        &self,
        from: Position,
        to: Position,
        hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    ) -> Visibility {
        if self.is_wall_between(from, to) {
            return Visibility::NotVisible;
        }
        if self.is_closed_hatch_between(from, to, hatchway_states) {
            return Visibility::NotVisible;
        }
        Visibility::Visible
    }

    // ----- 10. check_cover_boarding -----------------------------------------

    /// Boarding Actions cover check.
    ///
    /// A target has the Benefit of Cover if the LOS line passes within 1" of a
    /// wall segment or within 1" of an open hatchway's frame (the hatchway
    /// segment). If LOS is blocked entirely, cover is irrelevant so we return
    /// `CoverType::None` (the caller should check LOS first).
    ///
    /// Source: boarding_actions_complete_v3.md Section 3.3
    pub fn check_cover_boarding(
        &self,
        from: Position,
        to: Position,
        hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    ) -> CoverType {
        // If LOS is blocked, cover is irrelevant.
        if self.check_los_boarding(from, to, hatchway_states) == Visibility::NotVisible {
            return CoverType::None;
        }

        let cover_threshold_sq = {
            let r = Inches::from_inches(1).0 as i64;
            r * r
        };

        // Check proximity to wall segments
        for wall in &self.walls {
            let dist_sq = line_to_segment_min_distance_sq(from, to, wall.start, wall.end);
            if dist_sq <= cover_threshold_sq {
                return CoverType::BenefitOfCover;
            }
        }

        // Check proximity to open hatchway frames
        for hatch in &self.hatchways {
            let state = hatchway_states
                .get(&hatch.id)
                .copied()
                .unwrap_or(hatch.initial_state);

            if state.allows_los() {
                // Open hatchway frame can still grant cover when LOS passes near it
                let (h_start, h_end) = hatchway_segment(hatch);
                let dist_sq = line_to_segment_min_distance_sq(from, to, h_start, h_end);
                if dist_sq <= cover_threshold_sq {
                    return CoverType::BenefitOfCover;
                }
            }
        }

        CoverType::None
    }

    // ----- 11. shortest_path_distance ---------------------------------------

    /// Calculate the shortest legal path distance from one position to another,
    /// routing around walls and through open hatchways only.
    ///
    /// Algorithm: Dijkstra over a graph where each compartment is a node and
    /// each open hatchway is an edge. The cost of traversing a hatchway edge is
    /// `distance(current_pos, hatchway_center) + distance(hatchway_center, …)`
    /// chained through the route.
    ///
    /// Returns `None` if no legal path exists (all connecting hatchways closed).
    pub fn shortest_path_distance(
        &self,
        from: Position,
        to: Position,
        hatchway_states: &HashMap<HatchwayId, HatchwayState>,
    ) -> Option<Inches> {
        let src_comp = self.compartment_containing(from)?;
        let dst_comp = self.compartment_containing(to)?;

        // Same compartment — straight-line distance (no walls within a
        // compartment in the standard BA map model).
        if src_comp == dst_comp {
            return Some(from.distance(to));
        }

        // Build adjacency: compartment -> [(neighbour_compartment, hatchway_position)]
        // Only include open hatchways.
        let mut adjacency: HashMap<CompartmentId, Vec<(CompartmentId, Position)>> = HashMap::new();
        for hatch in &self.hatchways {
            let state = hatchway_states
                .get(&hatch.id)
                .copied()
                .unwrap_or(hatch.initial_state);
            if !state.allows_passage() {
                continue;
            }
            adjacency
                .entry(hatch.between.0)
                .or_default()
                .push((hatch.between.1, hatch.position));
            adjacency
                .entry(hatch.between.1)
                .or_default()
                .push((hatch.between.0, hatch.position));
        }

        // Dijkstra over compartment graph.
        //
        // `Position` does not implement `Ord`, so we only store
        // `(cost, CompartmentId)` in the BinaryHeap and track the entry
        // position separately in the `dist` map.
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        // dist[compartment] = (best_cost_mils, entry_position_into_compartment)
        let mut dist: HashMap<CompartmentId, (i32, Position)> = HashMap::new();
        let mut heap: BinaryHeap<Reverse<(i32, CompartmentId)>> = BinaryHeap::new();

        // Seed with source compartment
        dist.insert(src_comp, (0, from));
        heap.push(Reverse((0, src_comp)));

        while let Some(Reverse((cost, comp))) = heap.pop() {
            // Look up the entry position for this compartment visit
            let pos = match dist.get(&comp) {
                Some(&(best, p)) if best == cost => p,
                _ => continue, // stale entry
            };

            // If we reached the destination compartment, add straight-line
            // distance from the entry point to the final target.
            if comp == dst_comp {
                let final_cost = cost + pos.distance(to).0;
                return Some(Inches(final_cost));
            }

            // Explore neighbours through open hatchways
            if let Some(neighbours) = adjacency.get(&comp) {
                for &(neighbour_comp, hatch_pos) in neighbours {
                    let edge_cost = pos.distance(hatch_pos).0;
                    let new_cost = cost + edge_cost;

                    let dominated = dist
                        .get(&neighbour_comp)
                        .map_or(false, |&(best, _)| new_cost >= best);

                    if !dominated {
                        dist.insert(neighbour_comp, (new_cost, hatch_pos));
                        heap.push(Reverse((new_cost, neighbour_comp)));
                    }
                }
            }
        }

        // No path found
        None
    }

    // ----- 12. entry_zones_for_player ---------------------------------------

    /// All entry zones assigned to a given player.
    pub fn entry_zones_for_player(&self, player: PlayerId) -> Vec<&EntryZone> {
        self.entry_zones
            .iter()
            .filter(|ez| ez.player_assignment == Some(player))
            .collect()
    }

    // ----- 13. entry_zones_by_role ------------------------------------------

    /// All entry zones with the given role.
    pub fn entry_zones_by_role(&self, role: EntryZoneRole) -> Vec<&EntryZone> {
        self.entry_zones
            .iter()
            .filter(|ez| ez.role == role)
            .collect()
    }

    // ----- 14. objective ----------------------------------------------------

    /// Get an objective marker by its ID.
    pub fn objective(&self, id: ObjectiveId) -> Option<&BoardingObjectiveMarker> {
        self.objectives.iter().find(|o| o.id == id)
    }

    // ----- 15. objectives_with_tag ------------------------------------------

    /// Find all objectives carrying a specific tag.
    pub fn objectives_with_tag(&self, tag: &str) -> Vec<&BoardingObjectiveMarker> {
        self.objectives
            .iter()
            .filter(|o| o.tags.iter().any(|t| t == tag))
            .collect()
    }

    // ----- 16. regions_with_tag ---------------------------------------------

    /// Find all special regions carrying a specific tag.
    pub fn regions_with_tag(&self, tag: &str) -> Vec<&SpecialRegion> {
        self.special_regions
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }

    // ----- 17. region_containing --------------------------------------------

    /// All special regions whose boundary contains the given position.
    pub fn region_containing(&self, pos: Position) -> Vec<&SpecialRegion> {
        self.special_regions
            .iter()
            .filter(|r| r.boundary.contains(pos))
            .collect()
    }

    // ----- 18. is_within_hatchway_operate_range -----------------------------

    /// Is a unit position within 1" (BA_HATCHWAY_OPERATE_RANGE) of the hatchway?
    ///
    /// Source: boarding_actions_complete_v3.md Section 3.2
    pub fn is_within_hatchway_operate_range(
        &self,
        unit_pos: Position,
        hatchway_id: HatchwayId,
    ) -> bool {
        if let Some(hatch) = self.hatchway(hatchway_id) {
            let range_sq = {
                let r = Inches::BA_HATCHWAY_OPERATE_RANGE.0 as i64;
                r * r
            };
            let (h_start, h_end) = hatchway_segment(hatch);
            let dist_sq =
                point_to_line_segment_distance_squared(unit_pos, h_start, h_end);
            dist_sq <= range_sq
        } else {
            false
        }
    }

    // ----- 19. models_on_opposite_sides_of_hatchway -------------------------

    /// Are two positions on opposite sides of the given hatchway?
    ///
    /// True when `pos_a` is in one of the hatchway's connected compartments and
    /// `pos_b` is in the other.
    pub fn models_on_opposite_sides_of_hatchway(
        &self,
        pos_a: Position,
        pos_b: Position,
        hatchway_id: HatchwayId,
    ) -> bool {
        let hatch = match self.hatchway(hatchway_id) {
            Some(h) => h,
            None => return false,
        };

        let comp_a = self.compartment_containing(pos_a);
        let comp_b = self.compartment_containing(pos_b);

        match (comp_a, comp_b) {
            (Some(ca), Some(cb)) => {
                (ca == hatch.between.0 && cb == hatch.between.1)
                    || (ca == hatch.between.1 && cb == hatch.between.0)
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal: minimum squared distance between two line segments
// ---------------------------------------------------------------------------

/// Approximate minimum squared distance between two line segments by sampling
/// endpoints and closest-point projections. Fully integer arithmetic.
///
/// This is used for the cover proximity check: "does the LOS line pass within
/// 1 inch of a wall or hatchway frame?"
fn line_to_segment_min_distance_sq(
    l_start: Position,
    l_end: Position,
    s_start: Position,
    s_end: Position,
) -> i64 {
    // If the segments intersect the distance is 0
    if line_segments_intersect(l_start, l_end, s_start, s_end) {
        return 0;
    }

    // Check the four endpoint-to-segment distances
    let d1 = point_to_line_segment_distance_squared(l_start, s_start, s_end);
    let d2 = point_to_line_segment_distance_squared(l_end, s_start, s_end);
    let d3 = point_to_line_segment_distance_squared(s_start, l_start, l_end);
    let d4 = point_to_line_segment_distance_squared(s_end, l_start, l_end);

    d1.min(d2).min(d3).min(d4)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple test map with 2 rectangular compartments separated by a
    /// vertical wall and connected by a single hatchway.
    ///
    /// Layout (all coordinates in inches):
    ///
    /// ```text
    ///   (0,0) -------- (10,0) -------- (20,0)
    ///     |              |H|              |
    ///     | Comp A       | |   Comp B     |
    ///     |              | |              |
    ///   (0,10) ------- (10,10) ------- (20,10)
    /// ```
    ///
    /// Wall: the vertical line at x=10 from y=0 to y=4 and y=6 to y=10.
    /// Hatchway: at (10, 5) oriented Vertical with width 2" connecting
    /// the gap y=4..y=6.
    fn make_test_map() -> BoardingMap {
        let mut map = BoardingMap::new(BoardDimensions {
            width: Inches::from_inches(20),
            height: Inches::from_inches(10),
        });

        // Compartment A: left half
        map.compartments.push(Compartment {
            id: CompartmentId::new(0),
            name: "Compartment A".into(),
            boundary: Polygon::new(vec![
                Position::from_inches(0, 0),
                Position::from_inches(10, 0),
                Position::from_inches(10, 10),
                Position::from_inches(0, 10),
            ]),
            tags: vec![],
        });

        // Compartment B: right half
        map.compartments.push(Compartment {
            id: CompartmentId::new(1),
            name: "Compartment B".into(),
            boundary: Polygon::new(vec![
                Position::from_inches(10, 0),
                Position::from_inches(20, 0),
                Position::from_inches(20, 10),
                Position::from_inches(10, 10),
            ]),
            tags: vec![],
        });

        // Wall segments along x=10: below the hatchway (y=0 to y=4) and above (y=6 to y=10)
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

        // Hatchway at x=10, y=5, vertical orientation, width 2"
        map.hatchways.push(Hatchway {
            id: HatchwayId::new(0),
            position: Position::from_inches(10, 5),
            orientation: HatchwayOrientation::Vertical,
            width: Inches::from_inches(2),
            between: (CompartmentId::new(0), CompartmentId::new(1)),
            initial_state: HatchwayState::Open,
            tags: vec![],
        });

        // Entry zones
        map.entry_zones.push(EntryZone {
            id: 0,
            name: "Player A Entry".into(),
            role: EntryZoneRole::Main,
            boundary: Polygon::new(vec![
                Position::from_inches(0, 0),
                Position::from_inches(5, 0),
                Position::from_inches(5, 10),
                Position::from_inches(0, 10),
            ]),
            player_assignment: Some(PlayerId::new(0)),
            enabled: true,
        });
        map.entry_zones.push(EntryZone {
            id: 1,
            name: "Player B Entry".into(),
            role: EntryZoneRole::Main,
            boundary: Polygon::new(vec![
                Position::from_inches(15, 0),
                Position::from_inches(20, 0),
                Position::from_inches(20, 10),
                Position::from_inches(15, 10),
            ]),
            player_assignment: Some(PlayerId::new(1)),
            enabled: true,
        });
        map.entry_zones.push(EntryZone {
            id: 2,
            name: "Patrol Entry".into(),
            role: EntryZoneRole::Patrol,
            boundary: Polygon::new(vec![
                Position::from_inches(0, 0),
                Position::from_inches(3, 0),
                Position::from_inches(3, 3),
                Position::from_inches(0, 3),
            ]),
            player_assignment: Some(PlayerId::new(0)),
            enabled: true,
        });

        // Objectives
        map.objectives.push(BoardingObjectiveMarker {
            id: ObjectiveId::new(0),
            position: Position::from_inches(5, 5),
            label: "Alpha".into(),
            tags: vec!["primary".into()],
        });
        map.objectives.push(BoardingObjectiveMarker {
            id: ObjectiveId::new(1),
            position: Position::from_inches(15, 5),
            label: "Beta".into(),
            tags: vec!["primary".into(), "bonus".into()],
        });

        // Special regions
        map.special_regions.push(SpecialRegion {
            id: RegionId::new(0),
            name: "Furnace Zone".into(),
            boundary: Polygon::new(vec![
                Position::from_inches(12, 2),
                Position::from_inches(18, 2),
                Position::from_inches(18, 8),
                Position::from_inches(12, 8),
            ]),
            tags: vec!["furnace".into(), "hazard".into()],
        });

        map
    }

    /// Build the default hatchway states map from the test map's initial states.
    fn default_states(map: &BoardingMap) -> HashMap<HatchwayId, HatchwayState> {
        map.hatchways
            .iter()
            .map(|h| (h.id, h.initial_state))
            .collect()
    }

    // -- 1. test_compartment_containing --------------------------------------

    #[test]
    fn test_compartment_containing() {
        let map = make_test_map();

        // Point clearly in Compartment A
        assert_eq!(
            map.compartment_containing(Position::from_inches(5, 5)),
            Some(CompartmentId::new(0))
        );

        // Point clearly in Compartment B
        assert_eq!(
            map.compartment_containing(Position::from_inches(15, 5)),
            Some(CompartmentId::new(1))
        );

        // Point outside both compartments
        assert_eq!(
            map.compartment_containing(Position::from_inches(25, 5)),
            None
        );
    }

    // -- 2. test_hatchways_between -------------------------------------------

    #[test]
    fn test_hatchways_between() {
        let map = make_test_map();

        let found = map.hatchways_between(CompartmentId::new(0), CompartmentId::new(1));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, HatchwayId::new(0));

        // Reversed order should yield the same result
        let found_rev = map.hatchways_between(CompartmentId::new(1), CompartmentId::new(0));
        assert_eq!(found_rev.len(), 1);
        assert_eq!(found_rev[0].id, HatchwayId::new(0));

        // Non-existent pair
        let none = map.hatchways_between(CompartmentId::new(0), CompartmentId::new(99));
        assert!(none.is_empty());
    }

    // -- 3. test_wall_blocks_los ---------------------------------------------

    #[test]
    fn test_wall_blocks_los() {
        let map = make_test_map();
        let states = default_states(&map);

        // LOS line crossing the lower wall segment (y=2, which is in the
        // wall range y=0..4)
        let from = Position::from_inches(5, 2);
        let to = Position::from_inches(15, 2);
        assert_eq!(
            map.check_los_boarding(from, to, &states),
            Visibility::NotVisible
        );

        // Also verify the helper directly
        assert!(map.is_wall_between(from, to));
    }

    // -- 4. test_open_hatch_allows_los ---------------------------------------

    #[test]
    fn test_open_hatch_allows_los() {
        let map = make_test_map();
        let states = default_states(&map); // hatchway is Open by default

        // LOS through the hatchway gap (y=5, hatchway center)
        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);
        assert_eq!(
            map.check_los_boarding(from, to, &states),
            Visibility::Visible
        );
    }

    // -- 5. test_closed_hatch_blocks_los -------------------------------------

    #[test]
    fn test_closed_hatch_blocks_los() {
        let map = make_test_map();
        let mut states = default_states(&map);

        // Close the hatchway
        states.insert(HatchwayId::new(0), HatchwayState::Closed);

        // LOS through the hatchway gap should now be blocked
        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);
        assert_eq!(
            map.check_los_boarding(from, to, &states),
            Visibility::NotVisible
        );
    }

    // -- 6. test_shortest_path_through_open_hatch ----------------------------

    #[test]
    fn test_shortest_path_through_open_hatch() {
        let map = make_test_map();
        let states = default_states(&map); // hatchway open

        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);

        let dist = map.shortest_path_distance(from, to, &states);
        assert!(dist.is_some());
        let d = dist.unwrap();

        // Expected: from(5,5) -> hatchway(10,5) -> to(15,5) = 5 + 5 = 10"
        assert_eq!(d, Inches::from_inches(10));
    }

    // -- 7. test_no_path_through_closed_hatch --------------------------------

    #[test]
    fn test_no_path_through_closed_hatch() {
        let map = make_test_map();
        let mut states = default_states(&map);
        states.insert(HatchwayId::new(0), HatchwayState::Closed);

        let from = Position::from_inches(5, 5);
        let to = Position::from_inches(15, 5);

        let dist = map.shortest_path_distance(from, to, &states);
        assert!(dist.is_none());
    }

    // -- 8. test_entry_zone_lookup -------------------------------------------

    #[test]
    fn test_entry_zone_lookup() {
        let map = make_test_map();

        // By player
        let p0_zones = map.entry_zones_for_player(PlayerId::new(0));
        assert_eq!(p0_zones.len(), 2); // Main + Patrol
        let p1_zones = map.entry_zones_for_player(PlayerId::new(1));
        assert_eq!(p1_zones.len(), 1);

        // By role
        let main_zones = map.entry_zones_by_role(EntryZoneRole::Main);
        assert_eq!(main_zones.len(), 2);
        let patrol_zones = map.entry_zones_by_role(EntryZoneRole::Patrol);
        assert_eq!(patrol_zones.len(), 1);
        assert_eq!(patrol_zones[0].name, "Patrol Entry");
    }

    // -- 9. test_opposite_sides_of_hatchway ----------------------------------

    #[test]
    fn test_opposite_sides_of_hatchway() {
        let map = make_test_map();

        let pos_a = Position::from_inches(5, 5); // Compartment A
        let pos_b = Position::from_inches(15, 5); // Compartment B

        assert!(map.models_on_opposite_sides_of_hatchway(pos_a, pos_b, HatchwayId::new(0)));
        assert!(map.models_on_opposite_sides_of_hatchway(pos_b, pos_a, HatchwayId::new(0)));

        // Same side — should be false
        let pos_c = Position::from_inches(3, 3); // Also Compartment A
        assert!(!map.models_on_opposite_sides_of_hatchway(pos_a, pos_c, HatchwayId::new(0)));
    }

    // -- Additional coverage -------------------------------------------------

    #[test]
    fn test_objective_lookup() {
        let map = make_test_map();

        assert!(map.objective(ObjectiveId::new(0)).is_some());
        assert_eq!(
            map.objective(ObjectiveId::new(0)).unwrap().label,
            "Alpha"
        );
        assert!(map.objective(ObjectiveId::new(99)).is_none());
    }

    #[test]
    fn test_objectives_with_tag() {
        let map = make_test_map();

        let primary = map.objectives_with_tag("primary");
        assert_eq!(primary.len(), 2);

        let bonus = map.objectives_with_tag("bonus");
        assert_eq!(bonus.len(), 1);
        assert_eq!(bonus[0].label, "Beta");
    }

    #[test]
    fn test_regions_with_tag() {
        let map = make_test_map();

        let furnace = map.regions_with_tag("furnace");
        assert_eq!(furnace.len(), 1);
        assert_eq!(furnace[0].name, "Furnace Zone");

        let hazard = map.regions_with_tag("hazard");
        assert_eq!(hazard.len(), 1);

        let none = map.regions_with_tag("nonexistent");
        assert!(none.is_empty());
    }

    #[test]
    fn test_region_containing() {
        let map = make_test_map();

        // Position inside the furnace zone
        let inside = map.region_containing(Position::from_inches(15, 5));
        assert_eq!(inside.len(), 1);
        assert_eq!(inside[0].name, "Furnace Zone");

        // Position outside all special regions
        let outside = map.region_containing(Position::from_inches(3, 3));
        assert!(outside.is_empty());
    }

    #[test]
    fn test_hatchway_operate_range() {
        let map = make_test_map();

        // Right next to the hatchway at (10, 5)
        assert!(map.is_within_hatchway_operate_range(
            Position::from_inches(9, 5),
            HatchwayId::new(0)
        ));

        // Just within 1"
        assert!(map.is_within_hatchway_operate_range(
            Position::new(Inches::from_inches(9), Inches::from_inches(5)),
            HatchwayId::new(0)
        ));

        // Far away — more than 1"
        assert!(!map.is_within_hatchway_operate_range(
            Position::from_inches(5, 5),
            HatchwayId::new(0)
        ));
    }

    #[test]
    fn test_line_segments_intersect_basic() {
        // Crossing X
        let a1 = Position::from_inches(0, 0);
        let a2 = Position::from_inches(10, 10);
        let b1 = Position::from_inches(10, 0);
        let b2 = Position::from_inches(0, 10);
        assert!(line_segments_intersect(a1, a2, b1, b2));

        // Parallel, non-intersecting
        let c1 = Position::from_inches(0, 0);
        let c2 = Position::from_inches(10, 0);
        let d1 = Position::from_inches(0, 5);
        let d2 = Position::from_inches(10, 5);
        assert!(!line_segments_intersect(c1, c2, d1, d2));
    }

    #[test]
    fn test_point_to_line_segment_distance() {
        // Point directly above the midpoint of a horizontal segment
        let point = Position::from_inches(5, 3);
        let seg_start = Position::from_inches(0, 0);
        let seg_end = Position::from_inches(10, 0);
        let dist_sq = point_to_line_segment_distance_squared(point, seg_start, seg_end);
        // Distance is 3" = 3000 mils, so dist_sq = 9_000_000
        assert_eq!(dist_sq, 9_000_000);
    }

    #[test]
    fn test_line_intersects_circle_fn() {
        let line_start = Position::from_inches(0, 0);
        let line_end = Position::from_inches(10, 0);
        let center = Position::from_inches(5, 1);
        let radius = Inches::from_inches(2);
        assert!(line_intersects_circle(line_start, line_end, center, radius));

        let far_center = Position::from_inches(5, 5);
        assert!(!line_intersects_circle(
            line_start,
            line_end,
            far_center,
            radius
        ));
    }

    #[test]
    fn test_compartment_and_hatchway_lookup() {
        let map = make_test_map();

        let comp = map.compartment(CompartmentId::new(0));
        assert!(comp.is_some());
        assert_eq!(comp.unwrap().name, "Compartment A");

        let hatch = map.hatchway(HatchwayId::new(0));
        assert!(hatch.is_some());
        assert_eq!(hatch.unwrap().position, Position::from_inches(10, 5));

        let hatches = map.hatchways_for_compartment(CompartmentId::new(0));
        assert_eq!(hatches.len(), 1);
    }

    #[test]
    fn test_same_compartment_path_distance() {
        let map = make_test_map();
        let states = default_states(&map);

        // Both in compartment A — straight-line distance
        let from = Position::from_inches(2, 2);
        let to = Position::from_inches(8, 2);
        let dist = map.shortest_path_distance(from, to, &states);
        assert!(dist.is_some());
        assert_eq!(dist.unwrap(), Inches::from_inches(6));
    }

    #[test]
    fn test_cover_near_wall() {
        let map = make_test_map();
        let states = default_states(&map);

        // LOS through the open hatchway but very close to a wall edge
        // From (5, 4) to (15, 4): the y=4 endpoint of the lower wall at
        // (10, 0)-(10, 4) is right on the line. Should grant cover.
        let from = Position::from_inches(5, 4);
        let to = Position::from_inches(15, 4);

        // This line crosses the wall at (10, 0)-(10, 4) at the endpoint (10, 4),
        // so distance is 0 from the wall. LOS is blocked by the wall, so
        // cover check returns None (blocked LOS).
        // Test a line that is *near* the wall but doesn't cross it.
        // Line from (5, 4.5) to (15, 4.5) - passes 0.5" from the wall end at (10,4)
        let _from_blocked = from;
        let _to_blocked = to;
        let from2 = Position::new(
            Inches::from_inches(5),
            Inches::from_inches_frac(4, 500),
        );
        let to2 = Position::new(
            Inches::from_inches(15),
            Inches::from_inches_frac(4, 500),
        );

        // This line passes through the hatchway gap (y=4..6) but near the wall
        let cover = map.check_cover_boarding(from2, to2, &states);
        assert_eq!(cover, CoverType::BenefitOfCover);
    }
}
