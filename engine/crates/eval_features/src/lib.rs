//! WH40K Engine - EvalFeatures crate
//!
//! Feature extraction from GameState for position evaluation.
//! Produces structured feature vectors consumed by heuristic and NNUE evaluators.
//!
//! # Feature Categories
//!
//! - **Global features**: battle round, phase, VP/CP differentials, reserves
//! - **Objective features**: per-objective control, holding strength, threat
//! - **Unit features**: per-unit health, threat, position, engagement status
//! - **Positional features**: distance buckets, charge ranges, LOS coverage
//! - **Matchup features**: anti-armor/anti-infantry relevance, attrition indicators
//!
//! Source: implementation_v3.md Section 12.4 (Feature classes)

use serde::{Deserialize, Serialize};

use wh40k_core_types::{
    Inches, ObjectiveId, Phase, PlayerId, Position, UnitId,
};
use wh40k_game_core::state::GameState;
use wh40k_game_core::unit::UnitState;
use wh40k_geometry::{distance, within_range};

// ============================================================================
// Score type used throughout evaluation
// ============================================================================

/// A signed evaluation score in centipawns-like units.
/// Positive values favor the perspective player, negative values favor opponent.
/// Scale: ~100 per projected VP differential point.
/// Special values: SCORE_WIN = 100_000, SCORE_LOSS = -100_000
pub type Score = i32;

/// Score indicating a definite win.
pub const SCORE_WIN: Score = 100_000;
/// Score indicating a definite loss.
pub const SCORE_LOSS: Score = -100_000;
/// Score indicating a draw / even position.
pub const SCORE_DRAW: Score = 0;
/// Score indicating the position is unknown (e.g., not yet evaluated).
pub const SCORE_NONE: Score = i32::MIN + 1;

// ============================================================================
// Distance bucket classification
// ============================================================================

/// Distance classification for positional features.
/// Buckets represent tactical distance ranges in 40K.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistanceBucket {
    /// Within engagement range (1")
    Engaged,
    /// Very close (1"-3") - pile-in / consolidation range
    MeleeReach,
    /// Close (3"-6") - short charge range
    ShortCharge,
    /// Medium (6"-9") - medium charge range
    MediumCharge,
    /// Charge threat (9"-12") - maximum charge declaration range
    LongCharge,
    /// Short shooting (12"-18")
    ShortRange,
    /// Medium shooting (18"-24")
    MediumRange,
    /// Long range (24"+)
    LongRange,
    /// Beyond effective range or not applicable
    OutOfRange,
}

impl DistanceBucket {
    /// Classify a distance in Inches into a bucket.
    pub fn from_distance(dist: Inches) -> Self {
        let mils = dist.mils();
        if mils <= 1_000 {
            DistanceBucket::Engaged
        } else if mils <= 3_000 {
            DistanceBucket::MeleeReach
        } else if mils <= 6_000 {
            DistanceBucket::ShortCharge
        } else if mils <= 9_000 {
            DistanceBucket::MediumCharge
        } else if mils <= 12_000 {
            DistanceBucket::LongCharge
        } else if mils <= 18_000 {
            DistanceBucket::ShortRange
        } else if mils <= 24_000 {
            DistanceBucket::MediumRange
        } else if mils <= 36_000 {
            DistanceBucket::LongRange
        } else {
            DistanceBucket::OutOfRange
        }
    }

    /// Returns a numeric index (0-8) for the bucket, useful for feature vectors.
    pub fn index(self) -> usize {
        match self {
            DistanceBucket::Engaged => 0,
            DistanceBucket::MeleeReach => 1,
            DistanceBucket::ShortCharge => 2,
            DistanceBucket::MediumCharge => 3,
            DistanceBucket::LongCharge => 4,
            DistanceBucket::ShortRange => 5,
            DistanceBucket::MediumRange => 6,
            DistanceBucket::LongRange => 7,
            DistanceBucket::OutOfRange => 8,
        }
    }
}

// ============================================================================
// Global Features
// ============================================================================

/// Global features about the overall game state.
/// These are not specific to any unit or objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalFeatures {
    /// Current battle round (1-5).
    pub battle_round: u8,
    /// Current phase encoded as integer.
    pub phase: u8,
    /// VP differential (perspective player VP - opponent VP).
    pub vp_differential: i16,
    /// CP differential (perspective player CP - opponent CP).
    pub cp_differential: i8,
    /// Total models remaining for perspective player.
    pub own_models_remaining: u16,
    /// Total models remaining for opponent.
    pub enemy_models_remaining: u16,
    /// Model count ratio (own / starting) as fixed-point [0, 1000].
    pub own_model_ratio: u16,
    /// Model count ratio (enemy / starting) as fixed-point [0, 1000].
    pub enemy_model_ratio: u16,
    /// Total wounds remaining for perspective player.
    pub own_wounds_remaining: u16,
    /// Total wounds remaining for opponent.
    pub enemy_wounds_remaining: u16,
    /// Number of own units in reserves.
    pub own_reserves_count: u8,
    /// Number of enemy units in reserves.
    pub enemy_reserves_count: u8,
    /// Number of own units that are battle-shocked.
    pub own_battle_shocked_count: u8,
    /// Number of enemy units that are battle-shocked.
    pub enemy_battle_shocked_count: u8,
    /// Number of objectives controlled by perspective player.
    pub own_objectives_controlled: u8,
    /// Number of objectives controlled by opponent.
    pub enemy_objectives_controlled: u8,
    /// Number of contested objectives.
    pub contested_objectives: u8,
    /// Total OC value for perspective player (sum of all models on board).
    pub own_total_oc: u16,
    /// Total OC value for opponent.
    pub enemy_total_oc: u16,
    /// Whether perspective player has first turn this round.
    pub is_first_player: bool,
    /// Game progress ratio [0, 1000] (round/5 * 1000).
    pub game_progress: u16,
    /// Number of own units on the battlefield.
    pub own_units_on_board: u8,
    /// Number of enemy units on the battlefield.
    pub enemy_units_on_board: u8,
}

// ============================================================================
// Objective Features
// ============================================================================

/// Features for a single objective marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveFeatures {
    /// Objective ID.
    pub objective_id: ObjectiveId,
    /// Position on the board.
    pub position: Position,
    /// Who controls this objective: +1 = own, -1 = enemy, 0 = contested/none.
    pub controller: i8,
    /// Total OC of own models within range.
    pub own_oc_in_range: u16,
    /// Total OC of enemy models within range.
    pub enemy_oc_in_range: u16,
    /// OC advantage (own - enemy), clamped.
    pub oc_advantage: i16,
    /// Whether this objective is secured by perspective player.
    pub is_secured_by_own: bool,
    /// Whether this objective is secured by opponent.
    pub is_secured_by_enemy: bool,
    /// Distance bucket to nearest own unit.
    pub nearest_own_unit_distance: DistanceBucket,
    /// Distance bucket to nearest enemy unit.
    pub nearest_enemy_unit_distance: DistanceBucket,
    /// Number of own units within 6" (charge/control range).
    pub own_units_within_6: u8,
    /// Number of enemy units within 6".
    pub enemy_units_within_6: u8,
    /// Whether objective is in No Man's Land.
    pub is_no_mans_land: bool,
    /// Whether objective is in own deployment zone.
    pub is_own_dz: bool,
    /// Whether objective is in enemy deployment zone.
    pub is_enemy_dz: bool,
    /// Projected VP value of this objective (mission-dependent).
    pub vp_value: u8,
}

// ============================================================================
// Unit Features
// ============================================================================

/// Features for a single unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitFeatures {
    /// Unit ID.
    pub unit_id: UnitId,
    /// Owner: true = perspective player, false = opponent.
    pub is_own: bool,
    /// Models remaining as ratio of starting [0, 1000].
    pub models_remaining_ratio: u16,
    /// Wounds remaining as ratio of starting [0, 1000].
    pub wounds_remaining_ratio: u16,
    /// Effective OC value of the unit.
    pub effective_oc: u8,
    /// Whether the unit is battle-shocked.
    pub is_battle_shocked: bool,
    /// Whether the unit is engaged in melee.
    pub is_engaged: bool,
    /// Whether the unit is in reserves.
    pub is_in_reserves: bool,
    /// Whether the unit is on the battlefield.
    pub is_on_battlefield: bool,
    /// Whether the unit is a CHARACTER.
    pub is_character: bool,
    /// Whether the unit is BATTLELINE.
    pub is_battleline: bool,
    /// Whether the unit is the WARLORD.
    pub is_warlord: bool,
    /// Whether the unit has an invulnerable save.
    pub has_invulnerable_save: bool,
    /// Unit's base toughness.
    pub toughness: u8,
    /// Unit's best save value (lower is better, 2-7).
    pub best_save: u8,
    /// Total melee attacks the unit can make.
    pub total_melee_attacks: u16,
    /// Total ranged attacks the unit can make.
    pub total_ranged_attacks: u16,
    /// Maximum ranged weapon range in inches.
    pub max_weapon_range: u16,
    /// Maximum melee weapon strength.
    pub max_melee_strength: u8,
    /// Maximum ranged weapon strength.
    pub max_ranged_strength: u8,
    /// Best AP value among weapons (most negative).
    pub best_ap: i8,
    /// Maximum damage per attack.
    pub max_damage: u8,
    /// Unit threat score (estimated damage output potential, 0-1000).
    pub threat_score: u16,
    /// Unit durability score (estimated survivability, 0-1000).
    pub durability_score: u16,
    /// Unit value score (combined threat + durability + keywords, 0-1000).
    pub value_score: u16,
    /// Distance bucket to nearest objective.
    pub nearest_objective_distance: DistanceBucket,
    /// Distance bucket to nearest enemy unit.
    pub nearest_enemy_distance: DistanceBucket,
    /// Distance bucket to nearest friendly unit.
    pub nearest_ally_distance: DistanceBucket,
    /// Whether this unit can potentially charge an enemy this turn.
    pub can_charge: bool,
    /// Number of enemies within shooting range.
    pub enemies_in_shooting_range: u8,
    /// Number of enemies within charge range (12").
    pub enemies_in_charge_range: u8,
    /// Whether the unit has moved this turn.
    pub has_moved: bool,
    /// Whether the unit has shot this turn.
    pub has_shot: bool,
    /// Whether the unit has fought this turn.
    pub has_fought: bool,
    /// Whether the unit charged this turn.
    pub charged_this_turn: bool,
    /// Whether the unit advanced this turn.
    pub advanced_this_turn: bool,
    /// Reference position (first alive model's position).
    pub position: Option<Position>,
}

// ============================================================================
// Complete Feature Set
// ============================================================================

/// Complete feature set extracted from a GameState for a given perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSet {
    /// Which player's perspective these features are extracted for.
    pub perspective: PlayerId,
    /// Global game features.
    pub global: GlobalFeatures,
    /// Per-objective features.
    pub objectives: Vec<ObjectiveFeatures>,
    /// Per-unit features (all units, both players).
    pub units: Vec<UnitFeatures>,
}

// ============================================================================
// NNUE Sparse Feature Types
// ============================================================================

/// A single sparse feature: (index, value) pair for NNUE input.
/// Only non-zero features are stored for efficient sparse-to-dense computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SparseFeature {
    /// Feature index in the flat feature space [0, TOTAL_FEATURES).
    pub index: u16,
    /// Feature value. Binary features = 1, scalar features normalized to [-127, 127].
    pub value: i16,
}

/// Sparse feature vector for NNUE input.
/// Contains only the active (non-zero) features from a game state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparseFeatureVec {
    /// Active features sorted by index.
    pub features: Vec<SparseFeature>,
}

impl SparseFeatureVec {
    /// Create a new empty sparse feature vector.
    pub fn new() -> Self {
        Self { features: Vec::new() }
    }

    /// Create with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self { features: Vec::with_capacity(cap) }
    }

    /// Add a feature. Only adds if value is non-zero.
    pub fn push(&mut self, index: u16, value: i16) {
        if value != 0 {
            self.features.push(SparseFeature { index, value });
        }
    }

    /// Add a binary feature (value = 1).
    pub fn push_binary(&mut self, index: u16) {
        self.features.push(SparseFeature { index, value: 1 });
    }

    /// Number of active features.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// Whether the vector has no active features.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Sort features by index for consistent ordering.
    pub fn sort(&mut self) {
        self.features.sort_by_key(|f| f.index);
    }

    /// Get a dense vector representation (fills zeros for missing features).
    pub fn to_dense(&self, total_features: usize) -> Vec<i16> {
        let mut dense = vec![0i16; total_features];
        for f in &self.features {
            if (f.index as usize) < total_features {
                dense[f.index as usize] = f.value;
            }
        }
        dense
    }
}

/// Feature diff representing changes between two sparse feature vectors.
/// Used for incremental NNUE accumulator updates during search.
#[derive(Debug, Clone, Default)]
pub struct FeatureDiff {
    /// Features that were added (not present in old, present in new).
    pub added: Vec<SparseFeature>,
    /// Features that were removed (present in old, not in new).
    pub removed: Vec<SparseFeature>,
    /// Features that changed value: (old, new).
    pub changed: Vec<(SparseFeature, SparseFeature)>,
}

impl FeatureDiff {
    /// Whether the diff is empty (no changes).
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Total number of feature changes.
    pub fn change_count(&self) -> usize {
        self.added.len() + self.removed.len() + self.changed.len()
    }
}

/// NNUE feature schema definition.
/// Tracks the layout and version of the feature space for model compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NnueFeatureSchema {
    /// Schema version (must match between model and extractor).
    pub version: u32,
    /// Total feature dimensions.
    pub total_features: usize,
    /// Global feature section offset.
    pub global_offset: usize,
    /// Global feature section size.
    pub global_size: usize,
    /// Objective feature section offset.
    pub objective_offset: usize,
    /// Features per objective.
    pub objective_feature_size: usize,
    /// Maximum number of objectives.
    pub max_objectives: usize,
    /// Unit feature section offset.
    pub unit_offset: usize,
    /// Features per unit.
    pub unit_feature_size: usize,
    /// Maximum number of units.
    pub max_units: usize,
}

impl NnueFeatureSchema {
    /// Get the current schema definition.
    pub fn current() -> Self {
        Self {
            version: FEATURE_SCHEMA_VERSION,
            total_features: TOTAL_FEATURES,
            global_offset: GLOBAL_FEAT_OFFSET as usize,
            global_size: GLOBAL_FEAT_SIZE as usize,
            objective_offset: OBJ_FEAT_OFFSET as usize,
            objective_feature_size: OBJ_FEAT_PER_OBJ as usize,
            max_objectives: NNUE_MAX_OBJECTIVES,
            unit_offset: UNIT_FEAT_OFFSET as usize,
            unit_feature_size: UNIT_FEAT_PER_UNIT as usize,
            max_units: NNUE_MAX_UNITS,
        }
    }

    /// Check compatibility with another schema.
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.version == other.version && self.total_features == other.total_features
    }
}

// ============================================================================
// Feature Layout Constants
// ============================================================================

/// Current feature schema version. Increment when layout changes.
pub const FEATURE_SCHEMA_VERSION: u32 = 1;

/// Offset to global feature section.
pub const GLOBAL_FEAT_OFFSET: u16 = 0;
/// Battle round one-hot encoding: 5 features for rounds 1-5.
pub const GLOBAL_ROUND_ONEHOT: u16 = 0;
/// Phase one-hot encoding: 7 features for phases 0-6.
pub const GLOBAL_PHASE_ONEHOT: u16 = 5;
/// Scalar global features start at index 12.
pub const GLOBAL_SCALAR_START: u16 = 12;
/// Total size of global feature section.
pub const GLOBAL_FEAT_SIZE: u16 = 31;

/// Offset to objective feature section.
pub const OBJ_FEAT_OFFSET: u16 = GLOBAL_FEAT_SIZE;
/// Features per objective.
pub const OBJ_FEAT_PER_OBJ: u16 = 30;
/// Maximum objectives tracked.
pub const NNUE_MAX_OBJECTIVES: usize = 6;
/// Total objective feature section size.
pub const OBJ_FEAT_TOTAL: u16 = OBJ_FEAT_PER_OBJ * NNUE_MAX_OBJECTIVES as u16;

/// Offset to unit feature section.
pub const UNIT_FEAT_OFFSET: u16 = OBJ_FEAT_OFFSET + OBJ_FEAT_TOTAL;
/// Features per unit.
pub const UNIT_FEAT_PER_UNIT: u16 = 62;
/// Maximum units tracked (8 per side).
pub const NNUE_MAX_UNITS: usize = 16;
/// Total unit feature section size.
pub const UNIT_FEAT_TOTAL: u16 = UNIT_FEAT_PER_UNIT * NNUE_MAX_UNITS as u16;

/// Total feature space dimensions.
pub const TOTAL_FEATURES: usize = (UNIT_FEAT_OFFSET + UNIT_FEAT_TOTAL) as usize;

/// Maximum value for normalized scalar features.
pub const SCALAR_NORM_MAX: i16 = 127;

// ============================================================================
// Feature Extraction
// ============================================================================

/// Extract a complete feature set from the given game state,
/// from the perspective of the specified player.
pub fn extract_features(state: &GameState, perspective: PlayerId) -> FeatureSet {
    let opponent = state.opponent_id(perspective);

    let global = extract_global_features(state, perspective, opponent);
    let objectives = extract_objective_features(state, perspective, opponent);
    let units = extract_unit_features(state, perspective, opponent);

    FeatureSet {
        perspective,
        global,
        objectives,
        units,
    }
}

/// Extract global features from the game state.
fn extract_global_features(
    state: &GameState,
    perspective: PlayerId,
    opponent: PlayerId,
) -> GlobalFeatures {
    let own_player = state.player(perspective);
    let enemy_player = state.player(opponent);

    let own_units: Vec<&UnitState> = state
        .units
        .iter()
        .filter(|u| u.owner == perspective)
        .collect();
    let enemy_units: Vec<&UnitState> = state
        .units
        .iter()
        .filter(|u| u.owner == opponent)
        .collect();

    let own_models_remaining: u16 = own_units
        .iter()
        .map(|u| u.models_alive() as u16)
        .sum();
    let enemy_models_remaining: u16 = enemy_units
        .iter()
        .map(|u| u.models_alive() as u16)
        .sum();

    let own_starting_models: u16 = own_units
        .iter()
        .map(|u| u.models.len() as u16)
        .sum();
    let enemy_starting_models: u16 = enemy_units
        .iter()
        .map(|u| u.models.len() as u16)
        .sum();

    let own_model_ratio = if own_starting_models > 0 {
        ((own_models_remaining as u32 * 1000) / own_starting_models as u32) as u16
    } else {
        0
    };
    let enemy_model_ratio = if enemy_starting_models > 0 {
        ((enemy_models_remaining as u32 * 1000) / enemy_starting_models as u32) as u16
    } else {
        0
    };

    let own_wounds_remaining: u16 = own_units
        .iter()
        .flat_map(|u| u.models.iter())
        .filter(|m| m.alive)
        .map(|m| m.wounds_remaining.value() as u16)
        .sum();
    let enemy_wounds_remaining: u16 = enemy_units
        .iter()
        .flat_map(|u| u.models.iter())
        .filter(|m| m.alive)
        .map(|m| m.wounds_remaining.value() as u16)
        .sum();

    let own_reserves_count = own_units
        .iter()
        .filter(|u| u.is_in_reserves())
        .count() as u8;
    let enemy_reserves_count = enemy_units
        .iter()
        .filter(|u| u.is_in_reserves())
        .count() as u8;

    let own_battle_shocked_count = own_units
        .iter()
        .filter(|u| u.battle_shocked && u.is_on_battlefield())
        .count() as u8;
    let enemy_battle_shocked_count = enemy_units
        .iter()
        .filter(|u| u.battle_shocked && u.is_on_battlefield())
        .count() as u8;

    // Count objective control
    let mut own_objectives_controlled: u8 = 0;
    let mut enemy_objectives_controlled: u8 = 0;
    let mut contested_objectives: u8 = 0;

    for obj in &state.board.objectives {
        let (own_oc, enemy_oc) =
            compute_objective_oc(state, obj.position, perspective, opponent);
        if own_oc > enemy_oc && own_oc > 0 {
            own_objectives_controlled += 1;
        } else if enemy_oc > own_oc && enemy_oc > 0 {
            enemy_objectives_controlled += 1;
        } else if own_oc > 0 && enemy_oc > 0 {
            contested_objectives += 1;
        }
    }

    let own_total_oc: u16 = own_units
        .iter()
        .filter(|u| u.is_on_battlefield())
        .map(|u| u.effective_oc().value() as u16)
        .sum();
    let enemy_total_oc: u16 = enemy_units
        .iter()
        .filter(|u| u.is_on_battlefield())
        .map(|u| u.effective_oc().value() as u16)
        .sum();

    let own_units_on_board = own_units
        .iter()
        .filter(|u| u.is_on_battlefield())
        .count() as u8;
    let enemy_units_on_board = enemy_units
        .iter()
        .filter(|u| u.is_on_battlefield())
        .count() as u8;

    let round_num = state.battle_round.number();
    let game_progress = ((round_num as u16).min(5) * 1000) / 5;

    GlobalFeatures {
        battle_round: round_num,
        phase: phase_to_u8(state.current_phase),
        vp_differential: own_player.vp.value() - enemy_player.vp.value(),
        cp_differential: (own_player.cp.value() - enemy_player.cp.value()) as i8,
        own_models_remaining,
        enemy_models_remaining,
        own_model_ratio,
        enemy_model_ratio,
        own_wounds_remaining,
        enemy_wounds_remaining,
        own_reserves_count,
        enemy_reserves_count,
        own_battle_shocked_count,
        enemy_battle_shocked_count,
        own_objectives_controlled,
        enemy_objectives_controlled,
        contested_objectives,
        own_total_oc,
        enemy_total_oc,
        is_first_player: own_player.first_turn,
        game_progress,
        own_units_on_board,
        enemy_units_on_board,
    }
}

/// Extract per-objective features.
fn extract_objective_features(
    state: &GameState,
    perspective: PlayerId,
    opponent: PlayerId,
) -> Vec<ObjectiveFeatures> {
    let board_height = state.board.dimensions.height.mils();
    let dz_depth = board_height / 4; // roughly 7.5" for a 30" board

    state
        .board
        .objectives
        .iter()
        .map(|obj| {
            let (own_oc, enemy_oc) =
                compute_objective_oc(state, obj.position, perspective, opponent);

            let controller = if own_oc > enemy_oc && own_oc > 0 {
                1i8
            } else if enemy_oc > own_oc && enemy_oc > 0 {
                -1i8
            } else {
                0i8
            };

            let oc_advantage = own_oc as i16 - enemy_oc as i16;

            // Find nearest own unit distance
            let nearest_own_dist = nearest_unit_distance(state, obj.position, perspective);
            let nearest_enemy_dist = nearest_unit_distance(state, obj.position, opponent);

            // Count units within 6"
            let own_units_within_6 = count_units_within_range(
                state,
                obj.position,
                perspective,
                Inches::from_inches(6),
            );
            let enemy_units_within_6 = count_units_within_range(
                state,
                obj.position,
                opponent,
                Inches::from_inches(6),
            );

            // Determine zone location relative to perspective player
            let obj_y = obj.position.y.mils();

            let (own_dz_start, own_dz_end, enemy_dz_start, enemy_dz_end) =
                if state.player(perspective).first_turn {
                    // Attacker (first player) typically deploys at bottom
                    (0, dz_depth, board_height - dz_depth, board_height)
                } else {
                    (board_height - dz_depth, board_height, 0, dz_depth)
                };

            let is_own_dz = obj_y >= own_dz_start && obj_y <= own_dz_end;
            let is_enemy_dz = obj_y >= enemy_dz_start && obj_y <= enemy_dz_end;
            let is_no_mans_land = !is_own_dz && !is_enemy_dz;

            // Check secured status
            let is_secured_by_own = obj.control_status == wh40k_geometry::ObjectiveControlStatus::ControlledBy(perspective);
            let is_secured_by_enemy = obj.control_status == wh40k_geometry::ObjectiveControlStatus::ControlledBy(opponent);

            // Default VP value (mission-dependent, default 5)
            let vp_value = 5u8;

            ObjectiveFeatures {
                objective_id: obj.id,
                position: obj.position,
                controller,
                own_oc_in_range: own_oc,
                enemy_oc_in_range: enemy_oc,
                oc_advantage,
                is_secured_by_own,
                is_secured_by_enemy,
                nearest_own_unit_distance: nearest_own_dist,
                nearest_enemy_unit_distance: nearest_enemy_dist,
                own_units_within_6,
                enemy_units_within_6,
                is_no_mans_land,
                is_own_dz,
                is_enemy_dz,
                vp_value,
            }
        })
        .collect()
}

/// Extract per-unit features for all non-destroyed units.
fn extract_unit_features(
    state: &GameState,
    perspective: PlayerId,
    opponent: PlayerId,
) -> Vec<UnitFeatures> {
    state
        .units
        .iter()
        .filter(|u| !u.is_destroyed())
        .map(|unit| {
            let is_own = unit.owner == perspective;
            let ref_pos = unit.reference_position();

            // Models remaining ratio
            let alive = unit.models_alive() as u32;
            let total = unit.models.len() as u32;
            let models_remaining_ratio = if total > 0 {
                ((alive * 1000) / total) as u16
            } else {
                0
            };

            // Wounds remaining ratio
            let wounds_alive: u32 = unit
                .models
                .iter()
                .filter(|m| m.alive)
                .map(|m| m.wounds_remaining.value() as u32)
                .sum();
            let wounds_max: u32 = unit
                .models
                .iter()
                .map(|m| m.wounds_max.value() as u32)
                .sum();
            let wounds_remaining_ratio = if wounds_max > 0 {
                ((wounds_alive * 1000) / wounds_max) as u16
            } else {
                0
            };

            // Weapon stats
            let (total_melee_attacks, total_ranged_attacks, max_weapon_range,
                 max_melee_strength, max_ranged_strength, best_ap, max_damage) =
                compute_weapon_stats(unit);

            // Threat score
            let threat_score = compute_threat_score(unit);

            // Durability score
            let durability_score = compute_durability_score(unit);

            // Value score (combined)
            let value_score = compute_value_score(unit, threat_score, durability_score);

            // Distance features
            let nearest_objective_distance = if let Some(pos) = ref_pos {
                nearest_objective_distance_bucket(state, pos)
            } else {
                DistanceBucket::OutOfRange
            };

            let nearest_enemy_distance = if let Some(pos) = ref_pos {
                let enemy_player = if is_own { opponent } else { perspective };
                nearest_unit_distance(state, pos, enemy_player)
            } else {
                DistanceBucket::OutOfRange
            };

            let nearest_ally_distance = if let Some(pos) = ref_pos {
                nearest_allied_unit_distance(state, pos, unit.owner, unit.id)
            } else {
                DistanceBucket::OutOfRange
            };

            // Charge eligibility
            let can_charge = unit.is_on_battlefield()
                && !unit.battle_shocked
                && matches!(
                    unit.engagement_status,
                    wh40k_core_types::EngagementStatus::NotEngaged
                )
                && matches!(
                    nearest_enemy_distance,
                    DistanceBucket::LongCharge
                        | DistanceBucket::MediumCharge
                        | DistanceBucket::ShortCharge
                        | DistanceBucket::MeleeReach
                );

            // Count enemies in range
            let (enemies_in_shooting_range, enemies_in_charge_range) =
                if let Some(pos) = ref_pos {
                    count_enemies_in_range(state, pos, unit.owner, max_weapon_range)
                } else {
                    (0, 0)
                };

            // Turn action flags
            let has_moved = state.turn_flags.units_moved.contains(&unit.id);
            let has_shot = state.turn_flags.units_shot.contains(&unit.id);
            let has_fought = state.turn_flags.units_fought.contains(&unit.id);
            let charged_this_turn = state.turn_flags.charged_this_turn.contains(&unit.id);
            let advanced_this_turn = state.turn_flags.advanced_this_turn.contains(&unit.id);

            // Check if warlord: first character for this player with enhancement
            let is_warlord = unit.is_character()
                && state.player(unit.owner).enhancement_choice.is_some();

            // Best save
            let base_save = unit.base_armor_save.value();
            let inv_save = unit
                .invulnerable_save
                .map(|inv| inv.value())
                .unwrap_or(7);
            let best_save = base_save.min(inv_save);

            UnitFeatures {
                unit_id: unit.id,
                is_own,
                models_remaining_ratio,
                wounds_remaining_ratio,
                effective_oc: unit.effective_oc().value(),
                is_battle_shocked: unit.battle_shocked,
                is_engaged: matches!(
                    unit.engagement_status,
                    wh40k_core_types::EngagementStatus::Engaged
                ),
                is_in_reserves: unit.is_in_reserves(),
                is_on_battlefield: unit.is_on_battlefield(),
                is_character: unit.is_character(),
                is_battleline: unit.is_battleline(),
                is_warlord,
                has_invulnerable_save: unit.invulnerable_save.is_some(),
                toughness: unit.base_toughness.value(),
                best_save,
                total_melee_attacks,
                total_ranged_attacks,
                max_weapon_range,
                max_melee_strength,
                max_ranged_strength,
                best_ap,
                max_damage,
                threat_score,
                durability_score,
                value_score,
                nearest_objective_distance,
                nearest_enemy_distance,
                nearest_ally_distance,
                can_charge,
                enemies_in_shooting_range,
                enemies_in_charge_range,
                has_moved,
                has_shot,
                has_fought,
                charged_this_turn,
                advanced_this_turn,
                position: ref_pos,
            }
        })
        .collect()
}

// ============================================================================
// Helper functions
// ============================================================================

/// Encode a Phase as a u8 for feature vectors.
fn phase_to_u8(phase: Phase) -> u8 {
    match phase {
        Phase::PreBattle => 0,
        Phase::Command => 1,
        Phase::Movement => 2,
        Phase::Shooting => 3,
        Phase::Charge => 4,
        Phase::Fight => 5,
        Phase::GameEnd => 6,
    }
}

/// Compute total OC of each player's models within objective range (3").
/// Returns (player_a_oc, player_b_oc).
fn compute_objective_oc(
    state: &GameState,
    objective_pos: Position,
    player_a: PlayerId,
    player_b: PlayerId,
) -> (u16, u16) {
    let obj_range = Inches::from_inches(3);
    let mut oc_a: u16 = 0;
    let mut oc_b: u16 = 0;

    for unit in &state.units {
        if !unit.is_on_battlefield() {
            continue;
        }
        let unit_oc = unit.effective_oc().value() as u16;
        if unit_oc == 0 {
            continue;
        }
        // Check if any alive model is within objective range
        let has_model_in_range = unit.models.iter().any(|m| {
            m.alive && within_range(m.position, objective_pos, obj_range)
        });
        if has_model_in_range {
            if unit.owner == player_a {
                oc_a += unit_oc;
            } else if unit.owner == player_b {
                oc_b += unit_oc;
            }
        }
    }
    (oc_a, oc_b)
}

/// Find the distance bucket to the nearest unit of a given player from a position.
fn nearest_unit_distance(
    state: &GameState,
    from: Position,
    player: PlayerId,
) -> DistanceBucket {
    let mut min_dist = i32::MAX;
    for unit in &state.units {
        if unit.owner != player || !unit.is_on_battlefield() {
            continue;
        }
        if let Some(pos) = unit.reference_position() {
            let d = distance(from, pos).mils();
            if d < min_dist {
                min_dist = d;
            }
        }
    }
    if min_dist == i32::MAX {
        DistanceBucket::OutOfRange
    } else {
        DistanceBucket::from_distance(Inches::from_mils(min_dist))
    }
}

/// Find the distance bucket to nearest allied unit (excluding self).
fn nearest_allied_unit_distance(
    state: &GameState,
    from: Position,
    player: PlayerId,
    exclude_unit: UnitId,
) -> DistanceBucket {
    let mut min_dist = i32::MAX;
    for unit in &state.units {
        if unit.owner != player || unit.id == exclude_unit || !unit.is_on_battlefield() {
            continue;
        }
        if let Some(pos) = unit.reference_position() {
            let d = distance(from, pos).mils();
            if d < min_dist {
                min_dist = d;
            }
        }
    }
    if min_dist == i32::MAX {
        DistanceBucket::OutOfRange
    } else {
        DistanceBucket::from_distance(Inches::from_mils(min_dist))
    }
}

/// Find the distance bucket to nearest objective from a position.
fn nearest_objective_distance_bucket(state: &GameState, from: Position) -> DistanceBucket {
    let mut min_dist = i32::MAX;
    for obj in &state.board.objectives {
        let d = distance(from, obj.position).mils();
        if d < min_dist {
            min_dist = d;
        }
    }
    if min_dist == i32::MAX {
        DistanceBucket::OutOfRange
    } else {
        DistanceBucket::from_distance(Inches::from_mils(min_dist))
    }
}

/// Count units of a player within a given range of a position.
fn count_units_within_range(
    state: &GameState,
    from: Position,
    player: PlayerId,
    range: Inches,
) -> u8 {
    let mut count = 0u8;
    for unit in &state.units {
        if unit.owner != player || !unit.is_on_battlefield() {
            continue;
        }
        if let Some(pos) = unit.reference_position() {
            if within_range(from, pos, range) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

/// Count enemies in shooting range and charge range from a position.
fn count_enemies_in_range(
    state: &GameState,
    from: Position,
    own_player: PlayerId,
    max_weapon_range_inches: u16,
) -> (u8, u8) {
    let shoot_range = Inches::from_inches(max_weapon_range_inches as i32);
    let charge_range = Inches::from_inches(12);
    let mut in_shooting = 0u8;
    let mut in_charge = 0u8;

    for unit in &state.units {
        if unit.owner == own_player || !unit.is_on_battlefield() {
            continue;
        }
        if let Some(pos) = unit.reference_position() {
            let d = distance(from, pos);
            if d.mils() <= charge_range.mils() {
                in_charge = in_charge.saturating_add(1);
            }
            if d.mils() <= shoot_range.mils() {
                in_shooting = in_shooting.saturating_add(1);
            }
        }
    }
    (in_shooting, in_charge)
}

/// Compute weapon statistics for a unit.
/// Returns (total_melee_attacks, total_ranged_attacks, max_range_inches,
///          max_melee_strength, max_ranged_strength, best_ap, max_damage).
fn compute_weapon_stats(
    unit: &UnitState,
) -> (u16, u16, u16, u8, u8, i8, u8) {
    let mut total_melee: u16 = 0;
    let mut total_ranged: u16 = 0;
    let mut max_range: u16 = 0;
    let mut max_melee_s: u8 = 0;
    let mut max_ranged_s: u8 = 0;
    let mut best_ap: i8 = 0;
    let mut max_dmg: u8 = 0;

    for model in &unit.models {
        if !model.alive {
            continue;
        }
        for wp in &model.melee_weapons {
            let attacks = avg_attack_count(&wp.attacks);
            total_melee += attacks;
            max_melee_s = max_melee_s.max(wp.strength.value());
            best_ap = best_ap.min(wp.ap.value());
            max_dmg = max_dmg.max(avg_damage(&wp.damage));
        }
        for wp in &model.ranged_weapons {
            let attacks = avg_attack_count(&wp.attacks);
            total_ranged += attacks;
            let range_val = (wp.range.mils() / 1000) as u16;
            max_range = max_range.max(range_val);
            max_ranged_s = max_ranged_s.max(wp.strength.value());
            best_ap = best_ap.min(wp.ap.value());
            max_dmg = max_dmg.max(avg_damage(&wp.damage));
        }
    }

    (total_melee, total_ranged, max_range, max_melee_s, max_ranged_s, best_ap, max_dmg)
}

/// Average attack count for probabilistic attack values.
fn avg_attack_count(attacks: &wh40k_core_types::AttackCount) -> u16 {
    match attacks {
        wh40k_core_types::AttackCount::Fixed(n) => *n as u16,
        wh40k_core_types::AttackCount::D6 => 3,
        wh40k_core_types::AttackCount::D3 => 2,
        wh40k_core_types::AttackCount::D6Plus(n) => 3 + *n as u16,
        wh40k_core_types::AttackCount::D3Plus(n) => 2 + *n as u16,
    }
}

/// Average damage for probabilistic damage values.
fn avg_damage(damage: &wh40k_core_types::Damage) -> u8 {
    match damage {
        wh40k_core_types::Damage::Fixed(n) => *n,
        wh40k_core_types::Damage::D3 => 2,
        wh40k_core_types::Damage::D6 => 3,
        wh40k_core_types::Damage::D3Plus(n) => 2 + n,
        wh40k_core_types::Damage::D6Plus(n) => 3 + n,
    }
}

/// Compute a threat score for a unit (0-1000).
/// Higher = more dangerous offensively.
pub fn compute_threat_score(unit: &UnitState) -> u16 {
    if unit.is_destroyed() || !unit.is_on_battlefield() {
        return 0;
    }

    let mut score: u32 = 0;

    for model in &unit.models {
        if !model.alive {
            continue;
        }

        // Melee threat (primary in 40K Combat Patrol)
        for wp in &model.melee_weapons {
            let attacks = avg_attack_count(&wp.attacks) as u32;
            let skill_factor = (7u32).saturating_sub(wp.skill.value().min(6) as u32);
            let strength_factor = wp.strength.value().min(12) as u32;
            let ap_factor = (wp.ap.value().unsigned_abs() as u32).min(4) + 1;
            let dmg = avg_damage(&wp.damage) as u32;

            // Weapon ability bonuses (multiplier out of 10)
            let mut ability_mult = 10u32;
            for ability in &wp.abilities.abilities {
                match ability {
                    wh40k_core_types::WeaponAbility::LethalHits => ability_mult += 2,
                    wh40k_core_types::WeaponAbility::SustainedHits(_) => ability_mult += 2,
                    wh40k_core_types::WeaponAbility::DevastatingWounds => ability_mult += 3,
                    wh40k_core_types::WeaponAbility::Precision => ability_mult += 1,
                    wh40k_core_types::WeaponAbility::TwinLinked => ability_mult += 1,
                    _ => {}
                }
            }
            score += (attacks * skill_factor * strength_factor * ap_factor * dmg * ability_mult)
                / 100;
        }

        // Ranged threat (weighted 60% of melee)
        for wp in &model.ranged_weapons {
            let attacks = avg_attack_count(&wp.attacks) as u32;
            let skill_factor = (7u32).saturating_sub(wp.skill.value().min(6) as u32);
            let strength_factor = wp.strength.value().min(12) as u32;
            let ap_factor = (wp.ap.value().unsigned_abs() as u32).min(4) + 1;
            let dmg = avg_damage(&wp.damage) as u32;

            score += (attacks * skill_factor * strength_factor * ap_factor * dmg * 6) / 100;
        }
    }

    // CHARACTER bonus (high-value unit)
    if unit.is_character() {
        score = score * 12 / 10;
    }

    (score.min(1000)) as u16
}

/// Compute a durability score for a unit (0-1000).
/// Higher = harder to kill.
pub fn compute_durability_score(unit: &UnitState) -> u16 {
    if unit.is_destroyed() || !unit.is_on_battlefield() {
        return 0;
    }

    let mut score: u32 = 0;

    let toughness = unit.base_toughness.value() as u32;
    let save = unit.base_armor_save.value() as u32;
    let inv = unit
        .invulnerable_save
        .map(|i| i.value() as u32)
        .unwrap_or(7);
    let best_sv = save.min(inv);

    // Save factor: 2+ = 5, 3+ = 4, 4+ = 3, 5+ = 2, 6+ = 1, 7+ = 0
    let save_factor = if best_sv <= 6 { 7u32.saturating_sub(best_sv) } else { 0 };

    for model in &unit.models {
        if !model.alive {
            continue;
        }
        let wounds = model.wounds_remaining.value() as u32;
        let fnp_factor = model
            .feel_no_pain
            .map(|fnp| {
                let fnp_val = fnp.value() as u32;
                if fnp_val <= 6 { (7 - fnp_val) * 10 / 6 } else { 0 }
            })
            .unwrap_or(0);

        let model_durability = wounds * toughness * save_factor + wounds * fnp_factor;
        score += model_durability;
    }

    // Invulnerable save bonus
    if inv <= 5 {
        score = score * (12 + (6 - inv)) / 12;
    }

    (score.min(1000)) as u16
}

/// Compute a combined value score for a unit (0-1000).
pub fn compute_value_score(unit: &UnitState, threat: u16, durability: u16) -> u16 {
    if unit.is_destroyed() {
        return 0;
    }

    let mut value: u32 = 0;

    // Combine threat and durability (weighted sum)
    value += (threat as u32 * 40 + durability as u32 * 30) / 70;

    // OC bonus (important for scoring)
    let oc = unit.effective_oc().value() as u32;
    value += oc * 20;

    // CHARACTER value bonus
    if unit.is_character() {
        value += 100;
    }

    // BATTLELINE bonus (can secure objectives)
    if unit.is_battleline() {
        value += 50;
    }

    (value.min(1000)) as u16
}

// ============================================================================
// Sparse Feature Extraction for NNUE
// ============================================================================

/// Extract sparse features from a GameState for NNUE evaluation.
/// Combines feature extraction and sparse conversion in one step.
pub fn extract_sparse_features_from_state(
    state: &GameState,
    perspective: PlayerId,
) -> SparseFeatureVec {
    let features = extract_features(state, perspective);
    extract_sparse_features(&features)
}

/// Convert a FeatureSet into a sparse feature vector for NNUE input.
pub fn extract_sparse_features(features: &FeatureSet) -> SparseFeatureVec {
    let mut sparse = SparseFeatureVec::with_capacity(600);
    sparse_from_global(&features.global, &mut sparse);
    sparse_from_objectives(&features.objectives, &mut sparse);
    sparse_from_units(&features.units, &mut sparse);
    sparse.sort();
    sparse
}

/// Compute the difference between two sparse feature vectors.
/// Used for incremental NNUE accumulator updates during search.
pub fn compute_feature_diff(
    old: &SparseFeatureVec,
    new: &SparseFeatureVec,
) -> FeatureDiff {
    let mut diff = FeatureDiff::default();
    let mut old_map: std::collections::HashMap<u16, i16> =
        old.features.iter().map(|f| (f.index, f.value)).collect();

    for new_feat in &new.features {
        if let Some(&old_val) = old_map.get(&new_feat.index) {
            if old_val != new_feat.value {
                diff.changed.push((
                    SparseFeature { index: new_feat.index, value: old_val },
                    *new_feat,
                ));
            }
            old_map.remove(&new_feat.index);
        } else {
            diff.added.push(*new_feat);
        }
    }

    for (&idx, &val) in &old_map {
        diff.removed.push(SparseFeature { index: idx, value: val });
    }

    diff
}

/// Normalize a ratio (0-1000) to scalar range [0, 127].
fn normalize_ratio(ratio: u16) -> i16 {
    (ratio as i32 * SCALAR_NORM_MAX as i32 / 1000).min(SCALAR_NORM_MAX as i32) as i16
}

/// Normalize a signed differential to scalar range [-127, 127].
fn normalize_differential(val: i16, max_range: i16) -> i16 {
    if max_range == 0 { return 0; }
    (val as i32 * SCALAR_NORM_MAX as i32 / max_range as i32)
        .clamp(-(SCALAR_NORM_MAX as i32), SCALAR_NORM_MAX as i32) as i16
}

/// Normalize a count to scalar range [0, 127].
fn normalize_count(count: u16, max_count: u16) -> i16 {
    if max_count == 0 { return 0; }
    (count as i32 * SCALAR_NORM_MAX as i32 / max_count as i32).min(SCALAR_NORM_MAX as i32) as i16
}

/// Extract sparse features from GlobalFeatures.
fn sparse_from_global(global: &GlobalFeatures, sparse: &mut SparseFeatureVec) {
    let round_idx = global.battle_round.saturating_sub(1).min(4) as u16;
    sparse.push_binary(GLOBAL_ROUND_ONEHOT + round_idx);

    let phase_idx = global.phase.min(6) as u16;
    sparse.push_binary(GLOBAL_PHASE_ONEHOT + phase_idx);

    let base = GLOBAL_SCALAR_START;
    sparse.push(base, normalize_differential(global.vp_differential, 45));
    sparse.push(base + 1, normalize_differential(global.cp_differential as i16, 10));
    sparse.push(base + 2, normalize_ratio(global.own_model_ratio));
    sparse.push(base + 3, normalize_ratio(global.enemy_model_ratio));
    sparse.push(base + 4, normalize_count(global.own_wounds_remaining, 200));
    sparse.push(base + 5, normalize_count(global.enemy_wounds_remaining, 200));
    sparse.push(base + 6, normalize_count(global.own_reserves_count as u16, 8));
    sparse.push(base + 7, normalize_count(global.enemy_reserves_count as u16, 8));
    sparse.push(base + 8, normalize_count(global.own_battle_shocked_count as u16, 8));
    sparse.push(base + 9, normalize_count(global.enemy_battle_shocked_count as u16, 8));
    sparse.push(base + 10, normalize_count(global.own_objectives_controlled as u16, 6));
    sparse.push(base + 11, normalize_count(global.enemy_objectives_controlled as u16, 6));
    sparse.push(base + 12, normalize_count(global.contested_objectives as u16, 6));
    sparse.push(base + 13, normalize_count(global.own_total_oc, 50));
    sparse.push(base + 14, normalize_count(global.enemy_total_oc, 50));
    if global.is_first_player {
        sparse.push_binary(base + 15);
    }
    sparse.push(base + 16, normalize_ratio(global.game_progress));
    sparse.push(base + 17, normalize_count(global.own_units_on_board as u16, 8));
    sparse.push(base + 18, normalize_count(global.enemy_units_on_board as u16, 8));
}

/// Extract sparse features from ObjectiveFeatures.
fn sparse_from_objectives(objectives: &[ObjectiveFeatures], sparse: &mut SparseFeatureVec) {
    for (i, obj) in objectives.iter().enumerate() {
        if i >= NNUE_MAX_OBJECTIVES { break; }
        let base = OBJ_FEAT_OFFSET + (i as u16) * OBJ_FEAT_PER_OBJ;

        sparse.push(base, obj.controller as i16 * SCALAR_NORM_MAX);
        sparse.push(base + 1, normalize_count(obj.own_oc_in_range, 20));
        sparse.push(base + 2, normalize_count(obj.enemy_oc_in_range, 20));
        sparse.push(base + 3, normalize_differential(obj.oc_advantage, 20));
        if obj.is_secured_by_own { sparse.push_binary(base + 4); }
        if obj.is_secured_by_enemy { sparse.push_binary(base + 5); }

        sparse.push_binary(base + 6 + obj.nearest_own_unit_distance.index() as u16);
        sparse.push_binary(base + 15 + obj.nearest_enemy_unit_distance.index() as u16);

        sparse.push(base + 24, normalize_count(obj.own_units_within_6 as u16, 8));
        sparse.push(base + 25, normalize_count(obj.enemy_units_within_6 as u16, 8));
        if obj.is_no_mans_land { sparse.push_binary(base + 26); }
        if obj.is_own_dz { sparse.push_binary(base + 27); }
        if obj.is_enemy_dz { sparse.push_binary(base + 28); }
        sparse.push(base + 29, obj.vp_value as i16);
    }
}

/// Extract sparse features from UnitFeatures.
fn sparse_from_units(units: &[UnitFeatures], sparse: &mut SparseFeatureVec) {
    for (i, unit) in units.iter().enumerate() {
        if i >= NNUE_MAX_UNITS { break; }
        let base = UNIT_FEAT_OFFSET + (i as u16) * UNIT_FEAT_PER_UNIT;

        if unit.is_own { sparse.push_binary(base); }
        sparse.push(base + 1, normalize_ratio(unit.models_remaining_ratio));
        sparse.push(base + 2, normalize_ratio(unit.wounds_remaining_ratio));
        sparse.push(base + 3, normalize_count(unit.effective_oc as u16, 10));
        if unit.is_battle_shocked { sparse.push_binary(base + 4); }
        if unit.is_engaged { sparse.push_binary(base + 5); }
        if unit.is_in_reserves { sparse.push_binary(base + 6); }
        if unit.is_on_battlefield { sparse.push_binary(base + 7); }
        if unit.is_character { sparse.push_binary(base + 8); }
        if unit.is_battleline { sparse.push_binary(base + 9); }
        if unit.is_warlord { sparse.push_binary(base + 10); }
        if unit.has_invulnerable_save { sparse.push_binary(base + 11); }

        sparse.push(base + 12, normalize_count(unit.toughness as u16, 12));
        sparse.push(base + 13, normalize_count(unit.best_save as u16, 7));
        sparse.push(base + 14, normalize_count(unit.total_melee_attacks, 50));
        sparse.push(base + 15, normalize_count(unit.total_ranged_attacks, 50));
        sparse.push(base + 16, normalize_count(unit.max_weapon_range, 48));
        sparse.push(base + 17, normalize_count(unit.max_melee_strength as u16, 12));
        sparse.push(base + 18, normalize_count(unit.max_ranged_strength as u16, 12));
        sparse.push(base + 19, normalize_differential(unit.best_ap as i16, 4));
        sparse.push(base + 20, normalize_count(unit.max_damage as u16, 6));
        sparse.push(base + 21, normalize_count(unit.threat_score, 1000));
        sparse.push(base + 22, normalize_count(unit.durability_score, 1000));
        sparse.push(base + 23, normalize_count(unit.value_score, 1000));

        sparse.push_binary(base + 24 + unit.nearest_objective_distance.index() as u16);
        sparse.push_binary(base + 33 + unit.nearest_enemy_distance.index() as u16);
        sparse.push_binary(base + 42 + unit.nearest_ally_distance.index() as u16);

        if unit.can_charge { sparse.push_binary(base + 51); }
        sparse.push(base + 52, normalize_count(unit.enemies_in_shooting_range as u16, 8));
        sparse.push(base + 53, normalize_count(unit.enemies_in_charge_range as u16, 8));
        if unit.has_moved { sparse.push_binary(base + 54); }
        if unit.has_shot { sparse.push_binary(base + 55); }
        if unit.has_fought { sparse.push_binary(base + 56); }
        if unit.charged_this_turn { sparse.push_binary(base + 57); }
        if unit.advanced_this_turn { sparse.push_binary(base + 58); }

        sparse.push(base + 59, compute_anti_armor_relevance(unit));
        sparse.push(base + 60, compute_anti_infantry_relevance(unit));
        sparse.push(base + 61, compute_melee_threat_matchup(unit));
    }
}

/// Compute anti-armor relevance for a unit (0-127).
fn compute_anti_armor_relevance(unit: &UnitFeatures) -> i16 {
    if !unit.is_on_battlefield { return 0; }
    let mut score: i32 = 0;
    if unit.max_ranged_strength >= 8 { score += 40; }
    else if unit.max_ranged_strength >= 6 { score += 25; }
    if unit.max_melee_strength >= 8 { score += 35; }
    else if unit.max_melee_strength >= 6 { score += 20; }
    if unit.best_ap <= -3 { score += 30; }
    else if unit.best_ap <= -2 { score += 20; }
    else if unit.best_ap <= -1 { score += 10; }
    if unit.max_damage >= 3 { score += 20; }
    else if unit.max_damage >= 2 { score += 10; }
    score.min(SCALAR_NORM_MAX as i32) as i16
}

/// Compute anti-infantry relevance for a unit (0-127).
fn compute_anti_infantry_relevance(unit: &UnitFeatures) -> i16 {
    if !unit.is_on_battlefield { return 0; }
    let mut score: i32 = 0;
    if unit.total_ranged_attacks >= 10 { score += 40; }
    else if unit.total_ranged_attacks >= 5 { score += 25; }
    if unit.total_melee_attacks >= 8 { score += 30; }
    else if unit.total_melee_attacks >= 4 { score += 18; }
    if unit.best_ap <= -1 { score += 20; }
    if unit.max_ranged_strength >= 4 { score += 15; }
    if unit.max_melee_strength >= 4 { score += 15; }
    if unit.models_remaining_ratio > 750 { score += 10; }
    score.min(SCALAR_NORM_MAX as i32) as i16
}

/// Compute melee threat matchup score (0-127).
fn compute_melee_threat_matchup(unit: &UnitFeatures) -> i16 {
    if !unit.is_on_battlefield || unit.total_melee_attacks == 0 { return 0; }
    let mut score: i32 = 0;
    score += (unit.total_melee_attacks as i32).min(30) * 2;
    if unit.max_melee_strength >= 8 { score += 25; }
    else if unit.max_melee_strength >= 6 { score += 15; }
    else if unit.max_melee_strength >= 4 { score += 8; }
    if unit.can_charge && unit.enemies_in_charge_range > 0 { score += 20; }
    if unit.is_engaged { score += 15; }
    if unit.charged_this_turn { score += 10; }
    score.min(SCALAR_NORM_MAX as i32) as i16
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_bucket_classification() {
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(500)),
            DistanceBucket::Engaged
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(1000)),
            DistanceBucket::Engaged
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(1500)),
            DistanceBucket::MeleeReach
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(3000)),
            DistanceBucket::MeleeReach
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(5000)),
            DistanceBucket::ShortCharge
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(8000)),
            DistanceBucket::MediumCharge
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(11000)),
            DistanceBucket::LongCharge
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(15000)),
            DistanceBucket::ShortRange
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(22000)),
            DistanceBucket::MediumRange
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(30000)),
            DistanceBucket::LongRange
        );
        assert_eq!(
            DistanceBucket::from_distance(Inches::from_mils(50000)),
            DistanceBucket::OutOfRange
        );
    }

    #[test]
    fn test_distance_bucket_indices() {
        assert_eq!(DistanceBucket::Engaged.index(), 0);
        assert_eq!(DistanceBucket::MeleeReach.index(), 1);
        assert_eq!(DistanceBucket::ShortCharge.index(), 2);
        assert_eq!(DistanceBucket::MediumCharge.index(), 3);
        assert_eq!(DistanceBucket::LongCharge.index(), 4);
        assert_eq!(DistanceBucket::ShortRange.index(), 5);
        assert_eq!(DistanceBucket::MediumRange.index(), 6);
        assert_eq!(DistanceBucket::LongRange.index(), 7);
        assert_eq!(DistanceBucket::OutOfRange.index(), 8);
    }

    #[test]
    fn test_phase_encoding() {
        assert_eq!(phase_to_u8(Phase::PreBattle), 0);
        assert_eq!(phase_to_u8(Phase::Command), 1);
        assert_eq!(phase_to_u8(Phase::Movement), 2);
        assert_eq!(phase_to_u8(Phase::Shooting), 3);
        assert_eq!(phase_to_u8(Phase::Charge), 4);
        assert_eq!(phase_to_u8(Phase::Fight), 5);
        assert_eq!(phase_to_u8(Phase::GameEnd), 6);
    }

    #[test]
    fn test_score_constants() {
        assert!(SCORE_WIN > 0);
        assert!(SCORE_LOSS < 0);
        assert_eq!(SCORE_DRAW, 0);
        assert!(SCORE_WIN > SCORE_DRAW);
        assert!(SCORE_LOSS < SCORE_DRAW);
        assert!(SCORE_NONE < SCORE_LOSS);
    }

    #[test]
    fn test_avg_attack_count() {
        assert_eq!(avg_attack_count(&wh40k_core_types::AttackCount::Fixed(5)), 5);
        assert_eq!(avg_attack_count(&wh40k_core_types::AttackCount::D6), 3);
        assert_eq!(avg_attack_count(&wh40k_core_types::AttackCount::D3), 2);
        assert_eq!(avg_attack_count(&wh40k_core_types::AttackCount::D6Plus(2)), 5);
        assert_eq!(avg_attack_count(&wh40k_core_types::AttackCount::D3Plus(1)), 3);
    }

    #[test]
    fn test_avg_damage() {
        assert_eq!(avg_damage(&wh40k_core_types::Damage::Fixed(3)), 3);
        assert_eq!(avg_damage(&wh40k_core_types::Damage::D3), 2);
        assert_eq!(avg_damage(&wh40k_core_types::Damage::D6), 3);
        assert_eq!(avg_damage(&wh40k_core_types::Damage::D3Plus(1)), 3);
        assert_eq!(avg_damage(&wh40k_core_types::Damage::D6Plus(2)), 5);
    }

    // ========================================================================
    // NNUE Sparse Feature Tests
    // ========================================================================

    #[test]
    fn test_sparse_feature_vec_basic() {
        let mut sparse = SparseFeatureVec::new();
        assert!(sparse.is_empty());
        assert_eq!(sparse.len(), 0);

        sparse.push_binary(5);
        sparse.push(10, 42);
        sparse.push(15, 0); // should NOT be added (zero value)
        assert_eq!(sparse.len(), 2);
        assert!(!sparse.is_empty());

        assert_eq!(sparse.features[0], SparseFeature { index: 5, value: 1 });
        assert_eq!(sparse.features[1], SparseFeature { index: 10, value: 42 });
    }

    #[test]
    fn test_sparse_feature_vec_to_dense() {
        let mut sparse = SparseFeatureVec::new();
        sparse.push_binary(0);
        sparse.push(3, 50);
        sparse.push(7, -30);

        let dense = sparse.to_dense(10);
        assert_eq!(dense.len(), 10);
        assert_eq!(dense[0], 1);
        assert_eq!(dense[1], 0);
        assert_eq!(dense[3], 50);
        assert_eq!(dense[7], -30);
    }

    #[test]
    fn test_feature_diff_computation() {
        let mut old = SparseFeatureVec::new();
        old.push_binary(1);
        old.push(5, 100);
        old.push(10, 50);

        let mut new_feat = SparseFeatureVec::new();
        new_feat.push_binary(1);
        new_feat.push(5, 80);
        new_feat.push(20, 30);

        let diff = compute_feature_diff(&old, &new_feat);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].index, 20);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].index, 10);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].0.value, 100);
        assert_eq!(diff.changed[0].1.value, 80);
    }

    #[test]
    fn test_feature_schema_current() {
        let schema = NnueFeatureSchema::current();
        assert_eq!(schema.version, FEATURE_SCHEMA_VERSION);
        assert_eq!(schema.total_features, TOTAL_FEATURES);
        assert_eq!(schema.total_features, 1203);
        assert!(schema.is_compatible(&NnueFeatureSchema::current()));
    }

    #[test]
    fn test_normalize_functions() {
        assert_eq!(normalize_ratio(0), 0);
        assert_eq!(normalize_ratio(500), 63);
        assert_eq!(normalize_ratio(1000), 127);

        assert_eq!(normalize_differential(0, 45), 0);
        assert_eq!(normalize_differential(45, 45), 127);
        assert_eq!(normalize_differential(-45, 45), -127);

        assert_eq!(normalize_count(0, 8), 0);
        assert_eq!(normalize_count(8, 8), 127);
    }

    #[test]
    fn test_total_features_constant() {
        assert_eq!(GLOBAL_FEAT_SIZE, 31);
        assert_eq!(OBJ_FEAT_TOTAL, 180);
        assert_eq!(UNIT_FEAT_TOTAL, 992);
        assert_eq!(TOTAL_FEATURES, 1203);
        assert_eq!(UNIT_FEAT_OFFSET, 211);
        assert_eq!(OBJ_FEAT_OFFSET, 31);
    }
}
