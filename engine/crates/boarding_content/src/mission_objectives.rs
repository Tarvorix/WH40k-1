use serde::{Deserialize, Serialize};

/// Top-level asset for `boarding_actions_objectives_complete_v3.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingObjectivesAsset {
    pub schema_version: String,
    pub title: String,
    pub source_basis: SourceBasis,
    pub global_rules: GlobalObjectiveRules,
    pub missions: Vec<MissionObjectiveEntry>,
}

/// Source documentation references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBasis {
    pub primary_docs: Vec<String>,
    pub notes: Vec<String>,
}

/// Global rules that apply to all boarding actions missions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalObjectiveRules {
    pub mission_vp_cap: i16,
    pub battle_ready_bonus_vp: i16,
    pub battle_rounds: u8,
    pub round5_second_player_timing_default: String,
    pub common_progressive_template: CommonProgressiveTemplate,
}

/// The common progressive scoring template shared by many missions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonProgressiveTemplate {
    pub id: String,
    pub description: String,
    pub rounds: Vec<u8>,
    pub timing: String,
    pub per_satisfied_condition_vp: u8,
    pub default_conditions: Vec<String>,
    pub round5: Round5Timing,
}

/// Round 5 timing rules for first and second player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round5Timing {
    pub first_player_timing: String,
    pub second_player_timing: String,
}

/// A single mission's objective definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionObjectiveEntry {
    pub mission_id: String,
    pub mission_number: u32,
    pub mission_name: String,
    pub mission_type: String,
    pub roles: Vec<String>,
    pub objectives: Vec<ObjectiveDef>,
    #[serde(default)]
    pub implementation_notes: Vec<String>,
}

/// A single objective within a mission.
/// The `scoring` field has highly variable structure depending on the objective type
/// (threshold_table, per_marker_controlled, event_trigger, role_based, exclusive_outcome_table, etc.),
/// so it is represented as a generic JSON value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveDef {
    pub objective_id: String,
    pub name: String,
    pub objective_type: String,
    #[serde(default)]
    pub status: Option<String>,
    pub scoring: serde_json::Value,
}
