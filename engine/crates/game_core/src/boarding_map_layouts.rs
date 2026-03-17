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
// Layout: Y-AXIS deployment (Player A from top/high-y, Player B from bottom/low-y).
// Board 1 (x=0-24) contains a fortified strongroom in its center.
// Board 2 (x=24-48) has a standard 2x2 room grid with 3 objectives.
// Underdog entry zone on the left x-edge (alternate entry for underdog player).
//
// Source: DECK_SWEEPERS.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-12)
//
// Coordinate reference (image portrait orientation):
//   image-top = x=0, image-bottom = x=48
//   image-left = y=28, image-right = y=0
//   Green shields (Player A) on image-left = y=22-28 side
//   Red X marks (Player B) on image-right = y=0-6 side

fn build_ba_12() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // Board 1 (x=0-24): Strongroom layout
    map.compartments.push(compartment(0, "Board1 Upper Band", 0, 18, 24, 22));
    map.compartments.push(compartment(1, "Board1 Left Wing", 0, 10, 8, 18));
    map.compartments.push(compartment(2, "Strongroom", 8, 10, 16, 18));
    map.compartments.push(compartment(3, "Board1 Right Wing", 16, 10, 24, 18));
    map.compartments.push(compartment(4, "Board1 Lower Band", 0, 6, 24, 10));

    // Board 2 (x=24-48): Standard 2x2 grid
    map.compartments.push(compartment(5, "Board2 Upper Left", 24, 14, 36, 22));
    map.compartments.push(compartment(6, "Board2 Upper Right", 36, 14, 48, 22));
    map.compartments.push(compartment(7, "Board2 Lower Left", 24, 6, 36, 14));
    map.compartments.push(compartment(8, "Board2 Lower Right", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundaries (y=22 and y=6, horizontal walls spanning full x)
    // y=22 wall (below Player A entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 20, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 22, 32, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 22, 44, 22)); wid += 1;

    // y=6 wall (above Player B entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 20, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 44, 6)); wid += 1;

    // Strongroom walls (enclosed box at x=8-16, y=10-18)
    map.walls.push(wall(wid, 8, 18, 11, 18)); wid += 1;   // top of strongroom (left segment)
    map.walls.push(wall(wid, 13, 18, 16, 18)); wid += 1;   // top of strongroom (right segment)
    map.walls.push(wall(wid, 8, 10, 11, 10)); wid += 1;    // bottom of strongroom (left segment)
    map.walls.push(wall(wid, 13, 10, 16, 10)); wid += 1;   // bottom of strongroom (right segment)
    map.walls.push(wall(wid, 8, 10, 8, 13)); wid += 1;     // left of strongroom (bottom segment)
    map.walls.push(wall(wid, 8, 15, 8, 18)); wid += 1;     // left of strongroom (top segment)
    map.walls.push(wall(wid, 16, 10, 16, 13)); wid += 1;   // right of strongroom (bottom segment)
    map.walls.push(wall(wid, 16, 15, 16, 18)); wid += 1;   // right of strongroom (top segment)

    // Board1 horizontal dividers (y=18 and y=10 outside strongroom)
    map.walls.push(wall(wid, 0, 18, 8, 18)); wid += 1;     // left of strongroom top
    map.walls.push(wall(wid, 16, 18, 24, 18)); wid += 1;   // right of strongroom top
    map.walls.push(wall(wid, 0, 10, 8, 10)); wid += 1;     // left of strongroom bottom
    map.walls.push(wall(wid, 16, 10, 24, 10)); wid += 1;   // right of strongroom bottom

    // Board seam (x=24) with hatchway gaps
    map.walls.push(wall(wid, 24, 6, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;

    // Board 2 vertical wall (x=36) with hatchway gaps
    map.walls.push(wall(wid, 36, 6, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 22)); wid += 1;

    // Board 2 horizontal divider (y=14) with hatchway gaps
    map.walls.push(wall(wid, 24, 14, 30, 14)); wid += 1;
    map.walls.push(wall(wid, 32, 14, 36, 14)); wid += 1;
    map.walls.push(wall(wid, 36, 14, 42, 14)); wid += 1;
    map.walls.push(wall(wid, 44, 14, 48, 14)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry A (y=22) to upper rooms
    map.hatchways.push(hatch(0, 6, 22, Horizontal, 2, 0, 0, Open));    // to Board1 upper
    map.hatchways.push(hatch(1, 22, 22, Horizontal, 2, 0, 0, Open));   // to Board1 upper
    map.hatchways.push(hatch(2, 34, 22, Horizontal, 2, 0, 5, Open));   // to Board2 UL
    map.hatchways.push(hatch(3, 46, 22, Horizontal, 2, 0, 6, Open));   // to Board2 UR

    // Entry B (y=6) to lower rooms
    map.hatchways.push(hatch(4, 6, 6, Horizontal, 2, 4, 4, Open));     // to Board1 lower
    map.hatchways.push(hatch(5, 22, 6, Horizontal, 2, 4, 4, Open));    // to Board1 lower
    map.hatchways.push(hatch(6, 34, 6, Horizontal, 2, 7, 7, Open));    // to Board2 LL
    map.hatchways.push(hatch(7, 46, 6, Horizontal, 2, 8, 8, Open));    // to Board2 LR

    // Strongroom hatchways (all closed - must be breached)
    map.hatchways.push(hatch(8, 12, 18, Horizontal, 2, 0, 2, Closed)); // strongroom top
    map.hatchways.push(hatch(9, 12, 10, Horizontal, 2, 4, 2, Closed)); // strongroom bottom
    map.hatchways.push(hatch(10, 8, 14, Vertical, 2, 1, 2, Closed));   // strongroom left
    map.hatchways.push(hatch(11, 16, 14, Vertical, 2, 2, 3, Closed));  // strongroom right

    // Board seam hatchways (x=24)
    map.hatchways.push(hatch(12, 24, 14, Vertical, 2, 3, 5, Closed));  // upper half
    map.hatchways.push(hatch(13, 24, 8, Vertical, 2, 4, 7, Closed));   // lower half

    // Board 2 interior hatchways
    map.hatchways.push(hatch(14, 36, 14, Vertical, 2, 5, 6, Closed));  // between UL and UR
    map.hatchways.push(hatch(15, 36, 8, Vertical, 2, 7, 8, Closed));   // between LL and LR
    map.hatchways.push(hatch(16, 31, 14, Horizontal, 2, 5, 7, Closed)); // UL to LL
    map.hatchways.push(hatch(17, 43, 14, Horizontal, 2, 6, 8, Closed)); // UR to LR

    // --- Entry Zones ---
    // Player A (green) deploys from high-y edge (top of board)
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(0),
    ));
    // Player B (red) deploys from low-y edge (bottom of board)
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(1),
    ));
    // Underdog entry zone — left x-edge, available during Deploy Armies only
    map.entry_zones.push(entry_zone(
        2, "Underdog Entry Zone", EntryZoneRole::Underdog, 0, 6, 6, 22, None,
    ));

    // --- Objectives ---
    // Three objectives on Board 2 (visible in lower half of mission map image)
    map.objectives.push(objective(0, 30, 18, "Objective A"));
    map.objectives.push(objective(1, 30, 10, "Objective B"));
    map.objectives.push(objective(2, 42, 14, "Objective C"));

    map
}

// ---------------------------------------------------------------------------
// BA-13: The Pipeline (Symmetric, 4 objectives, power lines)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment (Player A from low-y/right-in-image,
// Player B from high-y/left-in-image). Staggered rooms creating a winding
// pipeline corridor. Objectives placed along the pipeline in a zigzag pattern.
// Two "A" markers and two "B" markers connected by Power Lines.
//
// Source: THE_PIPELINE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-13)
//
// The pipeline zigzags through rooms, alternating between upper (high-y)
// and lower (low-y) positions across the 4 column sections.

fn build_ba_13() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // The Pipeline uses a zigzag layout where rooms alternate between
    // upper and lower rows. Each column section is 12" wide.
    // Player A enters from low-y side, Player B from high-y side.
    map.compartments.push(compartment(0, "Section 1 Upper", 0, 14, 12, 22));
    map.compartments.push(compartment(1, "Section 1 Lower", 0, 6, 12, 14));
    map.compartments.push(compartment(2, "Section 2 Upper", 12, 14, 24, 22));
    map.compartments.push(compartment(3, "Section 2 Lower", 12, 6, 24, 14));
    map.compartments.push(compartment(4, "Section 3 Upper", 24, 14, 36, 22));
    map.compartments.push(compartment(5, "Section 3 Lower", 24, 6, 36, 14));
    map.compartments.push(compartment(6, "Section 4 Upper", 36, 14, 48, 22));
    map.compartments.push(compartment(7, "Section 4 Lower", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundaries
    // y=22 wall (below Player B entry on high-y side)
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 16, 22)); wid += 1;
    map.walls.push(wall(wid, 20, 22, 28, 22)); wid += 1;
    map.walls.push(wall(wid, 32, 22, 40, 22)); wid += 1;
    map.walls.push(wall(wid, 44, 22, 48, 22)); wid += 1;

    // y=6 wall (above Player A entry on low-y side)
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 20, 6, 28, 6)); wid += 1;
    map.walls.push(wall(wid, 32, 6, 40, 6)); wid += 1;
    map.walls.push(wall(wid, 44, 6, 48, 6)); wid += 1;

    // Vertical section dividers (x=12, 24, 36) with hatchway gaps
    // The zigzag pattern means hatchways alternate between upper and lower rooms
    map.walls.push(wall(wid, 12, 6, 12, 12)); wid += 1;
    map.walls.push(wall(wid, 12, 16, 12, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 22)); wid += 1;

    // Horizontal row divider (y=14) — the zigzag wall
    // Offset gaps create the pipeline path
    map.walls.push(wall(wid, 0, 14, 4, 14)); wid += 1;
    map.walls.push(wall(wid, 8, 14, 16, 14)); wid += 1;
    map.walls.push(wall(wid, 20, 14, 28, 14)); wid += 1;
    map.walls.push(wall(wid, 32, 14, 40, 14)); wid += 1;
    map.walls.push(wall(wid, 44, 14, 48, 14)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry A (low-y, y=6) to lower rooms
    map.hatchways.push(hatch(0, 6, 6, Horizontal, 2, 1, 1, Open));
    map.hatchways.push(hatch(1, 46, 6, Horizontal, 2, 7, 7, Open));

    // Entry B (high-y, y=22) to upper rooms
    map.hatchways.push(hatch(2, 6, 22, Horizontal, 2, 0, 0, Open));
    map.hatchways.push(hatch(3, 46, 22, Horizontal, 2, 6, 6, Open));

    // Pipeline zigzag hatchways (alternating upper/lower at section boundaries)
    // Section 1→2: lower passage (pipeline goes from S1-Lower to S2-Lower)
    map.hatchways.push(hatch(4, 12, 14, Vertical, 2, 1, 3, Closed));  // S1L to S2L
    // Section 2→3: upper passage (pipeline zigzags up)
    map.hatchways.push(hatch(5, 24, 14, Vertical, 2, 2, 4, Closed));  // S2U to S3U
    // Section 3→4: lower passage (pipeline zigzags down)
    map.hatchways.push(hatch(6, 36, 14, Vertical, 2, 5, 7, Closed));  // S3L to S4L

    // Cross-row hatchways within sections (connecting upper to lower)
    map.hatchways.push(hatch(7, 6, 14, Horizontal, 2, 0, 1, Closed));   // S1 upper-lower
    map.hatchways.push(hatch(8, 18, 14, Horizontal, 2, 2, 3, Closed));  // S2 upper-lower
    map.hatchways.push(hatch(9, 30, 14, Horizontal, 2, 4, 5, Closed));  // S3 upper-lower
    map.hatchways.push(hatch(10, 42, 14, Horizontal, 2, 6, 7, Closed)); // S4 upper-lower

    // --- Entry Zones ---
    // Player A (green) deploys from low-y edge (bottom of board)
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(0),
    ));
    // Player B (red) deploys from high-y edge (top of board)
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Pipeline objectives in zigzag: A markers in upper rooms, B markers in lower rooms
    map.objectives.push(objective(0, 6, 18, "Pipeline Marker A1"));
    map.objectives.push(objective(1, 18, 10, "Pipeline Marker B1"));
    map.objectives.push(objective(2, 30, 18, "Pipeline Marker A2"));
    map.objectives.push(objective(3, 42, 10, "Pipeline Marker B2"));

    // --- Special Regions ---
    // Power Line network connecting all four objectives
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Power Line Network".to_string(),
        boundary: rect_poly(0, 6, 48, 22),
        tags: vec!["power_lines".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-21: Power Struggle (Symmetric, 4 objectives, power lines, warlord kill)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment (Player A from high-y/image-left,
// Player B from low-y/image-right). Central power line corridor
// (highlighted blue in the mission map) connecting 4 objectives.
// Rooms flank the corridor. A fortified room on Board 1.
//
// Source: POWER_STRUGGLE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-21)

fn build_ba_21() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // Y-axis deployment with a power line corridor through the center.
    // Board 1 has a fortified room, Board 2 has flanking rooms.
    map.compartments.push(compartment(0, "Board1 Upper Left", 0, 14, 12, 22));
    map.compartments.push(compartment(1, "Board1 Upper Right", 12, 14, 24, 22));
    map.compartments.push(compartment(2, "Board1 Lower Left", 0, 6, 12, 14));
    map.compartments.push(compartment(3, "Board1 Lower Right", 12, 6, 24, 14));
    map.compartments.push(compartment(4, "Board2 Upper Left", 24, 14, 36, 22));
    map.compartments.push(compartment(5, "Board2 Upper Right", 36, 14, 48, 22));
    map.compartments.push(compartment(6, "Board2 Lower Left", 24, 6, 36, 14));
    map.compartments.push(compartment(7, "Board2 Lower Right", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundaries
    // y=22 wall (below Player A entry)
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 20, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 22, 32, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 22, 44, 22)); wid += 1;

    // y=6 wall (above Player B entry)
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 20, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 44, 6)); wid += 1;

    // Vertical section dividers
    map.walls.push(wall(wid, 12, 6, 12, 12)); wid += 1;
    map.walls.push(wall(wid, 12, 16, 12, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 22)); wid += 1;

    // Horizontal row divider (y=14)
    map.walls.push(wall(wid, 0, 14, 4, 14)); wid += 1;
    map.walls.push(wall(wid, 8, 14, 16, 14)); wid += 1;
    map.walls.push(wall(wid, 20, 14, 28, 14)); wid += 1;
    map.walls.push(wall(wid, 32, 14, 40, 14)); wid += 1;
    map.walls.push(wall(wid, 44, 14, 48, 14)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry A (y=22) to upper rooms
    map.hatchways.push(hatch(0, 6, 22, Horizontal, 2, 0, 0, Open));
    map.hatchways.push(hatch(1, 22, 22, Horizontal, 2, 1, 1, Open));
    map.hatchways.push(hatch(2, 34, 22, Horizontal, 2, 4, 4, Open));
    map.hatchways.push(hatch(3, 46, 22, Horizontal, 2, 5, 5, Open));

    // Entry B (y=6) to lower rooms
    map.hatchways.push(hatch(4, 6, 6, Horizontal, 2, 2, 2, Open));
    map.hatchways.push(hatch(5, 22, 6, Horizontal, 2, 3, 3, Open));
    map.hatchways.push(hatch(6, 34, 6, Horizontal, 2, 6, 6, Open));
    map.hatchways.push(hatch(7, 46, 6, Horizontal, 2, 7, 7, Open));

    // Interior hatchways
    map.hatchways.push(hatch(8, 6, 14, Horizontal, 2, 0, 2, Closed));
    map.hatchways.push(hatch(9, 18, 14, Horizontal, 2, 1, 3, Closed));
    map.hatchways.push(hatch(10, 30, 14, Horizontal, 2, 4, 6, Closed));
    map.hatchways.push(hatch(11, 42, 14, Horizontal, 2, 5, 7, Closed));

    // Section boundary hatchways
    map.hatchways.push(hatch(12, 12, 18, Vertical, 2, 0, 1, Closed));
    map.hatchways.push(hatch(13, 12, 10, Vertical, 2, 2, 3, Closed));
    map.hatchways.push(hatch(14, 24, 18, Vertical, 2, 1, 4, Closed));
    map.hatchways.push(hatch(15, 24, 10, Vertical, 2, 3, 6, Closed));
    map.hatchways.push(hatch(16, 36, 18, Vertical, 2, 4, 5, Closed));
    map.hatchways.push(hatch(17, 36, 10, Vertical, 2, 6, 7, Closed));

    // --- Entry Zones ---
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(1),
    ));

    // --- Objectives ---
    // Four objectives along the Power Line corridor (centered vertically)
    map.objectives.push(objective(0, 6, 14, "Objective A"));
    map.objectives.push(objective(1, 18, 14, "Objective B"));
    map.objectives.push(objective(2, 30, 14, "Objective C"));
    map.objectives.push(objective(3, 42, 14, "Objective D"));

    // --- Special Regions ---
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Power Lines".to_string(),
        boundary: rect_poly(0, 6, 48, 22),
        tags: vec!["power_lines".to_string(), "power_network".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-22: Death in the Dark (Symmetric, 4 objectives, 2 lighting areas)
// ---------------------------------------------------------------------------
// Layout: X-AXIS deployment (Player B from left x=0-6, Player A from right x=42-48).
// 8 rooms in a 4×2 grid in the playing area (x=6-42, y=0-28, divided at y=14
// and at x=15, 24, 33). Central enclosed room at approximately x=18-30, y=8-20.
// Two Lighting Areas divide the board into left and right halves, each containing
// 2 objectives. Lighting state rolls per area.
//
// Source: DEATH_IN_THE_DARK.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-22)

fn build_ba_22() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // C0: Player B entry (left strip)
    map.compartments.push(compartment(0, "Player B Entry Area", 0, 0, 6, 28));
    // C1-C8: 4×2 grid rooms in playing area (columns at x=6,15,24,33,42; rows at y=0,14,28)
    map.compartments.push(compartment(1, "Upper Left Room", 6, 14, 15, 28));
    map.compartments.push(compartment(2, "Lower Left Room", 6, 0, 15, 14));
    map.compartments.push(compartment(3, "Upper Center-Left", 15, 14, 24, 28));
    map.compartments.push(compartment(4, "Lower Center-Left", 15, 0, 24, 14));
    map.compartments.push(compartment(5, "Upper Center-Right", 24, 14, 33, 28));
    map.compartments.push(compartment(6, "Lower Center-Right", 24, 0, 33, 14));
    map.compartments.push(compartment(7, "Upper Right Room", 33, 14, 42, 28));
    map.compartments.push(compartment(8, "Lower Right Room", 33, 0, 42, 14));
    // C9: Player A entry (right strip)
    map.compartments.push(compartment(9, "Player A Entry Area", 42, 0, 48, 28));
    // C10: Central enclosed room (overlaps center of the grid)
    map.compartments.push(compartment(10, "Central Room", 18, 8, 30, 20));

    // --- Walls ---
    let mut wid = 0u32;

    // Player B entry boundary (x=6) with hatchway gaps
    map.walls.push(wall(wid, 6, 0, 6, 6)); wid += 1;
    map.walls.push(wall(wid, 6, 8, 6, 12)); wid += 1;
    map.walls.push(wall(wid, 6, 16, 6, 20)); wid += 1;
    map.walls.push(wall(wid, 6, 22, 6, 28)); wid += 1;

    // Column divider at x=15 with hatchway gaps
    map.walls.push(wall(wid, 15, 0, 15, 6)); wid += 1;
    map.walls.push(wall(wid, 15, 8, 15, 12)); wid += 1;
    map.walls.push(wall(wid, 15, 16, 15, 20)); wid += 1;
    map.walls.push(wall(wid, 15, 22, 15, 28)); wid += 1;

    // Board seam at x=24 with hatchway gaps
    map.walls.push(wall(wid, 24, 0, 24, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 8, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 20)); wid += 1;
    map.walls.push(wall(wid, 24, 22, 24, 28)); wid += 1;

    // Column divider at x=33 with hatchway gaps
    map.walls.push(wall(wid, 33, 0, 33, 6)); wid += 1;
    map.walls.push(wall(wid, 33, 8, 33, 12)); wid += 1;
    map.walls.push(wall(wid, 33, 16, 33, 20)); wid += 1;
    map.walls.push(wall(wid, 33, 22, 33, 28)); wid += 1;

    // Player A entry boundary (x=42) with hatchway gaps
    map.walls.push(wall(wid, 42, 0, 42, 6)); wid += 1;
    map.walls.push(wall(wid, 42, 8, 42, 12)); wid += 1;
    map.walls.push(wall(wid, 42, 16, 42, 20)); wid += 1;
    map.walls.push(wall(wid, 42, 22, 42, 28)); wid += 1;

    // Horizontal row divider at y=14 with hatchway gaps
    map.walls.push(wall(wid, 6, 14, 9, 14)); wid += 1;
    map.walls.push(wall(wid, 13, 14, 15, 14)); wid += 1;
    map.walls.push(wall(wid, 15, 14, 18, 14)); wid += 1;
    map.walls.push(wall(wid, 22, 14, 26, 14)); wid += 1;
    map.walls.push(wall(wid, 30, 14, 33, 14)); wid += 1;
    map.walls.push(wall(wid, 33, 14, 35, 14)); wid += 1;
    map.walls.push(wall(wid, 39, 14, 42, 14)); wid += 1;

    // Central enclosed room walls (x=18-30, y=8-20)
    map.walls.push(wall(wid, 18, 20, 23, 20)); wid += 1;   // top-left segment
    map.walls.push(wall(wid, 25, 20, 30, 20)); wid += 1;   // top-right segment
    map.walls.push(wall(wid, 18, 8, 23, 8)); wid += 1;     // bottom-left segment
    map.walls.push(wall(wid, 25, 8, 30, 8)); wid += 1;     // bottom-right segment
    map.walls.push(wall(wid, 18, 8, 18, 13)); wid += 1;    // left-bottom segment
    map.walls.push(wall(wid, 18, 15, 18, 20)); wid += 1;   // left-top segment
    map.walls.push(wall(wid, 30, 8, 30, 13)); wid += 1;    // right-bottom segment
    map.walls.push(wall(wid, 30, 15, 30, 20)); wid += 1;   // right-top segment

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Player B entry (x=6) to left rooms
    map.hatchways.push(hatch(0, 6, 7, Vertical, 2, 0, 2, Open));     // to lower-left
    map.hatchways.push(hatch(1, 6, 13, Vertical, 2, 0, 2, Open));    // to lower-left (upper gap)
    map.hatchways.push(hatch(2, 6, 21, Vertical, 2, 0, 1, Open));    // to upper-left

    // Player A entry (x=42) to right rooms
    map.hatchways.push(hatch(3, 42, 7, Vertical, 2, 8, 9, Open));    // to lower-right
    map.hatchways.push(hatch(4, 42, 13, Vertical, 2, 8, 9, Open));   // to lower-right (upper gap)
    map.hatchways.push(hatch(5, 42, 21, Vertical, 2, 7, 9, Open));   // to upper-right

    // Column dividers (vertical hatchways between rooms)
    map.hatchways.push(hatch(6, 15, 7, Vertical, 2, 2, 4, Closed));
    map.hatchways.push(hatch(7, 15, 21, Vertical, 2, 1, 3, Closed));
    map.hatchways.push(hatch(8, 24, 7, Vertical, 2, 4, 6, Closed));
    map.hatchways.push(hatch(9, 24, 21, Vertical, 2, 3, 5, Closed));
    map.hatchways.push(hatch(10, 33, 7, Vertical, 2, 6, 8, Closed));
    map.hatchways.push(hatch(11, 33, 21, Vertical, 2, 5, 7, Closed));

    // Row divider (horizontal hatchways at y=14 between upper/lower rooms)
    map.hatchways.push(hatch(12, 11, 14, Horizontal, 2, 1, 2, Closed));
    map.hatchways.push(hatch(13, 20, 14, Horizontal, 2, 3, 4, Closed));
    map.hatchways.push(hatch(14, 37, 14, Horizontal, 2, 7, 8, Closed));

    // Central room hatchways (connecting to adjacent grid rooms)
    map.hatchways.push(hatch(15, 24, 20, Horizontal, 2, 10, 10, Closed)); // central top
    map.hatchways.push(hatch(16, 24, 8, Horizontal, 2, 10, 10, Closed));  // central bottom
    map.hatchways.push(hatch(17, 18, 14, Vertical, 2, 4, 10, Closed));    // central left
    map.hatchways.push(hatch(18, 30, 14, Vertical, 2, 10, 6, Closed));    // central right

    // --- Entry Zones ---
    // Player B (red) entry at left (x=0-6)
    map.entry_zones.push(entry_zone(
        0, "Player B Entry", EntryZoneRole::Main, 0, 0, 6, 28, Some(1),
    ));
    // Player A (green) entry at right (x=42-48)
    map.entry_zones.push(entry_zone(
        1, "Player A Entry", EntryZoneRole::Main, 42, 0, 48, 28, Some(0),
    ));

    // --- Objectives ---
    // 4 objectives: 2 in each lighting area
    map.objectives.push(objective(0, 12, 18, "Objective A1"));
    map.objectives.push(objective(1, 12, 10, "Objective A2"));
    map.objectives.push(objective(2, 36, 18, "Objective B1"));
    map.objectives.push(objective(3, 36, 10, "Objective B2"));

    // --- Special Regions ---
    // Lighting Area 1 (left half of full board)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Lighting Area 1".to_string(),
        boundary: rect_poly(0, 0, 24, 28),
        tags: vec!["lighting_area".to_string()],
    });
    // Lighting Area 2 (right half of full board)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Lighting Area 2".to_string(),
        boundary: rect_poly(24, 0, 48, 28),
        tags: vec!["lighting_area".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-23: Hull Breach (Symmetric, 3 objectives, numbered compartments, venting)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment in specific compartments.
// Player A (green) deploys in Compartment 1 (lower-left) and Compartment 3 (lower-right).
// Player B (red) deploys in Compartment 4 (upper-left) and Compartment 6 (upper-right).
// 6 numbered compartments in a 3×2 grid with connecting passages.
// Compartments 1, 2, 4, 5 are ventable targets.
//
// Source: HULL_BREACH.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-23)

fn build_ba_23() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // 6 numbered compartments in a 3×2 grid plus connecting passages
    // C0: Compartment 1 — lower-left (Player A deploy)
    map.compartments.push(compartment(0, "Compartment 1", 0, 0, 12, 14));
    // C1: Compartment 4 — upper-left (Player B deploy)
    map.compartments.push(compartment(1, "Compartment 4", 0, 14, 12, 28));
    // C2: Compartment 5 — Board 1 center
    map.compartments.push(compartment(2, "Compartment 5", 12, 6, 24, 22));
    // C3: Upper Passage (Board 1)
    map.compartments.push(compartment(3, "Upper Passage 1", 12, 22, 24, 28));
    // C4: Lower Passage (Board 1)
    map.compartments.push(compartment(4, "Lower Passage 1", 12, 0, 24, 6));
    // C5: Compartment 2 — Board 2 center
    map.compartments.push(compartment(5, "Compartment 2", 24, 6, 36, 22));
    // C6: Upper Passage (Board 2)
    map.compartments.push(compartment(6, "Upper Passage 2", 24, 22, 36, 28));
    // C7: Lower Passage (Board 2)
    map.compartments.push(compartment(7, "Lower Passage 2", 24, 0, 36, 6));
    // C8: Compartment 6 — upper-right (Player B deploy)
    map.compartments.push(compartment(8, "Compartment 6", 36, 14, 48, 28));
    // C9: Compartment 3 — lower-right (Player A deploy)
    map.compartments.push(compartment(9, "Compartment 3", 36, 0, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Left column boundary (x=12) with hatchway gaps
    map.walls.push(wall(wid, 12, 0, 12, 4)); wid += 1;
    map.walls.push(wall(wid, 12, 8, 12, 12)); wid += 1;
    map.walls.push(wall(wid, 12, 16, 12, 20)); wid += 1;
    map.walls.push(wall(wid, 12, 24, 12, 28)); wid += 1;

    // Board seam (x=24) with hatchway gaps
    map.walls.push(wall(wid, 24, 0, 24, 4)); wid += 1;
    map.walls.push(wall(wid, 24, 8, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 20)); wid += 1;
    map.walls.push(wall(wid, 24, 24, 24, 28)); wid += 1;

    // Right column boundary (x=36) with hatchway gaps
    map.walls.push(wall(wid, 36, 0, 36, 4)); wid += 1;
    map.walls.push(wall(wid, 36, 8, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 20)); wid += 1;
    map.walls.push(wall(wid, 36, 24, 36, 28)); wid += 1;

    // Horizontal divider at y=14 (between upper and lower compartments on left and right)
    map.walls.push(wall(wid, 0, 14, 5, 14)); wid += 1;
    map.walls.push(wall(wid, 7, 14, 12, 14)); wid += 1;
    map.walls.push(wall(wid, 36, 14, 41, 14)); wid += 1;
    map.walls.push(wall(wid, 43, 14, 48, 14)); wid += 1;

    // Horizontal divider at y=6 (top of lower passages) with hatchway gaps
    map.walls.push(wall(wid, 12, 6, 17, 6)); wid += 1;
    map.walls.push(wall(wid, 21, 6, 24, 6)); wid += 1;
    map.walls.push(wall(wid, 24, 6, 29, 6)); wid += 1;
    map.walls.push(wall(wid, 33, 6, 36, 6)); wid += 1;

    // Horizontal divider at y=22 (bottom of upper passages) with hatchway gaps
    map.walls.push(wall(wid, 12, 22, 17, 22)); wid += 1;
    map.walls.push(wall(wid, 21, 22, 24, 22)); wid += 1;
    map.walls.push(wall(wid, 24, 22, 29, 22)); wid += 1;
    map.walls.push(wall(wid, 33, 22, 36, 22)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Left compartments to Compartment 5 (x=12)
    map.hatchways.push(hatch(0, 12, 5, Vertical, 2, 0, 4, Closed));    // Comp1 to Lower Passage
    map.hatchways.push(hatch(1, 12, 14, Vertical, 2, 0, 2, Closed));   // Comp1 to Comp5 (lower half)
    map.hatchways.push(hatch(2, 12, 14, Vertical, 2, 1, 2, Closed));   // Comp4 to Comp5 (upper half)
    map.hatchways.push(hatch(3, 12, 23, Vertical, 2, 1, 3, Closed));   // Comp4 to Upper Passage

    // Compartment 5 to passages (y=6 and y=22)
    map.hatchways.push(hatch(4, 19, 6, Horizontal, 2, 4, 2, Closed));  // Lower Passage to Comp5
    map.hatchways.push(hatch(5, 19, 22, Horizontal, 2, 2, 3, Closed)); // Comp5 to Upper Passage

    // Board seam (x=24) — Comp5 to Comp2
    map.hatchways.push(hatch(6, 24, 14, Vertical, 2, 2, 5, Closed));   // Comp5 to Comp2

    // Passages across board seam (x=24)
    map.hatchways.push(hatch(7, 24, 5, Vertical, 2, 4, 7, Closed));    // Lower Passage 1 to Lower Passage 2
    map.hatchways.push(hatch(8, 24, 23, Vertical, 2, 3, 6, Closed));   // Upper Passage 1 to Upper Passage 2

    // Compartment 2 to passages (y=6 and y=22)
    map.hatchways.push(hatch(9, 31, 6, Horizontal, 2, 7, 5, Closed));  // Lower Passage 2 to Comp2
    map.hatchways.push(hatch(10, 31, 22, Horizontal, 2, 5, 6, Closed)); // Comp2 to Upper Passage 2

    // Right compartments to Compartment 2 (x=36)
    map.hatchways.push(hatch(11, 36, 5, Vertical, 2, 7, 9, Closed));   // Lower Passage 2 to Comp3
    map.hatchways.push(hatch(12, 36, 14, Vertical, 2, 5, 9, Closed));  // Comp2 to Comp3 (lower half)
    map.hatchways.push(hatch(13, 36, 14, Vertical, 2, 5, 8, Closed));  // Comp2 to Comp6 (upper half)
    map.hatchways.push(hatch(14, 36, 23, Vertical, 2, 6, 8, Closed));  // Upper Passage 2 to Comp6

    // Horizontal hatchways at y=14 within left and right sides
    map.hatchways.push(hatch(15, 6, 14, Horizontal, 2, 1, 0, Open));   // Comp4 to Comp1 (entry)
    map.hatchways.push(hatch(16, 42, 14, Horizontal, 2, 8, 9, Open));  // Comp6 to Comp3 (entry)

    // --- Entry Zones ---
    // Player A (green) deploys in Compartment 1 (lower-left) and Compartment 3 (lower-right)
    map.entry_zones.push(entry_zone(
        0, "Player A Entry (Comp 1)", EntryZoneRole::Main, 0, 0, 12, 14, Some(0),
    ));
    map.entry_zones.push(entry_zone(
        1, "Player A Entry (Comp 3)", EntryZoneRole::Main, 36, 0, 48, 14, Some(0),
    ));
    // Player B (red) deploys in Compartment 4 (upper-left) and Compartment 6 (upper-right)
    map.entry_zones.push(entry_zone(
        2, "Player B Entry (Comp 4)", EntryZoneRole::Main, 0, 14, 12, 28, Some(1),
    ));
    map.entry_zones.push(entry_zone(
        3, "Player B Entry (Comp 6)", EntryZoneRole::Main, 36, 14, 48, 28, Some(1),
    ));

    // --- Objectives ---
    // Three datacore objective markers along y=14 centerline
    map.objectives.push(objective(0, 6, 14, "Datacore Alpha"));
    map.objectives.push(objective(1, 24, 14, "Datacore Beta"));
    map.objectives.push(objective(2, 42, 14, "Datacore Gamma"));

    // --- Special Regions ---
    // 4 ventable compartments: Compartment 1, 2, 4, 5
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Compartment 1".to_string(),
        boundary: rect_poly(0, 0, 12, 14),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Compartment 2".to_string(),
        boundary: rect_poly(24, 6, 36, 22),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(2),
        name: "Compartment 4".to_string(),
        boundary: rect_poly(0, 14, 12, 28),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(3),
        name: "Compartment 5".to_string(),
        boundary: rect_poly(12, 6, 24, 22),
        tags: vec!["compartment".to_string(), "ventable".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-31: Control Centre (Symmetric, 4 objectives, global hatch unlock)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment (Player A from top y=22-28, Player B from bottom y=0-6).
// Central Control Centre room spanning x=12-36, y=8-20, surrounded by flanking rooms.
// Objective A is the primary scoring marker. Objective B controls the "Unlock Overrides"
// ability to open all hatchways simultaneously.
//
// Source: CONTROL_CENTRE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-31)

fn build_ba_31() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // C0: Board1 Upper Left
    map.compartments.push(compartment(0, "Board1 Upper Left", 0, 14, 12, 22));
    // C1: Board1 Lower Left
    map.compartments.push(compartment(1, "Board1 Lower Left", 0, 6, 12, 14));
    // C2: Control Centre (large central room)
    map.compartments.push(compartment(2, "Control Centre", 12, 8, 36, 20));
    // C3: Upper Corridor (above control centre)
    map.compartments.push(compartment(3, "Upper Corridor", 12, 20, 36, 22));
    // C4: Lower Corridor (below control centre)
    map.compartments.push(compartment(4, "Lower Corridor", 12, 6, 36, 8));
    // C5: Board2 Upper Right
    map.compartments.push(compartment(5, "Board2 Upper Right", 36, 14, 48, 22));
    // C6: Board2 Lower Right
    map.compartments.push(compartment(6, "Board2 Lower Right", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundary at y=22 (below Player A entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 16, 22)); wid += 1;
    map.walls.push(wall(wid, 20, 22, 32, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 22, 40, 22)); wid += 1;
    map.walls.push(wall(wid, 44, 22, 48, 22)); wid += 1;

    // Entry zone boundary at y=6 (above Player B entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 20, 6, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 40, 6)); wid += 1;
    map.walls.push(wall(wid, 44, 6, 48, 6)); wid += 1;

    // Left column divider at x=12 with hatchway gaps
    map.walls.push(wall(wid, 12, 6, 12, 7)); wid += 1;
    map.walls.push(wall(wid, 12, 9, 12, 13)); wid += 1;
    map.walls.push(wall(wid, 12, 15, 12, 19)); wid += 1;
    map.walls.push(wall(wid, 12, 21, 12, 22)); wid += 1;

    // Right column divider at x=36 with hatchway gaps
    map.walls.push(wall(wid, 36, 6, 36, 7)); wid += 1;
    map.walls.push(wall(wid, 36, 9, 36, 13)); wid += 1;
    map.walls.push(wall(wid, 36, 15, 36, 19)); wid += 1;
    map.walls.push(wall(wid, 36, 21, 36, 22)); wid += 1;

    // Horizontal divider at y=14 (separating upper/lower flanking rooms)
    map.walls.push(wall(wid, 0, 14, 5, 14)); wid += 1;
    map.walls.push(wall(wid, 7, 14, 12, 14)); wid += 1;
    map.walls.push(wall(wid, 36, 14, 41, 14)); wid += 1;
    map.walls.push(wall(wid, 43, 14, 48, 14)); wid += 1;

    // Control Centre top/bottom walls (y=20 and y=8) with hatchway gaps
    map.walls.push(wall(wid, 12, 20, 17, 20)); wid += 1;
    map.walls.push(wall(wid, 21, 20, 27, 20)); wid += 1;
    map.walls.push(wall(wid, 31, 20, 36, 20)); wid += 1;
    map.walls.push(wall(wid, 12, 8, 17, 8)); wid += 1;
    map.walls.push(wall(wid, 21, 8, 27, 8)); wid += 1;
    map.walls.push(wall(wid, 31, 8, 36, 8)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry A (y=22) to rooms/corridors
    map.hatchways.push(hatch(0, 6, 22, Horizontal, 2, 0, 0, Open));    // to Board1 UL
    map.hatchways.push(hatch(1, 18, 22, Horizontal, 2, 3, 3, Open));   // to Upper Corridor
    map.hatchways.push(hatch(2, 34, 22, Horizontal, 2, 3, 3, Open));   // to Upper Corridor
    map.hatchways.push(hatch(3, 42, 22, Horizontal, 2, 5, 5, Open));   // to Board2 UR

    // Entry B (y=6) to rooms/corridors
    map.hatchways.push(hatch(4, 6, 6, Horizontal, 2, 1, 1, Open));     // to Board1 LL
    map.hatchways.push(hatch(5, 18, 6, Horizontal, 2, 4, 4, Open));    // to Lower Corridor
    map.hatchways.push(hatch(6, 34, 6, Horizontal, 2, 4, 4, Open));    // to Lower Corridor
    map.hatchways.push(hatch(7, 42, 6, Horizontal, 2, 6, 6, Open));    // to Board2 LR

    // Left flanking rooms to corridors/control centre (x=12)
    map.hatchways.push(hatch(8, 12, 8, Vertical, 2, 1, 4, Closed));    // Board1 LL to Lower Corridor
    map.hatchways.push(hatch(9, 12, 14, Vertical, 2, 1, 2, Closed));   // Board1 LL to Control Centre
    map.hatchways.push(hatch(10, 12, 14, Vertical, 2, 0, 2, Closed));  // Board1 UL to Control Centre
    map.hatchways.push(hatch(11, 12, 20, Vertical, 2, 0, 3, Closed));  // Board1 UL to Upper Corridor

    // Right flanking rooms to corridors/control centre (x=36)
    map.hatchways.push(hatch(12, 36, 8, Vertical, 2, 4, 6, Closed));   // Lower Corridor to Board2 LR
    map.hatchways.push(hatch(13, 36, 14, Vertical, 2, 2, 6, Closed));  // Control Centre to Board2 LR
    map.hatchways.push(hatch(14, 36, 14, Vertical, 2, 2, 5, Closed));  // Control Centre to Board2 UR
    map.hatchways.push(hatch(15, 36, 20, Vertical, 2, 3, 5, Closed));  // Upper Corridor to Board2 UR

    // Corridors to Control Centre (horizontal hatchways)
    map.hatchways.push(hatch(16, 19, 20, Horizontal, 2, 3, 2, Closed)); // Upper Corridor to CC
    map.hatchways.push(hatch(17, 29, 20, Horizontal, 2, 3, 2, Closed)); // Upper Corridor to CC
    map.hatchways.push(hatch(18, 19, 8, Horizontal, 2, 2, 4, Closed));  // CC to Lower Corridor
    map.hatchways.push(hatch(19, 29, 8, Horizontal, 2, 2, 4, Closed));  // CC to Lower Corridor

    // Left and right horizontal hatchways at y=14
    map.hatchways.push(hatch(20, 6, 14, Horizontal, 2, 0, 1, Closed));  // Board1 UL to Board1 LL
    map.hatchways.push(hatch(21, 42, 14, Horizontal, 2, 5, 6, Closed)); // Board2 UR to Board2 LR

    // --- Entry Zones ---
    // Player A (green) deploys from top (y=22-28)
    map.entry_zones.push(entry_zone(
        0, "Player A Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(0),
    ));
    // Player B (red) deploys from bottom (y=0-6)
    map.entry_zones.push(entry_zone(
        1, "Player B Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(1),
    ));

    // --- Objectives ---
    // Objective A: primary scoring marker (center of control room)
    map.objectives.push(objective(0, 24, 14, "Objective A"));
    // Objective B: controls Unlock Overrides
    map.objectives.push(objective(1, 24, 18, "Objective B"));
    // Two additional objectives in flanking rooms
    map.objectives.push(objective(2, 6, 14, "Objective C"));
    map.objectives.push(objective(3, 42, 14, "Objective D"));

    // --- Special Regions ---
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "Control Centre".to_string(),
        boundary: rect_poly(12, 8, 36, 20),
        tags: vec!["control_centre".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-32: The Furnace (Symmetric, 3 objectives, furnace zone, burners)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment (Player B/red from top y=22-28, Player A/green from
// bottom y=0-6). Large central Furnace room (x=12-36, y=8-20) surrounded by
// flanking rooms and bypass corridors. Furnace Control Zones flank the Furnace.
//
// Source: THE_FURNACE.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-32)

fn build_ba_32() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // C0: Board1 Upper Left
    map.compartments.push(compartment(0, "Board1 Upper Left", 0, 14, 12, 22));
    // C1: Board1 Lower Left
    map.compartments.push(compartment(1, "Board1 Lower Left", 0, 6, 12, 14));
    // C2: The Furnace (large central room)
    map.compartments.push(compartment(2, "The Furnace", 12, 8, 36, 20));
    // C3: Upper Bypass (above furnace)
    map.compartments.push(compartment(3, "Upper Bypass", 12, 20, 36, 22));
    // C4: Lower Bypass (below furnace)
    map.compartments.push(compartment(4, "Lower Bypass", 12, 6, 36, 8));
    // C5: Board2 Upper Right
    map.compartments.push(compartment(5, "Board2 Upper Right", 36, 14, 48, 22));
    // C6: Board2 Lower Right
    map.compartments.push(compartment(6, "Board2 Lower Right", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundary at y=22 (below Player B entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 16, 22)); wid += 1;
    map.walls.push(wall(wid, 20, 22, 32, 22)); wid += 1;
    map.walls.push(wall(wid, 36, 22, 40, 22)); wid += 1;
    map.walls.push(wall(wid, 44, 22, 48, 22)); wid += 1;

    // Entry zone boundary at y=6 (above Player A entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 20, 6, 32, 6)); wid += 1;
    map.walls.push(wall(wid, 36, 6, 40, 6)); wid += 1;
    map.walls.push(wall(wid, 44, 6, 48, 6)); wid += 1;

    // Left column divider at x=12 with hatchway gaps
    map.walls.push(wall(wid, 12, 6, 12, 7)); wid += 1;
    map.walls.push(wall(wid, 12, 9, 12, 13)); wid += 1;
    map.walls.push(wall(wid, 12, 15, 12, 19)); wid += 1;
    map.walls.push(wall(wid, 12, 21, 12, 22)); wid += 1;

    // Right column divider at x=36 with hatchway gaps
    map.walls.push(wall(wid, 36, 6, 36, 7)); wid += 1;
    map.walls.push(wall(wid, 36, 9, 36, 13)); wid += 1;
    map.walls.push(wall(wid, 36, 15, 36, 19)); wid += 1;
    map.walls.push(wall(wid, 36, 21, 36, 22)); wid += 1;

    // Horizontal divider at y=14 (separating upper/lower flanking rooms)
    map.walls.push(wall(wid, 0, 14, 5, 14)); wid += 1;
    map.walls.push(wall(wid, 7, 14, 12, 14)); wid += 1;
    map.walls.push(wall(wid, 36, 14, 41, 14)); wid += 1;
    map.walls.push(wall(wid, 43, 14, 48, 14)); wid += 1;

    // Furnace top/bottom walls (y=20 and y=8) with hatchway gaps
    map.walls.push(wall(wid, 12, 20, 17, 20)); wid += 1;
    map.walls.push(wall(wid, 21, 20, 27, 20)); wid += 1;
    map.walls.push(wall(wid, 31, 20, 36, 20)); wid += 1;
    map.walls.push(wall(wid, 12, 8, 17, 8)); wid += 1;
    map.walls.push(wall(wid, 21, 8, 27, 8)); wid += 1;
    map.walls.push(wall(wid, 31, 8, 36, 8)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry B (y=22, top) to rooms/bypass
    map.hatchways.push(hatch(0, 6, 22, Horizontal, 2, 0, 0, Open));    // to Board1 UL
    map.hatchways.push(hatch(1, 18, 22, Horizontal, 2, 3, 3, Open));   // to Upper Bypass
    map.hatchways.push(hatch(2, 34, 22, Horizontal, 2, 3, 3, Open));   // to Upper Bypass
    map.hatchways.push(hatch(3, 42, 22, Horizontal, 2, 5, 5, Open));   // to Board2 UR

    // Entry A (y=6, bottom) to rooms/bypass
    map.hatchways.push(hatch(4, 6, 6, Horizontal, 2, 1, 1, Open));     // to Board1 LL
    map.hatchways.push(hatch(5, 18, 6, Horizontal, 2, 4, 4, Open));    // to Lower Bypass
    map.hatchways.push(hatch(6, 34, 6, Horizontal, 2, 4, 4, Open));    // to Lower Bypass
    map.hatchways.push(hatch(7, 42, 6, Horizontal, 2, 6, 6, Open));    // to Board2 LR

    // Left flanking rooms to Furnace/bypass (x=12)
    map.hatchways.push(hatch(8, 12, 8, Vertical, 2, 1, 4, Closed));    // Board1 LL to Lower Bypass
    map.hatchways.push(hatch(9, 12, 14, Vertical, 2, 1, 2, Closed));   // Board1 LL to Furnace
    map.hatchways.push(hatch(10, 12, 14, Vertical, 2, 0, 2, Closed));  // Board1 UL to Furnace
    map.hatchways.push(hatch(11, 12, 20, Vertical, 2, 0, 3, Closed));  // Board1 UL to Upper Bypass

    // Right flanking rooms to Furnace/bypass (x=36)
    map.hatchways.push(hatch(12, 36, 8, Vertical, 2, 4, 6, Closed));   // Lower Bypass to Board2 LR
    map.hatchways.push(hatch(13, 36, 14, Vertical, 2, 2, 6, Closed));  // Furnace to Board2 LR
    map.hatchways.push(hatch(14, 36, 14, Vertical, 2, 2, 5, Closed));  // Furnace to Board2 UR
    map.hatchways.push(hatch(15, 36, 20, Vertical, 2, 3, 5, Closed));  // Upper Bypass to Board2 UR

    // Bypass to Furnace (horizontal hatchways)
    map.hatchways.push(hatch(16, 19, 20, Horizontal, 2, 3, 2, Closed)); // Upper Bypass to Furnace
    map.hatchways.push(hatch(17, 29, 20, Horizontal, 2, 3, 2, Closed)); // Upper Bypass to Furnace
    map.hatchways.push(hatch(18, 19, 8, Horizontal, 2, 2, 4, Closed));  // Furnace to Lower Bypass
    map.hatchways.push(hatch(19, 29, 8, Horizontal, 2, 2, 4, Closed));  // Furnace to Lower Bypass

    // Left and right horizontal hatchways at y=14
    map.hatchways.push(hatch(20, 6, 14, Horizontal, 2, 0, 1, Closed));  // Board1 UL to Board1 LL
    map.hatchways.push(hatch(21, 42, 14, Horizontal, 2, 5, 6, Closed)); // Board2 UR to Board2 LR

    // --- Entry Zones ---
    // Player B (red) deploys from top (y=22-28) — note red on high-y for this map
    map.entry_zones.push(entry_zone(
        0, "Player B Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(1),
    ));
    // Player A (green) deploys from bottom (y=0-6)
    map.entry_zones.push(entry_zone(
        1, "Player A Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(0),
    ));

    // --- Objectives ---
    // 3 objectives within/near the Furnace
    map.objectives.push(objective(0, 18, 14, "Furnace Objective A"));
    map.objectives.push(objective(1, 24, 14, "Furnace Objective B"));
    map.objectives.push(objective(2, 30, 14, "Furnace Objective C"));

    // --- Special Regions ---
    // The Furnace (central hazard zone)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(0),
        name: "The Furnace".to_string(),
        boundary: rect_poly(12, 8, 36, 20),
        tags: vec!["furnace".to_string()],
    });
    // Left Furnace Control Zone (flanking rooms on left)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(1),
        name: "Left Furnace Control Zone".to_string(),
        boundary: rect_poly(0, 6, 12, 22),
        tags: vec!["furnace_control_zone".to_string()],
    });
    // Right Furnace Control Zone (flanking rooms on right)
    map.special_regions.push(SpecialRegion {
        id: RegionId::new(2),
        name: "Right Furnace Control Zone".to_string(),
        boundary: rect_poly(36, 6, 48, 22),
        tags: vec!["furnace_control_zone".to_string()],
    });

    map
}

// ---------------------------------------------------------------------------
// BA-33: Rad Leak (Symmetric, 3 objectives, 4 radiation sectors)
// ---------------------------------------------------------------------------
// Layout: Y-AXIS deployment (Player B/red from top y=22-28, Player A/green from
// bottom y=0-6). 8 rooms in a standard 4×2 grid. Two "Critical" objective markers
// plus one additional. 4 radiation sector quadrant overlays with varying effects.
//
// Source: RAD_LEAK.PNG mission map
// Source: boarding_actions_maps_complete_v3.json (BA-33)

fn build_ba_33() -> BoardingMap {
    use HatchwayOrientation::*;
    use HatchwayState::*;

    let mut map = BoardingMap::new(BoardDimensions::BOARDING_ACTIONS);

    // --- Compartments ---
    // 8 rooms in standard 4×2 grid: columns at x=0,12,24,36,48; rows at y=6,14,22
    // C0: Upper Left
    map.compartments.push(compartment(0, "Upper Left Room", 0, 14, 12, 22));
    // C1: Lower Left
    map.compartments.push(compartment(1, "Lower Left Room", 0, 6, 12, 14));
    // C2: Upper Center-Left
    map.compartments.push(compartment(2, "Upper Center-Left", 12, 14, 24, 22));
    // C3: Lower Center-Left
    map.compartments.push(compartment(3, "Lower Center-Left", 12, 6, 24, 14));
    // C4: Upper Center-Right
    map.compartments.push(compartment(4, "Upper Center-Right", 24, 14, 36, 22));
    // C5: Lower Center-Right
    map.compartments.push(compartment(5, "Lower Center-Right", 24, 6, 36, 14));
    // C6: Upper Right
    map.compartments.push(compartment(6, "Upper Right Room", 36, 14, 48, 22));
    // C7: Lower Right
    map.compartments.push(compartment(7, "Lower Right Room", 36, 6, 48, 14));

    // --- Walls ---
    let mut wid = 0u32;

    // Entry zone boundary at y=22 (below Player B entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 22, 4, 22)); wid += 1;
    map.walls.push(wall(wid, 8, 22, 16, 22)); wid += 1;
    map.walls.push(wall(wid, 20, 22, 28, 22)); wid += 1;
    map.walls.push(wall(wid, 32, 22, 40, 22)); wid += 1;
    map.walls.push(wall(wid, 44, 22, 48, 22)); wid += 1;

    // Entry zone boundary at y=6 (above Player A entry) with hatchway gaps
    map.walls.push(wall(wid, 0, 6, 4, 6)); wid += 1;
    map.walls.push(wall(wid, 8, 6, 16, 6)); wid += 1;
    map.walls.push(wall(wid, 20, 6, 28, 6)); wid += 1;
    map.walls.push(wall(wid, 32, 6, 40, 6)); wid += 1;
    map.walls.push(wall(wid, 44, 6, 48, 6)); wid += 1;

    // Vertical column dividers with hatchway gaps
    // x=12
    map.walls.push(wall(wid, 12, 6, 12, 12)); wid += 1;
    map.walls.push(wall(wid, 12, 16, 12, 22)); wid += 1;
    // x=24 (board seam)
    map.walls.push(wall(wid, 24, 6, 24, 12)); wid += 1;
    map.walls.push(wall(wid, 24, 16, 24, 22)); wid += 1;
    // x=36
    map.walls.push(wall(wid, 36, 6, 36, 12)); wid += 1;
    map.walls.push(wall(wid, 36, 16, 36, 22)); wid += 1;

    // Horizontal row divider at y=14 with hatchway gaps
    map.walls.push(wall(wid, 0, 14, 4, 14)); wid += 1;
    map.walls.push(wall(wid, 8, 14, 16, 14)); wid += 1;
    map.walls.push(wall(wid, 20, 14, 28, 14)); wid += 1;
    map.walls.push(wall(wid, 32, 14, 40, 14)); wid += 1;
    map.walls.push(wall(wid, 44, 14, 48, 14)); wid += 1;

    // Perimeter
    map.walls.push(wall(wid, 0, 0, 48, 0)); wid += 1;
    map.walls.push(wall(wid, 0, 28, 48, 28)); wid += 1;
    map.walls.push(wall(wid, 0, 0, 0, 28)); wid += 1;
    map.walls.push(wall(wid, 48, 0, 48, 28));

    // --- Hatchways ---
    // Entry B (y=22, top) to upper rooms
    map.hatchways.push(hatch(0, 6, 22, Horizontal, 2, 0, 0, Open));
    map.hatchways.push(hatch(1, 18, 22, Horizontal, 2, 2, 2, Open));
    map.hatchways.push(hatch(2, 30, 22, Horizontal, 2, 4, 4, Open));
    map.hatchways.push(hatch(3, 42, 22, Horizontal, 2, 6, 6, Open));

    // Entry A (y=6, bottom) to lower rooms
    map.hatchways.push(hatch(4, 6, 6, Horizontal, 2, 1, 1, Open));
    map.hatchways.push(hatch(5, 18, 6, Horizontal, 2, 3, 3, Open));
    map.hatchways.push(hatch(6, 30, 6, Horizontal, 2, 5, 5, Open));
    map.hatchways.push(hatch(7, 42, 6, Horizontal, 2, 7, 7, Open));

    // Interior horizontal hatchways at y=14 (upper to lower rooms)
    map.hatchways.push(hatch(8, 6, 14, Horizontal, 2, 0, 1, Closed));
    map.hatchways.push(hatch(9, 18, 14, Horizontal, 2, 2, 3, Closed));
    map.hatchways.push(hatch(10, 30, 14, Horizontal, 2, 4, 5, Closed));
    map.hatchways.push(hatch(11, 42, 14, Horizontal, 2, 6, 7, Closed));

    // Interior vertical hatchways at column dividers
    map.hatchways.push(hatch(12, 12, 14, Vertical, 2, 0, 2, Closed));   // UL to UCL
    map.hatchways.push(hatch(13, 12, 10, Vertical, 2, 1, 3, Closed));   // LL to LCL
    map.hatchways.push(hatch(14, 24, 14, Vertical, 2, 2, 4, Closed));   // UCL to UCR
    map.hatchways.push(hatch(15, 24, 10, Vertical, 2, 3, 5, Closed));   // LCL to LCR
    map.hatchways.push(hatch(16, 36, 14, Vertical, 2, 4, 6, Closed));   // UCR to UR
    map.hatchways.push(hatch(17, 36, 10, Vertical, 2, 5, 7, Closed));   // LCR to LR

    // --- Entry Zones ---
    // Player B (red) deploys from top (y=22-28)
    map.entry_zones.push(entry_zone(
        0, "Player B Entry", EntryZoneRole::Main, 0, 22, 48, 28, Some(1),
    ));
    // Player A (green) deploys from bottom (y=0-6)
    map.entry_zones.push(entry_zone(
        1, "Player A Entry", EntryZoneRole::Main, 0, 0, 48, 6, Some(0),
    ));

    // --- Objectives ---
    // Two Critical objectives + one additional along y=14 centerline
    map.objectives.push(objective(0, 12, 14, "Critical Objective A"));
    map.objectives.push(objective(1, 36, 14, "Critical Objective B"));
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
        assert_eq!(map.compartments.len(), 9);
        assert_eq!(map.objectives.len(), 3);
        assert_eq!(map.entry_zones.len(), 3); // 2 main + 1 underdog
        assert_eq!(map.hatchways.len(), 18);
        // Underdog entry zone exists
        assert!(map.entry_zones.iter().any(|ez| ez.role == EntryZoneRole::Underdog));
        // Strongroom compartment exists
        assert!(map.compartments.iter().any(|c| c.name == "Strongroom"));
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
        assert_eq!(map.compartments.len(), 11); // 2 entry areas + 8 grid rooms + 1 central room
        assert_eq!(map.objectives.len(), 4); // 2 per lighting area
        assert_eq!(map.entry_zones.len(), 2);
        assert_eq!(map.hatchways.len(), 19);
        // Two Lighting Areas
        let lighting_areas: Vec<_> = map.special_regions.iter()
            .filter(|r| r.tags.contains(&"lighting_area".to_string()))
            .collect();
        assert_eq!(lighting_areas.len(), 2);
        // X-axis deployment: entry zones on long edges (left x=0-6, right x=42-48)
        // Player B entry is on left (x=0-6)
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(1)));
        // Player A entry is on right (x=42-48)
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(0)));
    }

    #[test]
    fn test_ba_23_hull_breach() {
        let map = build_ba_23();
        assert_eq!(map.compartments.len(), 10); // 6 numbered compartments + 4 passages
        assert_eq!(map.objectives.len(), 3); // 3 datacores
        assert_eq!(map.entry_zones.len(), 4); // 2 for Player A + 2 for Player B
        assert_eq!(map.hatchways.len(), 17);
        // Y-axis deployment in specific compartments
        // Player A in Comp 1 (lower-left) and Comp 3 (lower-right)
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(0)));
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(0)));
        // Player B in Comp 4 (upper-left) and Comp 6 (upper-right)
        assert_eq!(map.entry_zones[2].player_assignment, Some(PlayerId::new(1)));
        assert_eq!(map.entry_zones[3].player_assignment, Some(PlayerId::new(1)));
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
        assert_eq!(map.compartments.len(), 7); // 4 flanking rooms + CC + 2 corridors
        assert_eq!(map.objectives.len(), 4); // A, B + 2 more
        assert_eq!(map.entry_zones.len(), 2);
        assert_eq!(map.hatchways.len(), 22);
        // Y-axis deployment: Player A from top, Player B from bottom
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(0)));
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(1)));
        // Control Centre special region
        assert!(map.special_regions.iter().any(|r| r.name == "Control Centre"));
        // Objectives A and B exist
        assert!(map.objectives.iter().any(|o| o.label == "Objective A"));
        assert!(map.objectives.iter().any(|o| o.label == "Objective B"));
    }

    #[test]
    fn test_ba_32_the_furnace() {
        let map = build_ba_32();
        assert_eq!(map.compartments.len(), 7); // 4 flanking rooms + Furnace + 2 bypasses
        assert_eq!(map.objectives.len(), 3);
        assert_eq!(map.entry_zones.len(), 2);
        assert_eq!(map.hatchways.len(), 22);
        // Y-axis deployment: Player B (red) from top, Player A (green) from bottom
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(1))); // Player B on top
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(0))); // Player A on bottom
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
        assert_eq!(map.compartments.len(), 8); // 4×2 grid
        assert_eq!(map.objectives.len(), 3); // 2 critical + 1 additional
        assert_eq!(map.entry_zones.len(), 2);
        assert_eq!(map.hatchways.len(), 18);
        // Y-axis deployment: Player B (red) from top, Player A (green) from bottom
        assert_eq!(map.entry_zones[0].player_assignment, Some(PlayerId::new(1))); // Player B on top
        assert_eq!(map.entry_zones[1].player_assignment, Some(PlayerId::new(0))); // Player A on bottom
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
