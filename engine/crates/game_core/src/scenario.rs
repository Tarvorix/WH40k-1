//! Scenario loading and game initialization.
//!
//! Creates initial game state from faction selections, mission choice, and seed.
//!
//! Source: implementation_v3.md Section 8.1 (State model)
//! Source: CP_Rules.md - Combat Patrol game setup

use wh40k_core_types::{
    ArmorPenetration, ArmorSave, AttackCount, BaseSize, BattleRound, BoardDimensions, Damage,
    DatasheetId, FactionId, GameOutcome, Inches, Keyword, KeywordSet, Leadership, MissionId,
    ModelId, MoveCharacteristic, ObjectiveControl, Phase, PlayerId, Position, Skill, Strength,
    SubPhase, Toughness, UnitId, WeaponAbility, WeaponAbilitySet, WeaponId, WeaponProfile,
    WeaponType, Wounds,
};
use wh40k_dice::{DiceContext, DiceRoller, SeedBundle, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_command_system::CommandHistory;
use wh40k_geometry::{
    Board, DeploymentConfig, ObjectiveMarker,
    create_standard_deployment, create_search_and_destroy_deployment,
};

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
    /// - `patrol_squad_a` - Patrol squad choice for Player A (-1 or None = default/Wardens)
    /// - `patrol_squad_b` - Patrol squad choice for Player B (-1 or None = default/Wardens)
    ///
    /// # Returns
    /// A fully initialized GameState ready for the PreBattle phase.
    pub fn load_scenario(
        faction_a_id: FactionId,
        faction_b_id: FactionId,
        mission_id: Option<MissionId>,
        seed: [u8; 32],
    ) -> GameState {
        Self::load_scenario_with_squads(faction_a_id, faction_b_id, mission_id, seed, None, None)
    }

    /// Load a scenario with patrol squad selections.
    ///
    /// # Arguments
    /// - `faction_a_id` - Faction ID for Player A (Attacker)
    /// - `faction_b_id` - Faction ID for Player B (Defender)
    /// - `mission_id` - Which mission to play (or None for a default setup)
    /// - `seed` - Root seed for deterministic dice
    /// - `patrol_squad_a` - Patrol squad choice for Player A (Some(0)=Wardens, Some(1)=Allarus, None=default)
    /// - `patrol_squad_b` - Patrol squad choice for Player B (Some(0)=Wardens, Some(1)=Allarus, None=default)
    ///
    /// # Returns
    /// A fully initialized GameState ready for the PreBattle phase.
    pub fn load_scenario_with_squads(
        faction_a_id: FactionId,
        faction_b_id: FactionId,
        mission_id: Option<MissionId>,
        seed: [u8; 32],
        patrol_squad_a: Option<usize>,
        patrol_squad_b: Option<usize>,
    ) -> GameState {
        // Create the board
        let mut board = Board::combat_patrol();

        // Place objectives based on mission
        let objectives = Self::create_mission_objectives(mission_id);
        for obj in objectives {
            board.add_objective(obj);
        }

        // Create deployment zones based on mission deployment map type
        // Source: CP_Rules.md - Deployment Maps
        // Player A (PlayerId 0) = Attacker, Player B (PlayerId 1) = Defender
        let attacker_id = PlayerId::new(0);
        let defender_id = PlayerId::new(1);
        let deployment_config = Self::create_deployment_config(
            mission_id,
            &board.dimensions,
            attacker_id,
            defender_id,
        );

        // Create dice roller from seed
        let bundle = SeedBundle::new(seed, "game".to_string(), 0);
        let ctx = DiceContext::from_bundle(&bundle, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        // Create player states
        let mut player_a = {
            let mut p = PlayerState::new(PlayerId::new(0), "Player A".to_string());
            p.faction_id = Some(faction_a_id);
            p
        };
        let mut player_b = {
            let mut p = PlayerState::new(PlayerId::new(1), "Player B".to_string());
            p.faction_id = Some(faction_b_id);
            p
        };

        // Store patrol squad choices on players
        if let Some(sq) = patrol_squad_a {
            player_a.patrol_squad_choice = Some(sq);
        }
        if let Some(sq) = patrol_squad_b {
            player_b.patrol_squad_choice = Some(sq);
        }

        // Create units based on faction IDs and patrol squad choices
        let units_a = Self::create_faction_units(faction_a_id, PlayerId::new(0), patrol_squad_a);
        let units_b = Self::create_faction_units(faction_b_id, PlayerId::new(1), patrol_squad_b);

        let mut all_units = Vec::new();
        all_units.extend(units_a);
        all_units.extend(units_b);

        // Defender deploys first in Combat Patrol
        // Source: CP_Rules.md - "The Defender sets up their army first"
        // Set decision_owner to defender initially for deployment
        let initial_decision_owner = defender_id;

        GameState {
            content_version: "v1.0.0-combat-patrol".to_string(),
            scenario_id: mission_id,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: SubPhase::Deployment,
            decision_owner: initial_decision_owner,
            players: [player_a, player_b],
            units: all_units,
            board,
            deployment_config: Some(deployment_config),
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

    /// Create deployment configuration based on mission.
    ///
    /// Source: CP_Rules.md - Deployment Maps
    /// Mission 1 (Clash of Patrols): Search & Destroy deployment
    /// Missions 2-6: Standard deployment (9" strips)
    /// Default/None: Standard deployment
    fn create_deployment_config(
        mission_id: Option<MissionId>,
        board_dims: &BoardDimensions,
        attacker_id: PlayerId,
        defender_id: PlayerId,
    ) -> DeploymentConfig {
        match mission_id.map(|m| m.raw()) {
            Some(1) => {
                // Mission 1: Clash of Patrols — Search & Destroy deployment
                create_search_and_destroy_deployment(board_dims, attacker_id, defender_id)
            }
            _ => {
                // Missions 2-6 and default: Standard deployment (9" strips)
                create_standard_deployment(board_dims, attacker_id, defender_id)
            }
        }
    }

    /// Create objectives for the specified mission, or default objectives if no
    /// mission is specified.
    ///
    /// Each Combat Patrol mission has its own objective layout per CP_Rules.md §13.
    /// When no mission_id is provided, falls back to default 5-objective layout.
    ///
    /// Source: CP_Rules.md Section 13 - Combat Patrol Missions
    fn create_mission_objectives(mission_id: Option<MissionId>) -> Vec<ObjectiveMarker> {
        match mission_id.map(|m| m.raw()) {
            // Mission 1: Clash of Patrols — Search & Destroy style with 4 objectives
            // Source: CP_Rules.md §13, Mission 1
            // A (Attacker zone), B (Defender zone), C (NML left), D (NML right)
            Some(1) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(22, 9),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(22, 21),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(11, 15),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(33, 15),
                    "D",
                ),
            ],

            // Mission 2: Archeotech Recovery — Cross deployment with objectives
            // in NML and deployment zones
            // Source: CP_Rules.md §13, Mission 2
            // A (Attacker zone), B (Defender zone), C (NML), D (NML)
            Some(2) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(11, 6),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(33, 24),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(15, 15),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(29, 15),
                    "D",
                ),
            ],

            // Mission 3: Forward Outpost — 2 NML objectives + 1 in each DZ
            // Source: CP_Rules.md §13, Mission 3
            // A (Attacker DZ), B (Defender DZ), C (NML left), D (NML right)
            Some(3) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(22, 6),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(22, 24),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(11, 15),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(33, 15),
                    "D",
                ),
            ],

            // Mission 4: Scorched Earth — Multiple objectives across battlefield
            // with protected home objectives
            // Source: CP_Rules.md §13, Mission 4
            // A (Attacker zone, cannot be razed by Attacker),
            // B (Defender zone, cannot be razed by Defender),
            // C (NML), D (NML)
            Some(4) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(11, 6),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(33, 24),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(15, 15),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(29, 15),
                    "D",
                ),
            ],

            // Mission 5: Sweeping Raid — 4 objectives in line/diagonal pattern
            // Source: CP_Rules.md §13, Mission 5
            // A (Attacker zone, Defender scores 10VP at end),
            // B (Attacker zone, Defender scores 5VP at end),
            // C (Defender zone, Attacker scores 5VP at end),
            // D (Defender zone, Attacker scores 10VP at end)
            Some(5) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(11, 6),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(33, 6),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(11, 24),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(33, 24),
                    "D",
                ),
            ],

            // Mission 6: Display of Might — NML objectives (symbolic sites) + DZ objectives
            // Source: CP_Rules.md §13, Mission 6
            // A (Attacker zone), B (Defender zone),
            // C (NML, symbolic site), D (NML, symbolic site)
            Some(6) => vec![
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(0),
                    Position::from_inches(22, 6),
                    "A",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(1),
                    Position::from_inches(22, 24),
                    "B",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(2),
                    Position::from_inches(11, 15),
                    "C",
                ),
                ObjectiveMarker::new(
                    wh40k_core_types::ObjectiveId::new(3),
                    Position::from_inches(33, 15),
                    "D",
                ),
            ],

            // No mission or unrecognized mission: default 5-objective layout
            _ => Self::create_default_objectives(),
        }
    }

    /// Create default objectives for a Combat Patrol mission (fallback layout).
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
    ///
    /// `patrol_squad_choice`: For Custodes, Some(0) = Wardens, Some(1) = Allarus.
    /// None defaults to Wardens (squad 0). World Eaters have all fixed units.
    fn create_faction_units(faction_id: FactionId, owner: PlayerId, patrol_squad_choice: Option<usize>) -> Vec<UnitState> {
        let base_unit_id = if owner == PlayerId::new(0) { 0 } else { 100 };
        let base_model_id = if owner == PlayerId::new(0) { 0 } else { 1000 };

        match faction_id.raw() {
            0 => Self::create_custodes_units(owner, base_unit_id, base_model_id, patrol_squad_choice),
            1 => Self::create_world_eaters_units(owner, base_unit_id, base_model_id),
            _ => {
                // Unknown faction: create a generic patrol
                Self::create_generic_units(owner, base_unit_id, base_model_id)
            }
        }
    }

    /// Create Adeptus Custodes Combat Patrol units.
    ///
    /// Source: Custodes.md §2 (Force Composition)
    /// Fixed units:
    /// - Tristraen (Blade Champion, 1 model, T6, W6, 2+/4++, OC2)
    /// - Custodian Guard (3 models, T6, W4, 2+/4++, OC2) — W3 base +1 Praesidium Shield
    /// Choose One (patrol_squad_choice):
    /// - 0 (default): Custodian Wardens (3 models, T6, W3, 2+/4++, OC2)
    /// - 1: Allarus Custodians (2 models, T7, W4, 2+/4++, OC2, Terminator)
    fn create_custodes_units(
        owner: PlayerId,
        base_uid: u32,
        base_mid: u32,
        patrol_squad_choice: Option<usize>,
    ) -> Vec<UnitState> {
        let mut units = Vec::new();
        let mut model_counter = base_mid;

        // Tristraen (Blade Champion)
        // Source: Custodes.md - Tristraen datasheet
        // Melee only: Vaultswords with 3 profiles (Behemor, Hurricanus, Victus)
        {
            let unit_id = UnitId::new(base_uid);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(6),
                Position::from_inches(0, 0), // Undeployed
                BaseSize::MM40,
                Vec::new(), // No ranged weapons
                vec![
                    // Vaultswords – Behemor: 6A, WS2+, S7, AP-2, D2 [PRECISION]
                    WeaponProfile {
                        id: WeaponId::new(1000),
                        name: "Vaultswords – Behemor".to_string(),
                        weapon_type: WeaponType::Melee,
                        range: Inches::ZERO,
                        attacks: AttackCount::Fixed(6),
                        skill: Skill::TWO_PLUS,
                        strength: Strength::new(7),
                        ap: ArmorPenetration::MINUS_2,
                        damage: Damage::Fixed(2),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::Precision,
                        ]),
                    },
                    // Vaultswords – Hurricanus: 9A, WS2+, S5, AP-1, D1 [SUSTAINED HITS 1]
                    WeaponProfile {
                        id: WeaponId::new(1001),
                        name: "Vaultswords – Hurricanus".to_string(),
                        weapon_type: WeaponType::Melee,
                        range: Inches::ZERO,
                        attacks: AttackCount::Fixed(9),
                        skill: Skill::TWO_PLUS,
                        strength: Strength::new(5),
                        ap: ArmorPenetration::MINUS_1,
                        damage: Damage::Fixed(1),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::SustainedHits(1),
                        ]),
                    },
                    // Vaultswords – Victus: 5A, WS2+, S6, AP-3, D3 [DEVASTATING WOUNDS]
                    WeaponProfile {
                        id: WeaponId::new(1002),
                        name: "Vaultswords – Victus".to_string(),
                        weapon_type: WeaponType::Melee,
                        range: Inches::ZERO,
                        attacks: AttackCount::Fixed(5),
                        skill: Skill::TWO_PLUS,
                        strength: Strength::new(6),
                        ap: ArmorPenetration::MINUS_3,
                        damage: Damage::Fixed(3),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::DevastatingWounds,
                        ]),
                    },
                ],
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
        // Praesidium Shield: Add 1 to the bearer's Wounds characteristic (3 base + 1 = 4)
        // Source: Custodes.md - Custodian Guard datasheet
        // Each model: Ranged sentinel blade (12", 2A, BS2+, S4, AP-1, D2, [ASSAULT, PISTOL])
        //             Melee sentinel blade (5A, WS2+, S6, AP-2, D1)
        {
            let unit_id = UnitId::new(base_uid + 1);
            let models: Vec<ModelState> = (0..3)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(4),
                        Position::from_inches(0, 0),
                        BaseSize::MM40,
                        vec![
                            // Sentinel blade (ranged): 12", 2A, BS2+, S4, AP-1, D2 [ASSAULT, PISTOL]
                            WeaponProfile {
                                id: WeaponId::new(1003),
                                name: "Sentinel blade".to_string(),
                                weapon_type: WeaponType::Ranged,
                                range: Inches::from_inches(12),
                                attacks: AttackCount::Fixed(2),
                                skill: Skill::TWO_PLUS,
                                strength: Strength::new(4),
                                ap: ArmorPenetration::MINUS_1,
                                damage: Damage::Fixed(2),
                                abilities: WeaponAbilitySet::from_abilities(vec![
                                    WeaponAbility::Assault,
                                    WeaponAbility::Pistol,
                                ]),
                            },
                        ],
                        vec![
                            // Sentinel blade (melee): 5A, WS2+, S6, AP-2, D1
                            WeaponProfile {
                                id: WeaponId::new(1004),
                                name: "Sentinel blade".to_string(),
                                weapon_type: WeaponType::Melee,
                                range: Inches::ZERO,
                                attacks: AttackCount::Fixed(5),
                                skill: Skill::TWO_PLUS,
                                strength: Strength::new(6),
                                ap: ArmorPenetration::MINUS_2,
                                damage: Damage::Fixed(1),
                                abilities: WeaponAbilitySet::new(),
                            },
                        ],
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

        // Patrol Squad: Choose One
        // Source: Custodes.md §2 - "Choose One (Patrol Squads)"
        // patrol_squad_choice: 0 (default) = Custodian Wardens, 1 = Allarus Custodians
        let squad_choice = patrol_squad_choice.unwrap_or(0);

        if squad_choice == 0 {
        // Custodian Wardens (3 models)
        // Source: Custodes.md - Custodian Wardens datasheet
        // Model 0: Castellan axe (ranged 24", 2A, BS2+, S4, AP-1, D2 [ASSAULT])
        //          Castellan axe (melee 4A, WS2+, S9, AP-1, D3)
        // Models 1-2: Guardian spear (ranged 24", 2A, BS2+, S4, AP-1, D2 [ASSAULT])
        //             Guardian spear (melee 5A, WS2+, S7, AP-2, D2)
        {
            let unit_id = UnitId::new(base_uid + 2);
            let models: Vec<ModelState> = (0..3)
                .map(|i| {
                    let m = if i == 0 {
                        // Castellan axe model
                        ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(3),
                            Position::from_inches(0, 0),
                            BaseSize::MM40,
                            vec![
                                // Castellan axe (ranged): 24", 2A, BS2+, S4, AP-1, D2 [ASSAULT]
                                WeaponProfile {
                                    id: WeaponId::new(1005),
                                    name: "Castellan axe".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(24),
                                    attacks: AttackCount::Fixed(2),
                                    skill: Skill::TWO_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Assault,
                                    ]),
                                },
                            ],
                            vec![
                                // Castellan axe (melee): 4A, WS2+, S9, AP-1, D3
                                WeaponProfile {
                                    id: WeaponId::new(1006),
                                    name: "Castellan axe".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(4),
                                    skill: Skill::TWO_PLUS,
                                    strength: Strength::new(9),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(3),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        )
                    } else {
                        // Guardian spear models
                        ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(3),
                            Position::from_inches(0, 0),
                            BaseSize::MM40,
                            vec![
                                // Guardian spear (ranged): 24", 2A, BS2+, S4, AP-1, D2 [ASSAULT]
                                WeaponProfile {
                                    id: WeaponId::new(1007),
                                    name: "Guardian spear".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(24),
                                    attacks: AttackCount::Fixed(2),
                                    skill: Skill::TWO_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Assault,
                                    ]),
                                },
                            ],
                            vec![
                                // Guardian spear (melee): 5A, WS2+, S7, AP-2, D2
                                WeaponProfile {
                                    id: WeaponId::new(1008),
                                    name: "Guardian spear".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(5),
                                    skill: Skill::TWO_PLUS,
                                    strength: Strength::new(7),
                                    ap: ArmorPenetration::MINUS_2,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        )
                    };
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
        } else {
        // Allarus Custodians (2 models, T7, W4, Terminator)
        // Source: Custodes.md - Allarus Custodians datasheet
        // Each model: Ranged balistus grenade launcher (18", D6A, BS2+, S4, AP-1, D1, [BLAST])
        //             Ranged guardian spear (24", 2A, BS2+, S4, AP-1, D2, [ASSAULT])
        //             Melee guardian spear (5A, WS2+, S7, AP-2, D2)
        {
            let unit_id = UnitId::new(base_uid + 3);
            let models: Vec<ModelState> = (0..2)
                .map(|_i| {
                    let m = ModelState::new(
                        ModelId::new(model_counter),
                        unit_id,
                        Wounds::new(4),
                        Position::from_inches(0, 0),
                        BaseSize::MM40,
                        vec![
                            // Balistus grenade launcher: 18", D6A, BS2+, S4, AP-1, D1 [BLAST]
                            WeaponProfile {
                                id: WeaponId::new(1009),
                                name: "Balistus grenade launcher".to_string(),
                                weapon_type: WeaponType::Ranged,
                                range: Inches::from_inches(18),
                                attacks: AttackCount::D6,
                                skill: Skill::TWO_PLUS,
                                strength: Strength::new(4),
                                ap: ArmorPenetration::MINUS_1,
                                damage: Damage::Fixed(1),
                                abilities: WeaponAbilitySet::from_abilities(vec![
                                    WeaponAbility::Blast,
                                ]),
                            },
                            // Guardian spear (ranged): 24", 2A, BS2+, S4, AP-1, D2 [ASSAULT]
                            WeaponProfile {
                                id: WeaponId::new(1010),
                                name: "Guardian spear".to_string(),
                                weapon_type: WeaponType::Ranged,
                                range: Inches::from_inches(24),
                                attacks: AttackCount::Fixed(2),
                                skill: Skill::TWO_PLUS,
                                strength: Strength::new(4),
                                ap: ArmorPenetration::MINUS_1,
                                damage: Damage::Fixed(2),
                                abilities: WeaponAbilitySet::from_abilities(vec![
                                    WeaponAbility::Assault,
                                ]),
                            },
                        ],
                        vec![
                            // Guardian spear (melee): 5A, WS2+, S7, AP-2, D2
                            WeaponProfile {
                                id: WeaponId::new(1011),
                                name: "Guardian spear".to_string(),
                                weapon_type: WeaponType::Melee,
                                range: Inches::ZERO,
                                attacks: AttackCount::Fixed(5),
                                skill: Skill::TWO_PLUS,
                                strength: Strength::new(7),
                                ap: ArmorPenetration::MINUS_2,
                                damage: Damage::Fixed(2),
                                abilities: WeaponAbilitySet::new(),
                            },
                        ],
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
        } // end patrol squad choice if/else

        units
    }

    /// Create World Eaters (Frenzied Reavers) Combat Patrol units.
    ///
    /// Source: Frenzied_Reavers.md
    /// - Vorrakh (Daemon Prince, 1 model, T10, W10, 2+/4++, OC3, Monster)
    /// - Master of Executions (1 model, T4, W4, 3+, OC1, Character)
    /// - Khorne Berzerkers (10 models, T4, W2, 3+, OC2, Battleline)
    /// - Jakhals (10 models, T4, W1, 6+, OC1)
    fn create_world_eaters_units(
        owner: PlayerId,
        base_uid: u32,
        base_mid: u32,
    ) -> Vec<UnitState> {
        let mut units = Vec::new();
        let mut model_counter = base_mid;

        // Vorrakh (Daemon Prince)
        // Source: Frenzied_Reavers.md - Vorrakh datasheet
        // Ranged: Infernal cannon (24", 3A, BS3+, S5, AP-1, D2, [RAPID FIRE 1])
        // Melee: Hellforged weapons (16A, WS2+, S6, AP-1, D1)
        {
            let unit_id = UnitId::new(base_uid);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(10),
                Position::from_inches(0, 0),
                BaseSize::MM60,
                vec![
                    // Infernal cannon: 24", 3A, BS3+, S5, AP-1, D2 [RAPID FIRE 1]
                    WeaponProfile {
                        id: WeaponId::new(2000),
                        name: "Infernal cannon".to_string(),
                        weapon_type: WeaponType::Ranged,
                        range: Inches::from_inches(24),
                        attacks: AttackCount::Fixed(3),
                        skill: Skill::THREE_PLUS,
                        strength: Strength::new(5),
                        ap: ArmorPenetration::MINUS_1,
                        damage: Damage::Fixed(2),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::RapidFire(1),
                        ]),
                    },
                ],
                vec![
                    // Hellforged weapons: 16A, WS2+, S6, AP-1, D1
                    WeaponProfile {
                        id: WeaponId::new(2001),
                        name: "Hellforged weapons".to_string(),
                        weapon_type: WeaponType::Melee,
                        range: Inches::ZERO,
                        attacks: AttackCount::Fixed(16),
                        skill: Skill::TWO_PLUS,
                        strength: Strength::new(6),
                        ap: ArmorPenetration::MINUS_1,
                        damage: Damage::Fixed(1),
                        abilities: WeaponAbilitySet::new(),
                    },
                ],
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
                MoveCharacteristic::from_inches(10),
                Toughness::new(10),
                ArmorSave::TWO_PLUS,
                Some(wh40k_core_types::InvulnerableSave::FOUR_PLUS),
                Leadership::new(6),
                ObjectiveControl::new(3),
            );
            units.push(unit);
        }

        // Master of Executions
        // Source: Frenzied_Reavers.md - Master of Executions datasheet
        // Ranged: Bolt pistol (12", 1A, BS3+, S4, AP0, D1, [PISTOL])
        // Melee: Axe of dismemberment (5A, WS2+, S7, AP-2, D2, [DEVASTATING WOUNDS, PRECISION])
        {
            let unit_id = UnitId::new(base_uid + 1);
            let model = ModelState::new(
                ModelId::new(model_counter),
                unit_id,
                Wounds::new(4),
                Position::from_inches(0, 0),
                BaseSize::MM32,
                vec![
                    // Bolt pistol: 12", 1A, BS3+, S4, AP0, D1 [PISTOL]
                    WeaponProfile {
                        id: WeaponId::new(2002),
                        name: "Bolt pistol".to_string(),
                        weapon_type: WeaponType::Ranged,
                        range: Inches::from_inches(12),
                        attacks: AttackCount::Fixed(1),
                        skill: Skill::THREE_PLUS,
                        strength: Strength::new(4),
                        ap: ArmorPenetration::ZERO,
                        damage: Damage::Fixed(1),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::Pistol,
                        ]),
                    },
                ],
                vec![
                    // Axe of dismemberment: 5A, WS2+, S7, AP-2, D2 [DEVASTATING WOUNDS, PRECISION]
                    WeaponProfile {
                        id: WeaponId::new(2003),
                        name: "Axe of dismemberment".to_string(),
                        weapon_type: WeaponType::Melee,
                        range: Inches::ZERO,
                        attacks: AttackCount::Fixed(5),
                        skill: Skill::TWO_PLUS,
                        strength: Strength::new(7),
                        ap: ArmorPenetration::MINUS_2,
                        damage: Damage::Fixed(2),
                        abilities: WeaponAbilitySet::from_abilities(vec![
                            WeaponAbility::DevastatingWounds,
                            WeaponAbility::Precision,
                        ]),
                    },
                ],
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
                MoveCharacteristic::from_inches(8),
                Toughness::new(4),
                ArmorSave::THREE_PLUS,
                None,
                Leadership::new(6),
                ObjectiveControl::new(1),
            );
            units.push(unit);
        }

        // Khorne Berzerkers (10 models)
        // Source: Frenzied_Reavers.md - Khorne Berzerkers datasheet
        // Model 0 (Champion): plasma pistol standard + chainblade
        // Models 1-2: plasma pistol standard + chainblade
        // Models 3-4: bolt pistol + Khornate eviscerator
        // Models 5-9: bolt pistol + chainblade
        {
            let unit_id = UnitId::new(base_uid + 2);
            let models: Vec<ModelState> = (0..10)
                .map(|i| {
                    let m = match i {
                        // Model 0: Champion with plasma pistol (standard) + chainblade
                        0 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(2),
                            Position::from_inches(0, 0),
                            BaseSize::MM32,
                            vec![
                                // Plasma pistol (standard): 12", 1A, BS4+, S7, AP-2, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2004),
                                    name: "Plasma pistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(7),
                                    ap: ArmorPenetration::MINUS_2,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Chainblade: 4A, WS3+, S4, AP-1, D1
                                WeaponProfile {
                                    id: WeaponId::new(2005),
                                    name: "Chainblade".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(4),
                                    skill: Skill::THREE_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Models 1-2: plasma pistol (standard) + chainblade
                        1 | 2 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(2),
                            Position::from_inches(0, 0),
                            BaseSize::MM32,
                            vec![
                                // Plasma pistol (standard): 12", 1A, BS4+, S7, AP-2, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2004),
                                    name: "Plasma pistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(7),
                                    ap: ArmorPenetration::MINUS_2,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Chainblade: 4A, WS3+, S4, AP-1, D1
                                WeaponProfile {
                                    id: WeaponId::new(2005),
                                    name: "Chainblade".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(4),
                                    skill: Skill::THREE_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Models 3-4: bolt pistol + Khornate eviscerator
                        3 | 4 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(2),
                            Position::from_inches(0, 0),
                            BaseSize::MM32,
                            vec![
                                // Bolt pistol: 12", 1A, BS4+, S4, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2006),
                                    name: "Bolt pistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Khornate eviscerator: 3A, WS3+, S8, AP-2, D2
                                WeaponProfile {
                                    id: WeaponId::new(2007),
                                    name: "Khornate eviscerator".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(3),
                                    skill: Skill::THREE_PLUS,
                                    strength: Strength::new(8),
                                    ap: ArmorPenetration::MINUS_2,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Models 5-9: bolt pistol + chainblade
                        _ => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(2),
                            Position::from_inches(0, 0),
                            BaseSize::MM32,
                            vec![
                                // Bolt pistol: 12", 1A, BS4+, S4, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2006),
                                    name: "Bolt pistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Chainblade: 4A, WS3+, S4, AP-1, D1
                                WeaponProfile {
                                    id: WeaponId::new(2005),
                                    name: "Chainblade".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(4),
                                    skill: Skill::THREE_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                    };
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
                MoveCharacteristic::from_inches(8),
                Toughness::new(4),
                ArmorSave::THREE_PLUS,
                None,
                Leadership::new(6),
                ObjectiveControl::new(2),
            );
            units.push(unit);
        }

        // Jakhals (10 models)
        // Source: Frenzied_Reavers.md - Jakhals datasheet
        // Model 0 (Pack Leader): autopistol + chainblades
        // Model 1 (Dishonoured): autopistol + skullsmasher and mangler
        // Model 2: autopistol + mauler chainblade
        // Models 3-9: autopistol + chainblades
        {
            let unit_id = UnitId::new(base_uid + 3);
            let models: Vec<ModelState> = (0..10)
                .map(|i| {
                    let m = match i {
                        // Model 0: Pack Leader with autopistol + chainblades
                        0 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(1),
                            Position::from_inches(0, 0),
                            BaseSize::MM28,
                            vec![
                                // Autopistol: 12", 1A, BS4+, S3, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2008),
                                    name: "Autopistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Chainblades: 3A, WS4+, S3, AP0, D1
                                WeaponProfile {
                                    id: WeaponId::new(2009),
                                    name: "Chainblades".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(3),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Model 1: Dishonoured with autopistol + skullsmasher and mangler
                        1 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(1),
                            Position::from_inches(0, 0),
                            BaseSize::MM28,
                            vec![
                                // Autopistol: 12", 1A, BS4+, S3, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2008),
                                    name: "Autopistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Skullsmasher and mangler: 2A, WS4+, S4, AP-1, D2
                                WeaponProfile {
                                    id: WeaponId::new(2010),
                                    name: "Skullsmasher and mangler".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(2),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Model 2: autopistol + mauler chainblade
                        2 => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(1),
                            Position::from_inches(0, 0),
                            BaseSize::MM28,
                            vec![
                                // Autopistol: 12", 1A, BS4+, S3, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2008),
                                    name: "Autopistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Mauler chainblade: 3A, WS5+, S4, AP-1, D2
                                WeaponProfile {
                                    id: WeaponId::new(2011),
                                    name: "Mauler chainblade".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(3),
                                    skill: Skill::FIVE_PLUS,
                                    strength: Strength::new(4),
                                    ap: ArmorPenetration::MINUS_1,
                                    damage: Damage::Fixed(2),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                        // Models 3-9: autopistol + chainblades
                        _ => ModelState::new(
                            ModelId::new(model_counter),
                            unit_id,
                            Wounds::new(1),
                            Position::from_inches(0, 0),
                            BaseSize::MM28,
                            vec![
                                // Autopistol: 12", 1A, BS4+, S3, AP0, D1 [PISTOL]
                                WeaponProfile {
                                    id: WeaponId::new(2008),
                                    name: "Autopistol".to_string(),
                                    weapon_type: WeaponType::Ranged,
                                    range: Inches::from_inches(12),
                                    attacks: AttackCount::Fixed(1),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::from_abilities(vec![
                                        WeaponAbility::Pistol,
                                    ]),
                                },
                            ],
                            vec![
                                // Chainblades: 3A, WS4+, S3, AP0, D1
                                WeaponProfile {
                                    id: WeaponId::new(2009),
                                    name: "Chainblades".to_string(),
                                    weapon_type: WeaponType::Melee,
                                    range: Inches::ZERO,
                                    attacks: AttackCount::Fixed(3),
                                    skill: Skill::FOUR_PLUS,
                                    strength: Strength::new(3),
                                    ap: ArmorPenetration::ZERO,
                                    damage: Damage::Fixed(1),
                                    abilities: WeaponAbilitySet::new(),
                                },
                            ],
                            false,
                            None,
                        ),
                    };
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
                MoveCharacteristic::from_inches(7),
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

        // Verify Custodes units (3 units with default patrol squad = Wardens)
        // Source: Custodes.md §2 - Fixed: Tristraen + Guard; Default patrol: Wardens
        let custodes_units = state.units_for_player(PlayerId::new(0));
        assert_eq!(custodes_units.len(), 3);

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

        // Custodian Wardens (default patrol squad)
        let wardens = custodes_units
            .iter()
            .find(|u| u.name == "Custodian Wardens")
            .unwrap();
        assert_eq!(wardens.models.len(), 3);
        assert_eq!(wardens.base_toughness.value(), 6);
        assert_eq!(wardens.models[0].wounds_max.value(), 3);
        // No Allarus when using default squad
        assert!(custodes_units.iter().find(|u| u.name == "Allarus Custodians").is_none());

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
        assert_eq!(berzerkers.models.len(), 10);
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

        // Both players are Custodes with default patrol squad (Wardens):
        // 3 units each = 6 total (Tristraen + Guard + Wardens)
        assert_eq!(state.units.len(), 6);
    }

    #[test]
    fn test_custodes_with_allarus() {
        let seed = [0u8; 32];
        // Squad choice 1 = Allarus Custodians
        let state = ScenarioLoader::load_scenario_with_squads(
            FactionId::new(0), // Custodes
            FactionId::new(1), // World Eaters
            None,
            seed,
            Some(1), // Player A: Allarus
            None,    // Player B: default (World Eaters have no choice)
        );

        let custodes_units = state.units_for_player(PlayerId::new(0));
        // 3 units: Tristraen + Guard + Allarus (not Wardens)
        assert_eq!(custodes_units.len(), 3);
        assert!(custodes_units.iter().any(|u| u.name == "Tristraen"));
        assert!(custodes_units.iter().any(|u| u.name == "Custodian Guard"));
        assert!(custodes_units.iter().any(|u| u.name == "Allarus Custodians"));
        assert!(custodes_units.iter().all(|u| u.name != "Custodian Wardens"));

        // Verify Allarus stats match Custodes.md
        let allarus = custodes_units.iter().find(|u| u.name == "Allarus Custodians").unwrap();
        assert_eq!(allarus.models.len(), 2);
        assert_eq!(allarus.base_toughness.value(), 7);
        assert_eq!(allarus.models[0].wounds_max.value(), 4);
        assert!(allarus.has_keyword(Keyword::Terminator));

        // Verify patrol squad choice is stored on player
        assert_eq!(state.players[0].patrol_squad_choice, Some(1));
    }

    #[test]
    fn test_custodes_with_wardens() {
        let seed = [0u8; 32];
        // Squad choice 0 = Custodian Wardens
        let state = ScenarioLoader::load_scenario_with_squads(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
            Some(0), // Player A: Wardens
            None,
        );

        let custodes_units = state.units_for_player(PlayerId::new(0));
        assert_eq!(custodes_units.len(), 3);
        assert!(custodes_units.iter().any(|u| u.name == "Custodian Wardens"));
        assert!(custodes_units.iter().all(|u| u.name != "Allarus Custodians"));

        let wardens = custodes_units.iter().find(|u| u.name == "Custodian Wardens").unwrap();
        assert_eq!(wardens.models.len(), 3);
        assert_eq!(wardens.base_toughness.value(), 6);
        assert_eq!(wardens.models[0].wounds_max.value(), 3);

        // Standard config total: 1 + 3 + 3 = 7 models
        let total_models: usize = custodes_units.iter().map(|u| u.models.len()).sum();
        assert_eq!(total_models, 7);
    }

    #[test]
    fn test_custodes_allarus_total_models() {
        let seed = [0u8; 32];
        let state = ScenarioLoader::load_scenario_with_squads(
            FactionId::new(0),
            FactionId::new(1),
            None,
            seed,
            Some(1), // Allarus
            None,
        );

        let custodes_units = state.units_for_player(PlayerId::new(0));
        // Allarus config total: 1 + 3 + 2 = 6 models
        let total_models: usize = custodes_units.iter().map(|u| u.models.len()).sum();
        assert_eq!(total_models, 6);
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
        // Vorrakh(1) + MoE(1) + Berzerkers(10) + Jakhals(10) = 22
        assert_eq!(total_models, 22);
    }
}
