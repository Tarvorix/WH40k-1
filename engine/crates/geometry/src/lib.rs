//! WH40K Engine - Geometry crate
//!
//! Provides board representation, deployment zones, terrain, distance calculations,
//! coherency checking, line-of-sight, objective markers, and movement legality helpers.
//!
//! All distance calculations use fixed-point `Inches` from core_types for deterministic
//! behavior across platforms. No floating-point in game logic.
//!
//! Source: implementation_v3.md Section 6.2 (Layer B - Geometry)
//! Source: 40k_revised.md - Engagement Range, Coherency, Objectives, Movement
//! Source: CP_Rules.md - 44"x30" board, Combat Patrol deployment maps

pub mod formation;

use serde::{Deserialize, Serialize};
use wh40k_core_types::{
    BaseSize, BoardDimensions, CoverType, Inches, ObjectiveId, PlayerId, Polygon, Position, Rect,
    Visibility,
};

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

/// The game board, wrapping `BoardDimensions` with coordinate helpers.
///
/// Origin (0, 0) is the bottom-left corner. Player A's (Attacker) edge is the
/// bottom edge (y = 0). Player B's (Defender) edge is the top edge (y = height).
///
/// Source: CP_Rules.md - "44\" x 30\" battlefield"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    /// Width and height of the board in fixed-point inches.
    pub dimensions: BoardDimensions,
    /// Terrain features placed on the board.
    pub terrain: Vec<TerrainPiece>,
    /// Objective markers placed on the board.
    pub objectives: Vec<ObjectiveMarker>,
}

impl Board {
    /// Create a standard Combat Patrol board (44" x 30") with no terrain or objectives.
    pub fn combat_patrol() -> Self {
        Self {
            dimensions: BoardDimensions::COMBAT_PATROL,
            terrain: Vec::new(),
            objectives: Vec::new(),
        }
    }

    /// Create a board with custom dimensions.
    pub fn new(dimensions: BoardDimensions) -> Self {
        Self {
            dimensions,
            terrain: Vec::new(),
            objectives: Vec::new(),
        }
    }

    /// Width of the board.
    pub fn width(&self) -> Inches {
        self.dimensions.width
    }

    /// Height of the board.
    pub fn height(&self) -> Inches {
        self.dimensions.height
    }

    /// Center of the board.
    pub fn center(&self) -> Position {
        Position::new(self.dimensions.width / 2, self.dimensions.height / 2)
    }

    /// Check if a position is within the board boundaries.
    pub fn contains(&self, pos: Position) -> bool {
        self.dimensions.contains(pos)
    }

    /// Check if a model (circular base) is entirely within the board.
    /// The center must be at least `radius` away from all edges.
    pub fn contains_model(&self, pos: Position, base: BaseSize) -> bool {
        let radius = base.radius_inches();
        pos.x >= radius
            && pos.x <= self.dimensions.width - radius
            && pos.y >= radius
            && pos.y <= self.dimensions.height - radius
    }

    /// Add a terrain piece to the board.
    pub fn add_terrain(&mut self, terrain: TerrainPiece) {
        self.terrain.push(terrain);
    }

    /// Add an objective marker to the board.
    pub fn add_objective(&mut self, objective: ObjectiveMarker) {
        self.objectives.push(objective);
    }

    /// Get all terrain pieces.
    pub fn terrain_pieces(&self) -> &[TerrainPiece] {
        &self.terrain
    }

    /// Get all objective markers.
    pub fn objective_markers(&self) -> &[ObjectiveMarker] {
        &self.objectives
    }

    /// Get a mutable reference to an objective marker by ID.
    pub fn objective_marker_mut(&mut self, id: ObjectiveId) -> Option<&mut ObjectiveMarker> {
        self.objectives.iter_mut().find(|o| o.id == id)
    }

    /// Get an objective marker by ID.
    pub fn objective_marker(&self, id: ObjectiveId) -> Option<&ObjectiveMarker> {
        self.objectives.iter().find(|o| o.id == id)
    }

    /// Check if a position overlaps any terrain piece that is impassable.
    /// A model is considered overlapping if its center is within the terrain's
    /// footprint rectangle.
    pub fn overlaps_impassable_terrain(&self, pos: Position, base: BaseSize) -> bool {
        let radius = base.radius_inches();
        for terrain in &self.terrain {
            if terrain.properties.impassable {
                // Check if the model's bounding circle overlaps the terrain rect
                if rect_circle_overlap(&terrain.footprint, pos, radius) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a position overlaps any terrain footprint (regardless of passability).
    pub fn overlaps_any_terrain(&self, pos: Position, base: BaseSize) -> bool {
        let radius = base.radius_inches();
        for terrain in &self.terrain {
            if rect_circle_overlap(&terrain.footprint, pos, radius) {
                return true;
            }
        }
        false
    }

    /// Get all terrain pieces that provide cover between `from` and `to`.
    /// A terrain piece provides cover if the line from `from` to `to` intersects
    /// its footprint and the terrain has `provides_cover` set.
    pub fn cover_between(&self, from: Position, to: Position) -> CoverType {
        for terrain in &self.terrain {
            if terrain.properties.provides_cover
                && line_rect_intersect(from, to, &terrain.footprint)
            {
                return CoverType::BenefitOfCover;
            }
        }
        CoverType::None
    }

    /// Simplified line-of-sight check: trace a line between two positions and
    /// check if any LOS-blocking terrain volume intersects it.
    ///
    /// Returns `Visibility::NotVisible` if any blocking terrain intersects the line,
    /// otherwise `Visibility::Visible`.
    ///
    /// Source: implementation_v3.md Section 6.2 - "LOS using simplified line traces"
    pub fn check_los(&self, from: Position, to: Position) -> Visibility {
        for terrain in &self.terrain {
            if terrain.properties.blocks_los
                && line_rect_intersect(from, to, &terrain.footprint)
            {
                return Visibility::NotVisible;
            }
        }
        Visibility::Visible
    }

    /// Check LOS with height consideration. A terrain piece only blocks LOS if
    /// the line passes through it AND the terrain height is sufficient to block.
    /// For simplicity in the initial implementation, any LOS-blocking terrain
    /// that the line intersects is considered blocking regardless of height.
    /// Height is provided for future use when vertical LOS matters.
    pub fn check_los_with_height(
        &self,
        from: Position,
        _from_height: Inches,
        to: Position,
        _to_height: Inches,
    ) -> Visibility {
        // Initial implementation: delegate to basic LOS check.
        // Height-based LOS refinement is a future enhancement.
        self.check_los(from, to)
    }
}

// ---------------------------------------------------------------------------
// Deployment Zones
// ---------------------------------------------------------------------------

/// Identifies which deployment zone configuration is being used.
///
/// Source: CP_Rules.md - Combat Patrol mission deployment maps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeploymentMapType {
    /// Standard deployment: each player gets a 9" deep strip on their board edge.
    /// Attacker: bottom 9" strip (y = 0 to 9")
    /// Defender: top 9" strip (y = 21" to 30")
    Standard,
    /// Search & Destroy: diagonal corners deployment.
    /// Attacker: bottom-left triangle/rectangle corner area
    /// Defender: top-right triangle/rectangle corner area
    SearchAndDestroy,
}

/// A deployment zone defined as a polygon on the board.
///
/// Each player has one deployment zone. Models must be placed wholly within
/// their owner's deployment zone during deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentZone {
    /// Which player this zone belongs to.
    pub player: PlayerId,
    /// The polygon defining the zone boundary.
    pub polygon: Polygon,
    /// The deployment map type this zone was generated from.
    pub map_type: DeploymentMapType,
}

impl DeploymentZone {
    /// Check if a position (point) is within this deployment zone.
    pub fn contains(&self, pos: Position) -> bool {
        self.polygon.contains(pos)
    }

    /// Check if a model with the given base is wholly within this deployment zone.
    /// The model's entire base must be inside the polygon.
    pub fn wholly_contains_model(&self, pos: Position, base: BaseSize) -> bool {
        self.polygon.wholly_contains_circle(pos, base.radius_inches())
    }
}

/// Configuration holding both players' deployment zones for a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// The deployment map type.
    pub map_type: DeploymentMapType,
    /// Attacker's deployment zone.
    pub attacker_zone: DeploymentZone,
    /// Defender's deployment zone.
    pub defender_zone: DeploymentZone,
}

impl DeploymentConfig {
    /// Get the deployment zone for a given player.
    pub fn zone_for(&self, player: PlayerId) -> &DeploymentZone {
        if player == self.attacker_zone.player {
            &self.attacker_zone
        } else {
            &self.defender_zone
        }
    }
}

/// Create deployment zones for the Standard deployment map on a given board.
///
/// Standard deployment:
/// - Attacker (Player 0): 9" deep strip along the bottom edge (y=0 to y=9")
/// - Defender (Player 1): 9" deep strip along the top edge (y=21" to y=30")
/// - No Man's Land: 12" gap in the center (y=9" to y=21")
///
/// For the standard 44"x30" Combat Patrol board.
pub fn create_standard_deployment(
    board: &BoardDimensions,
    attacker_id: PlayerId,
    defender_id: PlayerId,
) -> DeploymentConfig {
    let depth = Inches::from_inches(9);

    // Attacker zone: bottom strip
    let attacker_poly = Polygon::new(vec![
        Position::new(Inches::ZERO, Inches::ZERO),
        Position::new(board.width, Inches::ZERO),
        Position::new(board.width, depth),
        Position::new(Inches::ZERO, depth),
    ]);

    // Defender zone: top strip
    let defender_poly = Polygon::new(vec![
        Position::new(Inches::ZERO, board.height - depth),
        Position::new(board.width, board.height - depth),
        Position::new(board.width, board.height),
        Position::new(Inches::ZERO, board.height),
    ]);

    DeploymentConfig {
        map_type: DeploymentMapType::Standard,
        attacker_zone: DeploymentZone {
            player: attacker_id,
            polygon: attacker_poly,
            map_type: DeploymentMapType::Standard,
        },
        defender_zone: DeploymentZone {
            player: defender_id,
            polygon: defender_poly,
            map_type: DeploymentMapType::Standard,
        },
    }
}

/// Create deployment zones for the Search & Destroy deployment map.
///
/// Search & Destroy uses diagonal corners:
/// - Attacker (Player 0): bottom-left corner - a right triangle formed by
///   the bottom-left corner, extending 12" along each edge with the hypotenuse
///   cutting diagonally. More precisely, each zone is an L-shaped area or
///   triangular wedge in the corner.
///
/// For Combat Patrol (44"x30"), the Search & Destroy zones are typically:
/// - Attacker: bottom-left quadrant area, defined as a polygon from (0,0)
///   extending 18" along the bottom and 12" up the left side, with a diagonal cut.
/// - Defender: top-right quadrant area, mirror image.
///
/// The exact shape used here: a quadrilateral in the corner, extending 18" along
/// the long axis (bottom/top edge) and 12" along the short axis (left/right edge),
/// with a diagonal closing line.
///
/// Attacker zone vertices (bottom-left):
///   (0, 0) -> (18", 0) -> (0, 12") -> back to (0, 0)
/// This is a right triangle.
///
/// Defender zone vertices (top-right, mirrored):
///   (44", 30") -> (26", 30") -> (44", 18") -> back to (44", 30")
pub fn create_search_and_destroy_deployment(
    board: &BoardDimensions,
    attacker_id: PlayerId,
    defender_id: PlayerId,
) -> DeploymentConfig {
    // Triangle extent along each axis
    let extent_x = Inches::from_inches(18);
    let extent_y = Inches::from_inches(12);

    // Attacker zone: bottom-left triangle
    let attacker_poly = Polygon::new(vec![
        Position::new(Inches::ZERO, Inches::ZERO),
        Position::new(extent_x, Inches::ZERO),
        Position::new(Inches::ZERO, extent_y),
    ]);

    // Defender zone: top-right triangle (mirrored)
    let defender_poly = Polygon::new(vec![
        Position::new(board.width, board.height),
        Position::new(board.width - extent_x, board.height),
        Position::new(board.width, board.height - extent_y),
    ]);

    DeploymentConfig {
        map_type: DeploymentMapType::SearchAndDestroy,
        attacker_zone: DeploymentZone {
            player: attacker_id,
            polygon: attacker_poly,
            map_type: DeploymentMapType::SearchAndDestroy,
        },
        defender_zone: DeploymentZone {
            player: defender_id,
            polygon: defender_poly,
            map_type: DeploymentMapType::SearchAndDestroy,
        },
    }
}

// ---------------------------------------------------------------------------
// Terrain
// ---------------------------------------------------------------------------

/// Properties describing what a terrain piece does.
///
/// Source: 40k_revised.md - Terrain Features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerrainProperties {
    /// Whether this terrain provides the Benefit of Cover to models behind/within it.
    pub provides_cover: bool,
    /// Whether this terrain blocks line of sight (opaque).
    pub blocks_los: bool,
    /// Whether models cannot move through this terrain (e.g., solid walls, sealed ruins).
    pub impassable: bool,
    /// Height level of the terrain in whole inches.
    /// 0 = ground level, typically 1-5 for various terrain types.
    /// Used for vertical LOS calculations and engagement range vertical checks.
    pub height: Inches,
}

impl TerrainProperties {
    /// A ruin: provides cover and blocks LOS, but models can move through.
    pub fn ruin(height: i32) -> Self {
        Self {
            provides_cover: true,
            blocks_los: true,
            impassable: false,
            height: Inches::from_inches(height),
        }
    }

    /// A wall or solid obstacle: provides cover, blocks LOS, and is impassable.
    pub fn wall(height: i32) -> Self {
        Self {
            provides_cover: true,
            blocks_los: true,
            impassable: true,
            height: Inches::from_inches(height),
        }
    }

    /// Light cover (e.g., craters, barricades): provides cover but does not block LOS.
    pub fn light_cover() -> Self {
        Self {
            provides_cover: true,
            blocks_los: false,
            impassable: false,
            height: Inches::ZERO,
        }
    }

    /// Dense cover (e.g., woods): provides cover and blocks LOS.
    pub fn dense_cover(height: i32) -> Self {
        Self {
            provides_cover: true,
            blocks_los: true,
            impassable: false,
            height: Inches::from_inches(height),
        }
    }

    /// Open ground / no terrain effect.
    pub fn open() -> Self {
        Self {
            provides_cover: false,
            blocks_los: false,
            impassable: false,
            height: Inches::ZERO,
        }
    }
}

/// A terrain piece placed on the board, represented as an axis-aligned rectangle
/// with associated properties.
///
/// Source: implementation_v3.md Section 6.2 - "terrain represented as polygons or
/// rectangles with tagged properties"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainPiece {
    /// A human-readable name for this terrain piece (e.g., "Ruin Alpha").
    pub name: String,
    /// The axis-aligned rectangular footprint of this terrain on the board.
    pub footprint: Rect,
    /// Properties describing what this terrain does.
    pub properties: TerrainProperties,
}

impl TerrainPiece {
    /// Create a new terrain piece.
    pub fn new(name: impl Into<String>, footprint: Rect, properties: TerrainProperties) -> Self {
        Self {
            name: name.into(),
            footprint,
            properties,
        }
    }

    /// Check if a position is within this terrain's footprint.
    pub fn contains(&self, pos: Position) -> bool {
        self.footprint.contains(pos)
    }

    /// Get the center of this terrain piece.
    pub fn center(&self) -> Position {
        self.footprint.center()
    }
}

// ---------------------------------------------------------------------------
// Distance Calculations
// ---------------------------------------------------------------------------

/// Euclidean distance between two positions using the fixed-point system.
/// This is the standard "measure from closest point" distance in 40K.
///
/// Uses `Position::distance()` which computes integer sqrt for determinism.
pub fn distance(a: Position, b: Position) -> Inches {
    a.distance(b)
}

/// Distance between two models accounting for base sizes.
///
/// In 40K, distances are measured from the closest point of the bases.
/// distance_between_models = center_distance - radius_a - radius_b
///
/// If the result is negative or zero, the models are touching or overlapping.
pub fn distance_between_models(
    pos_a: Position,
    base_a: BaseSize,
    pos_b: Position,
    base_b: BaseSize,
) -> Inches {
    let center_dist = pos_a.distance(pos_b);
    let radius_a = base_a.radius_inches();
    let radius_b = base_b.radius_inches();
    let model_dist = center_dist - radius_a - radius_b;
    // Clamp to zero: negative means overlapping
    if model_dist.mils() < 0 {
        Inches::ZERO
    } else {
        model_dist
    }
}

/// Check if two positions are within a given range of each other.
/// Uses squared comparison for efficiency (avoids sqrt).
pub fn within_range(a: Position, b: Position, range: Inches) -> bool {
    a.within_range(b, range)
}

/// Check if two models are within engagement range (1" horizontal, 5" vertical).
///
/// For the 2D board representation, we only check horizontal distance.
/// The `height_a` and `height_b` parameters represent the model's height level
/// (e.g., if on terrain). Vertical check: |height_a - height_b| <= 5".
///
/// Source: 40k_revised.md - "Engagement Range: within 1\" horizontally and 5\" vertically"
pub fn within_engagement_range(
    pos_a: Position,
    base_a: BaseSize,
    height_a: Inches,
    pos_b: Position,
    base_b: BaseSize,
    height_b: Inches,
) -> bool {
    let horiz_dist = distance_between_models(pos_a, base_a, pos_b, base_b);
    let vert_dist = (height_a - height_b).abs();
    horiz_dist <= Inches::ENGAGEMENT_RANGE && vert_dist <= Inches::ENGAGEMENT_RANGE_VERTICAL
}

/// Simplified engagement range check for ground-level models (no height difference).
pub fn within_engagement_range_2d(
    pos_a: Position,
    base_a: BaseSize,
    pos_b: Position,
    base_b: BaseSize,
) -> bool {
    within_engagement_range(pos_a, base_a, Inches::ZERO, pos_b, base_b, Inches::ZERO)
}

/// Check if a model is within objective control range (3" horizontal, 5" vertical).
///
/// Source: 40k_revised.md - "within 3\" horizontally and 5\" vertically of the
/// centre of an objective marker"
pub fn within_objective_range(
    model_pos: Position,
    model_base: BaseSize,
    model_height: Inches,
    objective_pos: Position,
    objective_height: Inches,
) -> bool {
    // Distance from model base edge to objective center
    let center_dist = model_pos.distance(objective_pos);
    let radius = model_base.radius_inches();
    let horiz_dist = if center_dist.mils() > radius.mils() {
        center_dist - radius
    } else {
        Inches::ZERO
    };
    let vert_dist = (model_height - objective_height).abs();
    horiz_dist <= Inches::OBJECTIVE_RANGE && vert_dist <= Inches::OBJECTIVE_RANGE_VERTICAL
}

/// Simplified objective range check for ground-level models.
pub fn within_objective_range_2d(
    model_pos: Position,
    model_base: BaseSize,
    objective_pos: Position,
) -> bool {
    within_objective_range(model_pos, model_base, Inches::ZERO, objective_pos, Inches::ZERO)
}

/// Check if two models are within coherency range (2" horizontal, 5" vertical).
///
/// Source: 40k_revised.md - "within 2\" horizontally and 5\" vertically"
pub fn within_coherency_range(
    pos_a: Position,
    base_a: BaseSize,
    height_a: Inches,
    pos_b: Position,
    base_b: BaseSize,
    height_b: Inches,
) -> bool {
    let horiz_dist = distance_between_models(pos_a, base_a, pos_b, base_b);
    let vert_dist = (height_a - height_b).abs();
    horiz_dist <= Inches::COHERENCY_RANGE && vert_dist <= Inches::COHERENCY_RANGE_VERTICAL
}

/// Simplified coherency check for ground-level models.
pub fn within_coherency_range_2d(
    pos_a: Position,
    base_a: BaseSize,
    pos_b: Position,
    base_b: BaseSize,
) -> bool {
    within_coherency_range(pos_a, base_a, Inches::ZERO, pos_b, base_b, Inches::ZERO)
}

/// Check if a model is far enough from all enemy models for Deep Strike placement.
/// The model must be more than 9" from every enemy model.
///
/// Source: 40k_revised.md - "more than 9\" horizontally away from all enemy models"
pub fn valid_deep_strike_position(
    pos: Position,
    base: BaseSize,
    enemy_positions: &[(Position, BaseSize)],
) -> bool {
    for &(enemy_pos, enemy_base) in enemy_positions {
        let dist = distance_between_models(pos, base, enemy_pos, enemy_base);
        if dist <= Inches::DEEP_STRIKE_MIN_DISTANCE {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Coherency Checker
// ---------------------------------------------------------------------------

/// Position and base info for a single model, used in coherency checks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelPosition {
    pub position: Position,
    pub base_size: BaseSize,
    /// Height level (0 for ground level).
    pub height: Inches,
}

impl ModelPosition {
    pub fn new(position: Position, base_size: BaseSize) -> Self {
        Self {
            position,
            base_size,
            height: Inches::ZERO,
        }
    }

    pub fn with_height(position: Position, base_size: BaseSize, height: Inches) -> Self {
        Self {
            position,
            base_size,
            height,
        }
    }
}

/// Result of a coherency check on a unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoherencyResult {
    /// The unit is in coherency.
    Coherent,
    /// The unit is out of coherency. Contains indices of models that violate coherency.
    OutOfCoherency {
        /// Indices of models that are not coherent with enough neighbors.
        violating_models: Vec<usize>,
    },
    /// Single model units are always coherent.
    SingleModel,
}

impl CoherencyResult {
    /// Whether the unit is in coherency.
    pub fn is_coherent(&self) -> bool {
        matches!(self, CoherencyResult::Coherent | CoherencyResult::SingleModel)
    }
}

/// Check unit coherency for a set of model positions.
///
/// Rules:
/// - Each model in the unit must be within 2" horizontally and 5" vertically
///   of at least one other model in the unit.
/// - If the unit has 7 or more models, each model must be within coherency
///   range of at least 2 other models in the unit.
///
/// Source: 40k_revised.md - "UNIT COHERENCY"
pub fn check_coherency(models: &[ModelPosition]) -> CoherencyResult {
    let count = models.len();

    if count <= 1 {
        return CoherencyResult::SingleModel;
    }

    // Determine the minimum number of coherent neighbors required
    let min_neighbors = if count >= 7 { 2 } else { 1 };

    let mut violating = Vec::new();

    for i in 0..count {
        let mut coherent_neighbors = 0;
        for j in 0..count {
            if i == j {
                continue;
            }
            if within_coherency_range(
                models[i].position,
                models[i].base_size,
                models[i].height,
                models[j].position,
                models[j].base_size,
                models[j].height,
            ) {
                coherent_neighbors += 1;
                if coherent_neighbors >= min_neighbors {
                    break;
                }
            }
        }
        if coherent_neighbors < min_neighbors {
            violating.push(i);
        }
    }

    if violating.is_empty() {
        CoherencyResult::Coherent
    } else {
        CoherencyResult::OutOfCoherency {
            violating_models: violating,
        }
    }
}

// ---------------------------------------------------------------------------
// Objective Markers
// ---------------------------------------------------------------------------

/// Who currently controls an objective marker.
///
/// Source: 40k_revised.md - "OBJECTIVE CONTROL"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectiveControlStatus {
    /// No player controls this objective (no models within range, or tied OC).
    Uncontrolled,
    /// A specific player controls this objective.
    ControlledBy(PlayerId),
    /// Both players have equal OC values contesting this objective.
    Contested,
}

/// An objective marker placed on the battlefield.
///
/// Objective markers are 40mm round markers. Models within 3" horizontally
/// and 5" vertically can contribute their OC value to contest/control it.
///
/// Source: 40k_revised.md - Objective Markers
/// Source: CP_Rules.md - Objective placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveMarker {
    /// Unique identifier for this objective.
    pub id: ObjectiveId,
    /// Center position of the objective marker on the board.
    pub position: Position,
    /// Height level of the objective (usually ground level = 0).
    pub height: Inches,
    /// Current control status.
    pub control_status: ObjectiveControlStatus,
    /// Optional label for the objective (e.g., "A", "B", "Center").
    pub label: String,
}

impl ObjectiveMarker {
    /// Create a new objective marker at the given position.
    pub fn new(id: ObjectiveId, position: Position, label: impl Into<String>) -> Self {
        Self {
            id,
            position,
            height: Inches::ZERO,
            control_status: ObjectiveControlStatus::Uncontrolled,
            label: label.into(),
        }
    }

    /// Create a new objective marker with height.
    pub fn with_height(
        id: ObjectiveId,
        position: Position,
        height: Inches,
        label: impl Into<String>,
    ) -> Self {
        Self {
            id,
            position,
            height,
            control_status: ObjectiveControlStatus::Uncontrolled,
            label: label.into(),
        }
    }

    /// Check if a model is within range to contest this objective.
    pub fn is_model_in_range(
        &self,
        model_pos: Position,
        model_base: BaseSize,
        model_height: Inches,
    ) -> bool {
        within_objective_range(model_pos, model_base, model_height, self.position, self.height)
    }

    /// Simplified range check for ground-level models.
    pub fn is_model_in_range_2d(&self, model_pos: Position, model_base: BaseSize) -> bool {
        within_objective_range_2d(model_pos, model_base, self.position)
    }

    /// Update the control status based on OC tallies.
    ///
    /// `player_oc` is a list of (PlayerId, total_oc) pairs for all players
    /// that have models within range of this objective.
    ///
    /// The player with the highest total OC controls the objective.
    /// If tied, the objective is contested. If no player has models in range,
    /// it remains uncontrolled.
    ///
    /// Source: 40k_revised.md - "The player whose models have the highest
    /// total OC controls the objective marker."
    pub fn update_control(&mut self, player_oc: &[(PlayerId, u32)]) {
        if player_oc.is_empty() {
            self.control_status = ObjectiveControlStatus::Uncontrolled;
            return;
        }

        // Find the highest OC value
        let max_oc = player_oc.iter().map(|(_, oc)| *oc).max().unwrap_or(0);

        if max_oc == 0 {
            self.control_status = ObjectiveControlStatus::Uncontrolled;
            return;
        }

        // Count how many players have the maximum OC
        let players_with_max: Vec<PlayerId> = player_oc
            .iter()
            .filter(|(_, oc)| *oc == max_oc)
            .map(|(pid, _)| *pid)
            .collect();

        if players_with_max.len() == 1 {
            self.control_status = ObjectiveControlStatus::ControlledBy(players_with_max[0]);
        } else {
            self.control_status = ObjectiveControlStatus::Contested;
        }
    }
}

// ---------------------------------------------------------------------------
// Movement Legality Helpers
// ---------------------------------------------------------------------------

/// Result of a movement legality check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementLegality {
    /// The move is legal.
    Legal,
    /// The move is illegal for the given reason.
    Illegal(MovementIllegalReason),
}

/// Reasons a move can be illegal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementIllegalReason {
    /// The destination is outside the board boundaries.
    OutOfBounds,
    /// The destination overlaps impassable terrain.
    OverlapsImpassableTerrain,
    /// The destination is not within the required deployment zone.
    OutsideDeploymentZone,
    /// The destination overlaps another model's base.
    OverlapsModel,
    /// The move exceeds the model's maximum move distance.
    ExceedsMaxDistance,
    /// The destination is too close to an enemy model (for Deep Strike).
    TooCloseToEnemy,
    /// The model would break unit coherency.
    BreaksCoherency,
}

/// Check if a target position is a legal position for a model on the board.
/// Checks: within board bounds and not overlapping impassable terrain.
pub fn is_valid_board_position(board: &Board, pos: Position, base: BaseSize) -> MovementLegality {
    if !board.contains_model(pos, base) {
        return MovementLegality::Illegal(MovementIllegalReason::OutOfBounds);
    }
    if board.overlaps_impassable_terrain(pos, base) {
        return MovementLegality::Illegal(MovementIllegalReason::OverlapsImpassableTerrain);
    }
    MovementLegality::Legal
}

/// Check if a target position is within a deployment zone.
pub fn is_in_deployment_zone(
    zone: &DeploymentZone,
    pos: Position,
    base: BaseSize,
) -> MovementLegality {
    if !zone.wholly_contains_model(pos, base) {
        return MovementLegality::Illegal(MovementIllegalReason::OutsideDeploymentZone);
    }
    MovementLegality::Legal
}

/// Check if a position overlaps any of the given model positions.
/// Returns true if the model at `pos` with `base` would overlap any model
/// in `other_models`.
pub fn overlaps_any_model(
    pos: Position,
    base: BaseSize,
    other_models: &[(Position, BaseSize)],
) -> bool {
    let radius = base.radius_inches();
    for &(other_pos, other_base) in other_models {
        let other_radius = other_base.radius_inches();
        let center_dist = pos.distance(other_pos);
        let min_dist = radius + other_radius;
        if center_dist < min_dist {
            return true;
        }
    }
    false
}

/// Full deployment legality check: position must be on the board, in the deployment
/// zone, not overlapping impassable terrain, and not overlapping other models.
pub fn check_deployment_legality(
    board: &Board,
    zone: &DeploymentZone,
    pos: Position,
    base: BaseSize,
    other_models: &[(Position, BaseSize)],
) -> MovementLegality {
    // Check board bounds
    let board_check = is_valid_board_position(board, pos, base);
    if board_check != MovementLegality::Legal {
        return board_check;
    }

    // Check deployment zone
    let zone_check = is_in_deployment_zone(zone, pos, base);
    if zone_check != MovementLegality::Legal {
        return zone_check;
    }

    // Check model overlap
    if overlaps_any_model(pos, base, other_models) {
        return MovementLegality::Illegal(MovementIllegalReason::OverlapsModel);
    }

    MovementLegality::Legal
}

/// Check movement legality for a normal move. The model must:
/// 1. End within board bounds
/// 2. Not overlap impassable terrain
/// 3. Not overlap other models
/// 4. Not exceed maximum movement distance
///
/// Note: this does NOT check coherency (that should be checked at the unit level
/// after all models in the unit have been moved).
pub fn check_move_legality(
    board: &Board,
    from: Position,
    to: Position,
    base: BaseSize,
    max_distance: Inches,
    other_models: &[(Position, BaseSize)],
) -> MovementLegality {
    // Check board bounds
    let board_check = is_valid_board_position(board, to, base);
    if board_check != MovementLegality::Legal {
        return board_check;
    }

    // Check distance
    let move_dist = from.distance(to);
    if move_dist > max_distance {
        return MovementLegality::Illegal(MovementIllegalReason::ExceedsMaxDistance);
    }

    // Check model overlap
    if overlaps_any_model(to, base, other_models) {
        return MovementLegality::Illegal(MovementIllegalReason::OverlapsModel);
    }

    MovementLegality::Legal
}

/// Check Deep Strike arrival legality. The model must:
/// 1. Be on the board
/// 2. Not overlap impassable terrain
/// 3. Not overlap other models
/// 4. Be more than 9" from all enemy models
pub fn check_deep_strike_legality(
    board: &Board,
    pos: Position,
    base: BaseSize,
    friendly_models: &[(Position, BaseSize)],
    enemy_models: &[(Position, BaseSize)],
) -> MovementLegality {
    // Check board bounds
    let board_check = is_valid_board_position(board, pos, base);
    if board_check != MovementLegality::Legal {
        return board_check;
    }

    // Check overlap with all models
    if overlaps_any_model(pos, base, friendly_models) || overlaps_any_model(pos, base, enemy_models)
    {
        return MovementLegality::Illegal(MovementIllegalReason::OverlapsModel);
    }

    // Check distance from enemies
    if !valid_deep_strike_position(pos, base, enemy_models) {
        return MovementLegality::Illegal(MovementIllegalReason::TooCloseToEnemy);
    }

    MovementLegality::Legal
}

// ---------------------------------------------------------------------------
// Line-Rect Intersection (for LOS)
// ---------------------------------------------------------------------------

/// Check if a line segment from `p0` to `p1` intersects an axis-aligned rectangle.
///
/// Uses the Cohen-Sutherland-style parametric approach with integer arithmetic
/// for full determinism.
///
/// Returns true if any part of the line segment passes through or touches the rect.
fn line_rect_intersect(p0: Position, p1: Position, rect: &Rect) -> bool {
    // Use the Liang-Barsky algorithm adapted for integer arithmetic.
    // Parametric line: P(t) = p0 + t*(p1 - p0), t in [0, 1]
    // We need to find if there exists a t in [0, 1] where P(t) is inside the rect.

    let dx = p1.x.mils() as i64 - p0.x.mils() as i64;
    let dy = p1.y.mils() as i64 - p0.y.mils() as i64;

    let x_min = rect.min_x.mils() as i64;
    let x_max = rect.max_x.mils() as i64;
    let y_min = rect.min_y.mils() as i64;
    let y_max = rect.max_y.mils() as i64;

    let p0x = p0.x.mils() as i64;
    let p0y = p0.y.mils() as i64;

    // We track the parametric interval [t_enter, t_exit] that is inside the rect.
    // Using rational arithmetic: t = numerator / denominator
    // We represent t_enter as t_enter_num / t_enter_den (always t_enter_den > 0)
    // and t_exit as t_exit_num / t_exit_den (always t_exit_den > 0).

    // Start with the full interval [0, 1]
    // t_enter = 0/1, t_exit = 1/1
    let mut t_enter_num: i64 = 0;
    let mut t_enter_den: i64 = 1;
    let mut t_exit_num: i64 = 1;
    let mut t_exit_den: i64 = 1;

    // Helper: clip_edge checks the boundary condition for one edge.
    // The line crosses the boundary at t = (boundary - p0_coord) / d_coord.
    // If d_coord > 0, the line enters through min and exits through max.
    // If d_coord < 0, the line enters through max and exits through min.
    // If d_coord == 0, the line is parallel; check if p0_coord is within bounds.

    // Clip against x_min and x_max
    if dx == 0 {
        // Parallel to y-axis
        if p0x < x_min || p0x > x_max {
            return false;
        }
    } else {
        // t at x_min: t = (x_min - p0x) / dx
        // t at x_max: t = (x_max - p0x) / dx
        let (t_min_num, t_min_den, t_max_num, t_max_den) = if dx > 0 {
            (x_min - p0x, dx, x_max - p0x, dx)
        } else {
            (x_max - p0x, dx, x_min - p0x, dx)
        };

        // Normalize: ensure denominator is positive for comparison
        let (t_min_num, t_min_den) = if t_min_den < 0 {
            (-t_min_num, -t_min_den)
        } else {
            (t_min_num, t_min_den)
        };
        let (t_max_num, t_max_den) = if t_max_den < 0 {
            (-t_max_num, -t_max_den)
        } else {
            (t_max_num, t_max_den)
        };

        // Update t_enter = max(t_enter, t_min)
        // Compare t_enter_num/t_enter_den vs t_min_num/t_min_den
        // Cross multiply: t_enter_num * t_min_den vs t_min_num * t_enter_den
        if t_min_num * t_enter_den > t_enter_num * t_min_den {
            t_enter_num = t_min_num;
            t_enter_den = t_min_den;
        }

        // Update t_exit = min(t_exit, t_max)
        if t_max_num * t_exit_den < t_exit_num * t_max_den {
            t_exit_num = t_max_num;
            t_exit_den = t_max_den;
        }

        // Early exit: if t_enter > t_exit, no intersection
        if t_enter_num * t_exit_den > t_exit_num * t_enter_den {
            return false;
        }
    }

    // Clip against y_min and y_max
    if dy == 0 {
        // Parallel to x-axis
        if p0y < y_min || p0y > y_max {
            return false;
        }
    } else {
        let (t_min_num, t_min_den, t_max_num, t_max_den) = if dy > 0 {
            (y_min - p0y, dy, y_max - p0y, dy)
        } else {
            (y_max - p0y, dy, y_min - p0y, dy)
        };

        let (t_min_num, t_min_den) = if t_min_den < 0 {
            (-t_min_num, -t_min_den)
        } else {
            (t_min_num, t_min_den)
        };
        let (t_max_num, t_max_den) = if t_max_den < 0 {
            (-t_max_num, -t_max_den)
        } else {
            (t_max_num, t_max_den)
        };

        if t_min_num * t_enter_den > t_enter_num * t_min_den {
            t_enter_num = t_min_num;
            t_enter_den = t_min_den;
        }

        if t_max_num * t_exit_den < t_exit_num * t_max_den {
            t_exit_num = t_max_num;
            t_exit_den = t_max_den;
        }

        if t_enter_num * t_exit_den > t_exit_num * t_enter_den {
            return false;
        }
    }

    // Check that the intersection interval overlaps [0, 1]
    // t_enter <= 1 and t_exit >= 0
    // t_enter_num / t_enter_den <= 1  =>  t_enter_num <= t_enter_den
    // t_exit_num / t_exit_den >= 0  =>  t_exit_num >= 0 (since t_exit_den > 0)
    t_enter_num <= t_enter_den && t_exit_num >= 0
}

/// Check if a circle (model base) overlaps an axis-aligned rectangle (terrain footprint).
///
/// The overlap check: the closest point on the rectangle to the circle center
/// is within the circle's radius.
fn rect_circle_overlap(rect: &Rect, center: Position, radius: Inches) -> bool {
    // Find the closest point on the rect to the center
    let closest_x = center.x.mils().max(rect.min_x.mils()).min(rect.max_x.mils());
    let closest_y = center.y.mils().max(rect.min_y.mils()).min(rect.max_y.mils());

    let dx = (center.x.mils() - closest_x) as i64;
    let dy = (center.y.mils() - closest_y) as i64;
    let dist_sq = dx * dx + dy * dy;
    let radius_sq = (radius.mils() as i64) * (radius.mils() as i64);

    dist_sq <= radius_sq
}

// ---------------------------------------------------------------------------
// Visibility Result
// ---------------------------------------------------------------------------

/// Extended visibility result that includes cover information.
///
/// When checking if a shooter can target a unit, we need to know:
/// 1. Whether the target is visible at all (LOS)
/// 2. Whether the target has the Benefit of Cover
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VisibilityResult {
    /// Basic visibility (can the shooter see the target?).
    pub visibility: Visibility,
    /// Cover type between shooter and target.
    pub cover: CoverType,
}

impl VisibilityResult {
    /// Target is visible with no cover.
    pub fn visible_no_cover() -> Self {
        Self {
            visibility: Visibility::Visible,
            cover: CoverType::None,
        }
    }

    /// Target is visible but has the Benefit of Cover.
    pub fn visible_with_cover() -> Self {
        Self {
            visibility: Visibility::Visible,
            cover: CoverType::BenefitOfCover,
        }
    }

    /// Target is not visible.
    pub fn not_visible() -> Self {
        Self {
            visibility: Visibility::NotVisible,
            cover: CoverType::None,
        }
    }

    /// Whether the target is visible.
    pub fn is_visible(&self) -> bool {
        self.visibility == Visibility::Visible
    }

    /// Whether the target has the Benefit of Cover.
    pub fn has_cover(&self) -> bool {
        self.cover == CoverType::BenefitOfCover
    }
}

/// Check visibility between a shooter and target, considering terrain on the board.
/// Returns a `VisibilityResult` with both LOS and cover information.
///
/// The check:
/// 1. Trace a line from shooter to target
/// 2. If any LOS-blocking terrain intersects the line, target is not visible
/// 3. If any cover-providing terrain intersects the line, target has cover
pub fn check_visibility(board: &Board, from: Position, to: Position) -> VisibilityResult {
    let los = board.check_los(from, to);
    if los == Visibility::NotVisible {
        return VisibilityResult::not_visible();
    }

    let cover = board.cover_between(from, to);
    VisibilityResult {
        visibility: Visibility::Visible,
        cover,
    }
}

// ---------------------------------------------------------------------------
// Standard Objective Layouts
// ---------------------------------------------------------------------------

/// Create the standard 4-objective layout for a Combat Patrol board.
///
/// Objectives are placed at evenly spaced positions in the center band of the board.
/// Typical CP missions have 4 objectives:
///   A: (11", 15") - left-center
///   B: (22", 10") - center-bottom
///   C: (22", 20") - center-top
///   D: (33", 15") - right-center
pub fn create_standard_objectives() -> Vec<ObjectiveMarker> {
    vec![
        ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(11, 15),
            "A",
        ),
        ObjectiveMarker::new(
            ObjectiveId::new(1),
            Position::from_inches(22, 10),
            "B",
        ),
        ObjectiveMarker::new(
            ObjectiveId::new(2),
            Position::from_inches(22, 20),
            "C",
        ),
        ObjectiveMarker::new(
            ObjectiveId::new(3),
            Position::from_inches(33, 15),
            "D",
        ),
    ]
}

/// Create a center objective layout (single objective at board center).
pub fn create_center_objective() -> Vec<ObjectiveMarker> {
    vec![ObjectiveMarker::new(
        ObjectiveId::new(0),
        Position::from_inches(22, 15),
        "Center",
    )]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Board tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_board_combat_patrol_dimensions() {
        let board = Board::combat_patrol();
        assert_eq!(board.width(), Inches::from_inches(44));
        assert_eq!(board.height(), Inches::from_inches(30));
    }

    #[test]
    fn test_board_center() {
        let board = Board::combat_patrol();
        let center = board.center();
        assert_eq!(center.x, Inches::from_inches(22));
        assert_eq!(center.y, Inches::from_inches(15));
    }

    #[test]
    fn test_board_contains_position() {
        let board = Board::combat_patrol();
        assert!(board.contains(Position::from_inches(0, 0)));
        assert!(board.contains(Position::from_inches(44, 30)));
        assert!(board.contains(Position::from_inches(22, 15)));
        assert!(!board.contains(Position::from_inches(45, 15)));
        assert!(!board.contains(Position::from_inches(-1, 15)));
        assert!(!board.contains(Position::from_inches(22, 31)));
        assert!(!board.contains(Position::from_inches(22, -1)));
    }

    #[test]
    fn test_board_contains_model() {
        let board = Board::combat_patrol();
        // A 32mm base model (radius ~0.63") at the center should be fine
        assert!(board.contains_model(Position::from_inches(22, 15), BaseSize::MM32));
        // A model right at the edge with a base would extend outside
        assert!(!board.contains_model(Position::from_inches(0, 0), BaseSize::MM32));
        // Model near the edge but with enough clearance
        assert!(board.contains_model(Position::from_inches(1, 1), BaseSize::MM32));
    }

    #[test]
    fn test_board_add_terrain() {
        let mut board = Board::combat_patrol();
        let terrain = TerrainPiece::new(
            "Ruin Alpha",
            Rect::from_inches(10, 10, 14, 14),
            TerrainProperties::ruin(3),
        );
        board.add_terrain(terrain);
        assert_eq!(board.terrain_pieces().len(), 1);
        assert_eq!(board.terrain_pieces()[0].name, "Ruin Alpha");
    }

    #[test]
    fn test_board_overlaps_impassable_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Wall",
            Rect::from_inches(20, 14, 24, 16),
            TerrainProperties::wall(3),
        ));
        // Model on top of the wall
        assert!(board.overlaps_impassable_terrain(Position::from_inches(22, 15), BaseSize::MM32));
        // Model far away from the wall
        assert!(!board.overlaps_impassable_terrain(Position::from_inches(5, 5), BaseSize::MM32));
    }

    #[test]
    fn test_board_does_not_flag_passable_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Ruin",
            Rect::from_inches(20, 14, 24, 16),
            TerrainProperties::ruin(3),
        ));
        // Ruins are not impassable, so this should return false
        assert!(!board.overlaps_impassable_terrain(Position::from_inches(22, 15), BaseSize::MM32));
    }

    // -----------------------------------------------------------------------
    // LOS tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_los_clear() {
        let board = Board::combat_patrol();
        let result = board.check_los(Position::from_inches(5, 5), Position::from_inches(40, 25));
        assert_eq!(result, Visibility::Visible);
    }

    #[test]
    fn test_los_blocked_by_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Wall",
            Rect::from_inches(20, 10, 24, 20),
            TerrainProperties::wall(5),
        ));
        // Line from (5, 15) to (40, 15) passes through the wall
        let result = board.check_los(
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert_eq!(result, Visibility::NotVisible);
    }

    #[test]
    fn test_los_not_blocked_by_non_los_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Crater",
            Rect::from_inches(20, 10, 24, 20),
            TerrainProperties::light_cover(),
        ));
        // Light cover doesn't block LOS
        let result = board.check_los(
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert_eq!(result, Visibility::Visible);
    }

    #[test]
    fn test_los_line_misses_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Wall",
            Rect::from_inches(20, 10, 24, 14),
            TerrainProperties::wall(5),
        ));
        // Line from (5, 25) to (40, 25) goes above the wall
        let result = board.check_los(
            Position::from_inches(5, 25),
            Position::from_inches(40, 25),
        );
        assert_eq!(result, Visibility::Visible);
    }

    // -----------------------------------------------------------------------
    // Cover tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cover_from_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Barricade",
            Rect::from_inches(20, 14, 24, 16),
            TerrainProperties {
                provides_cover: true,
                blocks_los: false,
                impassable: false,
                height: Inches::from_inches(1),
            },
        ));
        let cover = board.cover_between(
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert_eq!(cover, CoverType::BenefitOfCover);
    }

    #[test]
    fn test_no_cover_when_no_terrain() {
        let board = Board::combat_patrol();
        let cover = board.cover_between(
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert_eq!(cover, CoverType::None);
    }

    // -----------------------------------------------------------------------
    // Visibility Result tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_visibility_result_clear() {
        let board = Board::combat_patrol();
        let result = check_visibility(
            &board,
            Position::from_inches(5, 5),
            Position::from_inches(40, 25),
        );
        assert!(result.is_visible());
        assert!(!result.has_cover());
    }

    #[test]
    fn test_visibility_result_blocked() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Wall",
            Rect::from_inches(20, 10, 24, 20),
            TerrainProperties::wall(5),
        ));
        let result = check_visibility(
            &board,
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert!(!result.is_visible());
    }

    #[test]
    fn test_visibility_result_with_cover() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Barricade",
            Rect::from_inches(20, 14, 24, 16),
            TerrainProperties {
                provides_cover: true,
                blocks_los: false,
                impassable: false,
                height: Inches::from_inches(1),
            },
        ));
        let result = check_visibility(
            &board,
            Position::from_inches(5, 15),
            Position::from_inches(40, 15),
        );
        assert!(result.is_visible());
        assert!(result.has_cover());
    }

    // -----------------------------------------------------------------------
    // Deployment Zone tests - Standard
    // -----------------------------------------------------------------------

    #[test]
    fn test_standard_deployment_attacker_zone() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = &config.attacker_zone;
        // Inside attacker zone (bottom strip, y=0 to 9")
        assert!(zone.contains(Position::from_inches(22, 4)));
        assert!(zone.contains(Position::from_inches(1, 1)));
        // Outside attacker zone (in no man's land)
        assert!(!zone.contains(Position::from_inches(22, 15)));
        // Outside attacker zone (in defender zone)
        assert!(!zone.contains(Position::from_inches(22, 25)));
    }

    #[test]
    fn test_standard_deployment_defender_zone() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = &config.defender_zone;
        // Inside defender zone (top strip, y=21" to 30")
        assert!(zone.contains(Position::from_inches(22, 25)));
        assert!(zone.contains(Position::from_inches(40, 28)));
        // Outside defender zone
        assert!(!zone.contains(Position::from_inches(22, 15)));
        assert!(!zone.contains(Position::from_inches(22, 4)));
    }

    #[test]
    fn test_standard_deployment_no_mans_land() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        // The center of the board should not be in either deployment zone
        let center = Position::from_inches(22, 15);
        assert!(!config.attacker_zone.contains(center));
        assert!(!config.defender_zone.contains(center));
    }

    #[test]
    fn test_standard_deployment_zone_for() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = config.zone_for(PlayerId::new(0));
        assert_eq!(zone.player, PlayerId::new(0));
        let zone = config.zone_for(PlayerId::new(1));
        assert_eq!(zone.player, PlayerId::new(1));
    }

    #[test]
    fn test_standard_deployment_wholly_contains_model() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = &config.attacker_zone;
        // Model well inside the zone
        assert!(zone.wholly_contains_model(Position::from_inches(22, 4), BaseSize::MM32));
        // Model right at the edge of the zone - base extends into no man's land
        // At y=9", the model's base extends above the zone boundary
        assert!(!zone.wholly_contains_model(Position::from_inches(22, 9), BaseSize::MM32));
        // Model near the edge but with clearance (~8.5")
        assert!(zone.wholly_contains_model(
            Position::new(Inches::from_inches(22), Inches::from_mils(8000)),
            BaseSize::MM32,
        ));
    }

    // -----------------------------------------------------------------------
    // Deployment Zone tests - Search & Destroy
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_and_destroy_attacker_zone() {
        let config = create_search_and_destroy_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = &config.attacker_zone;
        // Inside the bottom-left triangle
        assert!(zone.contains(Position::from_inches(2, 2)));
        assert!(zone.contains(Position::from_inches(5, 2)));
        // Outside the triangle (center of board)
        assert!(!zone.contains(Position::from_inches(22, 15)));
        // Outside the triangle (top-right)
        assert!(!zone.contains(Position::from_inches(40, 25)));
    }

    #[test]
    fn test_search_and_destroy_defender_zone() {
        let config = create_search_and_destroy_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let zone = &config.defender_zone;
        // Inside the top-right triangle
        assert!(zone.contains(Position::from_inches(42, 28)));
        assert!(zone.contains(Position::from_inches(40, 28)));
        // Outside the triangle (center of board)
        assert!(!zone.contains(Position::from_inches(22, 15)));
        // Outside the triangle (bottom-left)
        assert!(!zone.contains(Position::from_inches(2, 2)));
    }

    #[test]
    fn test_search_and_destroy_symmetry() {
        let config = create_search_and_destroy_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        // A point near the bottom-left corner should be in attacker zone
        assert!(config.attacker_zone.contains(Position::from_inches(3, 2)));
        // The mirrored point near top-right should be in defender zone
        assert!(config.defender_zone.contains(Position::from_inches(41, 28)));
    }

    // -----------------------------------------------------------------------
    // Distance calculation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_euclidean_distance() {
        let a = Position::from_inches(0, 0);
        let b = Position::from_inches(3, 4);
        assert_eq!(distance(a, b), Inches::from_inches(5));
    }

    #[test]
    fn test_distance_zero() {
        let a = Position::from_inches(10, 10);
        assert_eq!(distance(a, a), Inches::ZERO);
    }

    #[test]
    fn test_distance_between_models() {
        // Two 32mm base models (radius ~0.63") 5" apart center-to-center
        let a = Position::from_inches(0, 0);
        let b = Position::from_inches(5, 0);
        let dist = distance_between_models(a, BaseSize::MM32, b, BaseSize::MM32);
        // 5" - 2*0.63" = ~3.74"
        let radius = BaseSize::MM32.radius_inches();
        let expected = Inches::from_inches(5) - radius - radius;
        assert_eq!(dist, expected);
    }

    #[test]
    fn test_distance_between_models_touching() {
        // Two models right on top of each other
        let a = Position::from_inches(5, 5);
        let b = Position::from_inches(5, 5);
        let dist = distance_between_models(a, BaseSize::MM32, b, BaseSize::MM32);
        assert_eq!(dist, Inches::ZERO);
    }

    #[test]
    fn test_within_range() {
        let a = Position::from_inches(0, 0);
        let b = Position::from_inches(3, 4);
        assert!(within_range(a, b, Inches::from_inches(5)));
        assert!(within_range(a, b, Inches::from_inches(6)));
        assert!(!within_range(a, b, Inches::from_inches(4)));
    }

    // -----------------------------------------------------------------------
    // Engagement range tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_within_engagement_range_close_models() {
        // Two 32mm base models 2" apart center-to-center
        // Actual edge distance: 2" - 2*~0.63" = ~0.74"
        // This is within 1" engagement range
        let a = Position::from_inches(10, 10);
        let b = Position::new(Inches::from_inches(12), Inches::from_inches(10));
        assert!(within_engagement_range_2d(a, BaseSize::MM32, b, BaseSize::MM32));
    }

    #[test]
    fn test_within_engagement_range_far_models() {
        // Two models 10" apart - definitely not in engagement range
        let a = Position::from_inches(5, 5);
        let b = Position::from_inches(15, 5);
        assert!(!within_engagement_range_2d(a, BaseSize::MM32, b, BaseSize::MM32));
    }

    #[test]
    fn test_within_engagement_range_with_height() {
        // Same horizontal position but different heights
        let a_pos = Position::from_inches(10, 10);
        let b_pos = Position::from_inches(10, 10);
        // Within 5" vertical - should be in engagement range
        assert!(within_engagement_range(
            a_pos, BaseSize::MM32, Inches::ZERO,
            b_pos, BaseSize::MM32, Inches::from_inches(4),
        ));
        // More than 5" vertical - should NOT be in engagement range
        assert!(!within_engagement_range(
            a_pos, BaseSize::MM32, Inches::ZERO,
            b_pos, BaseSize::MM32, Inches::from_inches(6),
        ));
    }

    // -----------------------------------------------------------------------
    // Objective range tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_within_objective_range() {
        let model_pos = Position::from_inches(10, 10);
        let obj_pos = Position::from_inches(13, 10);
        // 3" apart center-to-center, minus base radius (~0.63") = ~2.37"
        // This is within 3" objective range
        assert!(within_objective_range_2d(model_pos, BaseSize::MM32, obj_pos));
    }

    #[test]
    fn test_outside_objective_range() {
        let model_pos = Position::from_inches(10, 10);
        let obj_pos = Position::from_inches(14, 10);
        // 4" apart center-to-center, minus base radius (~0.63") = ~3.37"
        // This is outside 3" objective range
        assert!(!within_objective_range_2d(model_pos, BaseSize::MM32, obj_pos));
    }

    #[test]
    fn test_within_objective_range_on_objective() {
        let model_pos = Position::from_inches(10, 10);
        let obj_pos = Position::from_inches(10, 10);
        // Model is on top of the objective
        assert!(within_objective_range_2d(model_pos, BaseSize::MM32, obj_pos));
    }

    #[test]
    fn test_within_objective_range_with_height() {
        let model_pos = Position::from_inches(10, 10);
        let obj_pos = Position::from_inches(10, 10);
        // Within 5" vertical
        assert!(within_objective_range(
            model_pos, BaseSize::MM32, Inches::from_inches(4),
            obj_pos, Inches::ZERO,
        ));
        // More than 5" vertical
        assert!(!within_objective_range(
            model_pos, BaseSize::MM32, Inches::from_inches(6),
            obj_pos, Inches::ZERO,
        ));
    }

    // -----------------------------------------------------------------------
    // Coherency range tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_within_coherency_range() {
        // Two models 3" apart center-to-center
        // Edge distance: 3" - 2*~0.63" = ~1.74"
        // This is within 2" coherency range
        let a = Position::from_inches(10, 10);
        let b = Position::from_inches(13, 10);
        assert!(within_coherency_range_2d(a, BaseSize::MM32, b, BaseSize::MM32));
    }

    #[test]
    fn test_outside_coherency_range() {
        // Two models 6" apart
        // Edge distance: 6" - 2*~0.63" = ~4.74"
        // This is outside 2" coherency range
        let a = Position::from_inches(10, 10);
        let b = Position::from_inches(16, 10);
        assert!(!within_coherency_range_2d(a, BaseSize::MM32, b, BaseSize::MM32));
    }

    // -----------------------------------------------------------------------
    // Coherency checker tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_coherency_single_model() {
        let models = vec![ModelPosition::new(
            Position::from_inches(10, 10),
            BaseSize::MM32,
        )];
        let result = check_coherency(&models);
        assert_eq!(result, CoherencyResult::SingleModel);
        assert!(result.is_coherent());
    }

    #[test]
    fn test_coherency_empty() {
        let models: Vec<ModelPosition> = vec![];
        let result = check_coherency(&models);
        assert_eq!(result, CoherencyResult::SingleModel);
    }

    #[test]
    fn test_coherency_two_models_close() {
        let models = vec![
            ModelPosition::new(Position::from_inches(10, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(12, 10), BaseSize::MM32),
        ];
        let result = check_coherency(&models);
        assert_eq!(result, CoherencyResult::Coherent);
        assert!(result.is_coherent());
    }

    #[test]
    fn test_coherency_two_models_far() {
        let models = vec![
            ModelPosition::new(Position::from_inches(5, 5), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(20, 20), BaseSize::MM32),
        ];
        let result = check_coherency(&models);
        assert!(!result.is_coherent());
        match result {
            CoherencyResult::OutOfCoherency { violating_models } => {
                // Both models should be violating since neither is near the other
                assert_eq!(violating_models.len(), 2);
            }
            _ => panic!("Expected OutOfCoherency"),
        }
    }

    #[test]
    fn test_coherency_three_models_chain() {
        // Three models in a chain: A--B--C, each within 2" of the next
        let models = vec![
            ModelPosition::new(Position::from_inches(10, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(12, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(14, 10), BaseSize::MM32),
        ];
        let result = check_coherency(&models);
        // Each model only needs 1 neighbor (< 7 models)
        assert!(result.is_coherent());
    }

    #[test]
    fn test_coherency_seven_plus_models_needs_two_neighbors() {
        // 7 models in a line, each 2" apart center-to-center
        // Edge distance: ~2" - 2*0.63" = ~0.74", which is within 2" coherency
        // But each model on the ends only has 1 neighbor
        let mut models = Vec::new();
        for i in 0..7 {
            models.push(ModelPosition::new(
                Position::new(
                    Inches::from_inches(10) + Inches::from_inches(2) * i,
                    Inches::from_inches(10),
                ),
                BaseSize::MM32,
            ));
        }
        let result = check_coherency(&models);
        // With 7+ models, each needs 2 neighbors within coherency range
        // End models (index 0 and 6) only have 1 neighbor each
        assert!(!result.is_coherent());
        match result {
            CoherencyResult::OutOfCoherency { violating_models } => {
                assert!(violating_models.contains(&0));
                assert!(violating_models.contains(&6));
            }
            _ => panic!("Expected OutOfCoherency"),
        }
    }

    #[test]
    fn test_coherency_seven_models_clustered() {
        // 7 models all clustered very close together (all within coherency of each other)
        let mut models = Vec::new();
        // Place in a tight grid pattern: 3x3 minus 2, all within 1" of each other
        let positions = [
            (10, 10),
            (11, 10),
            (12, 10),
            (10, 11),
            (11, 11),
            (12, 11),
            (11, 12),
        ];
        for &(x, y) in &positions {
            models.push(ModelPosition::new(
                Position::from_inches(x, y),
                BaseSize::MM32,
            ));
        }
        let result = check_coherency(&models);
        // All models should have multiple neighbors within coherency range
        assert!(result.is_coherent());
    }

    #[test]
    fn test_coherency_six_models_only_needs_one_neighbor() {
        // 6 models in a line with 2" center spacing
        // With < 7 models, each only needs 1 neighbor
        let mut models = Vec::new();
        for i in 0..6 {
            models.push(ModelPosition::new(
                Position::new(
                    Inches::from_inches(10) + Inches::from_inches(2) * i,
                    Inches::from_inches(10),
                ),
                BaseSize::MM32,
            ));
        }
        let result = check_coherency(&models);
        assert!(result.is_coherent());
    }

    // -----------------------------------------------------------------------
    // Deep Strike tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_deep_strike_position_no_enemies() {
        let result = valid_deep_strike_position(
            Position::from_inches(22, 15),
            BaseSize::MM32,
            &[],
        );
        assert!(result);
    }

    #[test]
    fn test_valid_deep_strike_position_far_from_enemy() {
        let enemies = vec![(Position::from_inches(5, 5), BaseSize::MM32)];
        let result = valid_deep_strike_position(
            Position::from_inches(22, 22),
            BaseSize::MM32,
            &enemies,
        );
        // Distance is well over 9"
        assert!(result);
    }

    #[test]
    fn test_invalid_deep_strike_position_too_close() {
        let enemies = vec![(Position::from_inches(22, 15), BaseSize::MM32)];
        let result = valid_deep_strike_position(
            Position::from_inches(22, 20),
            BaseSize::MM32,
            &enemies,
        );
        // 5" apart center-to-center, minus bases = ~3.74"
        // This is less than 9"
        assert!(!result);
    }

    #[test]
    fn test_deep_strike_legality() {
        let board = Board::combat_patrol();
        let enemies = vec![(Position::from_inches(22, 15), BaseSize::MM32)];
        let friendlies: Vec<(Position, BaseSize)> = vec![];

        // Valid position far from enemy
        let result = check_deep_strike_legality(
            &board,
            Position::from_inches(5, 5),
            BaseSize::MM32,
            &friendlies,
            &enemies,
        );
        assert_eq!(result, MovementLegality::Legal);

        // Invalid: too close to enemy
        let result = check_deep_strike_legality(
            &board,
            Position::from_inches(22, 20),
            BaseSize::MM32,
            &friendlies,
            &enemies,
        );
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::TooCloseToEnemy)
        );
    }

    // -----------------------------------------------------------------------
    // Objective marker tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_objective_marker_creation() {
        let obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "Center",
        );
        assert_eq!(obj.id, ObjectiveId::new(0));
        assert_eq!(obj.position, Position::from_inches(22, 15));
        assert_eq!(obj.label, "Center");
        assert_eq!(obj.control_status, ObjectiveControlStatus::Uncontrolled);
    }

    #[test]
    fn test_objective_marker_model_in_range() {
        let obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "Center",
        );
        // Model right on the objective
        assert!(obj.is_model_in_range_2d(Position::from_inches(22, 15), BaseSize::MM32));
        // Model 2" away (within 3" range)
        assert!(obj.is_model_in_range_2d(Position::from_inches(24, 15), BaseSize::MM32));
        // Model 10" away (outside 3" range)
        assert!(!obj.is_model_in_range_2d(Position::from_inches(32, 15), BaseSize::MM32));
    }

    #[test]
    fn test_objective_control_single_player() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "A",
        );
        obj.update_control(&[(PlayerId::new(0), 4)]);
        assert_eq!(
            obj.control_status,
            ObjectiveControlStatus::ControlledBy(PlayerId::new(0))
        );
    }

    #[test]
    fn test_objective_control_contested() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "A",
        );
        obj.update_control(&[(PlayerId::new(0), 4), (PlayerId::new(1), 4)]);
        assert_eq!(obj.control_status, ObjectiveControlStatus::Contested);
    }

    #[test]
    fn test_objective_control_higher_oc_wins() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "A",
        );
        obj.update_control(&[(PlayerId::new(0), 2), (PlayerId::new(1), 6)]);
        assert_eq!(
            obj.control_status,
            ObjectiveControlStatus::ControlledBy(PlayerId::new(1))
        );
    }

    #[test]
    fn test_objective_control_no_players() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "A",
        );
        obj.update_control(&[]);
        assert_eq!(obj.control_status, ObjectiveControlStatus::Uncontrolled);
    }

    #[test]
    fn test_objective_control_zero_oc() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "A",
        );
        // Both players have 0 OC (e.g., battle-shocked units)
        obj.update_control(&[(PlayerId::new(0), 0), (PlayerId::new(1), 0)]);
        assert_eq!(obj.control_status, ObjectiveControlStatus::Uncontrolled);
    }

    #[test]
    fn test_standard_objectives_layout() {
        let objectives = create_standard_objectives();
        assert_eq!(objectives.len(), 4);
        assert_eq!(objectives[0].label, "A");
        assert_eq!(objectives[1].label, "B");
        assert_eq!(objectives[2].label, "C");
        assert_eq!(objectives[3].label, "D");
    }

    #[test]
    fn test_center_objective_layout() {
        let objectives = create_center_objective();
        assert_eq!(objectives.len(), 1);
        assert_eq!(objectives[0].position, Position::from_inches(22, 15));
    }

    // -----------------------------------------------------------------------
    // Movement legality tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_valid_board_position() {
        let board = Board::combat_patrol();
        let result = is_valid_board_position(&board, Position::from_inches(22, 15), BaseSize::MM32);
        assert_eq!(result, MovementLegality::Legal);
    }

    #[test]
    fn test_invalid_board_position_out_of_bounds() {
        let board = Board::combat_patrol();
        let result = is_valid_board_position(&board, Position::from_inches(45, 15), BaseSize::MM32);
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::OutOfBounds)
        );
    }

    #[test]
    fn test_invalid_board_position_impassable_terrain() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Wall",
            Rect::from_inches(20, 14, 24, 16),
            TerrainProperties::wall(3),
        ));
        let result = is_valid_board_position(&board, Position::from_inches(22, 15), BaseSize::MM32);
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::OverlapsImpassableTerrain)
        );
    }

    #[test]
    fn test_deployment_legality() {
        let board = Board::combat_patrol();
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );

        // Valid deployment for attacker
        let result = check_deployment_legality(
            &board,
            &config.attacker_zone,
            Position::from_inches(22, 4),
            BaseSize::MM32,
            &[],
        );
        assert_eq!(result, MovementLegality::Legal);

        // Invalid: outside deployment zone
        let result = check_deployment_legality(
            &board,
            &config.attacker_zone,
            Position::from_inches(22, 15),
            BaseSize::MM32,
            &[],
        );
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::OutsideDeploymentZone)
        );
    }

    #[test]
    fn test_deployment_legality_model_overlap() {
        let board = Board::combat_patrol();
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );

        let other_models = vec![(Position::from_inches(22, 4), BaseSize::MM32)];
        // Place model right on top of another
        let result = check_deployment_legality(
            &board,
            &config.attacker_zone,
            Position::from_inches(22, 4),
            BaseSize::MM32,
            &other_models,
        );
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::OverlapsModel)
        );
    }

    #[test]
    fn test_move_legality_within_distance() {
        let board = Board::combat_patrol();
        let result = check_move_legality(
            &board,
            Position::from_inches(22, 10),
            Position::from_inches(22, 15),
            BaseSize::MM32,
            Inches::from_inches(6),
            &[],
        );
        // 5" move with 6" max distance
        assert_eq!(result, MovementLegality::Legal);
    }

    #[test]
    fn test_move_legality_exceeds_distance() {
        let board = Board::combat_patrol();
        let result = check_move_legality(
            &board,
            Position::from_inches(22, 10),
            Position::from_inches(22, 20),
            BaseSize::MM32,
            Inches::from_inches(6),
            &[],
        );
        // 10" move with 6" max distance
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::ExceedsMaxDistance)
        );
    }

    #[test]
    fn test_move_legality_out_of_bounds() {
        let board = Board::combat_patrol();
        let result = check_move_legality(
            &board,
            Position::from_inches(43, 15),
            Position::from_inches(45, 15),
            BaseSize::MM32,
            Inches::from_inches(6),
            &[],
        );
        assert_eq!(
            result,
            MovementLegality::Illegal(MovementIllegalReason::OutOfBounds)
        );
    }

    #[test]
    fn test_overlaps_any_model() {
        let others = vec![
            (Position::from_inches(10, 10), BaseSize::MM32),
            (Position::from_inches(20, 10), BaseSize::MM32),
        ];
        // Right on top of first model
        assert!(overlaps_any_model(
            Position::from_inches(10, 10),
            BaseSize::MM32,
            &others,
        ));
        // Far away from all models
        assert!(!overlaps_any_model(
            Position::from_inches(30, 20),
            BaseSize::MM32,
            &others,
        ));
    }

    // -----------------------------------------------------------------------
    // Terrain tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_terrain_properties_ruin() {
        let props = TerrainProperties::ruin(3);
        assert!(props.provides_cover);
        assert!(props.blocks_los);
        assert!(!props.impassable);
        assert_eq!(props.height, Inches::from_inches(3));
    }

    #[test]
    fn test_terrain_properties_wall() {
        let props = TerrainProperties::wall(5);
        assert!(props.provides_cover);
        assert!(props.blocks_los);
        assert!(props.impassable);
        assert_eq!(props.height, Inches::from_inches(5));
    }

    #[test]
    fn test_terrain_properties_light_cover() {
        let props = TerrainProperties::light_cover();
        assert!(props.provides_cover);
        assert!(!props.blocks_los);
        assert!(!props.impassable);
    }

    #[test]
    fn test_terrain_properties_dense_cover() {
        let props = TerrainProperties::dense_cover(4);
        assert!(props.provides_cover);
        assert!(props.blocks_los);
        assert!(!props.impassable);
        assert_eq!(props.height, Inches::from_inches(4));
    }

    #[test]
    fn test_terrain_properties_open() {
        let props = TerrainProperties::open();
        assert!(!props.provides_cover);
        assert!(!props.blocks_los);
        assert!(!props.impassable);
    }

    #[test]
    fn test_terrain_piece_contains() {
        let terrain = TerrainPiece::new(
            "Test",
            Rect::from_inches(10, 10, 20, 20),
            TerrainProperties::ruin(3),
        );
        assert!(terrain.contains(Position::from_inches(15, 15)));
        assert!(!terrain.contains(Position::from_inches(5, 5)));
    }

    #[test]
    fn test_terrain_piece_center() {
        let terrain = TerrainPiece::new(
            "Test",
            Rect::from_inches(10, 10, 20, 20),
            TerrainProperties::ruin(3),
        );
        assert_eq!(terrain.center(), Position::from_inches(15, 15));
    }

    // -----------------------------------------------------------------------
    // Line-rect intersection tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_line_through_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Horizontal line through the center
        assert!(line_rect_intersect(
            Position::from_inches(0, 15),
            Position::from_inches(30, 15),
            &rect,
        ));
    }

    #[test]
    fn test_line_misses_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Line above the rect
        assert!(!line_rect_intersect(
            Position::from_inches(0, 25),
            Position::from_inches(30, 25),
            &rect,
        ));
    }

    #[test]
    fn test_line_ends_before_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Line that ends before reaching the rect
        assert!(!line_rect_intersect(
            Position::from_inches(0, 15),
            Position::from_inches(5, 15),
            &rect,
        ));
    }

    #[test]
    fn test_line_starts_inside_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Line starts inside the rect
        assert!(line_rect_intersect(
            Position::from_inches(15, 15),
            Position::from_inches(30, 15),
            &rect,
        ));
    }

    #[test]
    fn test_diagonal_line_through_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Diagonal line through the rect
        assert!(line_rect_intersect(
            Position::from_inches(0, 0),
            Position::from_inches(30, 30),
            &rect,
        ));
    }

    #[test]
    fn test_diagonal_line_misses_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Diagonal that goes below the rect
        assert!(!line_rect_intersect(
            Position::from_inches(0, 0),
            Position::from_inches(30, 5),
            &rect,
        ));
    }

    #[test]
    fn test_vertical_line_through_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Vertical line through the rect
        assert!(line_rect_intersect(
            Position::from_inches(15, 0),
            Position::from_inches(15, 30),
            &rect,
        ));
    }

    #[test]
    fn test_vertical_line_misses_rect() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Vertical line to the left of the rect
        assert!(!line_rect_intersect(
            Position::from_inches(5, 0),
            Position::from_inches(5, 30),
            &rect,
        ));
    }

    // -----------------------------------------------------------------------
    // Rect-circle overlap tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_rect_circle_overlap_inside() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Circle center inside the rect
        assert!(rect_circle_overlap(
            &rect,
            Position::from_inches(15, 15),
            Inches::from_inches(1),
        ));
    }

    #[test]
    fn test_rect_circle_overlap_near_edge() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Circle center just outside but overlapping
        assert!(rect_circle_overlap(
            &rect,
            Position::from_inches(21, 15),
            Inches::from_inches(2),
        ));
    }

    #[test]
    fn test_rect_circle_no_overlap() {
        let rect = Rect::from_inches(10, 10, 20, 20);
        // Circle center far from rect
        assert!(!rect_circle_overlap(
            &rect,
            Position::from_inches(30, 30),
            Inches::from_inches(1),
        ));
    }

    // -----------------------------------------------------------------------
    // Serialization tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_board_serialization() {
        let mut board = Board::combat_patrol();
        board.add_terrain(TerrainPiece::new(
            "Ruin",
            Rect::from_inches(10, 10, 14, 14),
            TerrainProperties::ruin(3),
        ));
        board.add_objective(ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "Center",
        ));
        let json = serde_json::to_string(&board).unwrap();
        let back: Board = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dimensions, board.dimensions);
        assert_eq!(back.terrain.len(), 1);
        assert_eq!(back.objectives.len(), 1);
    }

    #[test]
    fn test_deployment_config_serialization() {
        let config = create_standard_deployment(
            &BoardDimensions::COMBAT_PATROL,
            PlayerId::new(0),
            PlayerId::new(1),
        );
        let json = serde_json::to_string(&config).unwrap();
        let back: DeploymentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.map_type, config.map_type);
    }

    #[test]
    fn test_terrain_piece_serialization() {
        let terrain = TerrainPiece::new(
            "Wall",
            Rect::from_inches(10, 10, 14, 14),
            TerrainProperties::wall(5),
        );
        let json = serde_json::to_string(&terrain).unwrap();
        let back: TerrainPiece = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Wall");
        assert_eq!(back.properties, terrain.properties);
    }

    #[test]
    fn test_objective_marker_serialization() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(1),
            Position::from_inches(22, 15),
            "A",
        );
        obj.update_control(&[(PlayerId::new(0), 4)]);
        let json = serde_json::to_string(&obj).unwrap();
        let back: ObjectiveMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, obj.id);
        assert_eq!(back.control_status, ObjectiveControlStatus::ControlledBy(PlayerId::new(0)));
    }

    #[test]
    fn test_coherency_result_serialization() {
        let result = CoherencyResult::OutOfCoherency {
            violating_models: vec![0, 6],
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: CoherencyResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, result);
    }

    #[test]
    fn test_movement_legality_serialization() {
        let legality = MovementLegality::Illegal(MovementIllegalReason::OutOfBounds);
        let json = serde_json::to_string(&legality).unwrap();
        let back: MovementLegality = serde_json::from_str(&json).unwrap();
        assert_eq!(back, legality);
    }

    // -----------------------------------------------------------------------
    // Integration-style tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_board_setup_and_deployment() {
        let mut board = Board::combat_patrol();

        // Add terrain
        board.add_terrain(TerrainPiece::new(
            "Central Ruin",
            Rect::from_inches(20, 13, 24, 17),
            TerrainProperties::ruin(3),
        ));
        board.add_terrain(TerrainPiece::new(
            "Left Barricade",
            Rect::from_inches(8, 14, 12, 16),
            TerrainProperties::light_cover(),
        ));
        board.add_terrain(TerrainPiece::new(
            "Right Wall",
            Rect::from_inches(32, 12, 36, 18),
            TerrainProperties::wall(5),
        ));

        // Add objectives
        for obj in create_standard_objectives() {
            board.add_objective(obj);
        }

        assert_eq!(board.terrain_pieces().len(), 3);
        assert_eq!(board.objective_markers().len(), 4);

        // Create deployment zones
        let config = create_standard_deployment(
            &board.dimensions,
            PlayerId::new(0),
            PlayerId::new(1),
        );

        // Deploy a model for the attacker
        let deploy_pos = Position::from_inches(22, 4);
        let deploy_result = check_deployment_legality(
            &board,
            &config.attacker_zone,
            deploy_pos,
            BaseSize::MM32,
            &[],
        );
        assert_eq!(deploy_result, MovementLegality::Legal);
    }

    #[test]
    fn test_full_visibility_scenario() {
        let mut board = Board::combat_patrol();

        // Ruin in the center blocking LOS
        board.add_terrain(TerrainPiece::new(
            "Central Ruin",
            Rect::from_inches(20, 13, 24, 17),
            TerrainProperties::ruin(3),
        ));

        // Barricade providing cover but not blocking LOS
        board.add_terrain(TerrainPiece::new(
            "Barricade",
            Rect::from_inches(14, 14, 16, 16),
            TerrainProperties {
                provides_cover: true,
                blocks_los: false,
                impassable: false,
                height: Inches::from_inches(1),
            },
        ));

        let shooter = Position::from_inches(5, 15);
        let target_behind_ruin = Position::from_inches(30, 15);
        let target_behind_barricade = Position::from_inches(18, 15);
        let target_in_open = Position::from_inches(5, 25);

        // Shooter to target behind ruin: blocked by ruin (blocks LOS)
        let vis1 = check_visibility(&board, shooter, target_behind_ruin);
        assert!(!vis1.is_visible());

        // Shooter to target behind barricade: visible with cover
        let vis2 = check_visibility(&board, shooter, target_behind_barricade);
        assert!(vis2.is_visible());
        assert!(vis2.has_cover());

        // Shooter to target in the open: visible, no cover
        let vis3 = check_visibility(&board, shooter, target_in_open);
        assert!(vis3.is_visible());
        assert!(!vis3.has_cover());
    }

    #[test]
    fn test_full_coherency_scenario() {
        // Simulate a unit of 3 Custodian Guard (32mm bases) in coherency
        let models = vec![
            ModelPosition::new(Position::from_inches(20, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(21, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(22, 10), BaseSize::MM32),
        ];
        assert!(check_coherency(&models).is_coherent());

        // Move one model out of coherency
        let models_broken = vec![
            ModelPosition::new(Position::from_inches(20, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(21, 10), BaseSize::MM32),
            ModelPosition::new(Position::from_inches(30, 10), BaseSize::MM32),
        ];
        assert!(!check_coherency(&models_broken).is_coherent());
    }

    #[test]
    fn test_objective_control_scenario() {
        let mut obj = ObjectiveMarker::new(
            ObjectiveId::new(0),
            Position::from_inches(22, 15),
            "Center",
        );

        // Player 0 has a Custodian Guard (OC 2) and Player 1 has Jakhals (OC 1 each, 2 models)
        // Player 0 OC total: 2, Player 1 OC total: 2 -> Contested
        obj.update_control(&[(PlayerId::new(0), 2), (PlayerId::new(1), 2)]);
        assert_eq!(obj.control_status, ObjectiveControlStatus::Contested);

        // Player 1 brings more models: total OC 3
        obj.update_control(&[(PlayerId::new(0), 2), (PlayerId::new(1), 3)]);
        assert_eq!(
            obj.control_status,
            ObjectiveControlStatus::ControlledBy(PlayerId::new(1))
        );

        // Player 0's models are destroyed
        obj.update_control(&[(PlayerId::new(1), 3)]);
        assert_eq!(
            obj.control_status,
            ObjectiveControlStatus::ControlledBy(PlayerId::new(1))
        );

        // All models removed
        obj.update_control(&[]);
        assert_eq!(obj.control_status, ObjectiveControlStatus::Uncontrolled);
    }

    #[test]
    fn test_board_objective_lookup() {
        let mut board = Board::combat_patrol();
        for obj in create_standard_objectives() {
            board.add_objective(obj);
        }

        let obj = board.objective_marker(ObjectiveId::new(2));
        assert!(obj.is_some());
        assert_eq!(obj.unwrap().label, "C");

        let obj = board.objective_marker(ObjectiveId::new(99));
        assert!(obj.is_none());
    }

    #[test]
    fn test_board_objective_mut_lookup() {
        let mut board = Board::combat_patrol();
        for obj in create_standard_objectives() {
            board.add_objective(obj);
        }

        let obj = board.objective_marker_mut(ObjectiveId::new(1));
        assert!(obj.is_some());
        let obj = obj.unwrap();
        obj.update_control(&[(PlayerId::new(0), 5)]);
        assert_eq!(
            obj.control_status,
            ObjectiveControlStatus::ControlledBy(PlayerId::new(0))
        );
    }

    #[test]
    fn test_engagement_range_exact_boundary() {
        // Place two 25mm base models such that edge distance is exactly 1"
        // Base radius: 25/2 = 12.5mm = ~492 mils
        let radius = BaseSize::MM25.radius_inches();
        // Center distance for 1" edge gap: 1" + 2*radius
        let center_dist = Inches::ENGAGEMENT_RANGE + radius + radius;
        let a = Position::from_inches(10, 10);
        let b = Position::new(Inches::from_inches(10) + center_dist, Inches::from_inches(10));
        // At exactly 1" edge distance, should be within engagement range (<=)
        assert!(within_engagement_range_2d(a, BaseSize::MM25, b, BaseSize::MM25));
    }

    #[test]
    fn test_coherency_range_exact_boundary() {
        // Place two 25mm base models such that edge distance is exactly 2"
        let radius = BaseSize::MM25.radius_inches();
        let center_dist = Inches::COHERENCY_RANGE + radius + radius;
        let a = Position::from_inches(10, 10);
        let b = Position::new(Inches::from_inches(10) + center_dist, Inches::from_inches(10));
        assert!(within_coherency_range_2d(a, BaseSize::MM25, b, BaseSize::MM25));
    }

    #[test]
    fn test_objective_range_exact_boundary() {
        // Place a 25mm base model such that edge distance to objective is exactly 3"
        let radius = BaseSize::MM25.radius_inches();
        let center_dist = Inches::OBJECTIVE_RANGE + radius;
        let model_pos = Position::from_inches(10, 10);
        let obj_pos = Position::new(
            Inches::from_inches(10) + center_dist,
            Inches::from_inches(10),
        );
        assert!(within_objective_range_2d(model_pos, BaseSize::MM25, obj_pos));
    }
}
