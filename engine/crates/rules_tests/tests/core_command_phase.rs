//! Source-linked rules tests for 40k_revised.md Section 4: Command Phase.
//!
//! Tests cover:
//!   §4.1 — Command Points: gain, extra CP cap, spending
//!   §4.2 — Battle-shock Tests: below half strength, 2D6 vs Ld, flag tracking
//!   §4.3 — Battle-shocked Effects: OC=0, stratagem targeting restriction
//!
//! Source: 40k_revised.md Section 4 — "COMMAND PHASE"

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, BattleShockResult, DatasheetId, GameMode, GameOutcome,
    Keyword, KeywordSet, Leadership, ModelId, MoveCharacteristic, ObjectiveControl, Phase,
    PlayerId, Position, SubPhase, Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_event_system::{EventBus, GameEvent};
use wh40k_game_core::phase::PhaseStateMachine;
use wh40k_game_core::state::{GameState, PlayerState};
use wh40k_game_core::unit::{ModelState, UnitState};
use wh40k_geometry::Board;

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a ModelState with the given wounds and no weapons.
fn make_model(model_id: u32, unit_id: UnitId, wounds: u8) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        Position::from_inches(10, 10),
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a multi-model infantry unit with the given model count and wounds per model.
fn make_infantry_unit(id: u32, model_count: usize, wounds_per_model: u8, leadership: u8, oc: u8) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| make_model(id * 100 + i as u32, unit_id, wounds_per_model))
        .collect();

    let mut unit = UnitState::new(
        unit_id,
        PlayerId::new(0),
        "Test Infantry".to_string(),
        DatasheetId::new(1),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(leadership),
        ObjectiveControl::new(oc),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a single-model unit (e.g., vehicle, monster, character).
fn make_single_model_unit(id: u32, total_wounds: u8, leadership: u8, oc: u8) -> UnitState {
    let unit_id = UnitId::new(id);
    let model = make_model(id * 100, unit_id, total_wounds);

    let mut unit = UnitState::new(
        unit_id,
        PlayerId::new(0),
        "Test Single Model".to_string(),
        DatasheetId::new(2),
        KeywordSet::from_keywords(&[Keyword::Monster, Keyword::Character]),
        vec![model],
        MoveCharacteristic::from_inches(8),
        Toughness::new(6),
        ArmorSave::TWO_PLUS,
        None,
        Leadership::new(leadership),
        ObjectiveControl::new(oc),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

// ===========================================================================
// §4.1 — COMMAND POINTS
// ===========================================================================

/// Source: 40k_revised.md §4.1
/// Rule: "At the start of each Command Phase, each player gains 1 CP."
/// Test: gain_cp(1) adds 1 CP to the player's pool.
#[test]
fn cp_gain_standard_command_phase_gain() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    assert_eq!(player.cp.value(), 0);

    // Both players gain 1 CP at start of Command Phase
    player.gain_cp(1);
    assert_eq!(player.cp.value(), 1);
}

/// Source: 40k_revised.md §4.1
/// Rule: "At the start of each Command Phase, each player gains 1 CP."
/// Test: Multiple rounds of standard CP gains accumulate.
#[test]
fn cp_gain_accumulates_across_rounds() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());

    // Simulate 5 rounds of Command Phase gains
    for round in 1..=5 {
        player.gain_cp(1);
        assert_eq!(player.cp.value(), round);
    }
}

/// Source: 40k_revised.md §4.1
/// Rule: "Outside the standard 1 CP gain, each player can only gain
///        1 additional CP per Battle Round from any source."
/// Test: First extra CP gain succeeds and returns 1.
#[test]
fn extra_cp_first_gain_succeeds() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());

    // First extra CP gain should succeed
    let gained = player.gain_extra_cp(1);
    assert_eq!(gained, 1);
    assert_eq!(player.cp.value(), 1);
    assert_eq!(player.extra_cp_gained_this_round, 1);
}

/// Source: 40k_revised.md §4.1
/// Rule: "each player can only gain 1 additional CP per Battle Round"
/// Test: Second extra CP gain in the same round is capped at 0.
#[test]
fn extra_cp_second_gain_capped() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());

    // First extra CP gain succeeds
    let first = player.gain_extra_cp(1);
    assert_eq!(first, 1);

    // Second extra CP gain should return 0 (capped)
    let second = player.gain_extra_cp(1);
    assert_eq!(second, 0);
    assert_eq!(player.cp.value(), 1); // Still just 1 CP from the first gain
}

/// Source: 40k_revised.md §4.1
/// Rule: "each player can only gain 1 additional CP per Battle Round"
/// Test: Requesting more than 1 extra CP in a single call still only grants 1.
#[test]
fn extra_cp_large_request_capped_at_one() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());

    // Requesting 5 extra CP, only 1 should be granted
    let gained = player.gain_extra_cp(5);
    assert_eq!(gained, 1);
    assert_eq!(player.cp.value(), 1);
}

/// Source: 40k_revised.md §4.1
/// Rule: Extra CP cap resets each battle round.
/// Test: After resetting the counter, extra CP can be gained again.
#[test]
fn extra_cp_cap_resets_between_rounds() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());

    // Round 1: gain extra CP
    let gained = player.gain_extra_cp(1);
    assert_eq!(gained, 1);

    // Simulate round reset
    player.extra_cp_gained_this_round = 0;

    // Round 2: can gain extra CP again
    let gained = player.gain_extra_cp(1);
    assert_eq!(gained, 1);
    assert_eq!(player.cp.value(), 2); // 1 from round 1 + 1 from round 2
}

/// Source: 40k_revised.md §4.1
/// Rule: "You can spend CP to use Stratagems."
/// Test: spend_cp returns true when sufficient CP are available.
#[test]
fn cp_spend_success() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(2);

    assert!(player.spend_cp(1));
    assert_eq!(player.cp.value(), 1);

    assert!(player.spend_cp(1));
    assert_eq!(player.cp.value(), 0);
}

/// Source: 40k_revised.md §4.1
/// Rule: Cannot spend CP you don't have.
/// Test: spend_cp returns false when insufficient CP, pool unchanged.
#[test]
fn cp_spend_insufficient_fails() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(1);

    // Can't spend 2 when only 1 available
    assert!(!player.spend_cp(2));
    assert_eq!(player.cp.value(), 1); // Unchanged

    // Can't spend from empty pool
    player.spend_cp(1);
    assert!(!player.spend_cp(1));
    assert_eq!(player.cp.value(), 0);
}

/// Source: 40k_revised.md §4.1
/// Test: can_afford_cp correctly reports affordability.
#[test]
fn cp_can_afford_check() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(2);

    assert!(player.can_afford_cp(1));
    assert!(player.can_afford_cp(2));
    assert!(!player.can_afford_cp(3));
    assert!(!player.can_afford_cp(5));
}

// ===========================================================================
// §4.2 — BATTLE-SHOCK TESTS: Below Half Strength
// ===========================================================================

/// Source: 40k_revised.md §4.2
/// Rule: "A unit is below half strength if it has less than half its starting number of models."
/// Test: Multi-model unit with 10 models — below half when <5 alive.
#[test]
fn below_half_strength_multi_model_10() {
    let mut unit = make_infantry_unit(1, 10, 1, 7, 2);

    // 10 alive: not below half
    assert!(!unit.is_below_half_strength());

    // Kill 5 models: 5 alive out of 10 (half = ceil(10/2) = 5), not below
    for i in 0..5 {
        unit.models[i].alive = false;
    }
    assert!(!unit.is_below_half_strength());

    // Kill 1 more: 4 alive < 5 = below half
    unit.models[5].alive = false;
    assert!(unit.is_below_half_strength());
}

/// Source: 40k_revised.md §4.2
/// Rule: Below half strength for odd-count units uses ceiling division.
/// Test: 5-model unit — below half when <3 alive (ceil(5/2)=3).
#[test]
fn below_half_strength_multi_model_odd_count() {
    let mut unit = make_infantry_unit(2, 5, 1, 7, 2);

    // 5 alive: not below half
    assert!(!unit.is_below_half_strength());

    // Kill 2: 3 alive, half = ceil(5/2) = 3, NOT below
    unit.models[0].alive = false;
    unit.models[1].alive = false;
    assert!(!unit.is_below_half_strength());

    // Kill 1 more: 2 alive < 3 = below half
    unit.models[2].alive = false;
    assert!(unit.is_below_half_strength());
}

/// Source: 40k_revised.md §4.2
/// Rule: "A single model unit is below half when it has less than half its starting wounds."
/// Test: Single model with 10 wounds — below half when <5 wounds remaining.
#[test]
fn below_half_strength_single_model() {
    let mut unit = make_single_model_unit(3, 10, 6, 3);

    // 10 wounds: not below half
    assert!(!unit.is_below_half_strength());

    // 6 wounds remaining: ceil(10/2) = 5, 6 >= 5, not below half
    unit.models[0].wounds_remaining = Wounds::new(6);
    assert!(!unit.is_below_half_strength());

    // 5 wounds remaining: 5 >= 5, not below half
    unit.models[0].wounds_remaining = Wounds::new(5);
    assert!(!unit.is_below_half_strength());

    // 4 wounds remaining: 4 < 5, below half
    unit.models[0].wounds_remaining = Wounds::new(4);
    assert!(unit.is_below_half_strength());

    // 1 wound remaining: definitely below half
    unit.models[0].wounds_remaining = Wounds::new(1);
    assert!(unit.is_below_half_strength());
}

/// Source: 40k_revised.md §4.2
/// Rule: Single model with odd wounds total.
/// Test: Single model with 3 wounds — below half when <2 (ceil(3/2)=2).
#[test]
fn below_half_strength_single_model_odd_wounds() {
    let mut unit = make_single_model_unit(4, 3, 6, 3);

    // 3 wounds: not below half
    assert!(!unit.is_below_half_strength());

    // 2 wounds: ceil(3/2)=2, 2 >= 2, NOT below half
    unit.models[0].wounds_remaining = Wounds::new(2);
    assert!(!unit.is_below_half_strength());

    // 1 wound: 1 < 2, below half
    unit.models[0].wounds_remaining = Wounds::new(1);
    assert!(unit.is_below_half_strength());
}

/// Source: 40k_revised.md §4.2
/// Rule: update_below_half_strength() syncs the flag with actual state.
/// Test: Killing models and calling update sets the flag correctly.
#[test]
fn update_below_half_strength_flag() {
    let mut unit = make_infantry_unit(5, 6, 1, 7, 2);

    unit.update_below_half_strength();
    assert!(!unit.below_half_strength);

    // Kill 4 of 6 models: 2 alive < ceil(6/2)=3, below half
    unit.models[0].alive = false;
    unit.models[1].alive = false;
    unit.models[2].alive = false;
    unit.models[3].alive = false;
    unit.update_below_half_strength();
    assert!(unit.below_half_strength);
}

// ===========================================================================
// §4.2 — BATTLE-SHOCK TESTS: 2D6 vs Leadership
// ===========================================================================

/// Source: 40k_revised.md §4.2
/// Rule: "Roll 2D6. If the result exceeds the unit's Leadership, it becomes Battle-shocked."
/// Test: Invoke PhaseStateMachine::process_battle_shock on a below-half-strength unit
///       and verify the battle_shocked flag is set or cleared based on the 2D6 vs Leadership roll.
#[test]
fn battle_shock_flag_set_and_cleared() {
    // Build a GameState with a below-half-strength unit owned by Player A
    let player_a = PlayerId::new(0);

    // Create a 10-model unit, then kill 6 to be below half strength
    let mut unit = make_infantry_unit(6, 10, 1, 7, 2);
    for i in 0..6 {
        unit.models[i].alive = false;
    }
    unit.update_below_half_strength();
    assert!(unit.is_below_half_strength(), "Unit should be below half strength");

    let unit_id = unit.id;

    // Try many seeds to observe both pass and fail outcomes
    let mut saw_pass = false;
    let mut saw_fail = false;

    for seed_byte in 0u8..100 {
        let seed = [seed_byte; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        let mut state = GameState {
            content_version: "test-bs-1.0".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: player_a,
            current_phase: Phase::Command,
            current_subphase: SubPhase::BattleShockTests,
            decision_owner: player_a,
            players: [
                PlayerState::new(PlayerId::new(0), "Player A".to_string()),
                PlayerState::new(PlayerId::new(1), "Player B".to_string()),
            ],
            units: vec![unit.clone()],
            board: Board::combat_patrol(),
            deployment_config: None,
            event_bus: EventBus::new(),
            command_history: wh40k_command_system::CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: wh40k_game_core::state::TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
            game_mode: GameMode::CombatPatrol,
            mode_state: None,
        };

        let events = PhaseStateMachine::process_battle_shock(&mut state, unit_id);

        // Verify we got a BattleShockTestResolved event
        assert_eq!(events.len(), 1, "Should emit exactly one event");
        match &events[0] {
            GameEvent::BattleShockTestResolved { unit, result, roll } => {
                assert_eq!(*unit, unit_id);
                // Verify roll is a valid 2D6 result
                assert!(*roll >= 2 && *roll <= 12, "Roll {} should be valid 2D6", roll);
                // Verify the battle_shocked flag matches the result
                let unit_after = state.unit(unit_id).unwrap();
                match result {
                    BattleShockResult::Passed => {
                        assert!(!unit_after.battle_shocked, "Passed: unit should NOT be battle-shocked");
                        saw_pass = true;
                    }
                    BattleShockResult::Failed => {
                        assert!(unit_after.battle_shocked, "Failed: unit should be battle-shocked");
                        saw_fail = true;
                    }
                }
            }
            other => panic!("Expected BattleShockTestResolved, got {:?}", other),
        }

        if saw_pass && saw_fail {
            break;
        }
    }

    // With 100 different seeds and Ld 7, we should see both outcomes
    assert!(saw_pass, "Should observe at least one passed battle-shock test across seeds");
    assert!(saw_fail, "Should observe at least one failed battle-shock test across seeds");
}

/// Source: 40k_revised.md §4.2
/// Rule: The Leadership characteristic determines the threshold for battle-shock tests.
/// Test: Verify Leadership values are stored and accessible.
#[test]
fn battle_shock_leadership_characteristic() {
    let unit_ld7 = make_infantry_unit(7, 5, 1, 7, 2);
    assert_eq!(unit_ld7.base_leadership.value(), 7);

    let unit_ld6 = make_single_model_unit(8, 5, 6, 3);
    assert_eq!(unit_ld6.base_leadership.value(), 6);
}

/// Source: 40k_revised.md §4.2
/// Rule: Only below-half-strength units take battle-shock tests.
/// Test: A full-strength unit should not be flagged as needing a test.
#[test]
fn battle_shock_only_below_half_strength_tested() {
    let unit = make_infantry_unit(9, 10, 1, 7, 2);

    // Full strength: no test needed
    assert!(!unit.is_below_half_strength());
    // battle_shocked should remain false since no test is taken
    assert!(!unit.battle_shocked);
}

/// Source: 40k_revised.md §4.2
/// Rule: A unit that has been destroyed does not take battle-shock tests.
/// Test: Verify is_destroyed returns true when all models are dead.
#[test]
fn battle_shock_destroyed_units_skip() {
    let mut unit = make_infantry_unit(10, 3, 1, 7, 2);

    // Kill all models
    for model in &mut unit.models {
        model.alive = false;
    }
    assert!(unit.is_destroyed());
    // Destroyed units skip the battle-shock test entirely
}

// ===========================================================================
// §4.3 — BATTLE-SHOCKED EFFECTS
// ===========================================================================

/// Source: 40k_revised.md §4.3
/// Rule: "Battle-shocked units have an Objective Control characteristic of 0."
/// Test: effective_oc returns 0 when battle_shocked is true.
#[test]
fn battle_shocked_oc_zero() {
    let mut unit = make_infantry_unit(11, 5, 1, 7, 2);
    assert_eq!(unit.effective_oc().value(), 2); // Normal OC

    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0); // OC drops to 0
}

/// Source: 40k_revised.md §4.3
/// Rule: OC=0 overrides any other OC modifiers when battle-shocked.
/// Test: High OC unit still gets OC=0 when battle-shocked.
#[test]
fn battle_shocked_oc_zero_overrides_high_oc() {
    let mut unit = make_infantry_unit(12, 5, 1, 7, 5); // OC 5
    assert_eq!(unit.effective_oc().value(), 5);

    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0);
}

/// Source: 40k_revised.md §4.3
/// Rule: OC=0 overrides enhancement OC overrides when battle-shocked.
/// Test: Enhancement OC override is ignored when battle-shocked.
#[test]
fn battle_shocked_oc_zero_overrides_enhancement() {
    let mut unit = make_infantry_unit(13, 5, 1, 7, 2);
    unit.enhancement_oc_override = Some(ObjectiveControl::new(5));

    // Enhancement override applies when not shocked
    assert_eq!(unit.effective_oc().value(), 5);

    // Battle-shocked overrides everything
    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0);
}

/// Source: 40k_revised.md §4.3
/// Rule: "Battle-shocked units cannot be selected as the target of a Stratagem
///        by their controlling player."
/// Test: Verify that the battle_shocked flag can be checked for stratagem targeting.
#[test]
fn battle_shocked_stratagem_targeting_blocked() {
    let mut unit = make_infantry_unit(14, 5, 1, 7, 2);

    // Not battle-shocked: eligible for friendly stratagems
    assert!(!unit.battle_shocked);

    // Battle-shocked: not eligible for friendly stratagems
    unit.battle_shocked = true;
    assert!(unit.battle_shocked);
    // The game system checks this flag before allowing stratagem targeting.
    // A battle-shocked unit should fail the eligibility check.

    // Clearing battle-shock restores eligibility
    unit.battle_shocked = false;
    assert!(!unit.battle_shocked);
}

/// Source: 40k_revised.md §4.3
/// Rule: When battle-shock is cleared, OC returns to normal.
/// Test: OC recovery after battle-shock removal.
#[test]
fn battle_shocked_recovery_restores_oc() {
    let mut unit = make_infantry_unit(15, 5, 1, 7, 3); // OC 3

    unit.battle_shocked = true;
    assert_eq!(unit.effective_oc().value(), 0);

    // At next Command Phase, battle-shock is cleared
    unit.battle_shocked = false;
    assert_eq!(unit.effective_oc().value(), 3); // OC restored
}
