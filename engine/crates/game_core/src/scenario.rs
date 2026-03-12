//! Scenario loading and game initialization.
//!
//! Creates initial game state from faction selections, mission choice, and seed.
//!
//! Source: implementation_v3.md Section 8.1 (State model)
//! Source: CP_Rules.md - Combat Patrol game setup

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, DatasheetId, FactionId, GameOutcome, Keyword, KeywordSet,
    Leadership, MissionId, ModelId, MoveCharacteristic, ObjectiveControl, Phase, PlayerId,
    Position, SubPhase, Toughness, UnitId, Wounds,
};
use wh40k_dice::{DiceContext, DiceRoller, SeedBundle, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_command_system::CommandHistory;
use wh40k_geometry::{Board, ObjectiveMarker};

use crate::state::{GameState, PlayerState, TurnFlags};
use crate::unit::{ModelState, UnitState};

// ---------------------------------------------------------------------------
// ScenarioLoader
// ---------------------------------------------------------------------------

/// Loads a game scenario and creates the initial GameState.
///
/// Responsible for:
/// - Board setup (44"x30" for Combat Patrol)
/// - Objective placement per mission
/// - Unit creation from datasheets (or simple faction descriptions)
/// - Deployment zones
/// - Initial dice roller from seed
pub struct ScenarioLoader;

impl ScenarioLoader {
    /// Load a scenario and create the initial game state.
    ///
    /// # Arguments
    /// - `faction_a_id` - Faction ID for Player A (Attacker)
    /// - `faction_b_id` - Faction ID for Player B (Defender)
    /// - `mission_id` - Which mission to play (or None for a default setup)
    /// - `seed` - Root seed for deterministic dice
    ///
    /// # Returns
    /// A fully initialized GameState ready for the PreBattle phase.
    pub fn load_scenario(
        faction_a_id: FactionId,
        faction_b_id: FactionId,
        mission_id: Option<MissionId>,
        seed: [u8; 32],
    ) -> GameState {
        // Create the board
        let mut board = Board::combat_patrol();

        // Place objectives based on mission
        let objectives = Self::create_default_objectives();
        for obj in objectives {
            board.add_objective(obj);
        }

        // Create dice roller from seed
        let bundle = SeedBundle::new(seed, "game".to_string(), 0);
        let ctx = DiceContext::from_bundle(&bundle, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        // Create player states
        let player_a = {
            let mut p = PlayerState::new(PlayerId::new(0), "Player A".to_string());
            p.faction_id = Some(faction_a_id);
            p
        };
        let player_b = {
            let mut p = PlayerState::new(PlayerId::new(1), "Player B".to_string());
            p.faction_id = Some(faction_b_id);
            p
        };

        // Create units based on faction IDs
        let units_a = Self::create_faction_units(faction_a_id, PlayerId::new(0));
        let units_b = Self::create_faction_units(faction_b_id, PlayerId::new(1));

        let mut all_units = Vec::new();
        all_units.extend(units_a);
        all_units.extend(units_b);

        GameState {
            content_version: "v1.0.0-combat-patrol".to_string(),
            scenario_id: mission_id,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [player_a, player_b],
            units: all_units,
            board,
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        }
    }

    /// Create default objectives for a Combat Patrol mission.
    ///
    /// Standard Combat Patrol has objectives placed at:
    /// - Center of the board
    /// - Center of each player's half (roughly)
    ///
    /// For a 44"x30" board:
    /// - Objective A: (22", 15") - center
    /// - Objective B: (11", 8") - Player A's zone
    /// - Objective C: (33", 8") - Player A's zone
    /// - Objective D: (11", 22") - Player B's zone
    /// - Objective E: (33", 22") - Player B's zone
    fn create_default_objectives() -> Vec<ObjectiveMarker> {
        vec![
            ObjectiveMarker::new(
                wh40k_core_types::ObjectiveId::new(0),
                Position::from_inches(22, 15),
                "Center",
            ),
            ObjectiveMarker::new(
                wh40k_core_types::ObjectiveId::new(1),
                Position::from_inches(11, 8),
                "A",
            ),
            ObjectiveMarker::new(
                wh40k_core_types::ObjectiveId::new(2),
                Position::from_inches(33, 8),
                "B",
            ),
            ObjectiveMarker::new(
                wh40k_core_types::ObjectiveId::new(3),
                Position::from_inches(11, 22),
                "C",
            ),
            ObjectiveMarker::new(
                wh40k_core_types::ObjectiveId::new(4),
                Position::from_inches(33, 22),
                "D",
            ),
        ]
    }

    /// Create units for a given faction.
    ///
    /// This creates the Combat Patrol forces for the faction. The full content
    /// system will load datasheets from compiled content packs; this provides
    /// the initial implementation with hardcoded unit definitions for the
    /// two launch factions: World Eaters (Frenzied Reavers) and Custodes.
    ///
    /// FactionId(0) = Adeptus Custodes
    /// FactionId(1) = World Eaters (Frenzied Reavers)
    fn create_faction_units(faction_id: FactionId, owner: PlayerId) -> Vec<UnitState> {
        let base_unit_id = if owner == PlayerId::new(0) { 0 } else { 100 };
        let base_model_id = if owner == PlayerId::new(0) { 0 } else { 1000 };

        match faction_id.raw() {
            0 => Self::create_custodes_units(owner, base_unit_id, base_model_id),
            1 => Self::create_world_eaters_units(owner, base_unit_id, base_model_id),
            _ => {
                // Unknown faction: create a generic patrol
                Self::create_generic_units(owner, base_unit_id, base_model_id)
            }
        }
    }

    /// Create Adeptus Custodes Combat Patrol units.
    ///
    /// Source: Custodes.md
    /// - Tristraen (Blade Champion, 1 model, T6, W6, 2+/4++, OC2)
    /// - Custodian Guard (3 models, T6, W3, 2+/4++, OC2)
    /// - Custodian Wardens (3 models, T6, W3, 2+/4++, OC2)
    /// - Allarus Custodians (3 models, T7, W4, 2+/4++, OC2, Terminator)
    fn create_custodes_units(
        owner: PlayerId,
        base_uid: u32,
        base_mid: u32,
    ) -> Vec<UnitState> {
        let mut units = Vec::new();
        let mut model_counter = base_mid;

        // Tristraen (Blade Champion)
        {
            let unit_id = UnitId::new(base_uid);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(6),
                Position::from_inches(0, 0), // Undeployed
                BaseSize::MM40,
                Vec::new(), // Weapons loaded from content in Phase 2
                Vec::new(),
                true, // is leader
                None,
            );
            model_counter += 1;

            let unit = UnitState::new(
                unit_id,
                owner,
                "Tristraen".to_string(),
                DatasheetId::new(100),
                KeywordSet::TRISTRAEN_KEYWORDS,
                vec![model],
                MoveCharacteristic::from_inches(6),
                Toughness::new(6),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FOUR_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        // Custodian Guard (3 models)
        {
            let unit_id = UnitId::new(base_uid + 1);
            let models: Vec<ModelState> = (0..3)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(3),
                        Position::from_inches(0, 0),
                        BaseSize::MM40,
                        Vec::new(),
                        Vec::new(),
                        false,
                        None,
                    );
                    model_counter += 1;
                    m
                })
                .collect();

            let unit = UnitState::new(
                unit_id,
                owner,
                "Custodian Guard".to_string(),
                DatasheetId::new(101),
                KeywordSet::CUSTODIAN_GUARD_KEYWORDS,
                models,
                MoveCharacteristic::from_inches(6),
                Toughness::new(6),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FOUR_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        // Custodian Wardens (3 models)
        {
            let unit_id = UnitId::new(base_uid + 2);
            let models: Vec<ModelState> = (0..3)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(3),
                        Position::from_inches(0, 0),
                        BaseSize::MM40,
                        Vec::new(),
                        Vec::new(),
                        false,
                        None,
                    );
                    model_counter += 1;
                    m
                })
                .collect();

            let unit = UnitState::new(
                unit_id,
                owner,
                "Custodian Wardens".to_string(),
                DatasheetId::new(102),
                KeywordSet::CUSTODIAN_WARDENS_KEYWORDS,
                models,
                MoveCharacteristic::from_inches(6),
                Toughness::new(6),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FOUR_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        // Allarus Custodians (3 models, T7, W4, Terminator)
        {
            let unit_id = UnitId::new(base_uid + 3);
            let models: Vec<ModelState> = (0..3)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(4),
                        Position::from_inches(0, 0),
                        BaseSize::MM40,
                        Vec::new(),
                        Vec::new(),
                        false,
                        None,
                    );
                    model_counter += 1;
                    m
                })
                .collect();

            let unit = UnitState::new(
                unit_id,
                owner,
                "Allarus Custodians".to_string(),
                DatasheetId::new(103),
                KeywordSet::ALLARUS_CUSTODIANS_KEYWORDS,
                models,
                MoveCharacteristic::from_inches(5),
                Toughness::new(7),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FOUR_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        units
    }

    /// Create World Eaters (Frenzied Reavers) Combat Patrol units.
    ///
    /// Source: Frenzied_Reavers.md
    /// - Vorrakh (Daemon Prince, 1 model, T10, W10, 2+/5++, OC3, Monster)
    /// - Master of Executions (1 model, T4, W4, 3+, OC1, Character)
    /// - Khorne Berzerkers (5 models, T4, W2, 3+, OC2, Battleline)
    /// - Jakhals (10 models, T4, W1, 6+, OC1)
    fn create_world_eaters_units(
        owner: PlayerId,
        base_uid: u32,
        base_mid: u32,
    ) -> Vec<UnitState> {
        let mut units = Vec::new();
        let mut model_counter = base_mid;

        // Vorrakh (Daemon Prince)
        {
            let unit_id = UnitId::new(base_uid);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(10),
                Position::from_inches(0, 0),
                BaseSize::MM60,
                Vec::new(),
                Vec::new(),
                true,
                None,
            );
            model_counter += 1;

            let unit = UnitState::new(
                unit_id,
                owner,
                "Vorrakh".to_string(),
                DatasheetId::new(200),
                KeywordSet::VORRAKH_KEYWORDS,
                vec![model],
                MoveCharacteristic::from_inches(8),
                Toughness::new(10),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FIVE_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(3),
            );
            units.push(unit);
        }

        // Master of Executions
        {
            let unit_id = UnitId::new(base_uid + 1);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(4),
                Position::from_inches(0, 0),
                BaseSize::MM32,
                Vec::new(),
                Vec::new(),
                true,
                None,
            );
            model_counter += 1;

            let unit = UnitState::new(
                unit_id,
                owner,
                "Master of Executions".to_string(),
                DatasheetId::new(201),
                KeywordSet::MASTER_OF_EXECUTIONS_KEYWORDS,
                vec![model],
                MoveCharacteristic::from_inches(6),
                Toughness::new(4),
                ArmorSave::THREE_PLUS,
                None,
                Leadership::new(6),
                ObjectiveControl::new(1),
            );
            units.push(unit);
        }

        // Khorne Berzerkers (5 models)
        {
            let unit_id = UnitId::new(base_uid + 2);
            let models: Vec<ModelState> = (0..5)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(2),
                        Position::from_inches(0, 0),
                        BaseSize::MM32,
                        Vec::new(),
                        Vec::new(),
                        false,
                        None,
                    );
                    model_counter += 1;
                    m
                })
                .collect();

            let unit = UnitState::new(
                unit_id,
                owner,
                "Khorne Berzerkers".to_string(),
                DatasheetId::new(202),
                KeywordSet::BERZERKERS_KEYWORDS,
                models,
                MoveCharacteristic::from_inches(6),
                Toughness::new(4),
                ArmorSave::THREE_PLUS,
                None,
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        // Jakhals (10 models)
        {
            let unit_id = UnitId::new(base_uid + 3);
            let models: Vec<ModelState> = (0..10)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(1),
                        Position::from_inches(0, 0),
                        BaseSize::MM28,
                        Vec::new(),
                        Vec::new(),
                        false,
                        None,
                    );
                    model_counter += 1;
                    m
                })
                .collect();

            let unit = UnitState::new(
                unit_id,
                owner,
                "Jakhals".to_string(),
                DatasheetId::new(203),
                KeywordSet::JAKHALS_KEYWORDS,
                models,
                MoveCharacteristic::from_inches(6),
                Toughness::new(4),
                ArmorSave::SIX_PLUS,
                None,
                Leadership::new(7),
                ObjectiveControl::new(1),
            );
            units.push(unit);
        }

        units
    }

    /// Create generic units for an unknown faction (placeholder).
    fn create_generic_units(
        owner: PlayerId,
        base_uid: u32,
        base_mid: u32,
    ) -> Vec<UnitState> {
        let mut units = Vec::new();
        let mut model_counter = base_mid;

        // A single generic infantry unit with 5 models
        let unit_id = UnitId::new(base_uid);
        let models: Vec<ModelState> = (0..5)
            .map(|_| {
                let m = ModelState::new(
                    ModelId::new(model_counter),
                    unit_id,
                    Wounds::new(1),
                    Position::from_inches(0, 0),
                    BaseSize::MM32,
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                );
                model_counter += 1;
                m
            })
            .collect();

        let unit = UnitState::new(
            unit_id,
            owner,
            "Generic Infantry".to_string(),
            DatasheetId::new(999),
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            models,
            MoveCharacteristic::from_inches(6),
            Toughness::new(3),
            ArmorSave::FIVE_PLUS,
            None,
            Leadership::new(7),
            ObjectiveControl::new(2),
        );
        units.push(unit);

        units
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::UnitStatus;

    #[test]
    fn test_load_custodes_vs_world_eaters() {
        let seed = [42u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0), // Custodes
            FactionId::new(1), // World Eaters
            None,
            seed,
        );

        // Verify basic state
        assert_eq!(state.content_version, "v1.0.0-combat-patrol");
        assert_eq!(state.current_phase, Phase::PreBattle);
        assert_eq!(state.battle_round, BattleRound::new(1));
        assert!(state.is_in_progress());

        // Verify players
        assert_eq!(state.players[0].faction_id, Some(FactionId::new(0)));
        assert_eq!(state.players[1].faction_id, Some(FactionId::new(1)));

        // Verify board
        assert_eq!(state.board.width().whole_inches(), 44);
        assert_eq!(state.board.height().whole_inches(), 30);

        // Verify objectives
        assert_eq!(state.board.objective_markers().len(), 5);

        // Verify Custodes units (4 units)
        let custodes_units = state.units_for_player(PlayerId::new(0));
        assert_eq!(custodes_units.len(), 4);

        // Tristraen
        let tristraen = custodes_units.iter().find(|u| u.name == "Tristraen").unwrap();
        assert_eq!(tristraen.models.len(), 1);
        assert_eq!(tristraen.base_toughness.value(), 6);
        assert_eq!(tristraen.models[0].wounds_max.value(), 6);
        assert!(tristraen.is_character());
        assert!(tristraen.has_keyword(Keyword::AdeptusCustodes));

        // Custodian Guard
        let guard = custodes_units
            .iter()
            .find(|u| u.name == "Custodian Guard")
            .unwrap();
        assert_eq!(guard.models.len(), 3);
        assert!(guard.is_battleline());

        // Allarus Custodians
        let allarus = custodes_units
            .iter()
            .find(|u| u.name == "Allarus Custodians")
            .unwrap();
        assert_eq!(allarus.models.len(), 3);
        assert_eq!(allarus.base_toughness.value(), 7);
        assert_eq!(allarus.models[0].wounds_max.value(), 4);
        assert!(allarus.has_keyword(Keyword::Terminator));

        // Verify World Eaters units (4 units)
        let we_units = state.units_for_player(PlayerId::new(1));
        assert_eq!(we_units.len(), 4);

        // Vorrakh
        let vorrakh = we_units.iter().find(|u| u.name == "Vorrakh").unwrap();
        assert_eq!(vorrakh.models.len(), 1);
        assert_eq!(vorrakh.base_toughness.value(), 10);
        assert_eq!(vorrakh.models[0].wounds_max.value(), 10);
        assert!(vorrakh.is_character());
        assert!(vorrakh.is_monster());
        assert!(vorrakh.has_keyword(Keyword::Daemon));

        // Berzerkers
        let berzerkers = we_units
            .iter()
            .find(|u| u.name == "Khorne Berzerkers")
            .unwrap();
        assert_eq!(berzerkers.models.len(), 5);
        assert!(berzerkers.is_battleline());

        // Jakhals
        let jakhals = we_units.iter().find(|u| u.name == "Jakhals").unwrap();
        assert_eq!(jakhals.models.len(), 10);
        assert_eq!(jakhals.models[0].wounds_max.value(), 1);

        // Master of Executions
        let moe = we_units
            .iter()
            .find(|u| u.name == "Master of Executions")
            .unwrap();
        assert_eq!(moe.models.len(), 1);
        assert!(moe.is_character());
    }

    #[test]
    fn test_load_scenario_serialization() {
        let seed = [1u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            Some(MissionId::new(1)),
            seed,
        );

        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();

        assert_eq!(back.content_version, state.content_version);
        assert_eq!(back.scenario_id, Some(MissionId::new(1)));
        assert_eq!(back.units.len(), state.units.len());
    }

    #[test]
    fn test_load_scenario_deterministic() {
        let seed = [99u8; 32];

        let state1 = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
        );
        let state2 = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
        );

        // Same seed should produce the same state
        assert_eq!(state1.units.len(), state2.units.len());
        for (u1, u2) in state1.units.iter().zip(state2.units.iter()) {
            assert_eq!(u1.id, u2.id);
            assert_eq!(u1.name, u2.name);
            assert_eq!(u1.models.len(), u2.models.len());
        }
    }

    #[test]
    fn test_load_scenario_generic_faction() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(99), // Unknown
            FactionId::new(99), // Unknown
            None,
            seed,
        );

        // Should still create a valid game state with generic units
        assert!(!state.units.is_empty());
        assert!(state.is_in_progress());
    }

    #[test]
    fn test_default_objectives() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
        );

        let objectives = state.board.objective_markers();
        assert_eq!(objectives.len(), 5);

        // Center objective
        assert_eq!(objectives[0].position, Position::from_inches(22, 15));
    }

    #[test]
    fn test_all_units_start_undeployed() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
        );

        for unit in &state.units {
            assert_eq!(unit.status, UnitStatus::Undeployed);
        }
    }

    #[test]
    fn test_custodes_unit_count() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(0),
            None,
            seed,
        );

        // Both players are Custodes: 4 units each = 8 total
        assert_eq!(state.units.len(), 8);
    }

    #[test]
    fn test_world_eaters_total_models() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario(
            FactionId::new(1), // World Eaters
            FactionId::new(0), // Custodes
            None,
            seed,
        );

        let we_units = state.units_for_player(PlayerId::new(0));
        let total_models: usize = we_units.iter().map(|u| u.models.len()).sum();
        // Vorrakh(1) + MoE(1) + Berzerkers(5) + Jakhals(10) = 17
        assert_eq!(total_models, 17);
    }
}
