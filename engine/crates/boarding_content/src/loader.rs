use crate::faction_data::BoardingFactionDef;
use crate::mission_maps::BoardingMapsAsset;
use crate::mission_objectives::BoardingObjectivesAsset;
use crate::mission_tags::BoardingMissionTagsAsset;

/// Errors that can occur during asset loading.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Load the boarding actions maps asset from a JSON string.
pub fn load_maps_asset(json: &str) -> Result<BoardingMapsAsset, LoadError> {
    Ok(serde_json::from_str(json)?)
}

/// Load the boarding actions objectives asset from a JSON string.
pub fn load_objectives_asset(json: &str) -> Result<BoardingObjectivesAsset, LoadError> {
    Ok(serde_json::from_str(json)?)
}

/// Load the boarding actions mission tags asset from a JSON string.
pub fn load_mission_tags_asset(json: &str) -> Result<BoardingMissionTagsAsset, LoadError> {
    Ok(serde_json::from_str(json)?)
}

/// Load a faction definition from a JSON string.
pub fn load_faction(json: &str) -> Result<BoardingFactionDef, LoadError> {
    Ok(serde_json::from_str(json)?)
}

/// Validate cross-references between maps, objectives, and tags.
/// Checks that:
/// - Every mission in objectives has a corresponding entry in maps
/// - Every mission in tags has a corresponding entry in maps
/// - Mission IDs are consistent across all three files
pub fn validate_assets(
    maps: &BoardingMapsAsset,
    objectives: &BoardingObjectivesAsset,
    tags: &BoardingMissionTagsAsset,
) -> Result<(), LoadError> {
    let map_ids: std::collections::HashSet<&str> =
        maps.missions.iter().map(|m| m.id.as_str()).collect();

    // Check that all mission IDs in objectives exist in maps.
    // Note: objectives use zero-padded IDs for single-digit missions (e.g. "BA-01")
    // while maps use non-padded IDs (e.g. "BA-1"), so we normalize for comparison.
    for obj_mission in &objectives.missions {
        let obj_id = &obj_mission.mission_id;
        let normalized = normalize_mission_id(obj_id);
        if !map_ids.contains(normalized.as_str()) && !map_ids.contains(obj_id.as_str()) {
            return Err(LoadError::ValidationError(format!(
                "Objective mission {} not found in maps asset",
                obj_mission.mission_id
            )));
        }
    }

    // Check that all mission IDs in tags exist in maps.
    for tag_mission in &tags.missions {
        let tag_id = &tag_mission.mission_id;
        let normalized = normalize_mission_id(tag_id);
        if !map_ids.contains(normalized.as_str()) && !map_ids.contains(tag_id.as_str()) {
            return Err(LoadError::ValidationError(format!(
                "Tag mission {} not found in maps asset",
                tag_mission.mission_id
            )));
        }
    }

    Ok(())
}

/// Normalize a mission ID by removing zero-padding from the numeric part.
/// e.g., "BA-01" -> "BA-1", "BA-11" -> "BA-11"
fn normalize_mission_id(id: &str) -> String {
    if let Some(pos) = id.rfind('-') {
        let prefix = &id[..=pos];
        let number_part = &id[pos + 1..];
        if let Ok(num) = number_part.parse::<u32>() {
            return format!("{}{}", prefix, num);
        }
    }
    id.to_string()
}
