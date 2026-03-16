//! Mission Loader for Boarding Actions.
//!
//! Loads the 3 JSON asset files (maps, objectives, tags) and produces a unified
//! `BoardingMissionPackage` for a specific mission or all missions.
//!
//! Source: boarding_actions_maps_complete_v3.json
//! Source: boarding_actions_objectives_complete_v3.json
//! Source: boarding_actions_mission_tags_complete_v3.json

use serde::{Deserialize, Serialize};
use wh40k_boarding_content::mission_maps::{BoardingMapsAsset, MissionMapEntry};
use wh40k_boarding_content::mission_objectives::{BoardingObjectivesAsset, MissionObjectiveEntry};
use wh40k_boarding_content::mission_tags::{BoardingMissionTagsAsset, MissionTagEntry};
use wh40k_core_types::BoardingMissionType;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when loading a mission package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionLoadError {
    /// The requested mission ID was not found in the maps asset.
    MissionNotFound(String),
    /// The mission was found in maps but has no corresponding objectives entry.
    MissingObjectives(String),
    /// The mission was found in maps but has no corresponding tags entry.
    MissingTags(String),
}

impl std::fmt::Display for MissionLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MissionLoadError::MissionNotFound(id) => {
                write!(f, "Mission '{}' not found in maps asset", id)
            }
            MissionLoadError::MissingObjectives(id) => {
                write!(f, "Mission '{}' has no objectives entry", id)
            }
            MissionLoadError::MissingTags(id) => {
                write!(f, "Mission '{}' has no tags entry", id)
            }
        }
    }
}

impl std::error::Error for MissionLoadError {}

// ---------------------------------------------------------------------------
// Data structures for a unified mission package
// ---------------------------------------------------------------------------

/// A fully loaded, unified mission package combining map, objectives, and tag data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardingMissionPackage {
    /// Mission ID (e.g., "BA-11", "BA-1")
    pub mission_id: String,
    /// Human-readable name
    pub name: String,
    /// Whether the mission is symmetric or asymmetric
    pub mission_type: BoardingMissionType,
    /// Attacker/Defender role names for asymmetric missions
    pub roles: MissionRoles,
    /// Turn order override (e.g., "defender_first" for some asymmetric missions)
    pub turn_order_override: Option<String>,
    /// Tags describing mission mechanics (e.g., "lighting_areas", "radiation", "compartment_venting")
    pub tags: Vec<String>,
    /// Entry zone definitions from the map
    pub entry_zones: Vec<MissionEntryZone>,
    /// Special regions from the map (lighting areas, sectors, compartments, etc.)
    pub special_regions: Vec<MissionSpecialRegion>,
    /// Objective markers with labels and tags
    pub objectives: Vec<MissionObjective>,
    /// Hatchway special rules
    pub hatchway_rules: Vec<MissionHatchwayRule>,
    /// Scoring objectives (progressive and end-game)
    pub scoring_objectives: Vec<ScoringObjective>,
    /// Trigger bindings for mission-specific mechanics
    pub triggers: Vec<MissionTrigger>,
    /// Action bindings for mission-specific actions
    pub action_bindings: Vec<String>,
    /// Role-specific deployment/action rules
    pub role_rules: Vec<MissionRoleRule>,
    /// Mission-specific rules text/notes
    pub notes: Vec<String>,
    /// Underdog bonus (usually +1 CP)
    pub underdog_bonus: Option<String>,
}

/// Attacker/Defender role assignments for a mission (None for symmetric).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRoles {
    pub attacker: Option<String>,
    pub defender: Option<String>,
}

/// An entry zone defined by the mission map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionEntryZone {
    pub name: String,
    pub status: String,
}

/// A special region on the mission map (e.g., compartments, furnace, lighting areas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionSpecialRegion {
    pub name: String,
    pub tags: Vec<String>,
    pub status: String,
}

/// A labeled objective marker group on the mission map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionObjective {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub count: Option<u32>,
}

/// A hatchway special rule for a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionHatchwayRule {
    pub description: String,
    pub tags: Vec<String>,
}

/// A scoring objective (progressive or end-game) from the objectives JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringObjective {
    pub objective_id: String,
    pub name: String,
    pub objective_type: String,
    pub status: String,
    /// Raw scoring data from the objectives JSON (highly variable structure per mission)
    pub scoring: serde_json::Value,
}

/// A trigger binding for mission-specific mechanics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionTrigger {
    pub id: String,
    pub timing: String,
    pub condition: Option<String>,
    pub effect: Option<String>,
    pub effects: Option<Vec<String>>,
}

/// A role-specific rule for a mission (e.g., "Defender has first turn").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionRoleRule {
    pub role: String,
    pub rule: String,
}

// ---------------------------------------------------------------------------
// ID normalization
// ---------------------------------------------------------------------------

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

/// Check if two mission IDs match after normalization.
fn mission_ids_match(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    normalize_mission_id(a) == normalize_mission_id(b)
}

// ---------------------------------------------------------------------------
// Lookup helpers
// ---------------------------------------------------------------------------

/// Find a mission in the maps asset by ID (handles BA-1 vs BA-01 normalization).
fn find_map_entry<'a>(
    maps: &'a BoardingMapsAsset,
    mission_id: &str,
) -> Option<&'a MissionMapEntry> {
    maps.missions
        .iter()
        .find(|m| mission_ids_match(&m.id, mission_id))
}

/// Find a mission in the objectives asset by ID.
fn find_objectives_entry<'a>(
    objectives: &'a BoardingObjectivesAsset,
    mission_id: &str,
) -> Option<&'a MissionObjectiveEntry> {
    objectives
        .missions
        .iter()
        .find(|m| mission_ids_match(&m.mission_id, mission_id))
}

/// Find a mission in the tags asset by ID.
fn find_tags_entry<'a>(
    tags: &'a BoardingMissionTagsAsset,
    mission_id: &str,
) -> Option<&'a MissionTagEntry> {
    tags.missions
        .iter()
        .find(|m| mission_ids_match(&m.mission_id, mission_id))
}

// ---------------------------------------------------------------------------
// Package assembly
// ---------------------------------------------------------------------------

/// Parse a BoardingMissionType from a string (the maps and tags use "symmetric"/"asymmetric").
fn parse_mission_type(s: &str) -> BoardingMissionType {
    match s.to_lowercase().as_str() {
        "asymmetric" => BoardingMissionType::Asymmetric,
        _ => BoardingMissionType::Symmetric,
    }
}

/// Extract the count from a serde_json::Value that may be an integer or a string.
fn extract_count(val: &Option<serde_json::Value>) -> Option<u32> {
    match val {
        Some(serde_json::Value::Number(n)) => n.as_u64().map(|v| v as u32),
        _ => None,
    }
}

/// Detect underdog bonus from mission tags.
fn detect_underdog_bonus(mission_tags: &[String], triggers: &[MissionTrigger]) -> Option<String> {
    // Check mission_tags for underdog bonus pattern
    for tag in mission_tags {
        if tag.starts_with("underdog_bonus.") {
            return Some(tag.clone());
        }
    }
    // Also check triggers for underdog bonus
    for trigger in triggers {
        if trigger.id.contains("underdog_bonus") || trigger.id.contains("underdog") {
            if let Some(effect) = &trigger.effect {
                if effect.contains("CP") || effect.contains("underdog") {
                    return Some(effect.clone());
                }
            }
        }
    }
    None
}

/// Build a BoardingMissionPackage from the three asset entries.
fn build_package(
    map_entry: &MissionMapEntry,
    obj_entry: &MissionObjectiveEntry,
    tag_entry: &MissionTagEntry,
) -> BoardingMissionPackage {
    // Build entry zones from map data
    let entry_zones: Vec<MissionEntryZone> = map_entry
        .entry_zones
        .iter()
        .map(|ez| MissionEntryZone {
            name: ez.name.clone(),
            status: ez.status.clone(),
        })
        .collect();

    // Build special regions from map data, augmented by region tags from the tags asset
    let special_regions: Vec<MissionSpecialRegion> = map_entry
        .special_regions
        .iter()
        .map(|sr| {
            // Find matching region tags from the tags asset
            let region_tags: Vec<String> = tag_entry
                .region_tags
                .iter()
                .filter(|rt| {
                    // Match by label if present, otherwise by tag name containing region name
                    if let Some(label) = &rt.label {
                        label.to_lowercase().contains(&sr.name.to_lowercase())
                            || sr.name.to_lowercase().contains(&label.to_lowercase())
                    } else {
                        false
                    }
                })
                .map(|rt| rt.tag.clone())
                .collect();
            MissionSpecialRegion {
                name: sr.name.clone(),
                tags: region_tags,
                status: sr.status.clone(),
            }
        })
        .collect();

    // Build objectives from map labeled_objectives, augmented by objective tags
    let objectives: Vec<MissionObjective> = map_entry
        .labeled_objectives
        .iter()
        .map(|lo| {
            // Find matching objective tags from the tags asset
            let obj_tags: Vec<String> = tag_entry
                .objective_tags
                .iter()
                .filter(|ot| {
                    if let Some(label) = &ot.label {
                        label.to_lowercase().contains(&lo.name.to_lowercase())
                            || lo.name.to_lowercase().contains(&label.to_lowercase())
                    } else {
                        // If no label, include tag if it seems relevant based on tag name
                        false
                    }
                })
                .map(|ot| ot.tag.clone())
                .collect();

            // Also include any objective tags that don't have specific labels (global tags)
            let mut all_tags = obj_tags;
            for ot in &tag_entry.objective_tags {
                if ot.label.is_none() && !all_tags.contains(&ot.tag) {
                    all_tags.push(ot.tag.clone());
                }
            }

            MissionObjective {
                id: lo.name.clone(),
                name: lo.name.clone(),
                tags: all_tags,
                count: extract_count(&lo.count),
            }
        })
        .collect();

    // Build hatchway rules from map labeled_hatchways + hatchway tags
    let hatchway_rules: Vec<MissionHatchwayRule> = {
        let mut rules = Vec::new();

        // From map hatchways
        for lh in &map_entry.labeled_hatchways {
            let description = if let Some(rule) = &lh.rule {
                format!("{}: {}", lh.name, rule)
            } else {
                lh.name.clone()
            };
            let tags: Vec<String> = tag_entry
                .hatchway_tags
                .iter()
                .filter(|ht| {
                    ht.tag
                        .to_lowercase()
                        .contains(&lh.name.to_lowercase().replace(' ', "_"))
                        || lh.name.to_lowercase().contains(
                            &ht.tag
                                .to_lowercase()
                                .replace("hatchway.", "")
                                .replace("region.", ""),
                        )
                })
                .map(|ht| ht.tag.clone())
                .collect();
            rules.push(MissionHatchwayRule { description, tags });
        }

        // Add any hatchway tags that didn't match a specific labeled hatchway
        for ht in &tag_entry.hatchway_tags {
            let already_matched = rules.iter().any(|r| r.tags.contains(&ht.tag));
            if !already_matched {
                let description = if let Some(binding) = &ht.binding {
                    format!("{}: {}", ht.tag, binding)
                } else {
                    ht.tag.clone()
                };
                rules.push(MissionHatchwayRule {
                    description,
                    tags: vec![ht.tag.clone()],
                });
            }
        }

        rules
    };

    // Build scoring objectives from the objectives asset
    let scoring_objectives: Vec<ScoringObjective> = obj_entry
        .objectives
        .iter()
        .map(|od| ScoringObjective {
            objective_id: od.objective_id.clone(),
            name: od.name.clone(),
            objective_type: od.objective_type.clone(),
            status: od.status.clone().unwrap_or_else(|| "unknown".to_string()),
            scoring: od.scoring.clone(),
        })
        .collect();

    // Build triggers from the tags asset
    let triggers: Vec<MissionTrigger> = tag_entry
        .trigger_bindings
        .iter()
        .map(|tb| MissionTrigger {
            id: tb.id.clone(),
            timing: tb.timing.clone(),
            condition: tb.condition.clone(),
            effect: tb.effect.clone(),
            effects: tb.effects.clone(),
        })
        .collect();

    // Build action bindings from the tags asset
    let action_bindings: Vec<String> = tag_entry
        .action_bindings
        .iter()
        .map(|ab| ab.id.clone())
        .collect();

    // Build role rules from the tags asset
    let role_rules: Vec<MissionRoleRule> = tag_entry
        .role_rules
        .iter()
        .map(|rr| MissionRoleRule {
            role: rr.role.clone(),
            rule: rr.rule.clone(),
        })
        .collect();

    // Combine notes from map and tags
    let mut notes = map_entry.notes.clone();
    notes.extend(tag_entry.notes.clone());

    // Detect underdog bonus
    let underdog_bonus = detect_underdog_bonus(&tag_entry.mission_tags, &triggers);

    // Determine turn order override (prefer tags, fall back to map)
    let turn_order_override = tag_entry
        .turn_order_override
        .clone()
        .or(map_entry.turn_order_override.clone());

    BoardingMissionPackage {
        mission_id: map_entry.id.clone(),
        name: map_entry.name.clone(),
        mission_type: parse_mission_type(&map_entry.mission_type),
        roles: MissionRoles {
            attacker: map_entry.roles.attacker.clone(),
            defender: map_entry.roles.defender.clone(),
        },
        turn_order_override,
        tags: tag_entry.mission_tags.clone(),
        entry_zones,
        special_regions,
        objectives,
        hatchway_rules,
        scoring_objectives,
        triggers,
        action_bindings,
        role_rules,
        notes,
        underdog_bonus,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load a single mission by ID from the three asset files.
///
/// Handles ID normalization (BA-1 vs BA-01) automatically.
/// Returns an error if the mission is not found in any of the three assets.
pub fn load_mission(
    mission_id: &str,
    maps: &BoardingMapsAsset,
    objectives: &BoardingObjectivesAsset,
    tags: &BoardingMissionTagsAsset,
) -> Result<BoardingMissionPackage, MissionLoadError> {
    let map_entry = find_map_entry(maps, mission_id)
        .ok_or_else(|| MissionLoadError::MissionNotFound(mission_id.to_string()))?;

    let obj_entry = find_objectives_entry(objectives, mission_id)
        .ok_or_else(|| MissionLoadError::MissingObjectives(mission_id.to_string()))?;

    let tag_entry = find_tags_entry(tags, mission_id)
        .ok_or_else(|| MissionLoadError::MissingTags(mission_id.to_string()))?;

    Ok(build_package(map_entry, obj_entry, tag_entry))
}

/// Load all missions from the maps asset, combining with objectives and tags.
///
/// Missions that are present in the maps but missing from objectives or tags
/// will produce errors in the returned Vec.
pub fn load_all_missions(
    maps: &BoardingMapsAsset,
    objectives: &BoardingObjectivesAsset,
    tags: &BoardingMissionTagsAsset,
) -> Result<Vec<BoardingMissionPackage>, MissionLoadError> {
    let mut packages = Vec::with_capacity(maps.missions.len());

    for map_entry in &maps.missions {
        let mission_id = &map_entry.id;

        let obj_entry = find_objectives_entry(objectives, mission_id)
            .ok_or_else(|| MissionLoadError::MissingObjectives(mission_id.clone()))?;

        let tag_entry = find_tags_entry(tags, mission_id)
            .ok_or_else(|| MissionLoadError::MissingTags(mission_id.clone()))?;

        packages.push(build_package(map_entry, obj_entry, tag_entry));
    }

    Ok(packages)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_boarding_content::loader;

    /// Helper to load all three assets from the actual JSON files.
    fn load_all_assets() -> (BoardingMapsAsset, BoardingObjectivesAsset, BoardingMissionTagsAsset) {
        let maps_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_maps_complete_v3.json"
        ))
        .expect("Failed to read maps JSON");
        let obj_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_objectives_complete_v3.json"
        ))
        .expect("Failed to read objectives JSON");
        let tags_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_mission_tags_complete_v3.json"
        ))
        .expect("Failed to read tags JSON");

        let maps = loader::load_maps_asset(&maps_json).expect("Failed to parse maps asset");
        let objectives =
            loader::load_objectives_asset(&obj_json).expect("Failed to parse objectives asset");
        let tags =
            loader::load_mission_tags_asset(&tags_json).expect("Failed to parse tags asset");

        (maps, objectives, tags)
    }

    #[test]
    fn test_normalize_mission_id() {
        assert_eq!(normalize_mission_id("BA-01"), "BA-1");
        assert_eq!(normalize_mission_id("BA-11"), "BA-11");
        assert_eq!(normalize_mission_id("BA-1"), "BA-1");
        assert_eq!(normalize_mission_id("BA-06"), "BA-6");
        assert_eq!(normalize_mission_id("BA-33"), "BA-33");
    }

    #[test]
    fn test_mission_ids_match() {
        assert!(mission_ids_match("BA-1", "BA-01"));
        assert!(mission_ids_match("BA-01", "BA-1"));
        assert!(mission_ids_match("BA-11", "BA-11"));
        assert!(!mission_ids_match("BA-11", "BA-12"));
        assert!(mission_ids_match("BA-06", "BA-6"));
    }

    #[test]
    fn test_load_mission_ba_11() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.mission_id, "BA-11");
        assert_eq!(pkg.name, "Access Junction Primus");
        assert_eq!(pkg.mission_type, BoardingMissionType::Symmetric);
        assert!(pkg.roles.attacker.is_none());
        assert!(pkg.roles.defender.is_none());
        // Should have scoring objectives
        assert!(!pkg.scoring_objectives.is_empty());
        // Should have progressive and end-game objectives
        let has_progressive = pkg
            .scoring_objectives
            .iter()
            .any(|so| so.objective_type == "progressive");
        let has_end_game = pkg
            .scoring_objectives
            .iter()
            .any(|so| so.objective_type == "end_game");
        assert!(has_progressive, "BA-11 should have progressive scoring");
        assert!(has_end_game, "BA-11 should have end-game scoring");
    }

    #[test]
    fn test_load_mission_ba_01_asymmetric() {
        let (maps, objectives, tags) = load_all_assets();
        // Use normalized form to test normalization
        let pkg = load_mission("BA-01", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Void the Ship");
        assert_eq!(pkg.mission_type, BoardingMissionType::Asymmetric);
        assert!(pkg.roles.attacker.is_some());
        assert!(pkg.roles.defender.is_some());
        // Should have turn order override (defender_first)
        assert_eq!(
            pkg.turn_order_override.as_deref(),
            Some("defender_first")
        );
        // Should have role rules
        assert!(!pkg.role_rules.is_empty());
        // Should have airlock-related action bindings
        assert!(
            pkg.action_bindings
                .iter()
                .any(|a| a.contains("airlock")),
            "BA-01 should have airlock action binding"
        );
    }

    #[test]
    fn test_load_mission_ba_33_radiation() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-33", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Rad Leak");
        assert_eq!(pkg.mission_type, BoardingMissionType::Symmetric);
        // Should have radiation-related tags
        assert!(
            pkg.tags.iter().any(|t| t.contains("radiation")),
            "BA-33 should have radiation tag"
        );
        // Should have special regions (sectors)
        assert!(
            !pkg.special_regions.is_empty(),
            "BA-33 should have sector regions"
        );
        // Should have radiation triggers
        assert!(
            pkg.triggers.iter().any(|t| t.id.contains("rad")),
            "BA-33 should have radiation triggers"
        );
    }

    #[test]
    fn test_load_mission_ba_22_lighting() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-22", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Death in the Dark");
        // Should have lighting-related tags
        assert!(pkg.tags.iter().any(|t| t.contains("lighting")));
        // Should have lighting action binding
        assert!(pkg
            .action_bindings
            .iter()
            .any(|a| a.contains("lighting")));
    }

    #[test]
    fn test_load_mission_ba_06_corruption() {
        let (maps, objectives, tags) = load_all_assets();
        // Test normalization: BA-06 in objectives maps to BA-6 in maps
        let pkg = load_mission("BA-06", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Corrupt the Machine Spirit");
        assert_eq!(pkg.mission_type, BoardingMissionType::Asymmetric);
        // Should have corruption action binding
        assert!(pkg
            .action_bindings
            .iter()
            .any(|a| a.contains("corrupt")));
    }

    #[test]
    fn test_load_all_missions() {
        let (maps, objectives, tags) = load_all_assets();
        let packages = load_all_missions(&maps, &objectives, &tags).unwrap();

        // There should be 15 missions total (based on the maps asset)
        assert_eq!(packages.len(), 15);

        // Check a few expected missions are present
        let ids: Vec<&str> = packages.iter().map(|p| p.mission_id.as_str()).collect();
        assert!(ids.contains(&"BA-11"));
        assert!(ids.contains(&"BA-33"));

        // All should have at least one scoring objective
        for pkg in &packages {
            assert!(
                !pkg.scoring_objectives.is_empty(),
                "Mission {} should have scoring objectives",
                pkg.mission_id
            );
        }
    }

    #[test]
    fn test_load_mission_not_found() {
        let (maps, objectives, tags) = load_all_assets();
        let result = load_mission("BA-99", &maps, &objectives, &tags);
        assert!(result.is_err());
        match result {
            Err(MissionLoadError::MissionNotFound(id)) => assert_eq!(id, "BA-99"),
            _ => panic!("Expected MissionNotFound error"),
        }
    }

    #[test]
    fn test_package_serialization() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        // Should be serializable to JSON and back
        let json = serde_json::to_string(&pkg).unwrap();
        let deserialized: BoardingMissionPackage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.mission_id, "BA-11");
        assert_eq!(deserialized.name, "Access Junction Primus");
    }

    #[test]
    fn test_load_mission_ba_04_jailbreak() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-04", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Jailbreak");
        assert_eq!(pkg.mission_type, BoardingMissionType::Asymmetric);
        // Should have prison-related tags
        assert!(pkg.tags.iter().any(|t| t.contains("prison")));
        // Should have exclusive outcome table scoring
        let end_game = pkg
            .scoring_objectives
            .iter()
            .find(|so| so.objective_type == "end_game")
            .expect("BA-04 should have end_game objective");
        assert!(
            end_game.scoring.get("method").is_some(),
            "BA-04 end-game should have a scoring method"
        );
    }

    #[test]
    fn test_load_mission_ba_23_hull_breach() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-23", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Hull Breach");
        // Should have download action
        assert!(pkg
            .action_bindings
            .iter()
            .any(|a| a.contains("download")));
        // Should have vent action
        assert!(pkg
            .action_bindings
            .iter()
            .any(|a| a.contains("vent")));
    }

    #[test]
    fn test_load_mission_ba_02_vp_transfer() {
        let (maps, objectives, tags) = load_all_assets();
        let pkg = load_mission("BA-02", &maps, &objectives, &tags).unwrap();

        assert_eq!(pkg.name, "Pull Their Teeth");
        assert_eq!(pkg.mission_type, BoardingMissionType::Asymmetric);
        // Should have VP transfer tag
        assert!(pkg.tags.iter().any(|t| t.contains("vp_transfer")));
        // Should have initial state in scoring data
        let progressive = pkg
            .scoring_objectives
            .iter()
            .find(|so| so.objective_type == "progressive")
            .expect("BA-02 should have progressive objective");
        assert!(
            progressive.scoring.get("initial_state").is_some()
                || progressive.scoring.get("method").is_some(),
            "BA-02 progressive should have transfer scoring data"
        );
    }
}
