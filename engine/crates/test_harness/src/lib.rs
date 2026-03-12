//! WH40K Engine - Test Harness crate
//!
//! Provides `ScenarioBuilder` for constructing test game states via a builder
//! pattern, and `StateAssertion` for validating game state in tests.
//!
//! This crate defines intermediate test state structs (`TestGameState`,
//! `TestPlayer`, `TestUnit`, `TestModel`, `TestObjective`) rather than
//! depending on the full `GameState` from `game_core`.

use wh40k_core_types::{
    BoardDimensions, Inches, KeywordSet, ObjectiveId, Phase, PlayerId, Position, UnitId,
    UnitStatus, WeaponProfile,
};

// ---------------------------------------------------------------------------
// Test State Structs
// ---------------------------------------------------------------------------

/// A lightweight game state for testing purposes.
/// Not the full GameState from game_core; instead a simplified representation
/// that ScenarioBuilder constructs and StateAssertion validates against.
#[derive(Debug, Clone)]
pub struct TestGameState {
    /// Current battle round (1-5).
    pub battle_round: u8,
    /// Current phase.
    pub phase: Phase,
    /// The active player (whose turn it is).
    pub active_player: PlayerId,
    /// Players in the game.
    pub players: Vec<TestPlayer>,
    /// Units on the battlefield.
    pub units: Vec<TestUnit>,
    /// Objective markers on the battlefield.
    pub objectives: Vec<TestObjective>,
    /// Board dimensions.
    pub board: BoardDimensions,
}

impl TestGameState {
    /// Find a unit by its id.
    pub fn unit(&self, id: UnitId) -> Option<&TestUnit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// Find a unit by its id (mutable).
    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut TestUnit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    /// Find a unit by name (first match).
    pub fn unit_by_name(&self, name: &str) -> Option<&TestUnit> {
        self.units.iter().find(|u| u.name == name)
    }

    /// Find a player by id.
    pub fn player(&self, id: PlayerId) -> Option<&TestPlayer> {
        self.players.iter().find(|p| p.id == id)
    }

    /// Find a player by id (mutable).
    pub fn player_mut(&mut self, id: PlayerId) -> Option<&mut TestPlayer> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    /// Find an objective by id.
    pub fn objective(&self, id: ObjectiveId) -> Option<&TestObjective> {
        self.objectives.iter().find(|o| o.id == id)
    }

    /// Find an objective by id (mutable).
    pub fn objective_mut(&mut self, id: ObjectiveId) -> Option<&mut TestObjective> {
        self.objectives.iter_mut().find(|o| o.id == id)
    }
}

/// A player in the test game state.
#[derive(Debug, Clone)]
pub struct TestPlayer {
    /// Player identifier.
    pub id: PlayerId,
    /// Display name.
    pub name: String,
    /// Faction name (e.g. "Adeptus Custodes", "World Eaters").
    pub faction: String,
    /// Current command points.
    pub cp: i8,
    /// Current victory points.
    pub vp: i16,
}

/// A unit in the test game state.
#[derive(Debug, Clone)]
pub struct TestUnit {
    /// Unit identifier.
    pub id: UnitId,
    /// Owning player.
    pub owner: PlayerId,
    /// Display name.
    pub name: String,
    /// Keywords for this unit.
    pub keywords: KeywordSet,
    /// Models composing this unit.
    pub models: Vec<TestModel>,
    /// Aggregate unit position (centroid or leader position).
    pub position: Position,
    /// Unit status on the battlefield.
    pub status: UnitStatus,
    /// Whether this unit is battle-shocked.
    pub battle_shocked: bool,
    /// Whether this unit is currently within engagement range of an enemy.
    pub engaged: bool,
    /// Weapon profiles available to models in this unit.
    pub weapons: Vec<WeaponProfile>,
}

impl TestUnit {
    /// Number of models currently alive in this unit.
    pub fn models_alive(&self) -> usize {
        self.models.iter().filter(|m| m.alive).count()
    }

    /// Whether the unit has been destroyed (no models alive).
    pub fn is_destroyed(&self) -> bool {
        self.models_alive() == 0
    }

    /// Find a model by id.
    pub fn model(&self, id: u32) -> Option<&TestModel> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Find a model by id (mutable).
    pub fn model_mut(&mut self, id: u32) -> Option<&mut TestModel> {
        self.models.iter_mut().find(|m| m.id == id)
    }
}

/// An individual model in a test unit.
#[derive(Debug, Clone)]
pub struct TestModel {
    /// Model identifier (unique within unit, sequential from 0).
    pub id: u32,
    /// Whether this model is alive.
    pub alive: bool,
    /// Current wounds remaining.
    pub wounds_remaining: u8,
    /// Maximum wounds this model can have.
    pub wounds_max: u8,
    /// Position of this model on the battlefield.
    pub position: Position,
}

/// An objective marker on the battlefield.
#[derive(Debug, Clone)]
pub struct TestObjective {
    /// Objective identifier.
    pub id: ObjectiveId,
    /// Position on the battlefield.
    pub position: Position,
    /// Which player controls this objective (None if uncontrolled).
    pub controller: Option<PlayerId>,
}

// ---------------------------------------------------------------------------
// ScenarioBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing `TestGameState` instances in tests.
///
/// Uses the builder pattern so tests can declaratively set up game scenarios:
///
/// ```rust
/// use wh40k_test_harness::ScenarioBuilder;
/// use wh40k_core_types::{Phase, KeywordSet, Keyword};
///
/// let state = ScenarioBuilder::new()
///     .board(44, 30)
///     .battle_round(1)
///     .phase(Phase::Movement)
///     .add_player(0, "Alice", "Adeptus Custodes")
///     .add_player(1, "Bob", "World Eaters")
///     .active_player(0)
///     .set_cp(0, 0)
///     .set_cp(1, 0)
///     .add_objective(0, 22, 15)
///     .add_unit_builder(0, "Custodian Guard", |ub| {
///         ub.keywords(KeywordSet::CUSTODIAN_GUARD_KEYWORDS)
///           .position(10, 10)
///           .add_model(3, 3)
///           .add_model(3, 3)
///           .add_model(3, 3)
///     })
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ScenarioBuilder {
    battle_round: u8,
    phase: Phase,
    active_player: PlayerId,
    players: Vec<TestPlayer>,
    units: Vec<TestUnit>,
    objectives: Vec<TestObjective>,
    board: BoardDimensions,
    next_unit_id: u32,
    next_objective_id: u32,
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioBuilder {
    /// Create a new ScenarioBuilder with sensible defaults.
    /// Defaults to round 1, Command phase, player 0 active,
    /// Combat Patrol board size (44"x30").
    pub fn new() -> Self {
        Self {
            battle_round: 1,
            phase: Phase::Command,
            active_player: PlayerId::new(0),
            players: Vec::new(),
            units: Vec::new(),
            objectives: Vec::new(),
            board: BoardDimensions::COMBAT_PATROL,
            next_unit_id: 0,
            next_objective_id: 0,
        }
    }

    /// Set the board dimensions in whole inches.
    pub fn board(mut self, width: i32, height: i32) -> Self {
        self.board = BoardDimensions {
            width: Inches::from_inches(width),
            height: Inches::from_inches(height),
        };
        self
    }

    /// Set the board dimensions from a `BoardDimensions` value.
    pub fn board_dimensions(mut self, dims: BoardDimensions) -> Self {
        self.board = dims;
        self
    }

    /// Set the current battle round (1-5).
    pub fn battle_round(mut self, round: u8) -> Self {
        self.battle_round = round;
        self
    }

    /// Set the current phase.
    pub fn phase(mut self, phase: Phase) -> Self {
        self.phase = phase;
        self
    }

    /// Set the active player by raw id.
    pub fn active_player(mut self, player_id: u32) -> Self {
        self.active_player = PlayerId::new(player_id);
        self
    }

    /// Set the active player by PlayerId.
    pub fn active_player_id(mut self, player_id: PlayerId) -> Self {
        self.active_player = player_id;
        self
    }

    /// Add a player with the given raw id, name, and faction string.
    /// CP defaults to 0, VP defaults to 0.
    pub fn add_player(mut self, id: u32, name: &str, faction: &str) -> Self {
        self.players.push(TestPlayer {
            id: PlayerId::new(id),
            name: name.to_string(),
            faction: faction.to_string(),
            cp: 0,
            vp: 0,
        });
        self
    }

    /// Add a player with a `TestPlayer` struct directly.
    pub fn add_player_struct(mut self, player: TestPlayer) -> Self {
        self.players.push(player);
        self
    }

    /// Set command points for a player (by raw id).
    pub fn set_cp(mut self, player_id: u32, cp: i8) -> Self {
        let pid = PlayerId::new(player_id);
        if let Some(p) = self.players.iter_mut().find(|p| p.id == pid) {
            p.cp = cp;
        }
        self
    }

    /// Set victory points for a player (by raw id).
    pub fn set_vp(mut self, player_id: u32, vp: i16) -> Self {
        let pid = PlayerId::new(player_id);
        if let Some(p) = self.players.iter_mut().find(|p| p.id == pid) {
            p.vp = vp;
        }
        self
    }

    /// Add an objective marker at the given whole-inch position.
    /// The objective id is auto-assigned sequentially.
    pub fn add_objective(mut self, id: u32, x: i32, y: i32) -> Self {
        self.objectives.push(TestObjective {
            id: ObjectiveId::new(id),
            position: Position::from_inches(x, y),
            controller: None,
        });
        if id >= self.next_objective_id {
            self.next_objective_id = id + 1;
        }
        self
    }

    /// Add an objective at a position with a specified controller.
    pub fn add_objective_controlled(
        mut self,
        id: u32,
        x: i32,
        y: i32,
        controller: u32,
    ) -> Self {
        self.objectives.push(TestObjective {
            id: ObjectiveId::new(id),
            position: Position::from_inches(x, y),
            controller: Some(PlayerId::new(controller)),
        });
        if id >= self.next_objective_id {
            self.next_objective_id = id + 1;
        }
        self
    }

    /// Add a unit using a `UnitBuilder` callback for ergonomic model setup.
    /// The unit id is auto-assigned sequentially.
    ///
    /// `owner` is the raw player id.
    /// `name` is the unit name.
    /// `configure` is a closure that receives a `UnitBuilder` and returns it
    /// after configuration.
    pub fn add_unit_builder<F>(mut self, owner: u32, name: &str, configure: F) -> Self
    where
        F: FnOnce(UnitBuilder) -> UnitBuilder,
    {
        let unit_id = self.next_unit_id;
        self.next_unit_id += 1;

        let builder = UnitBuilder::new(UnitId::new(unit_id), PlayerId::new(owner), name);
        let builder = configure(builder);
        self.units.push(builder.build());
        self
    }

    /// Add a unit directly from a `TestUnit` struct.
    pub fn add_unit(mut self, unit: TestUnit) -> Self {
        if unit.id.raw() >= self.next_unit_id {
            self.next_unit_id = unit.id.raw() + 1;
        }
        self.units.push(unit);
        self
    }

    /// Add a simple unit with N identical models at a given position.
    /// This is a convenience method for quick setup without needing a UnitBuilder.
    ///
    /// All models have the same wounds_max and start fully healed.
    #[allow(clippy::too_many_arguments)]
    pub fn add_simple_unit(
        mut self,
        owner: u32,
        name: &str,
        keywords: KeywordSet,
        num_models: u32,
        wounds_per_model: u8,
        x: i32,
        y: i32,
    ) -> Self {
        let unit_id = self.next_unit_id;
        self.next_unit_id += 1;

        let pos = Position::from_inches(x, y);
        let models: Vec<TestModel> = (0..num_models)
            .map(|i| TestModel {
                id: i,
                alive: true,
                wounds_remaining: wounds_per_model,
                wounds_max: wounds_per_model,
                position: pos,
            })
            .collect();

        self.units.push(TestUnit {
            id: UnitId::new(unit_id),
            owner: PlayerId::new(owner),
            name: name.to_string(),
            keywords,
            models,
            position: pos,
            status: UnitStatus::OnBattlefield,
            battle_shocked: false,
            engaged: false,
            weapons: Vec::new(),
        });
        self
    }

    /// Consume the builder and produce a `TestGameState`.
    pub fn build(self) -> TestGameState {
        TestGameState {
            battle_round: self.battle_round,
            phase: self.phase,
            active_player: self.active_player,
            players: self.players,
            units: self.units,
            objectives: self.objectives,
            board: self.board,
        }
    }

    // -----------------------------------------------------------------------
    // Quick Setup Helpers
    // -----------------------------------------------------------------------

    /// Create a standard two-player Combat Patrol scaffold.
    ///
    /// Sets up:
    /// - 44"x30" board
    /// - Two players (id 0 = "Player 1" Adeptus Custodes, id 1 = "Player 2" World Eaters)
    /// - Both players start with 0 CP, 0 VP
    /// - Battle round 1, Command phase, player 0 active
    /// - Four objective markers in a standard diamond layout:
    ///   - Obj 0 at (22, 6)  - near player 1 deployment
    ///   - Obj 1 at (22, 24) - near player 2 deployment
    ///   - Obj 2 at (11, 15) - left center
    ///   - Obj 3 at (33, 15) - right center
    ///
    /// No units are placed; the caller should add units via `add_unit_builder`
    /// or `add_simple_unit` after calling this.
    pub fn two_player_cp_setup() -> Self {
        Self::new()
            .board(44, 30)
            .battle_round(1)
            .phase(Phase::Command)
            .add_player(0, "Player 1", "Adeptus Custodes")
            .add_player(1, "Player 2", "World Eaters")
            .active_player(0)
            .set_cp(0, 0)
            .set_cp(1, 0)
            .add_objective(0, 22, 6)
            .add_objective(1, 22, 24)
            .add_objective(2, 11, 15)
            .add_objective(3, 33, 15)
    }
}

// ---------------------------------------------------------------------------
// UnitBuilder
// ---------------------------------------------------------------------------

/// Builder for constructing a `TestUnit` within a `ScenarioBuilder`.
#[derive(Debug, Clone)]
pub struct UnitBuilder {
    id: UnitId,
    owner: PlayerId,
    name: String,
    keywords: KeywordSet,
    models: Vec<TestModel>,
    position: Position,
    status: UnitStatus,
    battle_shocked: bool,
    engaged: bool,
    weapons: Vec<WeaponProfile>,
    next_model_id: u32,
}

impl UnitBuilder {
    /// Create a new UnitBuilder.
    pub fn new(id: UnitId, owner: PlayerId, name: &str) -> Self {
        Self {
            id,
            owner,
            name: name.to_string(),
            keywords: KeywordSet::empty(),
            models: Vec::new(),
            position: Position::ORIGIN,
            status: UnitStatus::OnBattlefield,
            battle_shocked: false,
            engaged: false,
            weapons: Vec::new(),
            next_model_id: 0,
        }
    }

    /// Set the keywords for this unit.
    pub fn keywords(mut self, keywords: KeywordSet) -> Self {
        self.keywords = keywords;
        self
    }

    /// Set the unit position in whole inches.
    pub fn position(mut self, x: i32, y: i32) -> Self {
        self.position = Position::from_inches(x, y);
        self
    }

    /// Set the unit position from a `Position` value.
    pub fn position_exact(mut self, pos: Position) -> Self {
        self.position = pos;
        self
    }

    /// Set the unit status.
    pub fn status(mut self, status: UnitStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the unit as battle-shocked.
    pub fn battle_shocked(mut self, shocked: bool) -> Self {
        self.battle_shocked = shocked;
        self
    }

    /// Set the unit as engaged with an enemy.
    pub fn engaged(mut self, engaged: bool) -> Self {
        self.engaged = engaged;
        self
    }

    /// Add a model with the given wounds_max, starting fully healed.
    /// The model is placed at the unit's position by default.
    /// Model id is auto-assigned sequentially.
    pub fn add_model(mut self, wounds_max: u8, wounds_remaining: u8) -> Self {
        let model_id = self.next_model_id;
        self.next_model_id += 1;
        self.models.push(TestModel {
            id: model_id,
            alive: wounds_remaining > 0,
            wounds_remaining,
            wounds_max,
            position: self.position,
        });
        self
    }

    /// Add a model at a specific position with full wounds.
    pub fn add_model_at(mut self, wounds_max: u8, x: i32, y: i32) -> Self {
        let model_id = self.next_model_id;
        self.next_model_id += 1;
        self.models.push(TestModel {
            id: model_id,
            alive: true,
            wounds_remaining: wounds_max,
            wounds_max,
            position: Position::from_inches(x, y),
        });
        self
    }

    /// Add a model that is already dead (0 wounds remaining).
    pub fn add_dead_model(mut self, wounds_max: u8) -> Self {
        let model_id = self.next_model_id;
        self.next_model_id += 1;
        self.models.push(TestModel {
            id: model_id,
            alive: false,
            wounds_remaining: 0,
            wounds_max,
            position: self.position,
        });
        self
    }

    /// Add a model with specific wounds remaining (possibly damaged).
    pub fn add_wounded_model(mut self, wounds_max: u8, wounds_remaining: u8) -> Self {
        let model_id = self.next_model_id;
        self.next_model_id += 1;
        self.models.push(TestModel {
            id: model_id,
            alive: wounds_remaining > 0,
            wounds_remaining,
            wounds_max,
            position: self.position,
        });
        self
    }

    /// Add N identical models, all fully healed, at the unit's position.
    pub fn add_models(mut self, count: u32, wounds_max: u8) -> Self {
        for _ in 0..count {
            let model_id = self.next_model_id;
            self.next_model_id += 1;
            self.models.push(TestModel {
                id: model_id,
                alive: true,
                wounds_remaining: wounds_max,
                wounds_max,
                position: self.position,
            });
        }
        self
    }

    /// Add a weapon profile to this unit.
    pub fn add_weapon(mut self, weapon: WeaponProfile) -> Self {
        self.weapons.push(weapon);
        self
    }

    /// Consume the builder and produce a `TestUnit`.
    pub fn build(self) -> TestUnit {
        TestUnit {
            id: self.id,
            owner: self.owner,
            name: self.name,
            keywords: self.keywords,
            models: self.models,
            position: self.position,
            status: self.status,
            battle_shocked: self.battle_shocked,
            engaged: self.engaged,
            weapons: self.weapons,
        }
    }
}

// ---------------------------------------------------------------------------
// StateAssertion
// ---------------------------------------------------------------------------

/// Assertion framework for validating `TestGameState` in tests.
///
/// Each method panics with a descriptive message if the assertion fails,
/// making test failures easy to diagnose.
///
/// Usage:
/// ```rust,ignore
/// let state = ScenarioBuilder::two_player_cp_setup()
///     .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
///     .build();
///
/// StateAssertion::new(&state)
///     .assert_battle_round(1)
///     .assert_phase(Phase::Command)
///     .assert_unit_models_alive(UnitId::new(0), 3)
///     .assert_player_cp(PlayerId::new(0), 0);
/// ```
pub struct StateAssertion<'a> {
    state: &'a TestGameState,
}

impl<'a> StateAssertion<'a> {
    /// Create a new `StateAssertion` bound to the given state.
    pub fn new(state: &'a TestGameState) -> Self {
        Self { state }
    }

    // -----------------------------------------------------------------------
    // Game-level assertions
    // -----------------------------------------------------------------------

    /// Assert the game is in the given battle round.
    pub fn assert_battle_round(&self, expected: u8) -> &Self {
        assert_eq!(
            self.state.battle_round, expected,
            "Expected battle round {}, but was {}",
            expected, self.state.battle_round
        );
        self
    }

    /// Assert the game is in the given phase.
    pub fn assert_phase(&self, expected: Phase) -> &Self {
        assert_eq!(
            self.state.phase, expected,
            "Expected phase {:?}, but was {:?}",
            expected, self.state.phase
        );
        self
    }

    /// Assert the active player is the one with the given raw id.
    pub fn assert_active_player(&self, expected: u32) -> &Self {
        let expected_id = PlayerId::new(expected);
        assert_eq!(
            self.state.active_player, expected_id,
            "Expected active player {}, but was {}",
            expected, self.state.active_player
        );
        self
    }

    /// Assert the active player is the given PlayerId.
    pub fn assert_active_player_id(&self, expected: PlayerId) -> &Self {
        assert_eq!(
            self.state.active_player, expected,
            "Expected active player {:?}, but was {:?}",
            expected, self.state.active_player
        );
        self
    }

    /// Assert that the game is in the given phase and round simultaneously.
    pub fn assert_phase_and_round(&self, phase: Phase, round: u8) -> &Self {
        self.assert_phase(phase);
        self.assert_battle_round(round);
        self
    }

    // -----------------------------------------------------------------------
    // Player assertions
    // -----------------------------------------------------------------------

    /// Assert a player has the given number of command points.
    pub fn assert_player_cp(&self, player_id: PlayerId, expected_cp: i8) -> &Self {
        let player = self
            .state
            .player(player_id)
            .unwrap_or_else(|| panic!("Player {:?} not found in state", player_id));
        assert_eq!(
            player.cp, expected_cp,
            "Expected player {:?} ({}) to have {} CP, but had {} CP",
            player_id, player.name, expected_cp, player.cp
        );
        self
    }

    /// Assert a player has the given number of victory points.
    pub fn assert_player_vp(&self, player_id: PlayerId, expected_vp: i16) -> &Self {
        let player = self
            .state
            .player(player_id)
            .unwrap_or_else(|| panic!("Player {:?} not found in state", player_id));
        assert_eq!(
            player.vp, expected_vp,
            "Expected player {:?} ({}) to have {} VP, but had {} VP",
            player_id, player.name, expected_vp, player.vp
        );
        self
    }

    /// Assert a player has at least the given number of victory points.
    pub fn assert_player_vp_at_least(&self, player_id: PlayerId, min_vp: i16) -> &Self {
        let player = self
            .state
            .player(player_id)
            .unwrap_or_else(|| panic!("Player {:?} not found in state", player_id));
        assert!(
            player.vp >= min_vp,
            "Expected player {:?} ({}) to have at least {} VP, but had {} VP",
            player_id, player.name, min_vp, player.vp
        );
        self
    }

    /// Assert a player has at least the given number of command points.
    pub fn assert_player_cp_at_least(&self, player_id: PlayerId, min_cp: i8) -> &Self {
        let player = self
            .state
            .player(player_id)
            .unwrap_or_else(|| panic!("Player {:?} not found in state", player_id));
        assert!(
            player.cp >= min_cp,
            "Expected player {:?} ({}) to have at least {} CP, but had {} CP",
            player_id, player.name, min_cp, player.cp
        );
        self
    }

    // -----------------------------------------------------------------------
    // Unit assertions
    // -----------------------------------------------------------------------

    /// Assert a unit is at the given position (whole inches).
    pub fn assert_unit_at_position(&self, unit_id: UnitId, x: i32, y: i32) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let expected = Position::from_inches(x, y);
        assert_eq!(
            unit.position, expected,
            "Expected unit {:?} ({}) at position ({}, {}), but was at ({}, {})",
            unit_id,
            unit.name,
            x,
            y,
            unit.position.x.whole_inches(),
            unit.position.y.whole_inches()
        );
        self
    }

    /// Assert a unit is at the given `Position` exactly.
    pub fn assert_unit_at_position_exact(&self, unit_id: UnitId, expected: Position) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert_eq!(
            unit.position, expected,
            "Expected unit {:?} ({}) at position {:?}, but was at {:?}",
            unit_id, unit.name, expected, unit.position
        );
        self
    }

    /// Assert a unit has exactly N models alive.
    pub fn assert_unit_models_alive(&self, unit_id: UnitId, expected: usize) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let alive = unit.models_alive();
        assert_eq!(
            alive, expected,
            "Expected unit {:?} ({}) to have {} models alive, but had {}",
            unit_id, unit.name, expected, alive
        );
        self
    }

    /// Assert a unit is destroyed (0 models alive).
    pub fn assert_unit_destroyed(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            unit.is_destroyed(),
            "Expected unit {:?} ({}) to be destroyed, but had {} models alive",
            unit_id,
            unit.name,
            unit.models_alive()
        );
        self
    }

    /// Assert a unit is NOT destroyed (at least 1 model alive).
    pub fn assert_unit_alive(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            !unit.is_destroyed(),
            "Expected unit {:?} ({}) to be alive, but it was destroyed",
            unit_id, unit.name
        );
        self
    }

    /// Assert a specific model in a unit has exactly N wounds remaining.
    pub fn assert_model_wounds(
        &self,
        unit_id: UnitId,
        model_id: u32,
        expected_wounds: u8,
    ) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let model = unit
            .model(model_id)
            .unwrap_or_else(|| {
                panic!(
                    "Model {} not found in unit {:?} ({})",
                    model_id, unit_id, unit.name
                )
            });
        assert_eq!(
            model.wounds_remaining, expected_wounds,
            "Expected model {} in unit {:?} ({}) to have {} wounds remaining, but had {}",
            model_id, unit_id, unit.name, expected_wounds, model.wounds_remaining
        );
        self
    }

    /// Assert a specific model in a unit is alive.
    pub fn assert_model_alive(&self, unit_id: UnitId, model_id: u32) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let model = unit
            .model(model_id)
            .unwrap_or_else(|| {
                panic!(
                    "Model {} not found in unit {:?} ({})",
                    model_id, unit_id, unit.name
                )
            });
        assert!(
            model.alive,
            "Expected model {} in unit {:?} ({}) to be alive, but it was dead",
            model_id, unit_id, unit.name
        );
        self
    }

    /// Assert a specific model in a unit is dead.
    pub fn assert_model_dead(&self, unit_id: UnitId, model_id: u32) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let model = unit
            .model(model_id)
            .unwrap_or_else(|| {
                panic!(
                    "Model {} not found in unit {:?} ({})",
                    model_id, unit_id, unit.name
                )
            });
        assert!(
            !model.alive,
            "Expected model {} in unit {:?} ({}) to be dead, but it was alive with {} wounds",
            model_id, unit_id, unit.name, model.wounds_remaining
        );
        self
    }

    /// Assert a unit is battle-shocked.
    pub fn assert_unit_battle_shocked(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            unit.battle_shocked,
            "Expected unit {:?} ({}) to be battle-shocked, but it was not",
            unit_id, unit.name
        );
        self
    }

    /// Assert a unit is NOT battle-shocked.
    pub fn assert_unit_not_battle_shocked(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            !unit.battle_shocked,
            "Expected unit {:?} ({}) to not be battle-shocked, but it was",
            unit_id, unit.name
        );
        self
    }

    /// Assert a unit is engaged (within engagement range of an enemy).
    pub fn assert_unit_engaged(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            unit.engaged,
            "Expected unit {:?} ({}) to be engaged, but it was not",
            unit_id, unit.name
        );
        self
    }

    /// Assert a unit is NOT engaged.
    pub fn assert_unit_not_engaged(&self, unit_id: UnitId) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            !unit.engaged,
            "Expected unit {:?} ({}) to not be engaged, but it was",
            unit_id, unit.name
        );
        self
    }

    /// Assert a unit has the given status.
    pub fn assert_unit_status(&self, unit_id: UnitId, expected: UnitStatus) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert_eq!(
            unit.status, expected,
            "Expected unit {:?} ({}) to have status {:?}, but had {:?}",
            unit_id, unit.name, expected, unit.status
        );
        self
    }

    /// Assert a unit has the given keywords.
    pub fn assert_unit_has_keywords(&self, unit_id: UnitId, expected: KeywordSet) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        assert!(
            unit.keywords.has_all(expected),
            "Expected unit {:?} ({}) to have all keywords {:?}, but had {:?}",
            unit_id, unit.name, expected, unit.keywords
        );
        self
    }

    // -----------------------------------------------------------------------
    // Objective assertions
    // -----------------------------------------------------------------------

    /// Assert an objective is controlled by the given player.
    pub fn assert_objective_controlled_by(
        &self,
        objective_id: ObjectiveId,
        player_id: PlayerId,
    ) -> &Self {
        let obj = self
            .state
            .objective(objective_id)
            .unwrap_or_else(|| panic!("Objective {:?} not found in state", objective_id));
        assert_eq!(
            obj.controller,
            Some(player_id),
            "Expected objective {:?} to be controlled by {:?}, but controller was {:?}",
            objective_id, player_id, obj.controller
        );
        self
    }

    /// Assert an objective is not controlled by any player.
    pub fn assert_objective_uncontrolled(&self, objective_id: ObjectiveId) -> &Self {
        let obj = self
            .state
            .objective(objective_id)
            .unwrap_or_else(|| panic!("Objective {:?} not found in state", objective_id));
        assert_eq!(
            obj.controller, None,
            "Expected objective {:?} to be uncontrolled, but was controlled by {:?}",
            objective_id, obj.controller
        );
        self
    }

    /// Assert an objective is at the given position (whole inches).
    pub fn assert_objective_at_position(
        &self,
        objective_id: ObjectiveId,
        x: i32,
        y: i32,
    ) -> &Self {
        let obj = self
            .state
            .objective(objective_id)
            .unwrap_or_else(|| panic!("Objective {:?} not found in state", objective_id));
        let expected = Position::from_inches(x, y);
        assert_eq!(
            obj.position, expected,
            "Expected objective {:?} at ({}, {}), but was at ({}, {})",
            objective_id,
            x,
            y,
            obj.position.x.whole_inches(),
            obj.position.y.whole_inches()
        );
        self
    }

    // -----------------------------------------------------------------------
    // Aggregate assertions
    // -----------------------------------------------------------------------

    /// Assert the total number of units in the game.
    pub fn assert_total_units(&self, expected: usize) -> &Self {
        assert_eq!(
            self.state.units.len(),
            expected,
            "Expected {} total units, but found {}",
            expected,
            self.state.units.len()
        );
        self
    }

    /// Assert the total number of objectives in the game.
    pub fn assert_total_objectives(&self, expected: usize) -> &Self {
        assert_eq!(
            self.state.objectives.len(),
            expected,
            "Expected {} total objectives, but found {}",
            expected,
            self.state.objectives.len()
        );
        self
    }

    /// Assert the total number of players in the game.
    pub fn assert_total_players(&self, expected: usize) -> &Self {
        assert_eq!(
            self.state.players.len(),
            expected,
            "Expected {} total players, but found {}",
            expected,
            self.state.players.len()
        );
        self
    }

    /// Assert the number of units belonging to a given player.
    pub fn assert_player_unit_count(&self, player_id: PlayerId, expected: usize) -> &Self {
        let count = self
            .state
            .units
            .iter()
            .filter(|u| u.owner == player_id)
            .count();
        assert_eq!(
            count, expected,
            "Expected player {:?} to own {} units, but owned {}",
            player_id, expected, count
        );
        self
    }

    /// Assert a unit is within a given range (in whole inches) of a position.
    pub fn assert_unit_within_range_of(
        &self,
        unit_id: UnitId,
        x: i32,
        y: i32,
        range_inches: i32,
    ) -> &Self {
        let unit = self
            .state
            .unit(unit_id)
            .unwrap_or_else(|| panic!("Unit {:?} not found in state", unit_id));
        let target = Position::from_inches(x, y);
        let range = Inches::from_inches(range_inches);
        assert!(
            unit.position.within_range(target, range),
            "Expected unit {:?} ({}) at {:?} to be within {}\" of ({}, {}), but it was not",
            unit_id,
            unit.name,
            unit.position,
            range_inches,
            x,
            y
        );
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        ArmorPenetration, AttackCount, Damage, Inches, Keyword, KeywordSet, ObjectiveId, Phase,
        PlayerId, Position, Skill, Strength, UnitId, UnitStatus, WeaponAbility,
        WeaponAbilitySet, WeaponId, WeaponProfile, WeaponType,
    };

    // -----------------------------------------------------------------------
    // ScenarioBuilder tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_builder_defaults() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .build();

        assert_eq!(state.battle_round, 1);
        assert_eq!(state.phase, Phase::Command);
        assert_eq!(state.active_player, PlayerId::new(0));
        assert_eq!(state.board, BoardDimensions::COMBAT_PATROL);
        assert_eq!(state.players.len(), 1);
        assert!(state.units.is_empty());
        assert!(state.objectives.is_empty());
    }

    #[test]
    fn test_builder_board_dimensions() {
        let state = ScenarioBuilder::new()
            .board(60, 44)
            .add_player(0, "Alice", "Custodes")
            .build();

        assert_eq!(state.board.width, Inches::from_inches(60));
        assert_eq!(state.board.height, Inches::from_inches(44));
    }

    #[test]
    fn test_builder_battle_round_and_phase() {
        let state = ScenarioBuilder::new()
            .battle_round(3)
            .phase(Phase::Shooting)
            .add_player(0, "Alice", "Custodes")
            .build();

        assert_eq!(state.battle_round, 3);
        assert_eq!(state.phase, Phase::Shooting);
    }

    #[test]
    fn test_builder_two_players() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Adeptus Custodes")
            .add_player(1, "Bob", "World Eaters")
            .active_player(1)
            .set_cp(0, 2)
            .set_cp(1, 1)
            .set_vp(0, 10)
            .set_vp(1, 5)
            .build();

        assert_eq!(state.players.len(), 2);
        assert_eq!(state.active_player, PlayerId::new(1));

        let p0 = state.player(PlayerId::new(0)).unwrap();
        assert_eq!(p0.name, "Alice");
        assert_eq!(p0.faction, "Adeptus Custodes");
        assert_eq!(p0.cp, 2);
        assert_eq!(p0.vp, 10);

        let p1 = state.player(PlayerId::new(1)).unwrap();
        assert_eq!(p1.name, "Bob");
        assert_eq!(p1.faction, "World Eaters");
        assert_eq!(p1.cp, 1);
        assert_eq!(p1.vp, 5);
    }

    #[test]
    fn test_builder_objectives() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_objective(0, 22, 15)
            .add_objective(1, 11, 6)
            .add_objective_controlled(2, 33, 24, 0)
            .build();

        assert_eq!(state.objectives.len(), 3);

        let obj0 = state.objective(ObjectiveId::new(0)).unwrap();
        assert_eq!(obj0.position, Position::from_inches(22, 15));
        assert_eq!(obj0.controller, None);

        let obj2 = state.objective(ObjectiveId::new(2)).unwrap();
        assert_eq!(obj2.position, Position::from_inches(33, 24));
        assert_eq!(obj2.controller, Some(PlayerId::new(0)));
    }

    #[test]
    fn test_builder_simple_unit() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(
                0,
                "Custodian Guard",
                KeywordSet::CUSTODIAN_GUARD_KEYWORDS,
                3,
                3,
                10,
                10,
            )
            .build();

        assert_eq!(state.units.len(), 1);
        let unit = &state.units[0];
        assert_eq!(unit.id, UnitId::new(0));
        assert_eq!(unit.owner, PlayerId::new(0));
        assert_eq!(unit.name, "Custodian Guard");
        assert_eq!(unit.keywords, KeywordSet::CUSTODIAN_GUARD_KEYWORDS);
        assert_eq!(unit.models.len(), 3);
        assert_eq!(unit.position, Position::from_inches(10, 10));
        assert_eq!(unit.status, UnitStatus::OnBattlefield);
        assert!(!unit.battle_shocked);
        assert!(!unit.engaged);

        for (i, model) in unit.models.iter().enumerate() {
            assert_eq!(model.id, i as u32);
            assert!(model.alive);
            assert_eq!(model.wounds_remaining, 3);
            assert_eq!(model.wounds_max, 3);
            assert_eq!(model.position, Position::from_inches(10, 10));
        }
    }

    #[test]
    fn test_builder_unit_builder() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Custodian Wardens", |ub| {
                ub.keywords(KeywordSet::CUSTODIAN_WARDENS_KEYWORDS)
                    .position(15, 12)
                    .add_model(3, 3)
                    .add_model(3, 3)
                    .add_wounded_model(3, 1)
                    .battle_shocked(true)
                    .engaged(true)
            })
            .build();

        assert_eq!(state.units.len(), 1);
        let unit = &state.units[0];
        assert_eq!(unit.name, "Custodian Wardens");
        assert_eq!(unit.keywords, KeywordSet::CUSTODIAN_WARDENS_KEYWORDS);
        assert_eq!(unit.position, Position::from_inches(15, 12));
        assert!(unit.battle_shocked);
        assert!(unit.engaged);
        assert_eq!(unit.models.len(), 3);
        assert_eq!(unit.models_alive(), 3);
        assert_eq!(unit.models[2].wounds_remaining, 1);
    }

    #[test]
    fn test_builder_unit_with_dead_model() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(5, 5)
                    .add_model(3, 3)
                    .add_dead_model(3)
                    .add_model(3, 3)
            })
            .build();

        let unit = &state.units[0];
        assert_eq!(unit.models.len(), 3);
        assert_eq!(unit.models_alive(), 2);
        assert!(!unit.models[1].alive);
        assert_eq!(unit.models[1].wounds_remaining, 0);
    }

    #[test]
    fn test_builder_unit_with_models_at_positions() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10)
                    .add_model_at(3, 10, 10)
                    .add_model_at(3, 11, 10)
                    .add_model_at(3, 10, 11)
            })
            .build();

        let unit = &state.units[0];
        assert_eq!(unit.models[0].position, Position::from_inches(10, 10));
        assert_eq!(unit.models[1].position, Position::from_inches(11, 10));
        assert_eq!(unit.models[2].position, Position::from_inches(10, 11));
    }

    #[test]
    fn test_builder_unit_with_weapons() {
        let sentinel_blade = WeaponProfile {
            id: WeaponId::new(1),
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
        };

        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.keywords(KeywordSet::CUSTODIAN_GUARD_KEYWORDS)
                    .position(10, 10)
                    .add_models(3, 3)
                    .add_weapon(sentinel_blade)
            })
            .build();

        let unit = &state.units[0];
        assert_eq!(unit.weapons.len(), 1);
        assert_eq!(unit.weapons[0].name, "Sentinel blade");
    }

    #[test]
    fn test_builder_multiple_units() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_player(1, "Bob", "World Eaters")
            .add_simple_unit(0, "Guard", KeywordSet::CUSTODIAN_GUARD_KEYWORDS, 3, 3, 10, 5)
            .add_simple_unit(
                0,
                "Wardens",
                KeywordSet::CUSTODIAN_WARDENS_KEYWORDS,
                3,
                3,
                15,
                5,
            )
            .add_simple_unit(
                1,
                "Berzerkers",
                KeywordSet::BERZERKERS_KEYWORDS,
                5,
                2,
                10,
                25,
            )
            .build();

        assert_eq!(state.units.len(), 3);
        assert_eq!(state.units[0].id, UnitId::new(0));
        assert_eq!(state.units[1].id, UnitId::new(1));
        assert_eq!(state.units[2].id, UnitId::new(2));

        assert_eq!(state.units[0].owner, PlayerId::new(0));
        assert_eq!(state.units[2].owner, PlayerId::new(1));
    }

    #[test]
    fn test_two_player_cp_setup() {
        let state = ScenarioBuilder::two_player_cp_setup().build();

        assert_eq!(state.battle_round, 1);
        assert_eq!(state.phase, Phase::Command);
        assert_eq!(state.active_player, PlayerId::new(0));
        assert_eq!(state.board, BoardDimensions::COMBAT_PATROL);
        assert_eq!(state.players.len(), 2);
        assert_eq!(state.objectives.len(), 4);

        let p0 = state.player(PlayerId::new(0)).unwrap();
        assert_eq!(p0.name, "Player 1");
        assert_eq!(p0.faction, "Adeptus Custodes");
        assert_eq!(p0.cp, 0);
        assert_eq!(p0.vp, 0);

        let p1 = state.player(PlayerId::new(1)).unwrap();
        assert_eq!(p1.name, "Player 2");
        assert_eq!(p1.faction, "World Eaters");
        assert_eq!(p1.cp, 0);
        assert_eq!(p1.vp, 0);

        // Verify objective positions
        let obj0 = state.objective(ObjectiveId::new(0)).unwrap();
        assert_eq!(obj0.position, Position::from_inches(22, 6));
        let obj1 = state.objective(ObjectiveId::new(1)).unwrap();
        assert_eq!(obj1.position, Position::from_inches(22, 24));
        let obj2 = state.objective(ObjectiveId::new(2)).unwrap();
        assert_eq!(obj2.position, Position::from_inches(11, 15));
        let obj3 = state.objective(ObjectiveId::new(3)).unwrap();
        assert_eq!(obj3.position, Position::from_inches(33, 15));
    }

    #[test]
    fn test_two_player_cp_setup_with_units() {
        let state = ScenarioBuilder::two_player_cp_setup()
            .add_simple_unit(0, "Custodian Guard", KeywordSet::CUSTODIAN_GUARD_KEYWORDS, 3, 3, 10, 5)
            .add_simple_unit(1, "Berzerkers", KeywordSet::BERZERKERS_KEYWORDS, 5, 2, 10, 25)
            .build();

        assert_eq!(state.units.len(), 2);
        assert_eq!(state.units[0].owner, PlayerId::new(0));
        assert_eq!(state.units[1].owner, PlayerId::new(1));
    }

    #[test]
    fn test_builder_add_models_helper() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10).add_models(5, 3)
            })
            .build();

        let unit = &state.units[0];
        assert_eq!(unit.models.len(), 5);
        for model in &unit.models {
            assert!(model.alive);
            assert_eq!(model.wounds_max, 3);
            assert_eq!(model.wounds_remaining, 3);
        }
    }

    #[test]
    fn test_builder_unit_status() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Reserves Unit", |ub| {
                ub.position(0, 0)
                    .status(UnitStatus::InStrategicReserves)
                    .add_models(3, 3)
            })
            .build();

        let unit = &state.units[0];
        assert_eq!(unit.status, UnitStatus::InStrategicReserves);
    }

    // -----------------------------------------------------------------------
    // TestGameState helper method tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_game_state_unit_by_name() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .add_simple_unit(0, "Wardens", KeywordSet::empty(), 3, 3, 15, 15)
            .build();

        let guard = state.unit_by_name("Guard").unwrap();
        assert_eq!(guard.id, UnitId::new(0));

        let wardens = state.unit_by_name("Wardens").unwrap();
        assert_eq!(wardens.id, UnitId::new(1));

        assert!(state.unit_by_name("Nonexistent").is_none());
    }

    #[test]
    fn test_game_state_mutability() {
        let mut state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        // Mutate player CP
        state.player_mut(PlayerId::new(0)).unwrap().cp = 3;
        assert_eq!(state.player(PlayerId::new(0)).unwrap().cp, 3);

        // Mutate unit position
        state.unit_mut(UnitId::new(0)).unwrap().position = Position::from_inches(20, 20);
        assert_eq!(
            state.unit(UnitId::new(0)).unwrap().position,
            Position::from_inches(20, 20)
        );

        // Kill a model
        state.unit_mut(UnitId::new(0)).unwrap().model_mut(1).unwrap().alive = false;
        state.unit_mut(UnitId::new(0)).unwrap().model_mut(1).unwrap().wounds_remaining = 0;
        assert_eq!(state.unit(UnitId::new(0)).unwrap().models_alive(), 2);
    }

    // -----------------------------------------------------------------------
    // StateAssertion tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_assert_battle_round_and_phase() {
        let state = ScenarioBuilder::new()
            .battle_round(2)
            .phase(Phase::Shooting)
            .add_player(0, "Alice", "Custodes")
            .build();

        StateAssertion::new(&state)
            .assert_battle_round(2)
            .assert_phase(Phase::Shooting)
            .assert_phase_and_round(Phase::Shooting, 2);
    }

    #[test]
    fn test_assert_active_player() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_player(1, "Bob", "World Eaters")
            .active_player(1)
            .build();

        StateAssertion::new(&state)
            .assert_active_player(1)
            .assert_active_player_id(PlayerId::new(1));
    }

    #[test]
    fn test_assert_player_resources() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_player(1, "Bob", "World Eaters")
            .set_cp(0, 3)
            .set_vp(0, 15)
            .set_cp(1, 1)
            .set_vp(1, 10)
            .build();

        StateAssertion::new(&state)
            .assert_player_cp(PlayerId::new(0), 3)
            .assert_player_vp(PlayerId::new(0), 15)
            .assert_player_cp(PlayerId::new(1), 1)
            .assert_player_vp(PlayerId::new(1), 10)
            .assert_player_cp_at_least(PlayerId::new(0), 2)
            .assert_player_vp_at_least(PlayerId::new(1), 5);
    }

    #[test]
    fn test_assert_unit_position() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 15)
            .build();

        StateAssertion::new(&state)
            .assert_unit_at_position(UnitId::new(0), 10, 15)
            .assert_unit_at_position_exact(UnitId::new(0), Position::from_inches(10, 15));
    }

    #[test]
    fn test_assert_unit_models_alive() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10)
                    .add_model(3, 3)
                    .add_dead_model(3)
                    .add_model(3, 3)
            })
            .build();

        StateAssertion::new(&state)
            .assert_unit_models_alive(UnitId::new(0), 2)
            .assert_unit_alive(UnitId::new(0));
    }

    #[test]
    fn test_assert_unit_destroyed() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10)
                    .add_dead_model(3)
                    .add_dead_model(3)
                    .add_dead_model(3)
            })
            .build();

        StateAssertion::new(&state)
            .assert_unit_destroyed(UnitId::new(0))
            .assert_unit_models_alive(UnitId::new(0), 0);
    }

    #[test]
    fn test_assert_model_wounds() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10)
                    .add_model(3, 3)
                    .add_wounded_model(3, 1)
                    .add_dead_model(3)
            })
            .build();

        StateAssertion::new(&state)
            .assert_model_wounds(UnitId::new(0), 0, 3)
            .assert_model_alive(UnitId::new(0), 0)
            .assert_model_wounds(UnitId::new(0), 1, 1)
            .assert_model_alive(UnitId::new(0), 1)
            .assert_model_wounds(UnitId::new(0), 2, 0)
            .assert_model_dead(UnitId::new(0), 2);
    }

    #[test]
    fn test_assert_unit_battle_shocked() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Shocked", |ub| {
                ub.position(10, 10).add_models(3, 3).battle_shocked(true)
            })
            .add_unit_builder(0, "Normal", |ub| {
                ub.position(15, 15).add_models(3, 3)
            })
            .build();

        StateAssertion::new(&state)
            .assert_unit_battle_shocked(UnitId::new(0))
            .assert_unit_not_battle_shocked(UnitId::new(1));
    }

    #[test]
    fn test_assert_unit_engaged() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_player(1, "Bob", "World Eaters")
            .add_unit_builder(0, "Guard", |ub| {
                ub.position(10, 10).add_models(3, 3).engaged(true)
            })
            .add_unit_builder(1, "Berzerkers", |ub| {
                ub.position(10, 11).add_models(5, 2).engaged(true)
            })
            .build();

        StateAssertion::new(&state)
            .assert_unit_engaged(UnitId::new(0))
            .assert_unit_engaged(UnitId::new(1));
    }

    #[test]
    fn test_assert_unit_not_engaged() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        StateAssertion::new(&state).assert_unit_not_engaged(UnitId::new(0));
    }

    #[test]
    fn test_assert_unit_status() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_unit_builder(0, "Reserves", |ub| {
                ub.position(0, 0)
                    .status(UnitStatus::InStrategicReserves)
                    .add_models(3, 3)
            })
            .build();

        StateAssertion::new(&state)
            .assert_unit_status(UnitId::new(0), UnitStatus::InStrategicReserves);
    }

    #[test]
    fn test_assert_unit_keywords() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(
                0,
                "Guard",
                KeywordSet::CUSTODIAN_GUARD_KEYWORDS,
                3,
                3,
                10,
                10,
            )
            .build();

        let infantry_battleline =
            KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]);

        StateAssertion::new(&state)
            .assert_unit_has_keywords(UnitId::new(0), infantry_battleline);
    }

    #[test]
    fn test_assert_objective_controlled() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_objective_controlled(0, 22, 15, 0)
            .add_objective(1, 11, 6)
            .build();

        StateAssertion::new(&state)
            .assert_objective_controlled_by(ObjectiveId::new(0), PlayerId::new(0))
            .assert_objective_uncontrolled(ObjectiveId::new(1));
    }

    #[test]
    fn test_assert_objective_position() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_objective(0, 22, 15)
            .build();

        StateAssertion::new(&state).assert_objective_at_position(ObjectiveId::new(0), 22, 15);
    }

    #[test]
    fn test_assert_aggregates() {
        let state = ScenarioBuilder::two_player_cp_setup()
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 5)
            .add_simple_unit(0, "Wardens", KeywordSet::empty(), 3, 3, 15, 5)
            .add_simple_unit(1, "Berzerkers", KeywordSet::empty(), 5, 2, 10, 25)
            .build();

        StateAssertion::new(&state)
            .assert_total_players(2)
            .assert_total_units(3)
            .assert_total_objectives(4)
            .assert_player_unit_count(PlayerId::new(0), 2)
            .assert_player_unit_count(PlayerId::new(1), 1);
    }

    #[test]
    fn test_assert_unit_within_range() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        // Guard is at (10, 10), check within 5" of (13, 14) => distance = 5"
        StateAssertion::new(&state).assert_unit_within_range_of(UnitId::new(0), 13, 14, 5);
    }

    #[test]
    #[should_panic(expected = "Expected battle round 3, but was 1")]
    fn test_assert_battle_round_fails() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .build();

        StateAssertion::new(&state).assert_battle_round(3);
    }

    #[test]
    #[should_panic(expected = "Expected phase Shooting, but was Command")]
    fn test_assert_phase_fails() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .build();

        StateAssertion::new(&state).assert_phase(Phase::Shooting);
    }

    #[test]
    #[should_panic(expected = "Expected unit UnitId(0) (Guard) to have 5 models alive, but had 3")]
    fn test_assert_models_alive_fails() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        StateAssertion::new(&state).assert_unit_models_alive(UnitId::new(0), 5);
    }

    #[test]
    #[should_panic(expected = "Expected unit UnitId(0) (Guard) to be battle-shocked")]
    fn test_assert_battle_shocked_fails() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        StateAssertion::new(&state).assert_unit_battle_shocked(UnitId::new(0));
    }

    #[test]
    #[should_panic(expected = "Expected unit UnitId(0) (Guard) to be engaged")]
    fn test_assert_engaged_fails() {
        let state = ScenarioBuilder::new()
            .add_player(0, "Alice", "Custodes")
            .add_simple_unit(0, "Guard", KeywordSet::empty(), 3, 3, 10, 10)
            .build();

        StateAssertion::new(&state).assert_unit_engaged(UnitId::new(0));
    }

    // -----------------------------------------------------------------------
    // Chaining assertions test (demonstrates fluent API)
    // -----------------------------------------------------------------------

    #[test]
    fn test_chained_assertions() {
        let state = ScenarioBuilder::two_player_cp_setup()
            .battle_round(2)
            .phase(Phase::Fight)
            .active_player(1)
            .set_cp(0, 1)
            .set_vp(0, 10)
            .set_cp(1, 0)
            .set_vp(1, 5)
            .add_unit_builder(0, "Guard", |ub| {
                ub.keywords(KeywordSet::CUSTODIAN_GUARD_KEYWORDS)
                    .position(22, 15)
                    .add_model(3, 3)
                    .add_model(3, 3)
                    .add_wounded_model(3, 1)
                    .engaged(true)
            })
            .add_unit_builder(1, "Berzerkers", |ub| {
                ub.keywords(KeywordSet::BERZERKERS_KEYWORDS)
                    .position(22, 16)
                    .add_model(2, 2)
                    .add_model(2, 2)
                    .add_dead_model(2)
                    .add_model(2, 2)
                    .add_dead_model(2)
                    .engaged(true)
                    .battle_shocked(true)
            })
            .build();

        StateAssertion::new(&state)
            // Game state
            .assert_battle_round(2)
            .assert_phase(Phase::Fight)
            .assert_active_player(1)
            .assert_total_players(2)
            .assert_total_units(2)
            .assert_total_objectives(4)
            // Player resources
            .assert_player_cp(PlayerId::new(0), 1)
            .assert_player_vp(PlayerId::new(0), 10)
            .assert_player_cp(PlayerId::new(1), 0)
            .assert_player_vp(PlayerId::new(1), 5)
            // Guard unit
            .assert_unit_at_position(UnitId::new(0), 22, 15)
            .assert_unit_models_alive(UnitId::new(0), 3)
            .assert_unit_alive(UnitId::new(0))
            .assert_unit_engaged(UnitId::new(0))
            .assert_unit_not_battle_shocked(UnitId::new(0))
            .assert_model_wounds(UnitId::new(0), 2, 1)
            .assert_unit_has_keywords(
                UnitId::new(0),
                KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
            )
            // Berzerkers unit
            .assert_unit_at_position(UnitId::new(1), 22, 16)
            .assert_unit_models_alive(UnitId::new(1), 3)
            .assert_unit_alive(UnitId::new(1))
            .assert_unit_engaged(UnitId::new(1))
            .assert_unit_battle_shocked(UnitId::new(1))
            .assert_model_alive(UnitId::new(1), 0)
            .assert_model_dead(UnitId::new(1), 2)
            .assert_model_dead(UnitId::new(1), 4);
    }

    // -----------------------------------------------------------------------
    // Full scenario test (realistic Combat Patrol setup)
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_combat_patrol_scenario() {
        // Set up a realistic Combat Patrol scenario:
        // Custodes vs World Eaters, Round 1 Command Phase
        let state = ScenarioBuilder::two_player_cp_setup()
            // Custodes units
            .add_unit_builder(0, "Tristraen", |ub| {
                ub.keywords(KeywordSet::TRISTRAEN_KEYWORDS)
                    .position(22, 6)
                    .add_model(6, 6)
            })
            .add_unit_builder(0, "Custodian Guard", |ub| {
                ub.keywords(KeywordSet::CUSTODIAN_GUARD_KEYWORDS)
                    .position(18, 5)
                    .add_models(3, 3)
            })
            .add_unit_builder(0, "Custodian Wardens", |ub| {
                ub.keywords(KeywordSet::CUSTODIAN_WARDENS_KEYWORDS)
                    .position(26, 5)
                    .add_models(3, 3)
            })
            .add_unit_builder(0, "Allarus Custodians", |ub| {
                ub.keywords(KeywordSet::ALLARUS_CUSTODIANS_KEYWORDS)
                    .position(22, 3)
                    .status(UnitStatus::InStrategicReserves)
                    .add_models(2, 4)
            })
            // World Eaters units
            .add_unit_builder(1, "Vorrakh", |ub| {
                ub.keywords(KeywordSet::VORRAKH_KEYWORDS)
                    .position(22, 24)
                    .add_model(10, 10)
            })
            .add_unit_builder(1, "Master of Executions", |ub| {
                ub.keywords(KeywordSet::MASTER_OF_EXECUTIONS_KEYWORDS)
                    .position(20, 25)
                    .add_model(4, 4)
            })
            .add_unit_builder(1, "Khorne Berzerkers", |ub| {
                ub.keywords(KeywordSet::BERZERKERS_KEYWORDS)
                    .position(18, 25)
                    .add_models(5, 2)
            })
            .add_unit_builder(1, "Jakhals", |ub| {
                ub.keywords(KeywordSet::JAKHALS_KEYWORDS)
                    .position(26, 25)
                    .add_models(10, 1)
            })
            .build();

        let assert = StateAssertion::new(&state);

        // Verify basic setup
        assert
            .assert_battle_round(1)
            .assert_phase(Phase::Command)
            .assert_active_player(0)
            .assert_total_players(2)
            .assert_total_units(8)
            .assert_total_objectives(4);

        // Verify player unit counts
        assert
            .assert_player_unit_count(PlayerId::new(0), 4)
            .assert_player_unit_count(PlayerId::new(1), 4);

        // Verify all units start healthy and not battle-shocked
        for i in 0..8 {
            let uid = UnitId::new(i);
            assert
                .assert_unit_alive(uid)
                .assert_unit_not_battle_shocked(uid)
                .assert_unit_not_engaged(uid);
        }

        // Verify model counts
        assert
            .assert_unit_models_alive(UnitId::new(0), 1) // Tristraen (character)
            .assert_unit_models_alive(UnitId::new(1), 3) // Custodian Guard
            .assert_unit_models_alive(UnitId::new(2), 3) // Custodian Wardens
            .assert_unit_models_alive(UnitId::new(3), 2) // Allarus Custodians
            .assert_unit_models_alive(UnitId::new(4), 1) // Vorrakh
            .assert_unit_models_alive(UnitId::new(5), 1) // Master of Executions
            .assert_unit_models_alive(UnitId::new(6), 5) // Berzerkers
            .assert_unit_models_alive(UnitId::new(7), 10); // Jakhals

        // Verify Allarus in reserves
        assert.assert_unit_status(UnitId::new(3), UnitStatus::InStrategicReserves);

        // Verify keyword assertions
        assert
            .assert_unit_has_keywords(
                UnitId::new(0),
                KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Character]),
            )
            .assert_unit_has_keywords(
                UnitId::new(4),
                KeywordSet::from_keywords(&[Keyword::Monster, Keyword::Character, Keyword::Daemon]),
            );

        // All objectives should be uncontrolled at start
        for i in 0..4 {
            assert.assert_objective_uncontrolled(ObjectiveId::new(i));
        }

        // Both players start with 0 CP, 0 VP
        assert
            .assert_player_cp(PlayerId::new(0), 0)
            .assert_player_vp(PlayerId::new(0), 0)
            .assert_player_cp(PlayerId::new(1), 0)
            .assert_player_vp(PlayerId::new(1), 0);
    }
}
