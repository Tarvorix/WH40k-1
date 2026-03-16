//! Boarding Actions map geometry definitions.
//!
//! Defines the physical layout (compartments, walls, hatchways, entry zones, objectives)
//! for each Boarding Actions mission. Each mission has a unique map layout transcribed
//! from the official mission maps.
//!
//! The standard BA board is 48" × 28" (two 24"×28" boards side by side).
//!
//! Source: boarding_actions_complete_v3.md
//! Source: boarding_actions_maps_complete_v3.json

use wh40k_core_types::{
    BoardDimensions, CompartmentId, EntryZoneRole, HatchwayId, HatchwayOrientation,
    HatchwayState, Inches, MissionId, ObjectiveId, PlayerId, Polygon, Position, RegionId,
};
use wh40k_geometry::boarding::{
    BoardingMap, BoardingObjectiveMarker, Compartment, EntryZone, Hatchway, SpecialRegion,
    WallSegment,
};

/// Load the map geometry for a given mission ID.
/// Falls back to a generic symmetric layout if no specific map is defined.
pub fn load_mission_map(mission_id: Option<MissionId>) -> BoardingMap {
    match mission_id.map(|m| m.raw()) {
        Some(11) => build_ba_11(),
        Some(12) => build_ba_12(),
        Some(13) => build_ba_13(),
        Some(21) => build_ba_21(),
        Some(22) => build_ba_22(),
        Some(23) => build_ba_23(),
        Some(31) => build_ba_31(),
        Some(32) => build_ba_32(),
        Some(33) => build_ba_33(),
        Some(1) => build_ba_asymmetric(1),
        Some(2) => build_ba_asymmetric(2),
        Some(3) => build_ba_asymmetric(3),
        Some(4) => build_ba_asymmetric(4),
        Some(5) => build_ba_asymmetric(5),
        Some(6) => build_ba_asymmetric(6),
        _ => build_generic_symmetric(),
    }
}

// ---------------------------------------------------------------------------
// Helper: create polygon from corner points (in inches)
// ---------------------------------------------------------------------------

fn rect_poly(x1: i32, y1: i32, x2: i32, y2: i32) -> Polygon {
    Polygon::new(vec![
        Position::from_inches(x1, y1),
        Position::from_inches(x2, y1),
        Position::from_inches(x2, y2),
        Position::from_inches(x1, y2),
    ])
}

fn pos(x: i32, y: i32) -> Position {
    Position::from_inches(x, y)
}

fn wall(id: u32, x1: i32, y1: i32, x2: i32, y2: i32) -> WallSegment {
    WallSegment {
        id,
        start: pos(x1, y1),
        end: pos(x2, y2),
    }
}

fn hatch(
    id: u32,
    x: i32,
    y: i32,
    orient: HatchwayOrientation,
    width: i32,
    c1: u32,
    c2: u32,
    state: HatchwayState,
) -> Hatchway {
    Hatchway {
        id: HatchwayId::new(id),
        position: pos(x, y),
        orientation: orient,
        width: Inches::from_inches(width),
        between: (CompartmentId::new(c1), CompartmentId::new(c2)),
        initial_state: state,
        tags: Vec::new(),
    }
}

fn objective(id: u32, x: i32, y: i32, label: &str) -> BoardingObjectiveMarker {
    BoardingObjectiveMarker {
        id: ObjectiveId::new(id),
        position: pos(x, y),
        label: label.to_string(),
        tags: Vec::new(),
    }
}

fn entry_zone(
    id: u32,
    name: &str,
    role: EntryZoneRole,
    x1: i32, y1: i32, x2: i32, y2: i32,
    player: Option<u32>,
) -> EntryZone {
    EntryZone {
        id,
        name: name.to_string(),
        role,
        boundary: rect_poly(x1, y1, x2, y2),
        player_assignment: player.map(PlayerId::new),
        enabled: true,
    }
}

fn compartment(id: u32, name: &str, x1: i32, y1: i32, x2: i32, y2: i32) -> Compartment {
    Compartment {
        id: CompartmentId::new(id),
        name: name.to_string(),
        boundary: rect_poly(x1, y1, x2, y2),
        tags: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// BA-11: Access Junction Primus (Symmetric, 3 objectives)
// ---------------------------------------------------------------------------
// Layout: Mirrored control mission with central junction corridor.
// Three objective markers aligned through the central corridor/junction.
//
//  +------+------+----------+------+------+
//  |      |      |          |      |      |
//  | EZ-A | C-TL |  CENTER  | C-TR | EZ-B |
//  |      |      |   JUNC   |      |      |
//  |      +------+          +------+      |
//  |      |      |          |      |      |
//  |      | C-BL |          | C-BR |      |
//  |      |      |          |      |      |
//  +------+------+----------+------+------+
//
// Board: 48" × 28"
// Entry zones: 6" strips on left and right edges
// Central junction: 18-30" wide, 6-22" tall
// Four corner rooms (upper-left, lower-left, upper-right, lower-right)

fn build_ba_11() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // C0: Left entry area
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    // C1: Top-left room
    map.compartments.push(compartment(1, "Upper Left Room", 6, 16, 18, 28));
    // C2: Bottom-left room
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 18, 12));
    // C3: Central junction (the main corridor)
    map.compartments.push(compartment(3, "Central Junction", 18, 4, 30, 24));
    // C4: Top-right room
    map.compartments.push(compartment(4, "Upper Right Room", 30, 16, 42, 28));
    // C5: Bottom-right room
    map.compartments.push(compartment(5, "Lower Right Room", 30, 0, 42, 12));
    // C6: Right entry area
    map.compartments.push(compartment(6, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry to rooms - vertical wall at x=6 with gaps for hatchways
    map.walls.push(wall(wid, 6, 0, 6, 10)); wid += 1;    // bottom segment
    map.walls.push(wall(wid, 6, 12, 6, 16)); wid += 1;   // middle segment
    map.walls.push(wall(wid, 6, 18, 6, 28)); wid += 1;   // top segment

    // Left rooms to central junction - vertical wall at x=18 with gaps
    map.walls.push(wall(wid, 18, 0, 18, 6)); wid += 1;    // bottom
    map.walls.push(wall(wid, 18, 10, 18, 14)); wid += 1;  // middle-lower gap close
    map.walls.push(wall(wid, 18, 18, 18, 24)); wid += 1;  // middle-upper gap close
    map.walls.push(wall(wid, 18, 24, 18, 28)); wid += 1;  // top

    // Central junction to right rooms - vertical wall at x=30 with gaps
    map.walls.push(wall(wid, 30, 0, 30, 6)); wid += 1;
    map.walls.push(wall(wid, 30, 10, 30, 14)); wid += 1;
    map.walls.push(wall(wid, 30, 18, 30, 24)); wid += 1;
    map.walls.push(wall(wid, 30, 24, 30, 28)); wid += 1;

    // Right rooms to right entry - vertical wall at x=42 with gaps
    map.walls.push(wall(wid, 42, 0, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 42, 12, 42, 16)); wid += 1;
    map.walls.push(wall(wid, 42, 18, 42, 28)); wid += 1;

    // Horizontal walls separating upper/lower rooms
    // Left side: horizontal wall at y=12 (bottom of upper, top of lower)
    map.walls.push(wall(wid, 6, 12, 18, 12)); wid += 1;
    map.walls.push(wall(wid, 6, 16, 18, 16)); wid += 1;
    // Right side
    map.walls.push(wall(wid, 30, 12, 42, 12)); wid += 1;
    map.walls.push(wall(wid, 30, 16, 42, 16)); wid += 1;

    // Central junction top/bottom walls
    map.walls.push(wall(wid, 18, 4, 30, 4)); wid += 1;   // bottom of junction
    map.walls.push(wall(wid, 18, 24, 30, 24)); wid += 1;  // top of junction

    // Board perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;    // bottom edge
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;   // top edge
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;     // left edge
    map.walls.push(wall(wid, 48, 0, 48, 28)); // right edge

    // --- Hatchways ---
    // Left entry → rooms
    map.hatchways.push(hatch(0, 6, 11, Vertical, 2, 0, 2, Open));   // to lower-left
    map.hatchways.push(hatch(1, 6, 17, Vertical, 2, 0, 1, Open));   // to upper-left

    // Left rooms → central junction
    map.hatchways.push(hatch(2, 18, 8, Vertical, 2, 2, 3, Closed));  // lower-left to junction
    map.hatchways.push(hatch(3, 18, 20, Vertical, 2, 1, 3, Closed)); // upper-left to junction

    // Central junction → right rooms
    map.hatchways.push(hatch(4, 30, 8, Vertical, 2, 3, 5, Closed));  // junction to lower-right
    map.hatchways.push(hatch(5, 30, 20, Vertical, 2, 3, 4, Closed)); // junction to upper-right

    // Right rooms → right entry
    map.hatchways.push(hatch(6, 42, 11, Vertical, 2, 5, 6, Open));   // lower-right to entry
    map.hatchways.push(hatch(7, 42, 17, Vertical, 2, 4, 6, Open));   // upper-right to entry

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Three objectives aligned through the central corridor/junction
    map.objectives.push(objective(0, 14, 14, "Objective A"));
    map.objectives.push(objective(1, 24, 14, "Objective B"));
    map.objectives.push(objective(2, 34, 14, "Objective C"));

    // --- Special Regions ---
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Central Junction".to_string(),
        boundary: rect_poly(18, 4, 30, 24),
        tags: vec!["junction".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// Generic symmetric layout (fallback for missions without specific geometry)
// ---------------------------------------------------------------------------
// Same basic structure as BA-11 but slightly different proportions.
// Used for BA-12, BA-13, BA-21, BA-22, BA-23, BA-31, BA-32, BA-33 until
// mission-specific geometry is transcribed.

fn build_generic_symmetric() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // 7 compartments: entry-A, 2 left rooms, central corridor, 2 right rooms, entry-B
    map.compartments.push(compartment(0, "Entry Zone A", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Room A-Upper", 6, 15, 18, 28));
    map.compartments.push(compartment(2, "Room A-Lower", 6, 0, 18, 13));
    map.compartments.push(compartment(3, "Central Corridor", 18, 4, 30, 24));
    map.compartments.push(compartment(4, "Room B-Upper", 30, 15, 42, 28));
    map.compartments.push(compartment(5, "Room B-Lower", 30, 0, 42, 13));
    map.compartments.push(compartment(6, "Entry Zone B", 42, 0, 48, 28));

    // Walls
    let mut wid = 0u32;
    // Left entry boundary
    map.walls.push(wall(wid, 6, 0, 6, 11)); wid += 1;
    map.walls.push(wall(wid, 6, 13, 6, 15)); wid += 1;
    map.walls.push(wall(wid, 6, 17, 6, 28)); wid += 1;
    // Left rooms to center
    map.walls.push(wall(wid, 18, 0, 18, 7)); wid += 1;
    map.walls.push(wall(wid, 18, 11, 18, 17)); wid += 1;
    map.walls.push(wall(wid, 18, 21, 18, 28)); wid += 1;
    // Center to right rooms
    map.walls.push(wall(wid, 30, 0, 30, 7)); wid += 1;
    map.walls.push(wall(wid, 30, 11, 30, 17)); wid += 1;
    map.walls.push(wall(wid, 30, 21, 30, 28)); wid += 1;
    // Right rooms to right entry
    map.walls.push(wall(wid, 42, 0, 42, 11)); wid += 1;
    map.walls.push(wall(wid, 42, 13, 42, 15)); wid += 1;
    map.walls.push(wall(wid, 42, 17, 42, 28)); wid += 1;
    // Room horizontal dividers
    map.walls.push(wall(wid, 6, 13, 18, 13)); wid += 1;
    map.walls.push(wall(wid, 6, 15, 18, 15)); wid += 1;
    map.walls.push(wall(wid, 30, 13, 42, 13)); wid += 1;
    map.walls.push(wall(wid, 30, 15, 42, 15)); wid += 1;
    // Center top/bottom
    map.walls.push(wall(wid, 18, 4, 30, 4)); wid += 1;
    map.walls.push(wall(wid, 18, 24, 30, 24)); wid += 1;
    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // Hatchways
    map.hatchways.push(hatch(0, 6, 12, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 16, Vertical, 2, 0, 1, Open));
    map.hatchways.push(hatch(2, 18, 9, Vertical, 2, 2, 3, Closed));
    map.hatchways.push(hatch(3, 18, 19, Vertical, 2, 1, 3, Closed));
    map.hatchways.push(hatch(4, 30, 9, Vertical, 2, 3, 5, Closed));
    map.hatchways.push(hatch(5, 30, 19, Vertical, 2, 3, 4, Closed));
    map.hatchways.push(hatch(6, 42, 12, Vertical, 2, 5, 6, Open));
    map.hatchways.push(hatch(7, 42, 16, Vertical, 2, 4, 6, Open));

    // Entry zones
    map.entry_zones.push(entry_zone(0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0)));
    map.entry_zones.push(entry_zone(1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1)));

    // Default 3 objectives along center
    map.objectives.push(objective(0, 14, 14, "Objective A"));
    map.objectives.push(objective(1, 24, 14, "Objective B"));
    map.objectives.push(objective(2, 34, 14, "Objective C"));

    map
}

// ---------------------------------------------------------------------------
// BA-12: Deck Sweepers (Symmetric, 3 objectives, underdog entry zone)
// ---------------------------------------------------------------------------
// Layout: Central enclosed chamber with rooms branching on each side.
// Three objectives spread across the central band. Underdog entry zone
// at the board center bottom edge.
//
// Source: DECK_SWEEPERS.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-12)

fn build_ba_12() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 18, 16, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 16, 10));
    map.compartments.push(compartment(3, "Left Corridor", 6, 10, 16, 18));
    map.compartments.push(compartment(4, "Central Chamber", 16, 10, 32, 18));
    map.compartments.push(compartment(5, "Upper Passage", 16, 18, 32, 28));
    map.compartments.push(compartment(6, "Lower Passage", 16, 0, 32, 10));
    map.compartments.push(compartment(7, "Upper Right Room", 32, 18, 42, 28));
    map.compartments.push(compartment(8, "Lower Right Room", 32, 0, 42, 10));
    map.compartments.push(compartment(9, "Right Corridor", 32, 10, 42, 18));
    map.compartments.push(compartment(10, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 9)); wid += 1;
    map.walls.push(wall(wid, 6, 11, 6, 17)); wid += 1;
    map.walls.push(wall(wid, 6, 19, 6, 28)); wid += 1;

    // Left rooms to central (x=16)
    map.walls.push(wall(wid, 16, 0, 16, 8)); wid += 1;
    map.walls.push(wall(wid, 16, 12, 16, 16)); wid += 1;
    map.walls.push(wall(wid, 16, 20, 16, 28)); wid += 1;

    // Central chamber walls
    map.walls.push(wall(wid, 16, 10, 32, 10)); wid += 1;   // bottom of chamber
    map.walls.push(wall(wid, 16, 18, 32, 18)); wid += 1;   // top of chamber

    // Central to right rooms (x=32)
    map.walls.push(wall(wid, 32, 0, 32, 8)); wid += 1;
    map.walls.push(wall(wid, 32, 12, 32, 16)); wid += 1;
    map.walls.push(wall(wid, 32, 20, 32, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 9)); wid += 1;
    map.walls.push(wall(wid, 42, 11, 42, 17)); wid += 1;
    map.walls.push(wall(wid, 42, 19, 42, 28)); wid += 1;

    // Horizontal room dividers
    map.walls.push(wall(wid, 6, 10, 16, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 16, 18)); wid += 1;
    map.walls.push(wall(wid, 32, 10, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 42, 18)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Left entry to rooms
    map.hatchways.push(hatch(0, 6, 10, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 18, Vertical, 2, 0, 1, Open));

    // Left corridor to central chamber
    map.hatchways.push(hatch(2, 16, 14, Vertical, 2, 3, 4, Closed));

    // Left rooms to passages
    map.hatchways.push(hatch(3, 16, 9, Vertical, 2, 2, 6, Closed));
    map.hatchways.push(hatch(4, 16, 19, Vertical, 2, 1, 5, Closed));

    // Right corridor to central chamber
    map.hatchways.push(hatch(5, 32, 14, Vertical, 2, 4, 9, Closed));

    // Right rooms to passages
    map.hatchways.push(hatch(6, 32, 9, Vertical, 2, 6, 8, Closed));
    map.hatchways.push(hatch(7, 32, 19, Vertical, 2, 5, 7, Closed));

    // Right rooms to entry
    map.hatchways.push(hatch(8, 42, 10, Vertical, 2, 8, 10, Open));
    map.hatchways.push(hatch(9, 42, 18, Vertical, 2, 7, 10, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));
    // Underdog entry zone — center bottom edge, available during Deploy Armies only
    map.entry_zones.push(entry_zone(
        2, "Underdog Entry Zone", EntryZoneRole::Underdog, 20, 0, 28, 6, None,
    ));

    // --- Objectives ---
    map.objectives.push(objective(0, 16, 14, "Objective A"));
    map.objectives.push(objective(1, 24, 14, "Objective B"));
    map.objectives.push(objective(2, 32, 14, "Objective C"));

    map
}

// ---------------------------------------------------------------------------
// BA-13: The Pipeline (Symmetric, 4 objectives, power lines)
// ---------------------------------------------------------------------------
// Layout: Staggered rooms creating a winding pipeline corridor. Objectives
// placed along the pipeline in a zigzag pattern. Two "A" markers and two "B"
// markers connected by Power Lines for linked scoring.
//
// Source: THE_PIPELINE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-13)

fn build_ba_13() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // The Pipeline has a zigzag layout: rooms alternate between upper and lower
    // sections, creating a winding corridor through the board.
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 16, 14, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 14, 12));
    map.compartments.push(compartment(3, "Left Mid Upper", 14, 14, 24, 28));
    map.compartments.push(compartment(4, "Left Mid Lower", 14, 0, 24, 14));
    map.compartments.push(compartment(5, "Right Mid Upper", 24, 14, 34, 28));
    map.compartments.push(compartment(6, "Right Mid Lower", 24, 0, 34, 14));
    map.compartments.push(compartment(7, "Upper Right Room", 34, 16, 42, 28));
    map.compartments.push(compartment(8, "Lower Right Room", 34, 0, 42, 12));
    map.compartments.push(compartment(9, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 14, 6, 16)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 6, 28)); wid += 1;

    // Left rooms to mid sections (x=14)
    map.walls.push(wall(wid, 14, 0, 14, 6)); wid += 1;
    map.walls.push(wall(wid, 14, 8, 14, 12)); wid += 1;
    map.walls.push(wall(wid, 14, 16, 14, 22)); wid += 1;
    map.walls.push(wall(wid, 14, 24, 14, 28)); wid += 1;

    // Board center seam (x=24)
    map.walls.push(wall(wid, 24, 0, 24, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 8, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 24, 24, 28)); wid += 1;

    // Right mid to rooms (x=34)
    map.walls.push(wall(wid, 34, 0, 34, 6)); wid += 1;
    map.walls.push(wall(wid, 34, 8, 34, 12)); wid += 1;
    map.walls.push(wall(wid, 34, 16, 34, 22)); wid += 1;
    map.walls.push(wall(wid, 34, 24, 34, 28)); wid += 1;

    // Right rooms to entry (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 42, 14, 42, 16)); wid += 1;
    map.walls.push(wall(wid, 42, 18, 42, 28)); wid += 1;

    // Horizontal dividers
    map.walls.push(wall(wid, 6, 12, 14, 12)); wid += 1;
    map.walls.push(wall(wid, 6, 16, 14, 16)); wid += 1;
    map.walls.push(wall(wid, 14, 14, 24, 14)); wid += 1;
    map.walls.push(wall(wid, 24, 14, 34, 14)); wid += 1;
    map.walls.push(wall(wid, 34, 12, 42, 12)); wid += 1;
    map.walls.push(wall(wid, 34, 16, 42, 16)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Left entry to rooms
    map.hatchways.push(hatch(0, 6, 11, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 17, Vertical, 2, 0, 1, Open));

    // Pipeline zigzag hatchways
    map.hatchways.push(hatch(2, 14, 7, Vertical, 2, 2, 4, Closed));
    map.hatchways.push(hatch(3, 14, 23, Vertical, 2, 1, 3, Closed));
    map.hatchways.push(hatch(4, 24, 7, Vertical, 2, 4, 6, Closed));
    map.hatchways.push(hatch(5, 24, 23, Vertical, 2, 3, 5, Closed));
    map.hatchways.push(hatch(6, 34, 7, Vertical, 2, 6, 8, Closed));
    map.hatchways.push(hatch(7, 34, 23, Vertical, 2, 5, 7, Closed));

    // Right rooms to entry
    map.hatchways.push(hatch(8, 42, 11, Vertical, 2, 8, 9, Open));
    map.hatchways.push(hatch(9, 42, 17, Vertical, 2, 7, 9, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Pipeline objectives in zigzag: A markers at upper positions, B markers at lower
    map.objectives.push(objective(0, 10, 22, "Pipeline Marker A1"));
    map.objectives.push(objective(1, 19, 7, "Pipeline Marker B1"));
    map.objectives.push(objective(2, 29, 21, "Pipeline Marker A2"));
    map.objectives.push(objective(3, 38, 7, "Pipeline Marker B2"));

    // --- Special Regions ---
    // Power Line network connecting all four objectives
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Power Line Network".to_string(),
        boundary: rect_poly(6, 0, 42, 28),
        tags: vec!["power_lines".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-21: Power Struggle (Symmetric, 4 objectives, power lines, warlord kill)
// ---------------------------------------------------------------------------
// Layout: Central power line corridor (highlighted blue in the mission map)
// connecting 4 objectives. Rooms flank the corridor on both sides.
//
// Source: POWER_STRUGGLE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-21)

fn build_ba_21() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 18, 16, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 16, 10));
    map.compartments.push(compartment(3, "Left Side Corridor", 6, 10, 16, 18));
    map.compartments.push(compartment(4, "Power Line Corridor", 16, 6, 32, 22));
    map.compartments.push(compartment(5, "Upper Central Passage", 16, 22, 32, 28));
    map.compartments.push(compartment(6, "Lower Central Passage", 16, 0, 32, 6));
    map.compartments.push(compartment(7, "Upper Right Room", 32, 18, 42, 28));
    map.compartments.push(compartment(8, "Lower Right Room", 32, 0, 42, 10));
    map.compartments.push(compartment(9, "Right Side Corridor", 32, 10, 42, 18));
    map.compartments.push(compartment(10, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 9)); wid += 1;
    map.walls.push(wall(wid, 6, 11, 6, 17)); wid += 1;
    map.walls.push(wall(wid, 6, 19, 6, 28)); wid += 1;

    // Left to power line corridor (x=16)
    map.walls.push(wall(wid, 16, 0, 16, 4)); wid += 1;
    map.walls.push(wall(wid, 16, 8, 16, 12)); wid += 1;
    map.walls.push(wall(wid, 16, 16, 16, 20)); wid += 1;
    map.walls.push(wall(wid, 16, 24, 16, 28)); wid += 1;

    // Power line corridor boundaries
    map.walls.push(wall(wid, 16, 6, 32, 6)); wid += 1;     // bottom of corridor
    map.walls.push(wall(wid, 16, 22, 32, 22)); wid += 1;   // top of corridor

    // Right side of corridor (x=32)
    map.walls.push(wall(wid, 32, 0, 32, 4)); wid += 1;
    map.walls.push(wall(wid, 32, 8, 32, 12)); wid += 1;
    map.walls.push(wall(wid, 32, 16, 32, 20)); wid += 1;
    map.walls.push(wall(wid, 32, 24, 32, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 9)); wid += 1;
    map.walls.push(wall(wid, 42, 11, 42, 17)); wid += 1;
    map.walls.push(wall(wid, 42, 19, 42, 28)); wid += 1;

    // Horizontal room dividers
    map.walls.push(wall(wid, 6, 10, 16, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 16, 18)); wid += 1;
    map.walls.push(wall(wid, 32, 10, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 42, 18)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    map.hatchways.push(hatch(0, 6, 10, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 18, Vertical, 2, 0, 1, Open));

    // Into power line corridor
    map.hatchways.push(hatch(2, 16, 5, Vertical, 2, 6, 4, Closed));
    map.hatchways.push(hatch(3, 16, 14, Vertical, 2, 3, 4, Closed));
    map.hatchways.push(hatch(4, 16, 23, Vertical, 2, 5, 4, Closed));

    // Out of power line corridor
    map.hatchways.push(hatch(5, 32, 5, Vertical, 2, 4, 6, Closed));
    map.hatchways.push(hatch(6, 32, 14, Vertical, 2, 4, 9, Closed));
    map.hatchways.push(hatch(7, 32, 23, Vertical, 2, 4, 5, Closed));

    map.hatchways.push(hatch(8, 42, 10, Vertical, 2, 8, 10, Open));
    map.hatchways.push(hatch(9, 42, 18, Vertical, 2, 7, 10, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Four objectives along the Power Line corridor
    map.objectives.push(objective(0, 18, 14, "Objective A"));
    map.objectives.push(objective(1, 22, 14, "Objective B"));
    map.objectives.push(objective(2, 26, 14, "Objective C"));
    map.objectives.push(objective(3, 30, 14, "Objective D"));

    // --- Special Regions ---
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Power Lines".to_string(),
        boundary: rect_poly(16, 6, 32, 22),
        tags: vec!["power_lines".to_string(), "power_network".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-22: Death in the Dark (Symmetric, 4 objectives, 2 lighting areas)
// ---------------------------------------------------------------------------
// Layout: Entry zones on the SHORT edges of the board (y=0 and y=28 sides),
// not the long edges. Two Lighting Areas divide the board into left and right
// halves, each containing 2 objectives. Lighting state rolls per area.
//
// Source: DEATH_IN_THE_DARK.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-22)

fn build_ba_22() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // Entry zones run along the short edges (full width, 6" deep)
    map.compartments.push(compartment(0, "Entry Zone A", 0, 22, 48, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 0, 14, 12, 22));
    map.compartments.push(compartment(2, "Upper Center-Left", 12, 14, 24, 22));
    map.compartments.push(compartment(3, "Upper Center-Right", 24, 14, 36, 22));
    map.compartments.push(compartment(4, "Upper Right Room", 36, 14, 48, 22));
    map.compartments.push(compartment(5, "Lower Left Room", 0, 6, 12, 14));
    map.compartments.push(compartment(6, "Lower Center-Left", 12, 6, 24, 14));
    map.compartments.push(compartment(7, "Lower Center-Right", 24, 6, 36, 14));
    map.compartments.push(compartment(8, "Lower Right Room", 36, 6, 48, 14));
    map.compartments.push(compartment(9, "Entry Zone B", 0, 0, 48, 6));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundaries (horizontal walls)
    // Top entry zone boundary (y=22)
    map.walls.push(wall(wid, 0, 22, 10, 22)); wid += 1;
    map.walls.push(wall(wid, 14, 22, 22, 22)); wid += 1;
    map.walls.push(wall(wid, 26, 22, 34, 22)); wid += 1;
    map.walls.push(wall(wid, 38, 22, 48, 22)); wid += 1;

    // Bottom entry zone boundary (y=6)
    map.walls.push(wall(wid, 0, 6, 10, 6)); wid += 1;
    map.walls.push(wall(wid, 14, 6, 22, 6)); wid += 1;
    map.walls.push(wall(wid, 26, 6, 34, 6)); wid += 1;
    map.walls.push(wall(wid, 38, 6, 48, 6)); wid += 1;

    // Vertical room dividers
    map.walls.push(wall(wid, 12, 6, 12, 12)); wid += 1;
    map.walls.push(wall(wid, 12, 16, 12, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 22)); wid += 1;

    // Horizontal middle divider (y=14)
    map.walls.push(wall(wid, 0, 14, 10, 14)); wid += 1;
    map.walls.push(wall(wid, 14, 14, 22, 14)); wid += 1;
    map.walls.push(wall(wid, 26, 14, 34, 14)); wid += 1;
    map.walls.push(wall(wid, 38, 14, 48, 14)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry zone A (top) to upper rooms
    map.hatchways.push(hatch(0, 12, 22, Horizontal, 2, 0, 1, Open));
    map.hatchways.push(hatch(1, 24, 22, Horizontal, 2, 0, 2, Open));
    map.hatchways.push(hatch(2, 36, 22, Horizontal, 2, 0, 4, Open));

    // Upper to lower rooms (y=14 horizontal hatchways)
    map.hatchways.push(hatch(3, 6, 14, Horizontal, 2, 1, 5, Closed));
    map.hatchways.push(hatch(4, 18, 14, Horizontal, 2, 2, 6, Closed));
    map.hatchways.push(hatch(5, 30, 14, Horizontal, 2, 3, 7, Closed));
    map.hatchways.push(hatch(6, 42, 14, Horizontal, 2, 4, 8, Closed));

    // Vertical hatchways between rooms
    map.hatchways.push(hatch(7, 12, 14, Vertical, 2, 1, 2, Closed));
    map.hatchways.push(hatch(8, 24, 14, Vertical, 2, 6, 7, Closed));
    map.hatchways.push(hatch(9, 36, 14, Vertical, 2, 3, 4, Closed));

    // Entry zone B (bottom) to lower rooms
    map.hatchways.push(hatch(10, 12, 6, Horizontal, 2, 9, 5, Open));
    map.hatchways.push(hatch(11, 24, 6, Horizontal, 2, 9, 7, Open));
    map.hatchways.push(hatch(12, 36, 6, Horizontal, 2, 9, 8, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(1),
    ));

    // --- Objectives ---
    // 4 objectives split between two Lighting Areas (2 per area)
    map.objectives.push(objective(0, 8, 18, "Objective A1"));
    map.objectives.push(objective(1, 8, 10, "Objective A2"));
    map.objectives.push(objective(2, 40, 18, "Objective B1"));
    map.objectives.push(objective(3, 40, 10, "Objective B2"));

    // --- Special Regions ---
    // Lighting Area 1 (left half)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Lighting Area 1".to_string(),
        boundary: rect_poly(0, 6, 24, 22),
        tags: vec!["lighting_area".to_string()],
    });
    // Lighting Area 2 (right half)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Lighting Area 2".to_string(),
        boundary: rect_poly(24, 6, 48, 22),
        tags: vec!["lighting_area".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-23: Hull Breach (Symmetric, 3 objectives, numbered compartments, venting)
// ---------------------------------------------------------------------------
// Layout: Five explicitly numbered compartments used for venting mechanics.
// Three datacore objective markers. Compartments 1, 2, 4, 5 are the venting
// targets; the defender chooses which compartment to vent when triggered.
//
// Source: HULL_BREACH.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-23)

fn build_ba_23() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // Numbered compartments are crucial for venting mechanics
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Compartment 4", 6, 16, 18, 28));
    map.compartments.push(compartment(2, "Compartment 1", 6, 0, 18, 12));
    map.compartments.push(compartment(3, "Compartment 5", 18, 8, 30, 20));
    map.compartments.push(compartment(4, "Compartment 2", 18, 0, 30, 8));
    map.compartments.push(compartment(5, "Upper Passage", 18, 20, 30, 28));
    map.compartments.push(compartment(6, "Compartment 3", 30, 16, 42, 28));
    map.compartments.push(compartment(7, "Lower Right", 30, 0, 42, 12));
    map.compartments.push(compartment(8, "Right Corridor", 30, 12, 42, 16));
    map.compartments.push(compartment(9, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 14, 6, 16)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 6, 28)); wid += 1;

    // Compartments 4,1 to center (x=18)
    map.walls.push(wall(wid, 18, 0, 18, 6)); wid += 1;
    map.walls.push(wall(wid, 18, 10, 18, 16)); wid += 1;
    map.walls.push(wall(wid, 18, 22, 18, 28)); wid += 1;

    // Center to right compartments (x=30)
    map.walls.push(wall(wid, 30, 0, 30, 6)); wid += 1;
    map.walls.push(wall(wid, 30, 10, 30, 14)); wid += 1;
    map.walls.push(wall(wid, 30, 18, 30, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 42, 14, 42, 16)); wid += 1;
    map.walls.push(wall(wid, 42, 18, 42, 28)); wid += 1;

    // Horizontal dividers
    map.walls.push(wall(wid, 6, 12, 18, 12)); wid += 1;
    map.walls.push(wall(wid, 6, 16, 18, 16)); wid += 1;
    map.walls.push(wall(wid, 18, 8, 30, 8)); wid += 1;
    map.walls.push(wall(wid, 18, 20, 30, 20)); wid += 1;
    map.walls.push(wall(wid, 30, 12, 42, 12)); wid += 1;
    map.walls.push(wall(wid, 30, 16, 42, 16)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Left entry to compartments
    map.hatchways.push(hatch(0, 6, 11, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 17, Vertical, 2, 0, 1, Open));

    // Compartment 4 to Compartment 5
    map.hatchways.push(hatch(2, 18, 18, Vertical, 2, 1, 3, Closed));
    // Compartment 1 to Compartment 2
    map.hatchways.push(hatch(3, 18, 7, Vertical, 2, 2, 4, Closed));

    // Compartment 5 to Compartment 3
    map.hatchways.push(hatch(4, 30, 16, Vertical, 2, 3, 6, Closed));
    // Compartment 5 to Lower Right
    map.hatchways.push(hatch(5, 30, 7, Vertical, 2, 4, 7, Closed));

    // Upper Passage to Compartment 3
    map.hatchways.push(hatch(6, 18, 21, Vertical, 2, 5, 3, Closed));

    // Right side to entry
    map.hatchways.push(hatch(7, 42, 11, Vertical, 2, 7, 9, Open));
    map.hatchways.push(hatch(8, 42, 17, Vertical, 2, 6, 9, Open));

    // Cross-compartment hatchway
    map.hatchways.push(hatch(9, 42, 15, Vertical, 2, 8, 9, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Three datacore objective markers spread across compartments
    map.objectives.push(objective(0, 12, 22, "Datacore Alpha"));
    map.objectives.push(objective(1, 24, 14, "Datacore Beta"));
    map.objectives.push(objective(2, 36, 6, "Datacore Gamma"));

    // --- Special Regions ---
    // Numbered compartments used for venting mechanics
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Compartment 1".to_string(),
        boundary: rect_poly(6, 0, 18, 12),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Compartment 2".to_string(),
        boundary: rect_poly(18, 0, 30, 8),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(2),
        name: "Compartment 4".to_string(),
        boundary: rect_poly(6, 16, 18, 28),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(3),
        name: "Compartment 5".to_string(),
        boundary: rect_poly(18, 8, 30, 20),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-31: Control Centre (Symmetric, 4 objectives, global hatch unlock)
// ---------------------------------------------------------------------------
// Layout: Central control room surrounded by rooms on all sides.
// Objective A is the primary scoring marker. Objective B controls
// the "Unlock Overrides" ability to open all hatchways simultaneously.
//
// Source: CONTROL_CENTRE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-31)

fn build_ba_31() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 18, 16, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 16, 10));
    map.compartments.push(compartment(3, "Left Ante-room", 6, 10, 16, 18));
    map.compartments.push(compartment(4, "Control Centre", 16, 8, 32, 20));
    map.compartments.push(compartment(5, "Upper Corridor", 16, 20, 32, 28));
    map.compartments.push(compartment(6, "Lower Corridor", 16, 0, 32, 8));
    map.compartments.push(compartment(7, "Upper Right Room", 32, 18, 42, 28));
    map.compartments.push(compartment(8, "Lower Right Room", 32, 0, 42, 10));
    map.compartments.push(compartment(9, "Right Ante-room", 32, 10, 42, 18));
    map.compartments.push(compartment(10, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 9)); wid += 1;
    map.walls.push(wall(wid, 6, 11, 6, 17)); wid += 1;
    map.walls.push(wall(wid, 6, 19, 6, 28)); wid += 1;

    // Left to control centre (x=16)
    map.walls.push(wall(wid, 16, 0, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 16, 10, 16, 12)); wid += 1;
    map.walls.push(wall(wid, 16, 16, 16, 18)); wid += 1;
    map.walls.push(wall(wid, 16, 22, 16, 28)); wid += 1;

    // Control centre boundaries
    map.walls.push(wall(wid, 16, 8, 32, 8)); wid += 1;
    map.walls.push(wall(wid, 16, 20, 32, 20)); wid += 1;

    // Right of control centre (x=32)
    map.walls.push(wall(wid, 32, 0, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 32, 10, 32, 12)); wid += 1;
    map.walls.push(wall(wid, 32, 16, 32, 18)); wid += 1;
    map.walls.push(wall(wid, 32, 22, 32, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 9)); wid += 1;
    map.walls.push(wall(wid, 42, 11, 42, 17)); wid += 1;
    map.walls.push(wall(wid, 42, 19, 42, 28)); wid += 1;

    // Horizontal room dividers
    map.walls.push(wall(wid, 6, 10, 16, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 16, 18)); wid += 1;
    map.walls.push(wall(wid, 32, 10, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 42, 18)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    map.hatchways.push(hatch(0, 6, 10, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 18, Vertical, 2, 0, 1, Open));

    // Into control centre
    map.hatchways.push(hatch(2, 16, 7, Vertical, 2, 6, 4, Closed));
    map.hatchways.push(hatch(3, 16, 14, Vertical, 2, 3, 4, Closed));
    map.hatchways.push(hatch(4, 16, 21, Vertical, 2, 5, 4, Closed));

    // Out of control centre
    map.hatchways.push(hatch(5, 32, 7, Vertical, 2, 4, 6, Closed));
    map.hatchways.push(hatch(6, 32, 14, Vertical, 2, 4, 9, Closed));
    map.hatchways.push(hatch(7, 32, 21, Vertical, 2, 4, 5, Closed));

    map.hatchways.push(hatch(8, 42, 10, Vertical, 2, 8, 10, Open));
    map.hatchways.push(hatch(9, 42, 18, Vertical, 2, 7, 10, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Objective A: primary scoring marker (center of control room)
    map.objectives.push(objective(0, 24, 14, "Objective A"));
    // Objective B: controls Unlock Overrides (off-center in control room)
    map.objectives.push(objective(1, 24, 18, "Objective B"));
    // Two additional objectives in flanking rooms
    map.objectives.push(objective(2, 10, 14, "Objective C"));
    map.objectives.push(objective(3, 38, 14, "Objective D"));

    // --- Special Regions ---
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Control Centre".to_string(),
        boundary: rect_poly(16, 8, 32, 20),
        tags: vec!["control_centre".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-32: The Furnace (Symmetric, 3 objectives, furnace zone, burners)
// ---------------------------------------------------------------------------
// Layout: Large central Furnace area (dark shaded region) surrounded by
// rooms and corridors. Furnace Control Zones flank the Furnace. Any unit
// can Secure Site on objectives within the Furnace.
//
// Source: THE_FURNACE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-32)

fn build_ba_32() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 20, 16, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 16, 8));
    map.compartments.push(compartment(3, "Left Control Zone", 6, 8, 16, 20));
    map.compartments.push(compartment(4, "The Furnace", 16, 6, 32, 22));
    map.compartments.push(compartment(5, "Upper Bypass", 16, 22, 32, 28));
    map.compartments.push(compartment(6, "Lower Bypass", 16, 0, 32, 6));
    map.compartments.push(compartment(7, "Right Control Zone", 32, 8, 42, 20));
    map.compartments.push(compartment(8, "Upper Right Room", 32, 20, 42, 28));
    map.compartments.push(compartment(9, "Lower Right Room", 32, 0, 42, 8));
    map.compartments.push(compartment(10, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 7)); wid += 1;
    map.walls.push(wall(wid, 6, 9, 6, 19)); wid += 1;
    map.walls.push(wall(wid, 6, 21, 6, 28)); wid += 1;

    // Left to furnace (x=16)
    map.walls.push(wall(wid, 16, 0, 16, 4)); wid += 1;
    map.walls.push(wall(wid, 16, 8, 16, 10)); wid += 1;
    map.walls.push(wall(wid, 16, 18, 16, 20)); wid += 1;
    map.walls.push(wall(wid, 16, 24, 16, 28)); wid += 1;

    // Furnace boundaries
    map.walls.push(wall(wid, 16, 6, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 16, 22, 32, 22)); wid += 1;

    // Right of furnace (x=32)
    map.walls.push(wall(wid, 32, 0, 32, 4)); wid += 1;
    map.walls.push(wall(wid, 32, 8, 32, 10)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 32, 20)); wid += 1;
    map.walls.push(wall(wid, 32, 24, 32, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 7)); wid += 1;
    map.walls.push(wall(wid, 42, 9, 42, 19)); wid += 1;
    map.walls.push(wall(wid, 42, 21, 42, 28)); wid += 1;

    // Horizontal dividers
    map.walls.push(wall(wid, 6, 8, 16, 8)); wid += 1;
    map.walls.push(wall(wid, 6, 20, 16, 20)); wid += 1;
    map.walls.push(wall(wid, 32, 8, 42, 8)); wid += 1;
    map.walls.push(wall(wid, 32, 20, 42, 20)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    map.hatchways.push(hatch(0, 6, 8, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 20, Vertical, 2, 0, 1, Open));

    // Into the Furnace
    map.hatchways.push(hatch(2, 16, 5, Vertical, 2, 6, 4, Closed));
    map.hatchways.push(hatch(3, 16, 14, Vertical, 2, 3, 4, Closed));
    map.hatchways.push(hatch(4, 16, 23, Vertical, 2, 5, 4, Closed));

    // Out of the Furnace
    map.hatchways.push(hatch(5, 32, 5, Vertical, 2, 4, 6, Closed));
    map.hatchways.push(hatch(6, 32, 14, Vertical, 2, 4, 7, Closed));
    map.hatchways.push(hatch(7, 32, 23, Vertical, 2, 4, 5, Closed));

    map.hatchways.push(hatch(8, 42, 8, Vertical, 2, 9, 10, Open));
    map.hatchways.push(hatch(9, 42, 20, Vertical, 2, 8, 10, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Objectives within and around the Furnace
    map.objectives.push(objective(0, 20, 14, "Furnace Objective A"));
    map.objectives.push(objective(1, 24, 14, "Furnace Objective B"));
    map.objectives.push(objective(2, 28, 14, "Furnace Objective C"));

    // --- Special Regions ---
    // The Furnace (central hazard zone)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "The Furnace".to_string(),
        boundary: rect_poly(16, 6, 32, 22),
        tags: vec!["furnace".to_string()],
    });
    // Furnace Control Zones (flanking the Furnace)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Left Furnace Control Zone".to_string(),
        boundary: rect_poly(6, 8, 16, 20),
        tags: vec!["furnace_control_zone".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(2),
        name: "Right Furnace Control Zone".to_string(),
        boundary: rect_poly(32, 8, 42, 20),
        tags: vec!["furnace_control_zone".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-33: Rad Leak (Symmetric, 3 objectives, 4 radiation sectors)
// ---------------------------------------------------------------------------
// Layout: Standard room arrangement with 4 radiation sectors (A, B, C, D)
// overlaying the board in quadrant regions. Two "Critical" objective markers
// plus one additional. Radiation effects vary by sector per round.
//
// Source: RAD_LEAK.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-33)

fn build_ba_33() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    map.compartments.push(compartment(0, "Left Entry", 0, 0, 6, 28));
    map.compartments.push(compartment(1, "Upper Left Room", 6, 18, 16, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 16, 10));
    map.compartments.push(compartment(3, "Left Corridor", 6, 10, 16, 18));
    map.compartments.push(compartment(4, "Center Upper Left", 16, 16, 24, 28));
    map.compartments.push(compartment(5, "Center Lower Left", 16, 0, 24, 12));
    map.compartments.push(compartment(6, "Central Passage", 16, 12, 32, 16));
    map.compartments.push(compartment(7, "Center Upper Right", 24, 16, 32, 28));
    map.compartments.push(compartment(8, "Center Lower Right", 24, 0, 32, 12));
    map.compartments.push(compartment(9, "Upper Right Room", 32, 18, 42, 28));
    map.compartments.push(compartment(10, "Lower Right Room", 32, 0, 42, 10));
    map.compartments.push(compartment(11, "Right Corridor", 32, 10, 42, 18));
    map.compartments.push(compartment(12, "Right Entry", 42, 0, 48, 28));

    // --- Walls ---
    let mut wid = 0u32;

    // Left entry boundary (x=6)
    map.walls.push(wall(wid, 6, 0, 6, 9)); wid += 1;
    map.walls.push(wall(wid, 6, 11, 6, 17)); wid += 1;
    map.walls.push(wall(wid, 6, 19, 6, 28)); wid += 1;

    // Left rooms to center (x=16)
    map.walls.push(wall(wid, 16, 0, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 16, 8, 16, 14)); wid += 1;
    map.walls.push(wall(wid, 16, 18, 16, 24)); wid += 1;
    map.walls.push(wall(wid, 16, 26, 16, 28)); wid += 1;

    // Board center (x=24)
    map.walls.push(wall(wid, 24, 0, 24, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 8, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 24, 24, 28)); wid += 1;

    // Center to right rooms (x=32)
    map.walls.push(wall(wid, 32, 0, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 32, 8, 32, 14)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 32, 24)); wid += 1;
    map.walls.push(wall(wid, 32, 26, 32, 28)); wid += 1;

    // Right entry boundary (x=42)
    map.walls.push(wall(wid, 42, 0, 42, 9)); wid += 1;
    map.walls.push(wall(wid, 42, 11, 42, 17)); wid += 1;
    map.walls.push(wall(wid, 42, 19, 42, 28)); wid += 1;

    // Horizontal dividers
    map.walls.push(wall(wid, 6, 10, 16, 10)); wid += 1;
    map.walls.push(wall(wid, 6, 18, 16, 18)); wid += 1;
    map.walls.push(wall(wid, 16, 12, 32, 12)); wid += 1;
    map.walls.push(wall(wid, 16, 16, 32, 16)); wid += 1;
    map.walls.push(wall(wid, 32, 10, 42, 10)); wid += 1;
    map.walls.push(wall(wid, 32, 18, 42, 18)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    map.hatchways.push(hatch(0, 6, 10, Vertical, 2, 0, 2, Open));
    map.hatchways.push(hatch(1, 6, 18, Vertical, 2, 0, 1, Open));

    // Left to center
    map.hatchways.push(hatch(2, 16, 7, Vertical, 2, 2, 5, Closed));
    map.hatchways.push(hatch(3, 16, 16, Vertical, 2, 3, 6, Closed));
    map.hatchways.push(hatch(4, 16, 25, Vertical, 2, 1, 4, Closed));

    // Board center hatchways
    map.hatchways.push(hatch(5, 24, 7, Vertical, 2, 5, 8, Closed));
    map.hatchways.push(hatch(6, 24, 14, Vertical, 2, 6, 6, Closed));
    map.hatchways.push(hatch(7, 24, 23, Vertical, 2, 4, 7, Closed));

    // Center to right
    map.hatchways.push(hatch(8, 32, 7, Vertical, 2, 8, 10, Closed));
    map.hatchways.push(hatch(9, 32, 16, Vertical, 2, 6, 11, Closed));
    map.hatchways.push(hatch(10, 32, 25, Vertical, 2, 7, 9, Closed));

    map.hatchways.push(hatch(11, 42, 10, Vertical, 2, 10, 12, Open));
    map.hatchways.push(hatch(12, 42, 18, Vertical, 2, 9, 12, Open));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Two Critical objectives + one additional
    map.objectives.push(objective(0, 14, 14, "Critical Objective A"));
    map.objectives.push(objective(1, 34, 14, "Critical Objective B"));
    map.objectives.push(objective(2, 24, 14, "Objective C"));

    // --- Special Regions ---
    // Four radiation sectors (quadrant overlays)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Sector A".to_string(),
        boundary: rect_poly(0, 14, 24, 28),
        tags: vec!["radiation_sector".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Sector B".to_string(),
        boundary: rect_poly(24, 14, 48, 28),
        tags: vec!["radiation_sector".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(2),
        name: "Sector C".to_string(),
        boundary: rect_poly(24, 0, 48, 14),
        tags: vec!["radiation_sector".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(3),
        name: "Sector D".to_string(),
        boundary: rect_poly(0, 0, 24, 14),
        tags: vec!["radiation_sector".to_string()],
    });

    map
}

// Asymmetric missions — attacker/defender entry zones differ
fn build_ba_asymmetric(mission_num: u32) -> BoardingMap {
    let mut map = build_generic_symmetric();

    // Asymmetric: Player A = Attacker, Player B = Defender
    // Defender's entry is a smaller "guard" area, Attacker gets full side
    map.entry_zones.clear();
    map.entry_zones.push(entry_zone(0, "Attacker Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(0)));
    map.entry_zones.push(entry_zone(1, "Defender Entry", EntryZoneRole::Guard, 42, 8, 48, 20, Some(1)));

    // Vary objectives based on mission
    match mission_num {
        1 => {
            // BA-1 "Void the Ship" — 3 objectives, attacker must reach defender side
            map.objectives.clear();
            map.objectives.push(objective(0, 12, 14, "Objective A"));
            map.objectives.push(objective(1, 24, 14, "Objective B"));
            map.objectives.push(objective(2, 36, 14, "Objective C"));
        }
        2 => {
            // BA-2 "Pull Their Teeth" — Control Node + Loader objectives
            map.objectives.clear();
            map.objectives.push(objective(0, 24, 14, "Control Node"));
            map.objectives.push(objective(1, 36, 8, "Loader A"));
            map.objectives.push(objective(2, 36, 20, "Loader B"));
        }
        _ => {
            // Other asymmetric missions use default 3 objectives
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ba_11_map_has_all_components() {
        let map = build_ba_11();
        assert_eq!(map.dimensions, BoardDimensions::BOARDING_ACTIONS);
        assert_eq!(map.compartments.len(), 7);
        assert!(map.walls.len() >= 20);
        assert_eq!(map.hatchways.len(), 8);
        assert_eq!(map.entry_zones.len(), 2);
        assert_eq!(map.objectives.len(), 3);
        assert_eq!(map.special_regions.len(), 1);
    }

    #[test]
    fn test_ba_11_entry_zones_assigned_to_players() {
        let map = build_ba_11();
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(0)));
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(1)));
    }

    #[test]
    fn test_ba_11_hatchway_states() {
        let map = build_ba_11();
        // Entry hatchways should be open
        assert_eq!(map.hatchways[0].initial_state, HatchwayState::Open);
        assert_eq!(map.hatchways[1].initial_state, HatchwayState::Open);
        // Interior hatchways should be closed
        assert_eq!(map.hatchways[2].initial_state, HatchwayState::Closed);
        assert_eq!(map.hatchways[3].initial_state, HatchwayState::Closed);
    }

    #[test]
    fn test_generic_fallback_works() {
        let map = load_mission_map(None);
        assert_eq!(map.dimensions, BoardDimensions::BOARDING_ACTIONS);
        assert!(!map.compartments.is_empty());
        assert!(!map.walls.is_empty());
        assert!(!map.hatchways.is_empty());
        assert!(!map.entry_zones.is_empty());
        assert!(!map.objectives.is_empty());
    }

    #[test]
    fn test_all_missions_produce_valid_maps() {
        for id in [1, 2, 3, 4, 5, 6, 11, 12, 13, 21, 22, 23, 31, 32, 33] {
            let map = load_mission_map(Some(MissionId::new(id)));
            assert!(!map.compartments.is_empty(), "Mission {} has no compartments", id);
            assert!(!map.walls.is_empty(), "Mission {} has no walls", id);
            assert!(!map.hatchways.is_empty(), "Mission {} has no hatchways", id);
            assert!(!map.entry_zones.is_empty(), "Mission {} has no entry zones", id);
            assert!(!map.objectives.is_empty(), "Mission {} has no objectives", id);
        }
    }

    // --- Mission-specific tests ---

    #[test]
    fn test_ba_12_deck_sweepers() {
        let map = build_ba_12();
        assert_eq!(map.compartments.len(), 11);
        assert_eq!(map.objectives.len(), 3);
        assert_eq!(map.entry_zones.len(), 3); // 2 main + 1 underdog
        assert_eq!(map.hatchways.len(), 10);
        // Underdog entry zone exists
        assert!(map.entry_zones.iter().any(|ez| ez.role == EntryZoneRole::Underdog));
        // Central Chamber compartment exists
        assert!(map.compartments.iter().any(|c| c.name == "Central Chamber"));
    }

    #[test]
    fn test_ba_13_the_pipeline() {
        let map = build_ba_13();
        assert_eq!(map.objectives.len(), 4); // 2 A markers + 2 B markers
        assert_eq!(map.entry_zones.len(), 2);
        assert!(map.hatchways.len() >= 10);
        // Power Line special region
        assert!(map.special_regions.iter().any(|r| r.tags.contains(&"power_lines".to_string())));
    }

    #[test]
    fn test_ba_21_power_struggle() {
        let map = build_ba_21();
        assert_eq!(map.objectives.len(), 4); // 4 objectives along power line
        assert_eq!(map.entry_zones.len(), 2);
        // Power Lines special region
        assert!(map.special_regions.iter().any(|r| r.name == "Power Lines"));
    }

    #[test]
    fn test_ba_22_death_in_the_dark() {
        let map = build_ba_22();
        assert_eq!(map.objectives.len(), 4); // 2 per lighting area
        assert_eq!(map.entry_zones.len(), 2);
        // Two Lighting Areas
        let lighting_areas: Vec<_> = map.special_regions.iter()
            .filter(|r| r.tags.contains(&"lighting_area".to_string()))
            .collect();
        assert_eq!(lighting_areas.len(), 2);
        // Entry zones on short edges (y-axis), not long edges (x-axis)
        // Entry zone A spans full width (x=0-48) at y=22-28
        let ez_a_y_min = map.entry_zones[0].boundary.vertices.iter()
            .map(|v| v.y.0)
            .min().unwrap();
        assert!(ez_a_y_min >= Inches::from_inches(22).0
            || ez_a_y_min <= Inches::from_inches(6).0);
    }

    #[test]
    fn test_ba_23_hull_breach() {
        let map = build_ba_23();
        assert_eq!(map.objectives.len(), 3); // 3 datacores
        assert_eq!(map.entry_zones.len(), 2);
        // 4 ventable compartments as special regions
        let ventable: Vec<_> = map.special_regions.iter()
            .filter(|r| r.tags.contains(&"ventable".to_string()))
            .collect();
        assert_eq!(ventable.len(), 4);
        // Named compartments 1, 2, 4, 5
        assert!(ventable.iter().any(|r| r.name == "Compartment 1"));
        assert!(ventable.iter().any(|r| r.name == "Compartment 2"));
        assert!(ventable.iter().any(|r| r.name == "Compartment 4"));
        assert!(ventable.iter().any(|r| r.name == "Compartment 5"));
    }

    #[test]
    fn test_ba_31_control_centre() {
        let map = build_ba_31();
        assert_eq!(map.objectives.len(), 4); // A, B + 2 more
        assert_eq!(map.entry_zones.len(), 2);
        // Control Centre special region
        assert!(map.special_regions.iter().any(|r| r.name == "Control Centre"));
        // Objectives A and B exist
        assert!(map.objectives.iter().any(|o| o.label == "Objective A"));
        assert!(map.objectives.iter().any(|o| o.label == "Objective B"));
    }

    #[test]
    fn test_ba_32_the_furnace() {
        let map = build_ba_32();
        assert_eq!(map.objectives.len(), 3);
        assert_eq!(map.entry_zones.len(), 2);
        // Furnace special region
        assert!(map.special_regions.iter().any(|r| r.name == "The Furnace"));
        // Furnace Control Zones
        let control_zones: Vec<_> = map.special_regions.iter()
            .filter(|r| r.tags.contains(&"furnace_control_zone".to_string()))
            .collect();
        assert_eq!(control_zones.len(), 2);
    }

    #[test]
    fn test_ba_33_rad_leak() {
        let map = build_ba_33();
        assert_eq!(map.objectives.len(), 3); // 2 critical + 1 additional
        assert_eq!(map.entry_zones.len(), 2);
        // 4 radiation sectors
        let sectors: Vec<_> = map.special_regions.iter()
            .filter(|r| r.tags.contains(&"radiation_sector".to_string()))
            .collect();
        assert_eq!(sectors.len(), 4);
        // Critical objectives labeled
        assert!(map.objectives.iter().any(|o| o.label.contains("Critical")));
    }

    #[test]
    fn test_each_symmetric_mission_has_unique_layout() {
        // Verify no two symmetric missions have the same compartment count AND wall count
        let missions = [11, 12, 13, 21, 22, 23, 31, 32, 33];
        let layouts: Vec<_> = missions.iter().map(|&id| {
            let map = load_mission_map(Some(MissionId::new(id)));
            (id, map.compartments.len(), map.walls.len(), map.objectives.len(),
             map.hatchways.len(), map.special_regions.len())
        }).collect();

        // Each mission should differ from BA-11 in at least one structural metric
        let ba_11 = &layouts[0];
        for layout in &layouts[1..] {
            let differs = layout.1 != ba_11.1
                || layout.2 != ba_11.2
                || layout.3 != ba_11.3
                || layout.4 != ba_11.4
                || layout.5 != ba_11.5;
            assert!(differs,
                "Mission {} has identical structure to BA-11", layout.0);
        }
    }
}
