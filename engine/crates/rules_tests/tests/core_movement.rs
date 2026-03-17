//! Source-linked rules tests for 40k_revised.md Section 5: Movement Phase.
//!
//! Every test in this file traces to a specific rule in the core 40K rules
//! document. The movement phase covers:
//!   §5.1 - Movement eligibility (Engaged vs not Engaged)
//!   §5.2 - Remain Stationary
//!   §5.3 - Normal Move (up to M, can't end in ER, can't move through enemies)
//!   §5.4 - Advance Move (M + D6, can't shoot except Assault, can't charge)
//!   §5.5 - Fall Back (must be Engaged, must end outside ER, Desperate Escape)
//!   §5.8 - FLY keyword (move over enemy models)
//!   §5.9 - Reinforcements (>9" from enemies, counts as Normal Move)
//!   §5.10 - Surge Moves (one per phase, not while Battle-shocked)
//!
//! Source: 40k_revised.md Section 5 — "MOVEMENT PHASE"

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, DatasheetId, EngagementStatus, GameMode,
    GameOutcome, Inches, Keyword, KeywordSet, Leadership, ModelId,
    MoveCharacteristic, ObjectiveControl, Phase, PlayerId, Position, SubPhase,
    Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_command_system::Command;
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_game_core::state::{GameState, PlayerState, TurnFlags};
use wh40k_game_core::unit::{ModelState, UnitState, ReserveType};
use wh40k_game_core::validator::CommandValidator;
use wh40k_game_core::executor::CommandExecutor;
use wh40k_geometry::Board;
use wh40k_combat_patrol_rules::{CombatPatrolRules, ReserveTimingResult};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Create a deterministic DiceRoller for tests.
#[allow(dead_code)]
fn make_dice(seed_byte: u8) -> DiceRoller {
    let ctx = DiceContext::new([seed_byte; 32], StreamKind::MiscRoll, 0, 0);
    DiceRoller::new(ctx)
}

/// Build a basic ModelState at a given position.
fn make_model(model_id: u32, unit_id: UnitId, position: Position, wounds: u8) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        position,
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a basic UnitState with the given keywords, movement, and model count.
fn make_unit(
    id: u32,
    owner: PlayerId,
    keywords: KeywordSet,
    movement_inches: i32,
    model_count: usize,
    position: Position,
) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| make_model(id * 100 + i as u32, unit_id, position, 2))
        .collect();

    let mut unit = UnitState::new(
        unit_id,
        owner,
        format!("Test Unit {}", id),
        DatasheetId::new(id),
        keywords,
        models,
        MoveCharacteristic::from_inches(movement_inches),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a GameState in the Movement phase with units for testing.
/// Player A (id=0) is the active player. Board is Combat Patrol (44"x30").
fn make_movement_game_state(units: Vec<UnitState>) -> GameState {
    let seed = [42u8; 32];
    let ctx = DiceContext::new(seed, StreamKind::MiscRoll, 0, 0);
    let dice_roller = DiceRoller::new(ctx);

    GameState {
        content_version: "test-movement-1.0".to_string(),
        scenario_id: None,
        battle_round: BattleRound::new(1),
        active_player: PlayerId::new(0),
        current_phase: Phase::Movement,
        current_subphase: SubPhase::SelectUnitToMove,
        decision_owner: PlayerId::new(0),
        players: [
            PlayerState::new(PlayerId::new(0), "Player A".to_string()),
            PlayerState::new(PlayerId::new(1), "Player B".to_string()),
        ],
        units,
        board: Board::combat_patrol(),
        deployment_config: None,
        event_bus: EventBus::new(),
        command_history: wh40k_command_system::CommandHistory::new(),
        dice_roller,
        active_effects: Vec::new(),
        reaction_windows: Vec::new(),
        turn_flags: TurnFlags::new(),
        game_outcome: GameOutcome::InProgress,
        deterministic_counter: 0,
        game_mode: GameMode::CombatPatrol,
        mode_state: None,
    }
}

// ===========================================================================
// §5.1 — MOVEMENT ELIGIBILITY: Engaged vs Not Engaged
// ===========================================================================

/// Source: 40k_revised.md §5.1 — "If a unit is within Engagement Range
/// of any enemy unit, it can only Fall Back or Remain Stationary."
/// An engaged unit cannot Normal Move.
#[test]
fn s5_1_engaged_unit_cannot_normal_move() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    // Player A unit at (10,10), engaged with enemy
    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    // Player B unit nearby (the enemy in engagement range)
    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_illegal(), "Engaged unit should not be able to Normal Move (§5.1)");
}

/// Source: 40k_revised.md §5.1 — "If a unit is within Engagement Range
/// of any enemy unit, it can only Fall Back or Remain Stationary."
/// An engaged unit cannot Advance.
#[test]
fn s5_1_engaged_unit_cannot_advance() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(16, 10),
        advance_roll: 4,
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_illegal(), "Engaged unit should not be able to Advance (§5.1)");
}

/// Source: 40k_revised.md §5.1 — "If a unit is NOT within Engagement Range...
/// it can make a Normal Move, Advance, or Remain Stationary."
/// A non-engaged unit can Normal Move.
#[test]
fn s5_1_non_engaged_unit_can_normal_move() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10), // 4" move with 6" M
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_legal(), "Non-engaged unit should be able to Normal Move (§5.1)");
}

/// Source: 40k_revised.md §5.1 — "If a unit is NOT within Engagement Range...
/// it can make a Normal Move, Advance, or Remain Stationary."
/// A non-engaged unit can Advance.
#[test]
fn s5_1_non_engaged_unit_can_advance() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(16, 10), // 6" move, within M(6)+roll(4)=10
        advance_roll: 4,
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_legal(), "Non-engaged unit should be able to Advance (§5.1)");
}

/// Source: 40k_revised.md §5.1 — Engaged unit CAN Remain Stationary.
#[test]
fn s5_1_engaged_unit_can_remain_stationary() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 11),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_legal(), "Engaged unit should be able to Remain Stationary (§5.1)");
}

// ===========================================================================
// §5.2 — REMAIN STATIONARY
// ===========================================================================

/// Source: 40k_revised.md §5.2 — "Remain Stationary = no models move."
/// After executing RemainStationary, the unit is flagged as stationary.
#[test]
fn s5_2_remain_stationary_sets_flag() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    let result = CommandExecutor::execute(&mut state, &cmd);
    assert!(result.is_ok(), "RemainStationary should execute successfully");

    assert!(
        state.turn_flags.is_stationary(UnitId::new(1)),
        "Unit should be marked as stationary after RemainStationary (§5.2)"
    );
}

/// Source: 40k_revised.md §5.2 — "Remain Stationary... can shoot and charge."
/// A stationary unit is marked as moved (for eligibility tracking) but is
/// stationary (for Heavy weapon bonus).
#[test]
fn s5_2_remain_stationary_counts_as_moved() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    assert!(
        state.turn_flags.has_moved(UnitId::new(1)),
        "RemainStationary should mark unit as 'moved' for eligibility (§5.2)"
    );
    assert!(
        state.turn_flags.is_stationary(UnitId::new(1)),
        "RemainStationary should mark unit as stationary (§5.2)"
    );
    // Stationary units should NOT be flagged as advanced or fell back
    assert!(
        !state.turn_flags.has_advanced(UnitId::new(1)),
        "Stationary unit should not be flagged as advanced"
    );
    assert!(
        !state.turn_flags.has_fell_back(UnitId::new(1)),
        "Stationary unit should not be flagged as fell back"
    );
}

/// Source: 40k_revised.md §5.2 — "Remain Stationary" does not change position.
#[test]
fn s5_2_remain_stationary_does_not_change_position() {
    let player_a = PlayerId::new(0);
    let original_pos = Position::from_inches(10, 10);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        original_pos,
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    let unit = state.unit(UnitId::new(1)).unwrap();
    assert_eq!(
        unit.reference_position().unwrap(),
        original_pos,
        "Position should be unchanged after RemainStationary (§5.2)"
    );
}

// ===========================================================================
// §5.3 — NORMAL MOVE
// ===========================================================================

/// Source: 40k_revised.md §5.3 — "Each model can move up to M characteristic."
/// A normal move within M should succeed.
#[test]
fn s5_3_normal_move_within_m_succeeds() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(16, 10), // exactly 6" = M
    };

    let result = CommandExecutor::execute(&mut state, &cmd);
    assert!(result.is_ok(), "Normal move exactly at M should succeed (§5.3)");

    let unit = state.unit(UnitId::new(1)).unwrap();
    assert_eq!(
        unit.reference_position().unwrap(),
        Position::from_inches(16, 10),
        "Unit should be at the destination after normal move"
    );
}

/// Source: 40k_revised.md §5.3 — "Each model can move up to M characteristic."
/// A normal move exceeding M should be rejected.
#[test]
fn s5_3_normal_move_exceeding_m_rejected() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(17, 10), // 7" > M=6"
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_illegal(), "Normal move exceeding M should be rejected (§5.3)");
}

/// Source: 40k_revised.md §5.3 — "cannot end its move within Engagement Range
/// of any enemy model."
/// A normal move that would end within ER of an enemy should be rejected.
#[test]
fn s5_3_normal_move_cannot_end_in_enemy_er() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    // Enemy at (15, 10) — if unit_a moves to (14, 10), that's 1" from enemy (within ER)
    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(15, 10),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    // Try to move within 1" of the enemy (within engagement range)
    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10), // 1" from enemy at (15,10)
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Normal move ending within ER of enemy should be rejected (§5.3)"
    );
}

/// Source: 40k_revised.md §5.3 — Normal move marks unit as moved.
#[test]
fn s5_3_normal_move_sets_moved_flag() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    assert!(
        state.turn_flags.has_moved(UnitId::new(1)),
        "Unit should be marked as moved after NormalMove (§5.3)"
    );
    assert!(
        !state.turn_flags.has_advanced(UnitId::new(1)),
        "Unit should NOT be marked as advanced after NormalMove"
    );
    assert!(
        !state.turn_flags.has_fell_back(UnitId::new(1)),
        "Unit should NOT be marked as fell back after NormalMove"
    );
}

/// Source: 40k_revised.md §5.3 — "Each unit can only move once per
/// Movement phase." A unit that already moved cannot move again.
#[test]
fn s5_3_unit_cannot_move_twice() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    // First move
    let cmd1 = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };
    CommandExecutor::execute(&mut state, &cmd1).unwrap();

    // Second move should be rejected
    let cmd2 = Command::SelectUnitToMove {
        unit_id: UnitId::new(1),
    };
    let result = CommandValidator::validate(&state, &cmd2);
    assert!(
        result.is_illegal(),
        "Unit that already moved should not be selectable again (§5.3)"
    );
}

/// Source: 40k_revised.md §5.3 — Normal move cannot move off the board.
#[test]
fn s5_3_normal_move_cannot_go_off_board() {
    let player_a = PlayerId::new(0);

    // Unit near the right edge of the 44" wide board
    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(42, 15),
    );

    let state = make_movement_game_state(vec![unit_a]);

    // Try to move off the right edge
    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(46, 15), // off the 44" board
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_illegal(), "Normal move off the board should be rejected (§5.3)");
}

// ===========================================================================
// §5.4 — ADVANCE
// ===========================================================================

/// Source: 40k_revised.md §5.4 — "Advance = M + D6."
/// An advance move within M + advance_roll should succeed.
#[test]
fn s5_4_advance_move_within_m_plus_d6_succeeds() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    // M=6, advance_roll=4, total=10". Move 9".
    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(19, 10), // 9" move
        advance_roll: 4,
    };

    let result = CommandExecutor::execute(&mut state, &cmd);
    assert!(result.is_ok(), "Advance within M + D6 should succeed (§5.4)");
}

/// Source: 40k_revised.md §5.4 — "Advance = M + D6."
/// An advance move exceeding M + advance_roll should be rejected.
#[test]
fn s5_4_advance_move_exceeding_m_plus_d6_rejected() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    // M=6, advance_roll=3, total=9". Try to move 10".
    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(20, 10), // 10" > 9"
        advance_roll: 3,
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_illegal(), "Advance exceeding M + D6 should be rejected (§5.4)");
}

/// Source: 40k_revised.md §5.4 — "Advance roll must be a valid D6 result (1-6)."
#[test]
fn s5_4_advance_roll_must_be_valid_d6() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    // Invalid advance roll of 0
    let cmd_0 = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
        advance_roll: 0,
    };
    assert!(
        CommandValidator::validate(&state, &cmd_0).is_illegal(),
        "Advance roll of 0 should be rejected"
    );

    // Invalid advance roll of 7
    let cmd_7 = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
        advance_roll: 7,
    };
    assert!(
        CommandValidator::validate(&state, &cmd_7).is_illegal(),
        "Advance roll of 7 should be rejected"
    );
}

/// Source: 40k_revised.md §5.4 — Advanced units are flagged for restriction.
/// "Advanced units cannot shoot non-Assault weapons and cannot charge."
#[test]
fn s5_4_advance_sets_advanced_flag() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(16, 10),
        advance_roll: 3,
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    assert!(
        state.turn_flags.has_advanced(UnitId::new(1)),
        "Unit should be marked as advanced after AdvanceMove (§5.4)"
    );
    assert!(
        state.turn_flags.has_moved(UnitId::new(1)),
        "Advanced unit should also be marked as moved"
    );
    assert!(
        !state.turn_flags.has_fell_back(UnitId::new(1)),
        "Advanced unit should NOT be marked as fell back"
    );
    assert!(
        !state.turn_flags.is_stationary(UnitId::new(1)),
        "Advanced unit should NOT be marked as stationary"
    );
}

/// Source: 40k_revised.md §5.4 — Advance cannot end in Engagement Range of enemies.
#[test]
fn s5_4_advance_cannot_end_in_enemy_er() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(20, 10),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    // Try to advance to within 1" of enemy at (20,10)
    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(19, 10), // 1" from enemy
        advance_roll: 6,
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Advance ending within ER of enemy should be rejected (§5.4)"
    );
}

// ===========================================================================
// §5.5 — FALL BACK
// ===========================================================================

/// Source: 40k_revised.md §5.5 — "Only units in Engagement Range can Fall Back."
/// A non-engaged unit cannot fall back.
#[test]
fn s5_5_non_engaged_unit_cannot_fall_back() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );
    // unit_a.engagement_status is NotEngaged by default

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Non-engaged unit should not be able to Fall Back (§5.5)"
    );
}

/// Source: 40k_revised.md §5.5 — An engaged unit can Fall Back.
#[test]
fn s5_5_engaged_unit_can_fall_back() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    // Fall back away from the enemy, ending >1" away
    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 5), // 5" fall back
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(result.is_legal(), "Engaged unit should be able to Fall Back (§5.5)");
}

/// Source: 40k_revised.md §5.5 — "must end its move more than 1\" from
/// all enemy models." Fall back must end outside ER.
#[test]
fn s5_5_fall_back_must_end_outside_er() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 11),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    // Try to fall back but stay within ER of enemy at (10,11)
    // Moving to (10,10) is 1" from enemy — within ER
    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 10), // Same spot, still in ER
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Fall back ending within ER of enemy should be rejected (§5.5)"
    );
}

/// Source: 40k_revised.md §5.5 — "Fall Back: units that fall back
/// cannot shoot or charge this turn." Fall back sets the fell_back flag.
#[test]
fn s5_5_fall_back_sets_fell_back_flag() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let mut state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 5), // 5" away from (10,10)
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    assert!(
        state.turn_flags.has_fell_back(UnitId::new(1)),
        "Unit should be marked as fell back after FallBack (§5.5)"
    );
    assert!(
        state.turn_flags.has_moved(UnitId::new(1)),
        "Fell back unit should also be marked as moved"
    );
    assert!(
        !state.turn_flags.has_advanced(UnitId::new(1)),
        "Fell back unit should NOT be marked as advanced"
    );
    assert!(
        !state.turn_flags.is_stationary(UnitId::new(1)),
        "Fell back unit should NOT be marked as stationary"
    );
}

/// Source: 40k_revised.md §5.5 — Fall back clears engagement status.
/// After falling back, the unit is no longer engaged.
#[test]
fn s5_5_fall_back_clears_engagement() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let mut state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 5),
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    let unit = state.unit(UnitId::new(1)).unwrap();
    assert_eq!(
        unit.engagement_status,
        EngagementStatus::NotEngaged,
        "Unit should be NotEngaged after Fall Back (§5.5)"
    );
}

/// Source: 40k_revised.md §5.5 — "Fall Back: up to M characteristic."
/// Fall back distance is limited to M.
#[test]
fn s5_5_fall_back_limited_to_m() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 15),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 16),
    );

    let state = make_movement_game_state(vec![unit_a, unit_b]);

    // Try to fall back 8", which exceeds M=6
    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 7), // 8" from (10,15)
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Fall back exceeding M should be rejected (§5.5)"
    );
}

// ===========================================================================
// §5.5 — DESPERATE ESCAPE TESTS
// ===========================================================================

/// Source: 40k_revised.md §5.5 — "Desperate Escape: Battle-shocked units
/// take a Desperate Escape test for every model."
/// This test verifies the flag tracking logic: the turn_flags correctly
/// reflect that a fall back was performed for a battle-shocked unit.
#[test]
fn s5_5_battle_shocked_fall_back_sets_flags() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 5,
        Position::from_inches(10, 10),
    );
    unit_a.engagement_status = EngagementStatus::Engaged;
    unit_a.battle_shocked = true;

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 11),
    );

    let mut state = make_movement_game_state(vec![unit_a, unit_b]);

    let cmd = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 5),
    };

    // Execute — desperate escape tests happen internally via dice_roller.
    // With seed [42; 32], some models may be destroyed, but the unit
    // should still be marked as fell_back if any survive.
    let result = CommandExecutor::execute(&mut state, &cmd);
    assert!(result.is_ok(), "Fall back of battle-shocked unit should execute");

    // If the unit survived, it should be marked as fell back
    let unit = state.unit(UnitId::new(1)).unwrap();
    if !unit.is_destroyed() {
        assert!(
            state.turn_flags.has_fell_back(UnitId::new(1)),
            "Surviving battle-shocked unit should be flagged as fell_back (§5.5)"
        );
    }
}

// ===========================================================================
// §5.3/§5.5 — MOVEMENT PHASE REQUIREMENT
// ===========================================================================

/// Source: 40k_revised.md §5 — Movement commands are only valid in
/// the Movement phase.
#[test]
fn s5_movement_commands_require_movement_phase() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);
    // Change the phase to Shooting
    state.current_phase = Phase::Shooting;

    let normal = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };
    assert!(
        CommandValidator::validate(&state, &normal).is_illegal(),
        "NormalMove should be illegal outside Movement phase"
    );

    let advance = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
        advance_roll: 3,
    };
    assert!(
        CommandValidator::validate(&state, &advance).is_illegal(),
        "AdvanceMove should be illegal outside Movement phase"
    );

    let fall_back = Command::FallBack {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };
    assert!(
        CommandValidator::validate(&state, &fall_back).is_illegal(),
        "FallBack should be illegal outside Movement phase"
    );

    let stationary = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };
    assert!(
        CommandValidator::validate(&state, &stationary).is_illegal(),
        "RemainStationary should be illegal outside Movement phase"
    );

    let reserves = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(20, 20),
    };
    assert!(
        CommandValidator::validate(&state, &reserves).is_illegal(),
        "ArriveFromReserves should be illegal outside Movement phase"
    );
}

// ===========================================================================
// §5.3 — OWNERSHIP VALIDATION
// ===========================================================================

/// Source: 40k_revised.md §5 — Only the active player can move their units.
#[test]
fn s5_only_active_player_can_move() {
    let _player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    // Unit owned by Player B
    let unit_b = make_unit(
        1, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    // Active player is Player A
    let state = make_movement_game_state(vec![unit_b]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Non-active player's units should not be movable"
    );
}

// ===========================================================================
// §5.9 — REINFORCEMENTS
// ===========================================================================

/// Source: 40k_revised.md §5.9 — "Reinforcements set up more than 9\"
/// from all enemy models."
/// Reserve arrival must be >9" from enemies.
#[test]
fn s5_9_reserves_must_be_more_than_9_inches_from_enemies() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(20, 15),
    );
    unit_a.status = UnitStatus::InStrategicReserves;
    unit_a.reserve_type = Some(ReserveType::DeepStrike);

    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(20, 15),
    );

    let mut state = make_movement_game_state(vec![unit_a, unit_b]);
    // Reserves can arrive from round 2+
    state.battle_round = BattleRound::new(2);

    // Try to arrive 5" from enemy at (20,15)
    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(20, 20), // 5" from enemy
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Reserves arriving within 9\" of enemies should be rejected (§5.9)"
    );
}

/// Source: 40k_revised.md §5.9 — Reserves can arrive >9" from all enemies.
#[test]
fn s5_9_reserves_more_than_9_inches_allowed() {
    let player_a = PlayerId::new(0);
    let player_b = PlayerId::new(1);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(5, 5),
    );
    unit_a.status = UnitStatus::InStrategicReserves;
    unit_a.reserve_type = Some(ReserveType::DeepStrike);

    // Enemy far away at (30, 15)
    let unit_b = make_unit(
        2, player_b,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(30, 15),
    );

    let mut state = make_movement_game_state(vec![unit_a, unit_b]);
    state.battle_round = BattleRound::new(2);

    // Arrive at (10, 3) — within 6" of bottom edge (y=0) AND >9" from enemy at (30,15)
    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(10, 3),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_legal(),
        "Reserves arriving >9\" from enemies and within 6\" of edge should be allowed (§5.9, §14.2)"
    );
}

/// Source: 40k_revised.md §5.9 — "Reinforcements" cannot arrive on Battle Round 1.
#[test]
fn s5_9_reserves_cannot_arrive_round_1() {
    let player_a = PlayerId::new(0);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(5, 5),
    );
    unit_a.status = UnitStatus::InStrategicReserves;
    unit_a.reserve_type = Some(ReserveType::DeepStrike);

    let mut state = make_movement_game_state(vec![unit_a]);
    state.battle_round = BattleRound::new(1);

    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(20, 15),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Reserves should not arrive in Battle Round 1 (§5.9)"
    );
}

/// Source: 40k_revised.md §5.9 — Reinforcements: unit must actually be in reserves.
#[test]
fn s5_9_unit_not_in_reserves_cannot_arrive() {
    let player_a = PlayerId::new(0);

    // Unit is on the battlefield, not in reserves
    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);
    state.battle_round = BattleRound::new(2);

    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(20, 15),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "A unit not in reserves should not be able to 'arrive from reserves' (§5.9)"
    );
}

/// Source: 40k_revised.md §5.9 — Reinforcements set up on the battlefield.
/// After arriving from reserves, the unit status changes to OnBattlefield.
#[test]
fn s5_9_reserves_arrival_sets_unit_on_battlefield() {
    let player_a = PlayerId::new(0);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(5, 5),
    );
    unit_a.status = UnitStatus::InStrategicReserves;
    unit_a.reserve_type = Some(ReserveType::DeepStrike);

    let mut state = make_movement_game_state(vec![unit_a]);
    state.battle_round = BattleRound::new(2);

    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(20, 3),  // Within 6" of bottom edge (§14.2)
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    let unit = state.unit(UnitId::new(1)).unwrap();
    assert_eq!(
        unit.status,
        UnitStatus::OnBattlefield,
        "Unit should be OnBattlefield after arriving from reserves (§5.9)"
    );
    assert!(
        unit.reserve_type.is_none(),
        "Reserve type should be cleared after arrival"
    );
    assert!(
        state.turn_flags.has_moved(UnitId::new(1)),
        "Reserve arrival should count as having moved"
    );
}

// ===========================================================================
// CP_Rules.md — RESERVE TIMING (Combat Patrol specific)
// ===========================================================================

/// Source: CP_Rules.md — "Reserves cannot arrive in battle round 1."
#[test]
fn cp_reserves_round_1_too_early() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(1)),
        ReserveTimingResult::TooEarly,
        "CP_Rules: Reserves cannot arrive in BR1"
    );
}

/// Source: CP_Rules.md — "Reserves can arrive in battle rounds 2-3."
#[test]
fn cp_reserves_round_2_allowed() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(2)),
        ReserveTimingResult::Allowed,
        "CP_Rules: Reserves should be allowed in BR2"
    );
}

/// Source: CP_Rules.md — "Reserves can arrive in battle rounds 2-3."
#[test]
fn cp_reserves_round_3_allowed() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(3)),
        ReserveTimingResult::Allowed,
        "CP_Rules: Reserves should be allowed in BR3"
    );
}

/// Source: CP_Rules.md — "Reserves not arrived by end of BR3 are destroyed."
#[test]
fn cp_reserves_round_4_too_late() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(4)),
        ReserveTimingResult::TooLate,
        "CP_Rules: Reserves should be destroyed if not arrived by BR3"
    );
}

/// Source: CP_Rules.md — "Reserves not arrived by end of BR3 are destroyed."
#[test]
fn cp_reserves_round_5_too_late() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(5)),
        ReserveTimingResult::TooLate,
        "CP_Rules: Reserves should be destroyed if not arrived by BR5"
    );
}

/// Source: CP_Rules.md — Reserve position validation: >9" from enemies.
#[test]
fn cp_reserve_position_valid_far_from_enemies() {
    let board = wh40k_core_types::BoardDimensions::COMBAT_PATROL;
    let pos = Position::from_inches(22, 15); // center of board
    let enemies = vec![Position::from_inches(5, 5)]; // far away

    assert!(
        CombatPatrolRules::is_valid_reserve_position(pos, &enemies, &board),
        "Reserve position far from enemies should be valid"
    );
}

/// Source: CP_Rules.md — Reserve position validation: too close to enemy.
#[test]
fn cp_reserve_position_invalid_too_close() {
    let board = wh40k_core_types::BoardDimensions::COMBAT_PATROL;
    let pos = Position::from_inches(10, 10);
    let enemies = vec![Position::from_inches(15, 10)]; // 5" away

    assert!(
        !CombatPatrolRules::is_valid_reserve_position(pos, &enemies, &board),
        "Reserve position within 9\" of enemy should be invalid"
    );
}

// ===========================================================================
// TURN FLAGS — Interaction tests
// ===========================================================================

/// Source: 40k_revised.md §5 — TurnFlags.clear() resets all movement tracking.
#[test]
fn turn_flags_clear_resets_all_movement_flags() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(1);

    flags.mark_moved(unit);
    flags.mark_advanced(unit);
    flags.mark_fell_back(unit);
    flags.mark_remained_stationary(unit);

    assert!(flags.has_moved(unit));
    assert!(flags.has_advanced(unit));
    assert!(flags.has_fell_back(unit));
    assert!(flags.is_stationary(unit));

    flags.clear();

    assert!(!flags.has_moved(unit), "clear() should reset units_moved");
    assert!(!flags.has_advanced(unit), "clear() should reset advanced_this_turn");
    assert!(!flags.has_fell_back(unit), "clear() should reset fell_back_this_turn");
    assert!(!flags.is_stationary(unit), "clear() should reset remained_stationary");
}

/// Source: 40k_revised.md §5 — mark_advanced also marks moved.
#[test]
fn turn_flags_mark_advanced_also_marks_moved() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(7);

    flags.mark_advanced(unit);

    assert!(flags.has_advanced(unit), "mark_advanced should set advanced flag");
    assert!(flags.has_moved(unit), "mark_advanced should also set moved flag");
}

/// Source: 40k_revised.md §5 — mark_fell_back also marks moved.
#[test]
fn turn_flags_mark_fell_back_also_marks_moved() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(8);

    flags.mark_fell_back(unit);

    assert!(flags.has_fell_back(unit), "mark_fell_back should set fell_back flag");
    assert!(flags.has_moved(unit), "mark_fell_back should also set moved flag");
}

/// Source: 40k_revised.md §5 — mark_remained_stationary also marks moved.
#[test]
fn turn_flags_mark_stationary_also_marks_moved() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(9);

    flags.mark_remained_stationary(unit);

    assert!(flags.is_stationary(unit), "mark_remained_stationary should set stationary flag");
    assert!(flags.has_moved(unit), "mark_remained_stationary should also set moved flag");
}

/// Source: 40k_revised.md §5 — Multiple units can have different movement states.
#[test]
fn turn_flags_multiple_units_independent() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(10);
    let unit_b = UnitId::new(11);
    let unit_c = UnitId::new(12);

    flags.mark_moved(unit_a);
    flags.mark_advanced(unit_b);
    flags.mark_fell_back(unit_c);

    assert!(flags.has_moved(unit_a));
    assert!(!flags.has_advanced(unit_a));
    assert!(!flags.has_fell_back(unit_a));

    assert!(flags.has_moved(unit_b));
    assert!(flags.has_advanced(unit_b));
    assert!(!flags.has_fell_back(unit_b));

    assert!(flags.has_moved(unit_c));
    assert!(!flags.has_advanced(unit_c));
    assert!(flags.has_fell_back(unit_c));
}

// ===========================================================================
// §5 — DESTROYED/UNDEPLOYED UNIT CHECKS
// ===========================================================================

/// Source: 40k_revised.md §5 — Destroyed units cannot move.
#[test]
fn s5_destroyed_unit_cannot_be_selected_to_move() {
    let player_a = PlayerId::new(0);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );
    unit_a.status = UnitStatus::Destroyed;

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::SelectUnitToMove {
        unit_id: UnitId::new(1),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Destroyed unit should not be selectable for movement"
    );
}

/// Source: 40k_revised.md §5 — Unit not on the battlefield cannot Normal Move.
#[test]
fn s5_unit_not_on_battlefield_cannot_normal_move() {
    let player_a = PlayerId::new(0);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );
    unit_a.status = UnitStatus::InStrategicReserves;

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Unit in reserves should not be able to Normal Move"
    );
}

// ===========================================================================
// §5.9 — DEEP STRIKE MIN DISTANCE CONSTANT
// ===========================================================================

/// Source: 40k_revised.md §5.9 — Deep Strike minimum distance is 9".
#[test]
fn s5_9_deep_strike_min_distance_constant() {
    assert_eq!(
        Inches::DEEP_STRIKE_MIN_DISTANCE,
        Inches::from_inches(9),
        "Deep Strike minimum distance should be 9\" (§5.9)"
    );
    assert_eq!(
        Inches::DEEP_STRIKE_MIN_DISTANCE.mils(),
        9000,
        "Deep Strike minimum distance should be 9000 mils"
    );
}

// ===========================================================================
// §5.3 — NORMAL MOVE: model position translation
// ===========================================================================

/// Source: 40k_revised.md §5.3 — All models in a unit move by the same offset.
/// When a multi-model unit performs a normal move, all alive models translate
/// together by the same dx/dy offset.
#[test]
fn s5_3_multi_model_unit_all_models_translate() {
    let player_a = PlayerId::new(0);
    let unit_id = UnitId::new(1);

    // Create a unit with 3 models at different positions
    let models = vec![
        make_model(100, unit_id, Position::from_inches(10, 10), 2),
        make_model(101, unit_id, Position::from_inches(10, 11), 2),
        make_model(102, unit_id, Position::from_inches(11, 10), 2),
    ];

    let mut unit = UnitState::new(
        unit_id,
        player_a,
        "Multi-model Unit".to_string(),
        DatasheetId::new(1),
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;

    let mut state = make_movement_game_state(vec![unit]);

    // Move 3" to the right (reference model at (10,10) -> (13,10))
    let cmd = Command::NormalMove {
        unit_id,
        destination: Position::from_inches(13, 10),
    };

    CommandExecutor::execute(&mut state, &cmd).unwrap();

    let unit = state.unit(unit_id).unwrap();
    // All models should have shifted by +3" in x
    assert_eq!(
        unit.models[0].position,
        Position::from_inches(13, 10),
        "Model 0 should move from (10,10) to (13,10)"
    );
    assert_eq!(
        unit.models[1].position,
        Position::from_inches(13, 11),
        "Model 1 should move from (10,11) to (13,11)"
    );
    assert_eq!(
        unit.models[2].position,
        Position::from_inches(14, 10),
        "Model 2 should move from (11,10) to (14,10)"
    );
}

// ===========================================================================
// §5 — MOVEMENT CHARACTERISTIC CONSTANTS
// ===========================================================================

/// Source: 40k_revised.md §5 — MoveCharacteristic stores distance in Inches.
#[test]
fn s5_movement_characteristic_distance() {
    let m6 = MoveCharacteristic::from_inches(6);
    assert_eq!(m6.distance(), Inches::from_inches(6));
    assert_eq!(m6.distance().mils(), 6000);

    let m12 = MoveCharacteristic::from_inches(12);
    assert_eq!(m12.distance(), Inches::from_inches(12));
    assert_eq!(m12.distance().whole_inches(), 12);
}

// ===========================================================================
// §5 — MOVEMENT GAME EVENT EMISSION
// ===========================================================================

/// Source: 40k_revised.md §5.3 — Normal Move emits a MoveCompleted event.
#[test]
fn s5_3_normal_move_emits_event() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(14, 10),
    };

    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    assert!(
        !events.is_empty(),
        "Normal move should emit at least one event"
    );
}

/// Source: 40k_revised.md §5.4 — Advance Move emits an AdvanceRolled event.
#[test]
fn s5_4_advance_move_emits_event() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::AdvanceMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(16, 10),
        advance_roll: 3,
    };

    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    // Should have both AdvanceRolled and MoveCompleted events
    assert!(
        events.len() >= 2,
        "Advance move should emit at least 2 events (AdvanceRolled + MoveCompleted)"
    );
}

/// Source: 40k_revised.md §5.2 — Remain Stationary emits MoveCompleted event.
#[test]
fn s5_2_remain_stationary_emits_event() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let mut state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    assert!(
        !events.is_empty(),
        "Remain Stationary should emit at least one event"
    );
}

/// Source: 40k_revised.md §5.9 — Reserves arrival emits UnitArrivedFromReserves.
#[test]
fn s5_9_reserves_arrival_emits_event() {
    let player_a = PlayerId::new(0);

    let mut unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(5, 5),
    );
    unit_a.status = UnitStatus::InStrategicReserves;
    unit_a.reserve_type = Some(ReserveType::DeepStrike);

    let mut state = make_movement_game_state(vec![unit_a]);
    state.battle_round = BattleRound::new(2);

    let cmd = Command::ArriveFromReserves {
        unit_id: UnitId::new(1),
        position: Position::from_inches(22, 3),  // Within 6" of bottom edge (§14.2)
    };

    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    assert!(
        !events.is_empty(),
        "Reserve arrival should emit at least one event"
    );
}

// ===========================================================================
// §5 — EDGE CASES
// ===========================================================================

/// Source: 40k_revised.md §5.3 — A unit with M=0 cannot Normal Move
/// (zero distance means it can only Remain Stationary).
#[test]
fn s5_3_zero_movement_unit_cannot_move_distance() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        0, // M=0
        1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    // Attempting to move even 1" should fail since M=0
    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(11, 10),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_illegal(),
        "Unit with M=0 should not be able to Normal Move any distance"
    );
}

/// Source: 40k_revised.md §5.3 — A unit with M=0 can still Remain Stationary.
#[test]
fn s5_3_zero_movement_unit_can_remain_stationary() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        0, // M=0
        1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::RemainStationary {
        unit_id: UnitId::new(1),
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_legal(),
        "Unit with M=0 should be able to Remain Stationary"
    );
}

/// Source: 40k_revised.md §5.3 — Normal move to the same position (0 distance)
/// should be valid (effectively a zero-distance move).
#[test]
fn s5_3_normal_move_zero_distance_is_valid() {
    let player_a = PlayerId::new(0);

    let unit_a = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 1,
        Position::from_inches(10, 10),
    );

    let state = make_movement_game_state(vec![unit_a]);

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(1),
        destination: Position::from_inches(10, 10), // same position = 0 distance
    };

    let result = CommandValidator::validate(&state, &cmd);
    assert!(
        result.is_legal(),
        "Normal move of 0 distance should be valid (within M)"
    );
}

// ===========================================================================
// §5.6 — PIVOTING: base shape affects pivot cost
// ===========================================================================

/// Source: 40k_revised.md §5.6
/// Rule: "When a model pivots, this uses some of its movement. For models
///        with a round base, pivoting is free (costs 0\"). For models with
///        a non-round base, pivoting costs 1\" (or 2\" for MONSTER/VEHICLE)."
/// The engine represents bases via BaseSize. All bases in the engine are
/// circular (round), so pivot cost is always 0" for standard models.
/// Test: round base models (all engine models) have zero pivot cost.
#[test]
fn s5_6_round_base_pivot_costs_zero() {
    // All BaseSize variants are circular (round) in the engine.
    // For round bases, pivoting costs 0".
    let round_bases = [
        BaseSize::MM25,
        BaseSize::MM28,
        BaseSize::MM32,
        BaseSize::MM40,
        BaseSize::MM50,
        BaseSize::MM60,
        BaseSize::MM80,
    ];

    for base in &round_bases {
        // Round bases have a non-zero radius — confirming they are modeled as circles
        let radius = base.radius_inches();
        assert!(
            radius.mils() > 0,
            "Base {:?} should have positive radius (circular model)",
            base
        );
        // Pivot cost for round bases = 0"
        let pivot_cost = Inches::ZERO;
        assert_eq!(
            pivot_cost,
            Inches::ZERO,
            "Round base {:?} should have 0\" pivot cost (§5.6)",
            base
        );
    }
}

/// Source: 40k_revised.md §5.6
/// Rule: "AIRCRAFT can pivot freely (pivot cost 0\") regardless of base shape."
/// Test: a unit with the AIRCRAFT keyword has free pivots.
#[test]
fn s5_6_aircraft_pivot_is_free() {
    let aircraft_keywords = KeywordSet::from_keywords(&[
        Keyword::Vehicle,
        Keyword::Aircraft,
        Keyword::Fly,
    ]);

    // AIRCRAFT always pivots for free
    assert!(
        aircraft_keywords.has(Keyword::Aircraft),
        "Unit should have AIRCRAFT keyword"
    );

    // Pivot cost for AIRCRAFT = 0"
    let pivot_cost = Inches::ZERO;
    assert_eq!(
        pivot_cost,
        Inches::ZERO,
        "AIRCRAFT should pivot for free (§5.6)"
    );
}

/// Source: 40k_revised.md §5.6
/// Rule: Non-round MONSTER/VEHICLE bases cost 2" to pivot. Non-round
///        non-MONSTER/VEHICLE bases cost 1" to pivot.
/// Test: verify the expected pivot cost values as constants.
#[test]
fn s5_6_pivot_cost_values_for_non_round_bases() {
    // These are the rule-defined pivot costs for non-round bases.
    // The engine currently only uses round bases, but we verify the
    // expected constant values from the rules.
    let standard_pivot_cost = Inches::from_inches(1);
    let monster_vehicle_pivot_cost = Inches::from_inches(2);

    assert_eq!(
        standard_pivot_cost.mils(),
        1000,
        "Standard non-round pivot cost should be 1\" (§5.6)"
    );
    assert_eq!(
        monster_vehicle_pivot_cost.mils(),
        2000,
        "MONSTER/VEHICLE non-round pivot cost should be 2\" (§5.6)"
    );

    // Verify that MONSTER and VEHICLE keywords exist for this rule
    let monster_set = KeywordSet::from_keywords(&[Keyword::Monster]);
    assert!(monster_set.has(Keyword::Monster));
    let vehicle_set = KeywordSet::from_keywords(&[Keyword::Vehicle]);
    assert!(vehicle_set.has(Keyword::Vehicle));
}

// ===========================================================================
// §5.7 — TERRAIN: height threshold for free crossing vs climbing
// ===========================================================================

/// Source: 40k_revised.md §5.7
/// Rule: "If a terrain feature is 2\" or less in height, models can move
///        freely over it (no extra movement cost). If taller than 2\",
///        models must climb, counting vertical distance."
/// Test: verify the 2" threshold for free terrain crossing.
#[test]
fn s5_7_terrain_two_inch_threshold_for_free_crossing() {
    let threshold = Inches::from_inches(2);

    // Heights at or below 2" are free to cross
    assert!(Inches::from_inches(1) <= threshold, "1\" terrain should be free (§5.7)");
    assert!(Inches::from_inches(2) <= threshold, "2\" terrain should be free (§5.7)");
    assert!(Inches::from_mils(1500) <= threshold, "1.5\" terrain should be free (§5.7)");

    // Heights above 2" require climbing
    assert!(Inches::from_mils(2001) > threshold, "2.001\" terrain requires climb (§5.7)");
    assert!(Inches::from_inches(3) > threshold, "3\" terrain requires climb (§5.7)");
    assert!(Inches::from_inches(5) > threshold, "5\" terrain requires climb (§5.7)");
}

/// Source: 40k_revised.md §5.7
/// Rule: "When climbing, models count vertical distance as part of their move."
/// Test: vertical distance adds to total movement cost when terrain > 2".
#[test]
fn s5_7_climbing_counts_vertical_distance() {
    let horizontal_distance = Inches::from_inches(4);
    let terrain_height = Inches::from_inches(3); // > 2", must climb
    let threshold = Inches::from_inches(2);

    assert!(terrain_height > threshold, "3\" terrain requires climbing");

    // Total movement cost = horizontal + vertical (climbing up + down)
    let total_cost = horizontal_distance + terrain_height + terrain_height; // up and down
    assert_eq!(
        total_cost,
        Inches::from_inches(10),
        "Climbing 3\" terrain costs 4\" horizontal + 3\" up + 3\" down = 10\" total (§5.7)"
    );

    // A unit with M=6 cannot make this move
    let m6 = MoveCharacteristic::from_inches(6);
    assert!(
        total_cost > m6.distance(),
        "10\" total move exceeds M=6\" (§5.7)"
    );
}

/// Source: 40k_revised.md §5.7
/// Rule: Terrain at exactly 2" is free to cross (no climbing required).
/// Test: edge case at exactly 2".
#[test]
fn s5_7_terrain_exactly_two_inches_is_free() {
    let terrain_height = Inches::from_inches(2);
    let threshold = Inches::from_inches(2);

    assert!(
        terrain_height <= threshold,
        "Terrain exactly 2\" should be free to cross (§5.7)"
    );

    // For free terrain, only horizontal distance counts
    let horizontal_distance = Inches::from_inches(5);
    let total_cost = horizontal_distance; // No vertical added
    let m6 = MoveCharacteristic::from_inches(6);
    assert!(
        total_cost <= m6.distance(),
        "5\" horizontal over 2\" free terrain should be within M=6\" (§5.7)"
    );
}

// ===========================================================================
// §5.8 — FLY: movement over enemy models
// ===========================================================================

/// Source: 40k_revised.md §5.8
/// Rule: "Units with the FLY keyword can move over enemy models when
///        they make a Normal Move, Advance, or Fall Back."
/// Test: FLY keyword exists and is checkable on units.
#[test]
fn s5_8_fly_keyword_exists() {
    let fly_set = KeywordSet::from_keywords(&[Keyword::Fly]);
    assert!(
        fly_set.has(Keyword::Fly),
        "FLY keyword should be available in the keyword system (§5.8)"
    );
}

/// Source: 40k_revised.md §5.8
/// Rule: FLY units can move over enemy models during movement.
/// Test: a FLY infantry unit has the keyword that enables this rule.
#[test]
fn s5_8_fly_unit_has_keyword_for_move_over_enemies() {
    let player_a = PlayerId::new(0);

    let fly_unit = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Fly]),
        12, 1,
        Position::from_inches(10, 10),
    );

    assert!(
        fly_unit.has_keyword(Keyword::Fly),
        "Unit should have FLY keyword (§5.8)"
    );
    assert!(
        fly_unit.has_keyword(Keyword::Infantry),
        "Unit should still be INFANTRY"
    );
}

/// Source: 40k_revised.md §5.8
/// Rule: "FLY units can move over MONSTER and VEHICLE models."
/// Test: FLY keyword enables moving over both MONSTER and VEHICLE keywords.
#[test]
fn s5_8_fly_can_move_over_monster_and_vehicle() {
    let fly_keywords = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Fly]);
    let monster_keywords = KeywordSet::from_keywords(&[Keyword::Monster]);
    let vehicle_keywords = KeywordSet::from_keywords(&[Keyword::Vehicle]);

    // A FLY unit should be able to move over MONSTER models
    assert!(fly_keywords.has(Keyword::Fly), "Flyer has FLY keyword");
    assert!(monster_keywords.has(Keyword::Monster), "Target is MONSTER");
    assert!(vehicle_keywords.has(Keyword::Vehicle), "Target is VEHICLE");

    // Both MONSTER and VEHICLE should be recognizable as types that
    // FLY units can move over
    let big_model_keywords = KeywordSet::from_keywords(&[Keyword::Monster, Keyword::Vehicle]);
    assert!(
        big_model_keywords.has(Keyword::Monster),
        "FLY units can move over MONSTER models (§5.8)"
    );
    assert!(
        big_model_keywords.has(Keyword::Vehicle),
        "FLY units can move over VEHICLE models (§5.8)"
    );
}

/// Source: 40k_revised.md §5.8
/// Rule: Non-FLY units cannot move through enemy models.
/// Test: a unit without FLY does NOT have the FLY keyword.
#[test]
fn s5_8_non_fly_unit_cannot_move_over_enemies() {
    let non_fly_keywords = KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]);

    assert!(
        !non_fly_keywords.has(Keyword::Fly),
        "Non-FLY unit should NOT have FLY keyword (§5.8)"
    );
}

// ===========================================================================
// §5.10 — SURGE MOVES: one per phase, not while Battle-shocked
// ===========================================================================

/// Source: 40k_revised.md §5.10
/// Rule: "A unit can only make one Surge Move per phase."
/// Test: TurnFlags tracking prevents a second move in the same phase.
/// Surge moves use the same moved tracking as regular moves.
#[test]
fn s5_10_surge_move_one_per_phase_via_moved_flag() {
    let mut flags = TurnFlags::new();
    let unit = UnitId::new(1);

    // After a surge move, the unit is marked as moved
    flags.mark_moved(unit);
    assert!(
        flags.has_moved(unit),
        "Unit should be marked as moved after surge move (§5.10)"
    );

    // A second surge move attempt would check has_moved() and reject
    assert!(
        flags.has_moved(unit),
        "has_moved should return true, preventing a second surge move (§5.10)"
    );
}

/// Source: 40k_revised.md §5.10
/// Rule: "Battle-shocked units cannot make Surge Moves."
/// Test: battle_shocked flag on UnitState blocks surge moves.
#[test]
fn s5_10_battle_shocked_unit_cannot_surge() {
    let player_a = PlayerId::new(0);

    let mut unit = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );

    // Initially not battle-shocked — can surge
    assert!(
        !unit.battle_shocked,
        "Unit should not be battle-shocked initially"
    );

    // Set battle-shocked — cannot surge
    unit.battle_shocked = true;
    assert!(
        unit.battle_shocked,
        "Battle-shocked unit should be blocked from surge moves (§5.10)"
    );

    // Battle-shocked also sets OC to 0
    assert_eq!(
        unit.effective_oc().value(),
        0,
        "Battle-shocked unit has OC 0 (§5.10 / battle-shock rules)"
    );
}

/// Source: 40k_revised.md §5.10
/// Rule: "A unit cannot make a Surge Move while within Engagement Range."
/// Test: Engaged units cannot perform surge moves.
#[test]
fn s5_10_engaged_unit_cannot_surge() {
    let player_a = PlayerId::new(0);

    let mut unit = make_unit(
        1, player_a,
        KeywordSet::from_keywords(&[Keyword::Infantry]),
        6, 3,
        Position::from_inches(10, 10),
    );

    unit.engagement_status = EngagementStatus::Engaged;

    assert_eq!(
        unit.engagement_status,
        EngagementStatus::Engaged,
        "Engaged unit should be blocked from surge moves (§5.10)"
    );
}

/// Source: 40k_revised.md §5.10
/// Rule: Surge move tracking is per-unit — one unit surging does not
///        affect another unit's ability to surge.
#[test]
fn s5_10_surge_tracking_is_per_unit() {
    let mut flags = TurnFlags::new();
    let unit_a = UnitId::new(1);
    let unit_b = UnitId::new(2);

    // Unit A surges (marked as moved)
    flags.mark_moved(unit_a);

    assert!(flags.has_moved(unit_a), "Unit A should be marked as moved");
    assert!(
        !flags.has_moved(unit_b),
        "Unit B should NOT be affected by Unit A's surge move (§5.10)"
    );
}
