use serde::{Deserialize, Serialize};

/// Top-level asset for `boarding_actions_maps_complete_v3.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingMapsAsset {
    pub artifact: String,
    pub version: String,
    pub source: serde_json::Value,
    pub schema_notes: serde_json::Value,
    pub global: GlobalConfig,
    pub missions: Vec<MissionMapEntry>,
}

/// Global battlefield configuration shared by all boarding actions missions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub battlefield: BattlefieldConfig,
    pub common_region_labels: Vec<String>,
}

/// Battlefield-level configuration flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattlefieldConfig {
    pub boards: u32,
    pub zones: String,
    pub terrain_defined_by_mission_map: bool,
    pub entry_zones_defined_by_mission_map: bool,
    pub objective_markers_defined_by_mission_map: bool,
    pub starting_hatch_states_defined_by_mission_map: bool,
}

/// A single mission's map metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionMapEntry {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub mission_type: String,
    pub roles: MissionRoles,
    pub turn_order_override: Option<String>,
    pub map_ref: String,
    pub map_extraction_status: MapExtractionStatus,
    pub tags: Vec<String>,
    pub entry_zones: Vec<EntryZoneEntry>,
    pub special_regions: Vec<SpecialRegionEntry>,
    pub labeled_objectives: Vec<LabeledObjectiveEntry>,
    pub labeled_hatchways: Vec<LabeledHatchwayEntry>,
    pub notes: Vec<String>,
    #[serde(default)]
    pub transcription_method: Option<String>,
}

/// Attacker/Defender role assignments for a mission (null for symmetric).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRoles {
    pub attacker: Option<String>,
    pub defender: Option<String>,
}

/// Extraction status for various map geometry elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapExtractionStatus {
    pub geometry: String,
    pub hatchway_initial_states: String,
    pub objective_coordinates: String,
    pub entry_zone_coordinates: String,
}

/// An entry zone defined by the mission map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryZoneEntry {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub availability: Option<String>,
    #[serde(default)]
    pub deployment_only_for: Option<String>,
    #[serde(default)]
    pub reserve_restriction: Option<String>,
    #[serde(default)]
    pub count: Option<serde_json::Value>,
}

/// A special region on the mission map (e.g., compartments, furnace, lighting areas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialRegionEntry {
    pub name: String,
    pub status: String,
}

/// A labeled objective marker group on the mission map.
/// The `count` field can be either an integer or a string like `"map_defined"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledObjectiveEntry {
    pub name: String,
    #[serde(default)]
    pub count: Option<serde_json::Value>,
    pub status: String,
}

/// A labeled hatchway group on the mission map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabeledHatchwayEntry {
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub rule: Option<String>,
    #[serde(default)]
    pub count: Option<serde_json::Value>,
}
