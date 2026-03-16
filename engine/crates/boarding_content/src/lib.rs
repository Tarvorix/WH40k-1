pub mod faction_data;
pub mod loader;
pub mod mission_maps;
pub mod mission_objectives;
pub mod mission_tags;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_maps_asset() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_maps_complete_v3.json"
        ))
        .unwrap();
        let asset = loader::load_maps_asset(&json).unwrap();
        assert_eq!(asset.missions.len(), 15);
        // Verify first mission
        assert_eq!(asset.missions[0].id, "BA-11");
        assert_eq!(asset.missions[0].name, "Access Junction Primus");
    }

    #[test]
    fn test_load_objectives_asset() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_objectives_complete_v3.json"
        ))
        .unwrap();
        let asset = loader::load_objectives_asset(&json).unwrap();
        assert_eq!(asset.missions.len(), 15);
    }

    #[test]
    fn test_load_mission_tags_asset() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_mission_tags_complete_v3.json"
        ))
        .unwrap();
        let asset = loader::load_mission_tags_asset(&json).unwrap();
        assert_eq!(asset.missions.len(), 15);
    }

    #[test]
    fn test_validate_assets() {
        let maps_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_maps_complete_v3.json"
        ))
        .unwrap();
        let obj_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_objectives_complete_v3.json"
        ))
        .unwrap();
        let tags_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_mission_tags_complete_v3.json"
        ))
        .unwrap();

        let maps = loader::load_maps_asset(&maps_json).unwrap();
        let objectives = loader::load_objectives_asset(&obj_json).unwrap();
        let tags = loader::load_mission_tags_asset(&tags_json).unwrap();

        loader::validate_assets(&maps, &objectives, &tags).unwrap();
    }

    #[test]
    fn test_load_faction_space_marines_terminator_assault() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/space_marines_terminator_assault.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_name, "Adeptus Astartes");
        assert_eq!(faction.faction_keyword, "ADEPTUS ASTARTES");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Terminator Assault");
        assert_eq!(faction.detachments[0].allowed_units.len(), 16);
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert_eq!(faction.datasheets.len(), 16);
    }

    #[test]
    fn test_load_faction_world_eaters_boarding_butchers() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/world_eaters_boarding_butchers.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_name, "World Eaters");
        assert_eq!(faction.faction_keyword, "WORLD EATERS");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Boarding Butchers");
        assert_eq!(faction.detachments[0].allowed_units.len(), 8);
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert_eq!(faction.datasheets.len(), 8);
    }

    #[test]
    fn test_load_faction_world_eaters_skullsworn() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/world_eaters_skullsworn.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_name, "World Eaters");
        assert_eq!(faction.faction_keyword, "WORLD EATERS");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Skullsworn");
        assert_eq!(faction.detachments[0].allowed_units.len(), 5);
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert_eq!(faction.datasheets.len(), 5);
    }

    #[test]
    fn test_load_faction_csm_champions_of_chaos() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/csm_champions_of_chaos.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_keyword, "HERETIC ASTARTES");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Champions of Chaos");
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert!(faction.datasheets.len() >= 6);
    }

    #[test]
    fn test_load_faction_csm_underdeck_uprising() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/csm_underdeck_uprising.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_keyword, "HERETIC ASTARTES");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Underdeck Uprising");
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert!(faction.datasheets.len() >= 7);
    }

    #[test]
    fn test_load_faction_astra_militarum_tempestus() {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/boarding_actions/factions/astra_militarum_tempestus.json"
        ))
        .unwrap();
        let faction = loader::load_faction(&json).unwrap();
        assert_eq!(faction.faction_keyword, "ASTRA MILITARUM");
        assert_eq!(faction.detachments.len(), 1);
        assert_eq!(faction.detachments[0].detachment_name, "Tempestus Boarding Regiment");
        assert_eq!(faction.detachments[0].stratagems.len(), 4);
        assert_eq!(faction.detachments[0].enhancements.len(), 2);
        assert_eq!(faction.datasheets.len(), 5);
    }

    #[test]
    fn test_all_factions_have_complete_datasheets() {
        let faction_files = [
            "space_marines_terminator_assault",
            "world_eaters_boarding_butchers",
            "world_eaters_skullsworn",
            "csm_champions_of_chaos",
            "csm_underdeck_uprising",
            "astra_militarum_tempestus",
        ];
        for name in &faction_files {
            let path = format!(
                "{}/../../../content/boarding_actions/factions/{}.json",
                env!("CARGO_MANIFEST_DIR"),
                name
            );
            let json = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
            let faction: faction_data::BoardingFactionDef = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", name, e));

            // Every faction must have at least 1 detachment
            assert!(!faction.detachments.is_empty(), "{} has no detachments", name);
            // Every detachment must have stratagems and enhancements
            for det in &faction.detachments {
                assert!(!det.stratagems.is_empty(), "{}/{} has no stratagems", name, det.detachment_name);
                assert!(!det.enhancements.is_empty(), "{}/{} has no enhancements", name, det.detachment_name);
                assert!(!det.allowed_units.is_empty(), "{}/{} has no allowed units", name, det.detachment_name);
            }
            // Every datasheet must have at least melee weapons and a profile
            for ds in &faction.datasheets {
                assert!(!ds.melee_weapons.is_empty(), "{}/{} has no melee weapons", name, ds.name);
                assert!(ds.profile.toughness > 0, "{}/{} has zero toughness", name, ds.name);
                assert!(ds.profile.wounds > 0, "{}/{} has zero wounds", name, ds.name);
                assert!(!ds.points.is_empty(), "{}/{} has no points cost", name, ds.name);
            }
        }
    }
}
