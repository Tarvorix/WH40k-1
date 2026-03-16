use serde::{Deserialize, Serialize};

/// Top-level asset for `boarding_actions_mission_tags_complete_v3.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingMissionTagsAsset {
    pub artifact: String,
    pub version: String,
    pub source_basis: serde_json::Value,
    pub schema_notes: serde_json::Value,
    pub global_region_tags: Vec<GlobalTagDef>,
    pub global_action_tags: Vec<GlobalActionTagDef>,
    pub missions: Vec<MissionTagEntry>,
}

/// A global region/objective tag definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTagDef {
    pub tag: String,
    pub description: String,
}

/// A global action tag definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalActionTagDef {
    pub id: String,
    pub timing: String,
    pub description: String,
}

/// Per-mission tag/trigger/action binding entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionTagEntry {
    pub mission_id: String,
    pub mission_number: u32,
    pub mission_name: String,
    pub mission_type: String,
    pub turn_order_override: Option<String>,
    pub role_rules: Vec<RoleRule>,
    pub region_tags: Vec<RegionTagBinding>,
    pub objective_tags: Vec<ObjectiveTagBinding>,
    pub hatchway_tags: Vec<HatchwayTagBinding>,
    pub action_bindings: Vec<ActionBinding>,
    pub trigger_bindings: Vec<TriggerBinding>,
    pub mission_tags: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A role-specific rule for a mission (e.g., "Defender has first turn").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRule {
    pub role: String,
    pub rule: String,
}

/// A region tag binding for a specific mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionTagBinding {
    pub tag: String,
    #[serde(default)]
    pub label: Option<String>,
    pub status: String,
    #[serde(default)]
    pub binding: Option<String>,
    #[serde(default)]
    pub count: Option<serde_json::Value>,
}

/// An objective tag binding for a specific mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveTagBinding {
    pub tag: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub binding: Option<String>,
    #[serde(default)]
    pub count: Option<serde_json::Value>,
}

/// A hatchway tag binding for a specific mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HatchwayTagBinding {
    pub tag: String,
    pub status: String,
    #[serde(default)]
    pub binding: Option<String>,
}

/// An action binding for a specific mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionBinding {
    pub id: String,
    pub status: String,
    #[serde(default)]
    pub targets: Option<String>,
}

/// A trigger binding for a specific mission.
/// Some triggers use `effect` (single string) while others use `effects` (array).
/// Some have optional `condition`, `actor`, and `status` fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerBinding {
    pub id: String,
    pub timing: String,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub effects: Option<Vec<String>>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}
