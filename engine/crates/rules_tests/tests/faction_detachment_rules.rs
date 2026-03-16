//! Source-linked rules tests for faction-specific and detachment-specific rules.
//!
//! Tests Combat Patrol faction mechanics:
//!   - Custodes: Martial Ka'tah stances, Vaultswords profiles, Watchman of Terra,
//!     unit datasheets, keyword validation
//!   - World Eaters: Blessings of Khorne, Blood Tithe tracking, unit datasheets,
//!     keyword validation
//!   - ScenarioLoader faction unit creation for both factions
//!
//! Tests Boarding Actions detachment mechanics:
//!   - All 6 BA faction JSON files load successfully
//!   - Correct datasheet counts per detachment
//!   - Stratagems and enhancements defined per detachment
//!   - Mustering rules: allowed units match detachment restrictions
//!   - Space Marines Terminator Assault keyword checks
//!   - World Eaters Boarding Butchers keyword checks
//!   - CSM detachments have different allowed units
//!
//! Source: Custodes.md, Frenzied_Reavers.md
//! Source: Space_Marines_BA_Terminator_Assault.md, World_Eaters_BA_Boarding_Butchers.md
//! Source: boarding_actions_complete_v3.md Section 6

use wh40k_core_types::{
    FactionId, Keyword, KeywordSet, MissionId, ModelId, PlayerId, UnitId,
};
use wh40k_content_schema::{
    AbilitySchema, AbilityType, EnhancementSchema, FactionSchema, RulePrimitive, StanceDef,
};
use wh40k_event_system::BlessingOfKhorne;
use wh40k_faction_runtime::{
    BlessingAllocation, BlessingValidator, FactionRuntime,
};
use wh40k_game_core::ScenarioLoader;
use wh40k_game_core::state::TurnFlags;
use wh40k_boarding_content::faction_data::BoardingFactionDef;
use wh40k_boarding_content::loader;

// ===========================================================================
// Helpers
// ===========================================================================

fn make_custodes_schema() -> FactionSchema {
    FactionSchema {
        id: FactionId::new(0),
        name: "Tristraen's Gilded Blades".to_string(),
        faction_keywords: KeywordSet::from_keywords(&[
            Keyword::Imperium,
            Keyword::AdeptusCustodes,
        ]),
        faction_ability: AbilitySchema {
            name: "Martial Ka'tah".to_string(),
            description: "Each time a unit with this ability is selected to fight, choose one stance.".to_string(),
            ability_type: AbilityType::FactionAbility,
            effects: vec![RulePrimitive::StanceChoice {
                stances: vec![
                    StanceDef {
                        name: "Dacatarai".to_string(),
                        effects: vec![],
                        description: "Melee weapons have [SUSTAINED HITS 1].".to_string(),
                    },
                    StanceDef {
                        name: "Rendax".to_string(),
                        effects: vec![],
                        description: "Melee weapons have [LETHAL HITS].".to_string(),
                    },
                ],
            }],
            trigger: None,
        },
        datasheets: Vec::new(),
        stratagems: Vec::new(),
        enhancements: vec![
            EnhancementSchema {
                name: "Watchman of Terra".to_string(),
                description: "While in Engagement Range, OC becomes 4.".to_string(),
                is_default: true,
                effects: vec![],
                conditions: vec![],
            },
            EnhancementSchema {
                name: "Warrior Exemplar".to_string(),
                description: "Gain 1CP on 3+ when destroying enemy unit.".to_string(),
                is_default: false,
                effects: vec![],
                conditions: vec![],
            },
        ],
        secondary_objectives: Vec::new(),
    }
}

fn make_world_eaters_schema() -> FactionSchema {
    FactionSchema {
        id: FactionId::new(1),
        name: "Frenzied Reavers".to_string(),
        faction_keywords: KeywordSet::from_keywords(&[
            Keyword::Chaos,
            Keyword::Khorne,
            Keyword::WorldEaters,
        ]),
        faction_ability: AbilitySchema {
            name: "Blessings of Khorne".to_string(),
            description: "Roll 5D6 at start of battle round, allocate to blessings.".to_string(),
            ability_type: AbilityType::FactionAbility,
            effects: vec![RulePrimitive::DicePoolAllocation {
                num_dice: 5,
                max_blessings: 2,
                blessings: Vec::new(),
            }],
            trigger: None,
        },
        datasheets: Vec::new(),
        stratagems: Vec::new(),
        enhancements: vec![
            EnhancementSchema {
                name: "Fearsome Presence".to_string(),
                description: "OC 5 while not Battle-shocked.".to_string(),
                is_default: true,
                effects: vec![],
                conditions: vec![],
            },
            EnhancementSchema {
                name: "Bane of the Craven".to_string(),
                description: "Enemy Desperate Escape tests penalized.".to_string(),
                is_default: false,
                effects: vec![],
                conditions: vec![],
            },
        ],
        secondary_objectives: Vec::new(),
    }
}

fn load_ba_faction(filename: &str) -> BoardingFactionDef {
    let path = format!(
        "{}/../../../content/boarding_actions/factions/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        filename
    );
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    loader::load_faction(&json)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", filename, e))
}

// ===========================================================================
// COMBAT PATROL: Custodes — Ka'tah Stances
// ===========================================================================

/// Source: Custodes.md — Martial Ka'tah
/// Ka'tah stances (Dacatarai, Rendax) exist and are extracted from the
/// FactionSchema into the FactionInstance.
#[test]
fn custodes_katah_stances_exist_in_faction_instance() {
    let schema = make_custodes_schema();
    let instance = FactionRuntime::load_faction(&schema);

    assert_eq!(
        instance.available_stances.len(),
        2,
        "Custodes should have exactly 2 Ka'tah stances"
    );
    assert!(
        instance.available_stances.contains(&"Dacatarai".to_string()),
        "Custodes should have Dacatarai stance"
    );
    assert!(
        instance.available_stances.contains(&"Rendax".to_string()),
        "Custodes should have Rendax stance"
    );
}

/// Source: Custodes.md — Martial Ka'tah
/// TurnFlags tracks Ka'tah stance choices per unit using set_ka_tah_stance
/// and get_ka_tah_stance, and clears them on turn clear.
#[test]
fn custodes_katah_stances_tracked_in_turn_flags() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(0);
    let unit_b = UnitId::new(1);

    // Initially no stances set
    assert!(
        flags.get_ka_tah_stance(unit_a).is_none(),
        "No stance should be set initially"
    );

    // Set stances per unit — different units can have different stances
    flags.set_ka_tah_stance(unit_a, "Dacatarai".to_string());
    flags.set_ka_tah_stance(unit_b, "Rendax".to_string());

    assert_eq!(
        flags.get_ka_tah_stance(unit_a),
        Some("Dacatarai"),
        "Unit A should have Dacatarai stance"
    );
    assert_eq!(
        flags.get_ka_tah_stance(unit_b),
        Some("Rendax"),
        "Unit B should have Rendax stance"
    );

    // Clear resets all stance choices
    flags.clear();
    assert!(
        flags.get_ka_tah_stance(unit_a).is_none(),
        "Stance should be cleared after turn clear"
    );
    assert!(
        flags.get_ka_tah_stance(unit_b).is_none(),
        "Stance should be cleared after turn clear"
    );
}

// ===========================================================================
// COMBAT PATROL: Custodes — Vaultswords
// ===========================================================================

/// Source: Custodes.md — Tristraen's Vaultswords
/// TurnFlags tracks Vaultswords profile choices per model (Behemor, Hurricanus,
/// Victus) using set_vaultswords_profile and get_vaultswords_profile.
#[test]
fn custodes_vaultswords_profiles_tracked_in_turn_flags() {
    let mut flags = TurnFlags::new();
    let model_id = ModelId::new(0);

    assert!(
        flags.get_vaultswords_profile(model_id).is_none(),
        "No profile should be set initially"
    );

    // Set Behemor profile
    flags.set_vaultswords_profile(model_id, "Behemor".to_string());
    assert_eq!(
        flags.get_vaultswords_profile(model_id),
        Some("Behemor"),
        "Model should have Behemor profile"
    );

    // Override with Hurricanus
    flags.set_vaultswords_profile(model_id, "Hurricanus".to_string());
    assert_eq!(
        flags.get_vaultswords_profile(model_id),
        Some("Hurricanus"),
        "Model profile should be overwritten to Hurricanus"
    );

    // Override with Victus
    flags.set_vaultswords_profile(model_id, "Victus".to_string());
    assert_eq!(
        flags.get_vaultswords_profile(model_id),
        Some("Victus"),
        "Model profile should be overwritten to Victus"
    );

    // Clear resets all profile choices
    flags.clear();
    assert!(
        flags.get_vaultswords_profile(model_id).is_none(),
        "Profile should be cleared after turn clear"
    );
}

// ===========================================================================
// COMBAT PATROL: Custodes — Watchman of Terra enhancement
// ===========================================================================

/// Source: Custodes.md — Enhancements
/// The Custodes FactionSchema includes the Watchman of Terra enhancement
/// (default) and Warrior Exemplar (optional). Watchman of Terra overrides
/// Tristraen's OC to 4 while in Engagement Range.
#[test]
fn custodes_watchman_of_terra_enhancement_exists() {
    let schema = make_custodes_schema();
    let instance = FactionRuntime::load_faction(&schema);

    assert_eq!(instance.enhancements.len(), 2, "Custodes should have 2 enhancements");

    let watchman = instance
        .enhancements
        .iter()
        .find(|e| e.name == "Watchman of Terra");
    assert!(watchman.is_some(), "Watchman of Terra enhancement should exist");
    assert!(
        watchman.unwrap().is_default,
        "Watchman of Terra should be the default enhancement"
    );

    let exemplar = instance
        .enhancements
        .iter()
        .find(|e| e.name == "Warrior Exemplar");
    assert!(exemplar.is_some(), "Warrior Exemplar enhancement should exist");
    assert!(
        !exemplar.unwrap().is_default,
        "Warrior Exemplar should NOT be the default enhancement"
    );
}

// ===========================================================================
// COMBAT PATROL: Custodes — Unit Datasheets via ScenarioLoader
// ===========================================================================

/// Source: Custodes.md — Force Composition
/// ScenarioLoader creates Tristraen (Blade Champion), Custodian Guard, and
/// the default patrol squad (Custodian Wardens) for faction ID 0.
#[test]
fn custodes_unit_datasheets_load_correctly_default_squad() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0), // Custodes
        FactionId::new(1), // World Eaters
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let custodes_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(0))
        .collect();

    // Custodes default has 3 units: Tristraen, Custodian Guard, Custodian Wardens
    assert_eq!(
        custodes_units.len(),
        3,
        "Custodes should have 3 units (default squad)"
    );

    let tristraen = custodes_units.iter().find(|u| u.name == "Tristraen");
    assert!(tristraen.is_some(), "Tristraen (Blade Champion) should exist");
    let tristraen = tristraen.unwrap();
    assert_eq!(tristraen.models.len(), 1, "Tristraen is a single model");
    assert!(
        tristraen.has_keyword(Keyword::Character),
        "Tristraen should be a CHARACTER"
    );
    assert!(
        tristraen.has_keyword(Keyword::BladeChampion),
        "Tristraen should have BLADE CHAMPION keyword"
    );

    let guard = custodes_units.iter().find(|u| u.name == "Custodian Guard");
    assert!(guard.is_some(), "Custodian Guard should exist");
    let guard = guard.unwrap();
    assert_eq!(guard.models.len(), 3, "Custodian Guard should have 3 models");
    assert!(
        guard.has_keyword(Keyword::Battleline),
        "Custodian Guard should have BATTLELINE keyword"
    );

    let wardens = custodes_units
        .iter()
        .find(|u| u.name == "Custodian Wardens");
    assert!(wardens.is_some(), "Custodian Wardens (default squad) should exist");
    assert_eq!(
        wardens.unwrap().models.len(),
        3,
        "Custodian Wardens should have 3 models"
    );
}

/// Source: Custodes.md — Force Composition (Allarus alternative)
/// When patrol_squad_choice=1, Allarus Custodians (2 models, T7, TERMINATOR)
/// replace Custodian Wardens.
#[test]
fn custodes_unit_datasheets_allarus_squad() {
    let state = ScenarioLoader::load_scenario_with_squads(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [42u8; 32],
        Some(1), // Allarus for player A
        None,
    );

    let custodes_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(0))
        .collect();

    assert_eq!(
        custodes_units.len(),
        3,
        "Custodes should still have 3 units with Allarus"
    );

    let allarus = custodes_units
        .iter()
        .find(|u| u.name == "Allarus Custodians");
    assert!(allarus.is_some(), "Allarus Custodians should exist when squad choice is 1");
    let allarus = allarus.unwrap();
    assert_eq!(allarus.models.len(), 2, "Allarus should have 2 models");
    assert!(
        allarus.has_keyword(Keyword::Terminator),
        "Allarus should have TERMINATOR keyword"
    );

    // Wardens should not exist with Allarus choice
    let wardens = custodes_units
        .iter()
        .find(|u| u.name == "Custodian Wardens");
    assert!(
        wardens.is_none(),
        "Custodian Wardens should NOT exist when Allarus is chosen"
    );
}

// ===========================================================================
// COMBAT PATROL: World Eaters — Blessings of Khorne
// ===========================================================================

/// Source: Frenzied_Reavers.md — Blessings of Khorne
/// The Blessings of Khorne faction ability exists in the World Eaters faction
/// schema with a DicePoolAllocation effect (5 dice, max 2 blessings).
#[test]
fn world_eaters_blessings_of_khorne_exists() {
    let schema = make_world_eaters_schema();
    let instance = FactionRuntime::load_faction(&schema);

    assert_eq!(
        instance.faction_ability_name, "Blessings of Khorne",
        "World Eaters faction ability should be Blessings of Khorne"
    );
    // World Eaters have no stances (stances are Custodes only)
    assert!(
        instance.available_stances.is_empty(),
        "World Eaters should have no stances"
    );
    // Blessings start empty and are populated at round start
    assert!(
        instance.active_blessings.is_empty(),
        "Active blessings should start empty"
    );
}

/// Source: Frenzied_Reavers.md — Blessings of Khorne
/// The BlessingValidator correctly validates legal blessing allocations
/// using dice from a 5D6 roll, including Martial Excellence via Triple.
#[test]
fn world_eaters_blessings_valid_allocation_two_blessings() {
    // Roll: [4, 4, 3, 3, 6] — Double 4+ for Martial Excellence, Double 3+ for Total Carnage
    let dice = vec![4, 4, 3, 3, 6];
    let allocations = vec![
        BlessingAllocation {
            blessing: BlessingOfKhorne::MartialExcellence,
            dice_indices: vec![0, 1], // 4, 4 = Double 4+
        },
        BlessingAllocation {
            blessing: BlessingOfKhorne::TotalCarnage,
            dice_indices: vec![2, 3], // 3, 3 = Double 2+
        },
    ];

    let result = BlessingValidator::validate_allocation(&dice, &allocations);
    assert!(
        result.is_ok(),
        "Legal allocation of two blessings should pass: {:?}",
        result
    );
}

/// Source: Frenzied_Reavers.md — Blessings of Khorne
/// Allocations that don't meet blessing requirements are rejected.
#[test]
fn world_eaters_blessings_invalid_requirement_rejected() {
    // Roll: [2, 3, 4, 5, 6] — no matching pair for Double requirement
    let dice = vec![2, 3, 4, 5, 6];
    let allocations = vec![BlessingAllocation {
        blessing: BlessingOfKhorne::RageFuelledInvigoration,
        dice_indices: vec![0, 1], // 2, 3 — not a matching pair
    }];

    let result = BlessingValidator::validate_allocation(&dice, &allocations);
    assert!(
        result.is_err(),
        "Mismatched dice should not satisfy Double requirement"
    );
}

// ===========================================================================
// COMBAT PATROL: World Eaters — Blood Tithe Tracking
// ===========================================================================

/// Source: Frenzied_Reavers.md — Blessings of Khorne
/// FactionRoundFlags tracks active blessings and the dice rolled,
/// representing the "Blood Tithe" state for the round.
#[test]
fn world_eaters_blood_tithe_tracking() {
    use wh40k_game_core::state::FactionRoundFlags;

    let mut flags = FactionRoundFlags::default();

    // Initially empty
    assert!(flags.blessing_dice.is_empty(), "Dice should start empty");
    assert!(
        flags.active_blessings.is_empty(),
        "No blessings should be active initially"
    );
    assert!(
        !flags.has_blessing("Martial Excellence"),
        "Martial Excellence should not be active initially"
    );

    // Simulate a Blessings of Khorne roll
    flags.blessing_dice = vec![4, 4, 3, 3, 6];
    flags.add_blessing("Martial Excellence".to_string());
    flags.add_blessing("Total Carnage".to_string());

    assert!(flags.has_blessing("Martial Excellence"));
    assert!(flags.has_blessing("Total Carnage"));
    assert!(!flags.has_blessing("Rage-fuelled Invigoration"));

    // Adding same blessing again should not duplicate
    flags.add_blessing("Martial Excellence".to_string());
    assert_eq!(
        flags.active_blessings.len(),
        2,
        "Duplicate blessing add should not increase count"
    );

    // Clear resets for new round
    flags.clear();
    assert!(flags.blessing_dice.is_empty());
    assert!(flags.active_blessings.is_empty());
    assert!(!flags.has_blessing("Martial Excellence"));
}

// ===========================================================================
// COMBAT PATROL: World Eaters — Unit Datasheets via ScenarioLoader
// ===========================================================================

/// Source: Frenzied_Reavers.md — Force Composition
/// ScenarioLoader creates all 4 World Eaters units: Vorrakh (Daemon Prince),
/// Master of Executions, Khorne Berzerkers, Jakhals.
#[test]
fn world_eaters_unit_datasheets_load_correctly() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1), // World Eaters
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let we_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(1))
        .collect();

    assert_eq!(
        we_units.len(),
        4,
        "World Eaters should have 4 units"
    );

    // Vorrakh (Daemon Prince)
    let vorrakh = we_units.iter().find(|u| u.name == "Vorrakh");
    assert!(vorrakh.is_some(), "Vorrakh (Daemon Prince) should exist");
    let vorrakh = vorrakh.unwrap();
    assert_eq!(vorrakh.models.len(), 1, "Vorrakh is a single model");
    assert!(
        vorrakh.has_keyword(Keyword::Monster),
        "Vorrakh should have MONSTER keyword"
    );
    assert!(
        vorrakh.has_keyword(Keyword::Character),
        "Vorrakh should have CHARACTER keyword"
    );
    assert!(
        vorrakh.has_keyword(Keyword::Daemon),
        "Vorrakh should have DAEMON keyword"
    );

    // Master of Executions
    let moe = we_units
        .iter()
        .find(|u| u.name == "Master of Executions");
    assert!(moe.is_some(), "Master of Executions should exist");
    let moe = moe.unwrap();
    assert_eq!(moe.models.len(), 1, "Master of Executions is a single model");
    assert!(
        moe.has_keyword(Keyword::MasterOfExecutions),
        "MoE should have MASTER_OF_EXECUTIONS keyword"
    );

    // Khorne Berzerkers
    let berzerkers = we_units
        .iter()
        .find(|u| u.name == "Khorne Berzerkers");
    assert!(berzerkers.is_some(), "Khorne Berzerkers should exist");
    let berzerkers = berzerkers.unwrap();
    assert_eq!(
        berzerkers.models.len(),
        10,
        "Berzerkers should have 10 models"
    );
    assert!(
        berzerkers.has_keyword(Keyword::Battleline),
        "Berzerkers should have BATTLELINE keyword"
    );

    // Jakhals
    let jakhals = we_units.iter().find(|u| u.name == "Jakhals");
    assert!(jakhals.is_some(), "Jakhals should exist");
    assert_eq!(
        jakhals.unwrap().models.len(),
        10,
        "Jakhals should have 10 models"
    );
}

// ===========================================================================
// COMBAT PATROL: Both Factions — Keyword Validation
// ===========================================================================

/// Source: Custodes.md — All Custodes units should have ADEPTUS_CUSTODES keyword.
#[test]
fn custodes_all_units_have_adeptus_custodes_keyword() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let custodes_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(0))
        .collect();

    for unit in &custodes_units {
        assert!(
            unit.has_keyword(Keyword::AdeptusCustodes),
            "Custodes unit '{}' should have ADEPTUS_CUSTODES keyword",
            unit.name
        );
    }
}

/// Source: Frenzied_Reavers.md — All World Eaters units should have WORLD_EATERS keyword.
#[test]
fn world_eaters_all_units_have_world_eaters_keyword() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let we_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(1))
        .collect();

    for unit in &we_units {
        assert!(
            unit.has_keyword(Keyword::WorldEaters),
            "World Eaters unit '{}' should have WORLD_EATERS keyword",
            unit.name
        );
    }
}

/// Source: Frenzied_Reavers.md — All World Eaters units should have KHORNE keyword.
#[test]
fn world_eaters_all_units_have_khorne_keyword() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let we_units: Vec<_> = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(1))
        .collect();

    for unit in &we_units {
        assert!(
            unit.has_keyword(Keyword::Khorne),
            "World Eaters unit '{}' should have KHORNE keyword",
            unit.name
        );
    }
}

// ===========================================================================
// COMBAT PATROL: ScenarioLoader creates correct units for each faction
// ===========================================================================

/// Source: CP_Rules.md — ScenarioLoader creates different unit compositions
/// for Custodes (low model count, elite) vs World Eaters (high model count).
#[test]
fn scenario_loader_faction_model_counts_match_fluff() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0), // Custodes
        FactionId::new(1), // World Eaters
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    let custodes_total_models: usize = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(0))
        .map(|u| u.models.len())
        .sum();

    let we_total_models: usize = state
        .units
        .iter()
        .filter(|u| u.owner == PlayerId::new(1))
        .map(|u| u.models.len())
        .sum();

    // Custodes: 1 (Tristraen) + 3 (Guard) + 3 (Wardens) = 7 models
    assert_eq!(
        custodes_total_models, 7,
        "Custodes should have 7 models (default squad)"
    );

    // World Eaters: 1 (Vorrakh) + 1 (MoE) + 10 (Berzerkers) + 10 (Jakhals) = 22 models
    assert_eq!(
        we_total_models, 22,
        "World Eaters should have 22 models"
    );
}

// ===========================================================================
// BOARDING ACTIONS: Load all 6 BA faction JSON files
// ===========================================================================

/// Source: boarding_actions_complete_v3.md
/// All 6 BA faction JSON files should load without errors.
#[test]
fn ba_all_six_faction_json_files_load_successfully() {
    let faction_files = [
        "space_marines_terminator_assault",
        "world_eaters_boarding_butchers",
        "world_eaters_skullsworn",
        "csm_champions_of_chaos",
        "csm_underdeck_uprising",
        "astra_militarum_tempestus",
    ];

    for filename in &faction_files {
        let faction = load_ba_faction(filename);
        assert!(
            !faction.faction_name.is_empty(),
            "Faction '{}' should have a non-empty name",
            filename
        );
        assert!(
            !faction.faction_keyword.is_empty(),
            "Faction '{}' should have a non-empty faction keyword",
            filename
        );
        assert!(
            !faction.detachments.is_empty(),
            "Faction '{}' should have at least one detachment",
            filename
        );
    }
}

// ===========================================================================
// BOARDING ACTIONS: Correct datasheet counts per detachment
// ===========================================================================

/// Source: Space_Marines_BA_Terminator_Assault.md — 16 datasheets
/// Source: World_Eaters_BA_Boarding_Butchers.md — 8 datasheets
/// Source: World_Eaters_BA_Skullsworn.md — 5 datasheets
/// Source: Astra_Militarum_BA_Tempestus_Boarding_Regiment.md — 5 datasheets
#[test]
fn ba_each_faction_has_correct_datasheet_count() {
    let sm = load_ba_faction("space_marines_terminator_assault");
    assert_eq!(
        sm.datasheets.len(),
        16,
        "Space Marines Terminator Assault should have 16 datasheets"
    );

    let we_bb = load_ba_faction("world_eaters_boarding_butchers");
    assert_eq!(
        we_bb.datasheets.len(),
        8,
        "World Eaters Boarding Butchers should have 8 datasheets"
    );

    let we_ss = load_ba_faction("world_eaters_skullsworn");
    assert_eq!(
        we_ss.datasheets.len(),
        5,
        "World Eaters Skullsworn should have 5 datasheets"
    );

    let am = load_ba_faction("astra_militarum_tempestus");
    assert_eq!(
        am.datasheets.len(),
        5,
        "Astra Militarum Tempestus should have 5 datasheets"
    );

    // CSM factions have at least their minimum datasheet counts
    let csm_coc = load_ba_faction("csm_champions_of_chaos");
    assert!(
        csm_coc.datasheets.len() >= 6,
        "CSM Champions of Chaos should have at least 6 datasheets, got {}",
        csm_coc.datasheets.len()
    );

    let csm_uu = load_ba_faction("csm_underdeck_uprising");
    assert!(
        csm_uu.datasheets.len() >= 7,
        "CSM Underdeck Uprising should have at least 7 datasheets, got {}",
        csm_uu.datasheets.len()
    );
}

// ===========================================================================
// BOARDING ACTIONS: Each detachment has stratagems and enhancements
// ===========================================================================

/// Source: boarding_actions_complete_v3.md
/// Every BA detachment must have stratagems and enhancements defined.
#[test]
fn ba_each_detachment_has_stratagems_and_enhancements() {
    let faction_files = [
        "space_marines_terminator_assault",
        "world_eaters_boarding_butchers",
        "world_eaters_skullsworn",
        "csm_champions_of_chaos",
        "csm_underdeck_uprising",
        "astra_militarum_tempestus",
    ];

    for filename in &faction_files {
        let faction = load_ba_faction(filename);
        for det in &faction.detachments {
            assert_eq!(
                det.stratagems.len(),
                4,
                "{}/{} should have exactly 4 stratagems",
                filename,
                det.detachment_name
            );
            assert_eq!(
                det.enhancements.len(),
                2,
                "{}/{} should have exactly 2 enhancements",
                filename,
                det.detachment_name
            );
        }
    }
}

// ===========================================================================
// BOARDING ACTIONS: Mustering rules — allowed units match detachment
// ===========================================================================

/// Source: Space_Marines_BA_Terminator_Assault.md — 16 allowed units
/// Source: World_Eaters_BA_Boarding_Butchers.md — 8 allowed units
#[test]
fn ba_mustering_allowed_units_match_detachment_restrictions() {
    let sm = load_ba_faction("space_marines_terminator_assault");
    assert_eq!(
        sm.detachments[0].allowed_units.len(),
        16,
        "SM Terminator Assault should allow 16 unit types"
    );

    let we_bb = load_ba_faction("world_eaters_boarding_butchers");
    assert_eq!(
        we_bb.detachments[0].allowed_units.len(),
        8,
        "WE Boarding Butchers should allow 8 unit types"
    );

    let we_ss = load_ba_faction("world_eaters_skullsworn");
    assert_eq!(
        we_ss.detachments[0].allowed_units.len(),
        5,
        "WE Skullsworn should allow 5 unit types"
    );
}

// ===========================================================================
// BOARDING ACTIONS: Space Marines Terminator Assault — keyword checks
// ===========================================================================

/// Source: Space_Marines_BA_Terminator_Assault.md — Section 1
/// Faction keyword should be ADEPTUS ASTARTES and detachment is Terminator Assault.
#[test]
fn ba_sm_terminator_assault_faction_and_detachment_keywords() {
    let sm = load_ba_faction("space_marines_terminator_assault");

    assert_eq!(
        sm.faction_name, "Adeptus Astartes",
        "SM faction name should be 'Adeptus Astartes'"
    );
    assert_eq!(
        sm.faction_keyword, "ADEPTUS ASTARTES",
        "SM faction keyword should be 'ADEPTUS ASTARTES'"
    );
    assert_eq!(
        sm.detachments[0].detachment_name, "Terminator Assault",
        "Detachment name should be 'Terminator Assault'"
    );
}

/// Source: Space_Marines_BA_Terminator_Assault.md — Datasheets
/// All SM Terminator Assault datasheets should have at least one melee weapon
/// and valid profiles.
#[test]
fn ba_sm_terminator_assault_datasheets_have_valid_profiles() {
    let sm = load_ba_faction("space_marines_terminator_assault");

    for ds in &sm.datasheets {
        assert!(
            !ds.melee_weapons.is_empty(),
            "SM datasheet '{}' should have at least one melee weapon",
            ds.name
        );
        assert!(
            ds.profile.toughness > 0,
            "SM datasheet '{}' should have toughness > 0",
            ds.name
        );
        assert!(
            ds.profile.wounds > 0,
            "SM datasheet '{}' should have wounds > 0",
            ds.name
        );
        assert!(
            !ds.points.is_empty(),
            "SM datasheet '{}' should have at least one points cost entry",
            ds.name
        );
    }
}

/// Source: Space_Marines_BA_Terminator_Assault.md — Section 7
/// SM Terminator Assault should have specific stratagems:
/// Tower of Strength, Focusing Shrine, Carve a Path, Cleansing Sweep
#[test]
fn ba_sm_terminator_assault_specific_stratagems() {
    let sm = load_ba_faction("space_marines_terminator_assault");
    let det = &sm.detachments[0];

    let strat_names: Vec<&str> = det.stratagems.iter().map(|s| s.name.as_str()).collect();
    assert!(
        strat_names.contains(&"Tower of Strength"),
        "Missing stratagem 'Tower of Strength', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Focusing Shrine"),
        "Missing stratagem 'Focusing Shrine', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Carve a Path"),
        "Missing stratagem 'Carve a Path', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Cleansing Sweep"),
        "Missing stratagem 'Cleansing Sweep', got: {:?}",
        strat_names
    );
}

// ===========================================================================
// BOARDING ACTIONS: World Eaters Boarding Butchers — keyword checks
// ===========================================================================

/// Source: World_Eaters_BA_Boarding_Butchers.md — Section 1
/// Faction keyword should be WORLD EATERS and detachment is Boarding Butchers.
#[test]
fn ba_we_boarding_butchers_faction_and_detachment_keywords() {
    let we = load_ba_faction("world_eaters_boarding_butchers");

    assert_eq!(
        we.faction_name, "World Eaters",
        "WE faction name should be 'World Eaters'"
    );
    assert_eq!(
        we.faction_keyword, "WORLD EATERS",
        "WE faction keyword should be 'WORLD EATERS'"
    );
    assert_eq!(
        we.detachments[0].detachment_name, "Boarding Butchers",
        "Detachment name should be 'Boarding Butchers'"
    );
}

/// Source: World_Eaters_BA_Boarding_Butchers.md — Section 9
/// Boarding Butchers should have specific stratagems:
/// Terrifying Screams, Unstoppable Rage, Savage Resilience, Cowards' Bane
#[test]
fn ba_we_boarding_butchers_specific_stratagems() {
    let we = load_ba_faction("world_eaters_boarding_butchers");
    let det = &we.detachments[0];

    let strat_names: Vec<&str> = det.stratagems.iter().map(|s| s.name.as_str()).collect();
    assert!(
        strat_names.contains(&"Terrifying Screams"),
        "Missing stratagem 'Terrifying Screams', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Unstoppable Rage"),
        "Missing stratagem 'Unstoppable Rage', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Savage Resilience"),
        "Missing stratagem 'Savage Resilience', got: {:?}",
        strat_names
    );
    assert!(
        strat_names.contains(&"Cowards' Bane") || strat_names.contains(&"Coward's Bane"),
        "Missing stratagem 'Cowards' Bane', got: {:?}",
        strat_names
    );
}

/// Source: World_Eaters_BA_Boarding_Butchers.md — Section 8
/// Boarding Butchers enhancements: Chosen of Khorne, Battle Lust
#[test]
fn ba_we_boarding_butchers_specific_enhancements() {
    let we = load_ba_faction("world_eaters_boarding_butchers");
    let det = &we.detachments[0];

    let enh_names: Vec<&str> = det.enhancements.iter().map(|e| e.name.as_str()).collect();
    assert!(
        enh_names.contains(&"Chosen of Khorne"),
        "Missing enhancement 'Chosen of Khorne', got: {:?}",
        enh_names
    );
    assert!(
        enh_names.contains(&"Battle Lust"),
        "Missing enhancement 'Battle Lust', got: {:?}",
        enh_names
    );
}

/// Source: World_Eaters_BA_Boarding_Butchers.md — All WE datasheets
/// should have KHORNE in their keywords list.
#[test]
fn ba_we_boarding_butchers_datasheets_have_khorne_keyword() {
    let we = load_ba_faction("world_eaters_boarding_butchers");

    for ds in &we.datasheets {
        let has_khorne = ds
            .keywords
            .iter()
            .any(|k| k.to_uppercase().contains("KHORNE"));
        assert!(
            has_khorne,
            "WE Boarding Butchers datasheet '{}' should have KHORNE keyword, got: {:?}",
            ds.name, ds.keywords
        );
    }
}

// ===========================================================================
// BOARDING ACTIONS: CSM — both detachments have different allowed units
// ===========================================================================

/// Source: Chaos_Space_Marines_BA_Champions_of_Chaos.md,
///         Chaos_Space_Marines_BA_Underdeck_Uprising.md
/// The two CSM detachments (Champions of Chaos, Underdeck Uprising) should
/// have different allowed unit lists.
#[test]
fn ba_csm_detachments_have_different_allowed_units() {
    let coc = load_ba_faction("csm_champions_of_chaos");
    let uu = load_ba_faction("csm_underdeck_uprising");

    let coc_units: Vec<&str> = coc.detachments[0]
        .allowed_units
        .iter()
        .map(|u| u.datasheet_name.as_str())
        .collect();
    let uu_units: Vec<&str> = uu.detachments[0]
        .allowed_units
        .iter()
        .map(|u| u.datasheet_name.as_str())
        .collect();

    // Both should be HERETIC ASTARTES faction
    assert_eq!(
        coc.faction_keyword, "HERETIC ASTARTES",
        "CSM Champions of Chaos should be HERETIC ASTARTES"
    );
    assert_eq!(
        uu.faction_keyword, "HERETIC ASTARTES",
        "CSM Underdeck Uprising should be HERETIC ASTARTES"
    );

    // The detachment names should differ
    assert_ne!(
        coc.detachments[0].detachment_name,
        uu.detachments[0].detachment_name,
        "CSM detachment names should differ"
    );

    // The allowed unit lists should differ in content
    assert_ne!(
        coc_units, uu_units,
        "CSM Champions of Chaos and Underdeck Uprising should have different allowed units"
    );

    // They should have different total counts
    assert_ne!(
        coc_units.len(),
        uu_units.len(),
        "CSM detachments should have different allowed unit counts: CoC={}, UU={}",
        coc_units.len(),
        uu_units.len()
    );
}

/// Source: Chaos_Space_Marines_BA_Champions_of_Chaos.md,
///         Chaos_Space_Marines_BA_Underdeck_Uprising.md
/// Both CSM detachments should have their own unique stratagems.
#[test]
fn ba_csm_detachments_have_different_stratagems() {
    let coc = load_ba_faction("csm_champions_of_chaos");
    let uu = load_ba_faction("csm_underdeck_uprising");

    let coc_strats: Vec<&str> = coc.detachments[0]
        .stratagems
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    let uu_strats: Vec<&str> = uu.detachments[0]
        .stratagems
        .iter()
        .map(|s| s.name.as_str())
        .collect();

    assert_ne!(
        coc_strats, uu_strats,
        "CSM Champions of Chaos and Underdeck Uprising should have different stratagems"
    );
}

// ===========================================================================
// BOARDING ACTIONS: All datasheets have valid weapon and profile data
// ===========================================================================

/// Source: boarding_actions_complete_v3.md
/// Every BA datasheet across all factions must have at least one melee weapon,
/// non-zero toughness, non-zero wounds, and a points cost.
#[test]
fn ba_all_datasheets_have_valid_weapon_and_profile_data() {
    let faction_files = [
        "space_marines_terminator_assault",
        "world_eaters_boarding_butchers",
        "world_eaters_skullsworn",
        "csm_champions_of_chaos",
        "csm_underdeck_uprising",
        "astra_militarum_tempestus",
    ];

    for filename in &faction_files {
        let faction = load_ba_faction(filename);
        for ds in &faction.datasheets {
            assert!(
                !ds.melee_weapons.is_empty(),
                "{}/{} has no melee weapons",
                filename, ds.name
            );
            assert!(
                ds.profile.toughness > 0,
                "{}/{} has zero toughness",
                filename, ds.name
            );
            assert!(
                ds.profile.wounds > 0,
                "{}/{} has zero wounds",
                filename, ds.name
            );
            assert!(
                !ds.points.is_empty(),
                "{}/{} has no points cost",
                filename, ds.name
            );
        }
    }
}
