//! Purpose-built JSON-serializable view model structs for the TypeScript GUI.
//!
//! These structs project engine-internal types into a flat, string-friendly
//! format suitable for crossing the WASM boundary as JSON strings.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Top-level game snapshot
// ---------------------------------------------------------------------------

/// Complete game state projection for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameView {
    /// Current battle round number (1-5).
    pub battle_round: u8,
    /// Current phase name (e.g. "Command", "Movement").
    pub phase: String,
    /// Current sub-phase name.
    pub subphase: String,
    /// Index of the active player (0 or 1).
    pub active_player: u32,
    /// Index of the player who must make the next decision.
    pub decision_owner: u32,
    /// Both players.
    pub players: [PlayerView; 2],
    /// All units (alive, destroyed, in reserves, undeployed).
    pub units: Vec<UnitView>,
    /// Board layout (dimensions, terrain, objectives, deployment zones).
    pub board: BoardView,
    /// Recent event log entries.
    pub events: Vec<EventView>,
    /// Game outcome (in_progress, victory, draw).
    pub outcome: GameOutcomeView,
    /// Whether the game is still in progress.
    pub in_progress: bool,
    /// Content version for replay verification.
    pub content_version: String,
    /// Scenario / mission id if set.
    pub scenario_id: Option<u32>,
}

// ---------------------------------------------------------------------------
// Player
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerView {
    /// Player index (0 or 1).
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Faction identifier, if selected.
    pub faction_id: Option<u32>,
    /// Current Command Points.
    pub cp: i8,
    /// Current Victory Points.
    pub vp: i16,
    /// Primary VP breakdown.
    pub primary_vp: i16,
    /// Secondary VP breakdown.
    pub secondary_vp: i16,
    /// Selected enhancement, if any.
    pub enhancement_choice: Option<u32>,
    /// Selected secondary objective, if any.
    pub secondary_choice: Option<u32>,
    /// Whether this player goes first.
    pub first_turn: bool,
    /// Active blessings (World Eaters faction).
    pub active_blessings: Vec<String>,
    /// Blessing dice pool (World Eaters faction).
    pub blessing_dice: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Unit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitView {
    /// Unit id.
    pub id: u32,
    /// Owner player index.
    pub owner: u32,
    /// Display name.
    pub name: String,
    /// Datasheet identifier.
    pub datasheet_id: u32,
    /// Keywords as strings.
    pub keywords: Vec<String>,
    /// Individual models.
    pub models: Vec<ModelView>,
    /// Movement characteristic in inches (float).
    pub movement: f64,
    /// Toughness.
    pub toughness: u8,
    /// Armor save (e.g. "3+", "2+").
    pub armor_save: String,
    /// Invulnerable save, if any (e.g. "4+").
    pub invulnerable_save: Option<String>,
    /// Leadership characteristic.
    pub leadership: u8,
    /// Objective control.
    pub oc: u8,
    /// Current status ("OnBattlefield", "Destroyed", "InStrategicReserves", "Undeployed").
    pub status: String,
    /// Whether the unit is battle-shocked.
    pub battle_shocked: bool,
    /// Whether below half starting strength.
    pub below_half_strength: bool,
    /// Engagement status ("Engaged" or "NotEngaged").
    pub engagement_status: String,
    /// Attached leader unit id, if any.
    pub attached_leader: Option<u32>,
    /// Bodyguard for unit id, if any.
    pub bodyguard_for: Option<u32>,
    /// Reserve type if in reserves.
    pub reserve_type: Option<String>,
    /// Number of alive models.
    pub models_alive: usize,
    /// Starting model count.
    pub starting_model_count: usize,
    /// Total wounds remaining across all models.
    pub total_wounds_remaining: u16,
    /// Wargear ability descriptions.
    pub wargear_abilities: Vec<WargearAbilityView>,
    /// Whether this unit is a character.
    pub is_character: bool,
    /// Whether this unit is infantry.
    pub is_infantry: bool,
    /// Reference position (centroid of alive models), if any.
    pub position: Option<PositionView>,
    /// Turn action flags.
    pub turn_flags: UnitTurnFlagsView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTurnFlagsView {
    pub has_moved: bool,
    pub has_shot: bool,
    pub has_charged: bool,
    pub has_fought: bool,
    pub has_advanced: bool,
    pub has_fell_back: bool,
    pub is_stationary: bool,
    pub charged_this_turn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WargearAbilityView {
    pub name: String,
    pub description: String,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelView {
    /// Model id.
    pub id: u32,
    /// Parent unit id.
    pub unit_id: u32,
    /// Whether the model is alive.
    pub alive: bool,
    /// Maximum wounds.
    pub wounds_max: u8,
    /// Current wounds remaining.
    pub wounds_remaining: u8,
    /// Position on the board.
    pub position: PositionView,
    /// Base size in mm.
    pub base_size_mm: u16,
    /// Ranged weapon profiles.
    pub ranged_weapons: Vec<WeaponView>,
    /// Melee weapon profiles.
    pub melee_weapons: Vec<WeaponView>,
    /// Whether this model is a leader/character model.
    pub is_leader: bool,
    /// Feel No Pain threshold, if any (e.g. "5+").
    pub feel_no_pain: Option<String>,
    /// Wound allocation status.
    pub allocation_status: String,
}

// ---------------------------------------------------------------------------
// Weapon
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponView {
    /// Weapon id.
    pub id: u32,
    /// Weapon name.
    pub name: String,
    /// "Ranged" or "Melee".
    pub weapon_type: String,
    /// Range in inches (0 for melee).
    pub range: f64,
    /// Attacks characteristic as display string (e.g. "3", "D6", "D3+1").
    pub attacks: String,
    /// Skill to hit (e.g. "3+", "4+"). For Torrent weapons may be "N/A".
    pub skill: String,
    /// Strength value.
    pub strength: u8,
    /// AP value as display string (e.g. "0", "-1", "-3").
    pub ap: String,
    /// Damage characteristic as display string (e.g. "1", "D3", "D6+1").
    pub damage: String,
    /// Weapon abilities as display strings (e.g. ["Assault", "Lethal Hits", "Sustained Hits 1"]).
    pub abilities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Board / Terrain / Objectives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardView {
    /// Board width in inches.
    pub width: f64,
    /// Board height in inches.
    pub height: f64,
    /// Terrain pieces.
    pub terrain: Vec<TerrainView>,
    /// Objective markers.
    pub objectives: Vec<ObjectiveView>,
    /// Deployment zones (one per player).
    pub deployment_zones: Vec<DeploymentZoneView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainView {
    /// Terrain id.
    pub id: u32,
    /// Terrain name.
    pub name: String,
    /// Bounding rectangle.
    pub rect: RectView,
    /// Whether this terrain provides cover.
    pub provides_cover: bool,
    /// Whether this terrain blocks line of sight.
    pub blocks_los: bool,
    /// Whether this terrain is impassable.
    pub impassable: bool,
    /// Height in inches.
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RectView {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveView {
    /// Objective id.
    pub id: u32,
    /// Center position.
    pub position: PositionView,
    /// Label (e.g. "A", "B").
    pub label: String,
    /// Control status: "Uncontrolled", "ControlledBy(0)", "ControlledBy(1)", "Contested".
    pub control_status: String,
    /// Controlling player index, if any.
    pub controlling_player: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentZoneView {
    /// Which player this zone belongs to.
    pub player: u32,
    /// Polygon vertices (list of positions).
    pub vertices: Vec<PositionView>,
    /// Map type ("Standard" or "SearchAndDestroy").
    pub map_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionView {
    /// X coordinate in inches.
    pub x: f64,
    /// Y coordinate in inches.
    pub y: f64,
}

// ---------------------------------------------------------------------------
// Events (combat log)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    /// Event type name (e.g. "HitRollMade", "DamageInflicted").
    pub event_type: String,
    /// Human-readable description.
    pub description: String,
    /// Involved player index, if applicable.
    pub player: Option<u32>,
    /// Involved unit ids, if applicable.
    pub unit_ids: Vec<u32>,
    /// Battle round when this event occurred.
    pub round: u8,
    /// Phase when this event occurred.
    pub phase: String,
}

// ---------------------------------------------------------------------------
// Decision surface
// ---------------------------------------------------------------------------

/// The set of legal actions available at the current decision point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionSurfaceView {
    /// Type of decision (e.g. "MovementAction", "ShootingTarget").
    pub decision_type: String,
    /// Which player must decide.
    pub owner: u32,
    /// Legal actions.
    pub actions: Vec<ActionView>,
    /// Whether this decision is mandatory (vs optional like stratagem windows).
    pub is_mandatory: bool,
    /// Number of available options.
    pub num_options: usize,
    /// Whether there is exactly one forced action.
    pub is_forced: bool,
}

/// A single legal action in the decision surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionView {
    /// Index into the legal_commands array (used to call apply_action).
    pub index: usize,
    /// Human-readable label for this action.
    pub label: String,
    /// Command category (e.g. "NormalMove", "DeclareShootingTargets").
    pub command_type: String,
    /// Primary unit involved, if any.
    pub unit_id: Option<u32>,
    /// Target unit, if any.
    pub target_id: Option<u32>,
    /// Destination position, if any (movement commands).
    pub position: Option<PositionView>,
    /// Player who issues this command.
    pub player: Option<u32>,
}

// ---------------------------------------------------------------------------
// AI result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResultView {
    /// Label of the best action.
    pub best_action_label: String,
    /// Commands composing the best action (as JSON indices into decision surface).
    pub best_action_commands: Vec<String>,
    /// Evaluation score.
    pub score: i32,
    /// Number of nodes searched.
    pub nodes_evaluated: u64,
    /// Maximum depth reached.
    pub max_depth: u8,
    /// Time taken in milliseconds.
    pub time_ms: u64,
    /// Principal variation labels.
    pub pv: Vec<String>,
    /// Top candidate actions with scores.
    pub candidates: Vec<AiCandidateView>,
    /// Tactical intent of the best action.
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCandidateView {
    /// Action label.
    pub label: String,
    /// Evaluation score.
    pub score: i32,
    /// Tactical intent.
    pub intent: String,
}

// ---------------------------------------------------------------------------
// Game outcome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameOutcomeView {
    /// "in_progress", "victory", or "draw".
    pub status: String,
    /// Winning player index, if victory.
    pub winner: Option<u32>,
    /// Final VP for player 0.
    pub player_0_vp: i16,
    /// Final VP for player 1.
    pub player_1_vp: i16,
}

// ---------------------------------------------------------------------------
// Replay info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayInfoView {
    /// Format version.
    pub format_version: u32,
    /// Total number of frames.
    pub total_frames: usize,
    /// Current frame index.
    pub current_frame: usize,
    /// Game outcome.
    pub outcome: GameOutcomeView,
    /// Player info.
    pub players: Vec<ReplayPlayerInfoView>,
    /// Whether there are more frames to play.
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayPlayerInfoView {
    pub player_id: u32,
    pub name: String,
    pub faction_id: Option<u32>,
    pub enhancement: Option<u32>,
    pub secondary: Option<u32>,
}

// ---------------------------------------------------------------------------
// Validation result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResultView {
    pub valid: bool,
    pub reason: Option<String>,
}
