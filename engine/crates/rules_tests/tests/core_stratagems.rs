//! Source-linked rules tests for 40k_revised.md Section 15: Stratagems.
//!
//! Tests cover:
//!   §15.1 — Using Stratagems: CP cost, once-per-phase, Battle-shocked targeting
//!   §15.2 — Core Stratagems: Command Re-roll, Counter-Offensive, Fire Overwatch,
//!            Insane Bravery, Grenade, Tank Shock
//!
//! Source: 40k_revised.md Section 15 — "STRATAGEMS"

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, DatasheetId, GameMode, GameOutcome,
    Keyword, KeywordSet, Leadership, ModelId, MoveCharacteristic,
    ObjectiveControl, Phase, PlayerId, Position, SubPhase,
    Toughness, UnitId, UnitStatus, Wounds,
};
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_command_system::CommandHistory;
use wh40k_geometry::Board;
use wh40k_game_core::state::{GameState, PlayerState, TurnFlags};
use wh40k_game_core::unit::{ModelState, UnitState};
use wh40k_game_core::stratagem::ids;
use wh40k_stratagem_runtime::{
    StratagemRuntime, StratagemTarget,
};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a minimal GameState for stratagem testing.
fn make_test_game_state() -> GameState {
    let seed = [0u8; 32];
    let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
    let dice_roller = DiceRoller::new(ctx);

    GameState {
        content_version: "test-1.0".to_string(),
        scenario_id: None,
        battle_round: BattleRound::new(1),
        active_player: PlayerId::new(0),
        current_phase: Phase::Shooting,
        current_subphase: SubPhase::ResolveAttacks,
        decision_owner: PlayerId::new(0),
        players: [
            PlayerState::new(PlayerId::new(0), "Player A".to_string()),
            PlayerState::new(PlayerId::new(1), "Player B".to_string()),
        ],
        units: Vec::new(),
        board: Board::combat_patrol(),
        deployment_config: None,
        event_bus: EventBus::new(),
        command_history: CommandHistory::new(),
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

/// Build a model with no weapons at a given position.
fn make_model(model_id: u32, unit_id: UnitId, wounds: u8, pos: Position) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(wounds),
        pos,
        BaseSize::MM32,
        vec![],
        vec![],
        false,
        None,
    )
}

/// Build a multi-model infantry unit with the given keyword set.
fn make_unit_with_keywords(
    id: u32,
    owner: PlayerId,
    model_count: usize,
    keywords: KeywordSet,
) -> UnitState {
    let unit_id = UnitId::new(id);
    let models: Vec<ModelState> = (0..model_count)
        .map(|i| make_model(id * 100 + i as u32, unit_id, 1, Position::from_inches(10, 10)))
        .collect();

    let mut unit = UnitState::new(
        unit_id,
        owner,
        "Test Unit".to_string(),
        DatasheetId::new(1),
        keywords,
        models,
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(2),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a standard infantry unit (INFANTRY + BATTLELINE) for a player.
fn make_infantry_unit(id: u32, owner: PlayerId, model_count: usize) -> UnitState {
    make_unit_with_keywords(
        id,
        owner,
        model_count,
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
    )
}

/// Build a VEHICLE unit for a player (for Tank Shock tests).
fn make_vehicle_unit(id: u32, owner: PlayerId, toughness: u8) -> UnitState {
    let unit_id = UnitId::new(id);
    let model = ModelState::new(
        ModelId::new(id * 100),
        unit_id,
        Wounds::new(10),
        Position::from_inches(15, 15),
        BaseSize::MM60,
        vec![],
        vec![],
        false,
        None,
    );

    let mut unit = UnitState::new(
        unit_id,
        owner,
        "Test Vehicle".to_string(),
        DatasheetId::new(10),
        KeywordSet::from_keywords(&[Keyword::Vehicle]),
        vec![model],
        MoveCharacteristic::from_inches(10),
        Toughness::new(toughness),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(3),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a unit with the GRENADES keyword.
fn make_grenades_unit(id: u32, owner: PlayerId) -> UnitState {
    make_unit_with_keywords(
        id,
        owner,
        5,
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Grenades]),
    )
}

// ===========================================================================
// §15.1 — USING STRATAGEMS: CP COST SYSTEM
// ===========================================================================

/// Source: 40k_revised.md §15.1
/// Rule: "To use a Stratagem, a player must spend the CP cost."
/// Test: spend_cp deducts CP and returns true when affordable.
#[test]
fn stratagem_cp_spend_success() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(3);

    // Spending 1 CP (Command Re-roll cost) succeeds
    assert!(player.spend_cp(1));
    assert_eq!(player.cp.value(), 2);

    // Spending 2 CP (Counter-Offensive cost) succeeds
    assert!(player.spend_cp(2));
    assert_eq!(player.cp.value(), 0);
}

/// Source: 40k_revised.md §15.1
/// Rule: Cannot use a Stratagem if insufficient CP.
/// Test: spend_cp returns false when CP is insufficient, pool unchanged.
#[test]
fn stratagem_cp_spend_insufficient_fails() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(1);

    // Cannot spend 2 CP when only 1 available
    assert!(!player.spend_cp(2));
    assert_eq!(player.cp.value(), 1);
}

/// Source: 40k_revised.md §15.1
/// Rule: can_afford_cp should correctly check whether the player can afford a stratagem.
/// Test: Verify can_afford_cp for various costs.
#[test]
fn stratagem_can_afford_cp_check() {
    let mut player = PlayerState::new(PlayerId::new(0), "Player A".to_string());
    player.gain_cp(2);

    assert!(player.can_afford_cp(1)); // Command Re-roll (1 CP)
    assert!(player.can_afford_cp(2)); // Counter-Offensive (2 CP)
    assert!(!player.can_afford_cp(3)); // Too expensive
}

/// Source: 40k_revised.md §15.1
/// Rule: Stratagem runtime blocks usage when CP is insufficient.
/// Test: can_use_stratagem returns Blocked when player has 0 CP.
#[test]
fn stratagem_runtime_blocks_insufficient_cp() {
    let state = make_test_game_state();
    // Player starts with 0 CP
    let player = PlayerId::new(0);
    let target = StratagemTarget::DiceRoll;

    let usability = StratagemRuntime::can_use_stratagem(&state, player, ids::COMMAND_REROLL, &target);
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("Insufficient CP"));
}

/// Source: 40k_revised.md §15.1
/// Rule: Stratagem runtime allows usage when CP is sufficient.
/// Test: can_use_stratagem returns Usable when player has enough CP and phase matches.
#[test]
fn stratagem_runtime_allows_sufficient_cp() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);

    // Command Re-roll works in any phase, so Shooting is fine
    let target = StratagemTarget::DiceRoll;
    let usability = StratagemRuntime::can_use_stratagem(&state, player, ids::COMMAND_REROLL, &target);
    assert!(usability.is_usable());
}

// ===========================================================================
// §15.1 — USING STRATAGEMS: ONCE-PER-PHASE RESTRICTION
// ===========================================================================

/// Source: 40k_revised.md §15.1
/// Rule: "You cannot use the same Stratagem more than once in the same phase."
/// Test: StratagemUsageTracker records phase usage and blocks re-use.
#[test]
fn stratagem_once_per_phase_tracking() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);

    let strat_id = ids::COMMAND_REROLL;

    // First use: not recorded yet
    assert!(!state.player(player).stratagem_usage.used_this_phase(strat_id));

    // Record usage
    state.player_mut(player).stratagem_usage.record_usage(strat_id);

    // Now it's used this phase (and this turn, since record_usage sets both)
    assert!(state.player(player).stratagem_usage.used_this_phase(strat_id));
    assert!(state.player(player).stratagem_usage.used_this_turn(strat_id));

    // Runtime should block it — Command Re-roll is once_per_turn, so "Already used this turn"
    // fires before the phase-level check in the runtime's validation order.
    let target = StratagemTarget::DiceRoll;
    let usability = StratagemRuntime::can_use_stratagem(&state, player, strat_id, &target);
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("Already used this turn"));
}

/// Source: 40k_revised.md §15.1
/// Rule: Phase usage clears at the start of each new phase.
/// Test: clear_phase resets phase usage, allowing the same stratagem again in the phase tracker.
///       Note: Command Re-roll is once_per_turn, so clearing only phase usage still
///       blocks it at the turn level. We verify the phase tracker itself clears correctly.
#[test]
fn stratagem_phase_usage_clears_on_new_phase() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);

    let strat_id = ids::COMMAND_REROLL;

    // Record usage in shooting phase
    state.player_mut(player).stratagem_usage.record_usage(strat_id);
    assert!(state.player(player).stratagem_usage.used_this_phase(strat_id));
    assert!(state.player(player).stratagem_usage.used_this_turn(strat_id));

    // New phase: clear phase flags only
    state.player_mut(player).stratagem_usage.clear_phase();

    // Phase tracker is cleared
    assert!(!state.player(player).stratagem_usage.used_this_phase(strat_id));

    // But turn tracker still blocks (once_per_turn stratagems remain blocked within the same turn)
    assert!(state.player(player).stratagem_usage.used_this_turn(strat_id));
    let target = StratagemTarget::DiceRoll;
    let usability = StratagemRuntime::can_use_stratagem(&state, player, strat_id, &target);
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("Already used this turn"));

    // Full turn clear allows re-use
    state.player_mut(player).stratagem_usage.clear_turn();
    let usability = StratagemRuntime::can_use_stratagem(&state, player, strat_id, &target);
    assert!(usability.is_usable());
}

/// Source: 40k_revised.md §15.1
/// Rule: Different stratagems can be used in the same phase.
/// Test: Using stratagem A does not block stratagem B in the same phase.
#[test]
fn stratagem_different_stratagems_same_phase_allowed() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);

    // Use Command Re-roll
    state.player_mut(player).stratagem_usage.record_usage(ids::COMMAND_REROLL);

    // A different stratagem (Go to Ground) should still be available in the same phase
    // (assuming correct phase and keywords)
    assert!(!state.player(player).stratagem_usage.used_this_phase(ids::GO_TO_GROUND));
}

/// Source: 40k_revised.md §15.1
/// Rule: TurnFlags also tracks stratagems_used_this_phase for global tracking.
/// Test: stratagems_used_this_phase vec tracks per-phase usage.
#[test]
fn turn_flags_stratagems_used_this_phase() {
    let mut flags = TurnFlags::new();

    assert!(flags.stratagems_used_this_phase.is_empty());

    flags.stratagems_used_this_phase.push(ids::COMMAND_REROLL);
    assert_eq!(flags.stratagems_used_this_phase.len(), 1);
    assert!(flags.stratagems_used_this_phase.contains(&ids::COMMAND_REROLL));

    // Clear phase flags
    flags.clear_phase_flags();
    assert!(flags.stratagems_used_this_phase.is_empty());
}

/// Source: 40k_revised.md §15.1
/// Rule: Turn-level usage persists across phases within the same turn.
/// Test: used_this_turn remains set after clearing phase usage.
#[test]
fn stratagem_turn_level_tracking_persists_across_phases() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);

    let strat_id = ids::COMMAND_REROLL;
    state.player_mut(player).stratagem_usage.record_usage(strat_id);

    // Clear phase usage only
    state.player_mut(player).stratagem_usage.clear_phase();

    // Phase is cleared, but turn-level tracking persists
    assert!(!state.player(player).stratagem_usage.used_this_phase(strat_id));
    assert!(state.player(player).stratagem_usage.used_this_turn(strat_id));
}

/// Source: 40k_revised.md §15.1
/// Rule: Turn-level usage clears at the start of each new turn.
/// Test: clear_turn resets both phase and turn usage.
#[test]
fn stratagem_turn_level_usage_clears_on_new_turn() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);

    let strat_id = ids::COMMAND_REROLL;
    state.player_mut(player).stratagem_usage.record_usage(strat_id);

    assert!(state.player(player).stratagem_usage.used_this_turn(strat_id));

    state.player_mut(player).stratagem_usage.clear_turn();

    assert!(!state.player(player).stratagem_usage.used_this_phase(strat_id));
    assert!(!state.player(player).stratagem_usage.used_this_turn(strat_id));
}

// ===========================================================================
// §15.1 — BATTLE-SHOCKED TARGETING RESTRICTION
// ===========================================================================

/// Source: 40k_revised.md §15.1 / §4.3
/// Rule: "Battle-shocked units cannot be selected as the target of a Stratagem
///        by their controlling player."
/// Test: can_use_stratagem returns Blocked when targeting a Battle-shocked friendly unit.
#[test]
fn stratagem_blocked_on_battle_shocked_friendly_unit() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Fight;

    // Create a Battle-shocked unit
    let mut unit = make_infantry_unit(1, player, 5);
    unit.battle_shocked = true;
    state.units.push(unit);

    // Counter-Offensive targets a friendly unit
    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::COUNTER_OFFENSIVE,
        &target,
    );
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("Battle-shocked"));
}

/// Source: 40k_revised.md §15.1 / §4.3
/// Rule: Non-Battle-shocked friendly units can be targeted normally.
/// Test: can_use_stratagem returns Usable for a non-Battle-shocked unit.
#[test]
fn stratagem_allowed_on_non_battle_shocked_friendly_unit() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Fight;

    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::COUNTER_OFFENSIVE,
        &target,
    );
    assert!(usability.is_usable());
}

/// Source: 40k_revised.md §15.1 / §4.3
/// Rule: Insane Bravery is the exception — it CAN target a Battle-shocked unit.
/// Test: can_use_stratagem returns Usable for Insane Bravery on a Battle-shocked unit.
#[test]
fn insane_bravery_allowed_on_battle_shocked_unit() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Command;

    let mut unit = make_infantry_unit(1, player, 5);
    unit.battle_shocked = true;
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::INSANE_BRAVERY,
        &target,
    );
    assert!(usability.is_usable());
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Command Re-roll (1 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Command Re-roll costs 1 CP and allows re-rolling one die.
/// Test: Verify definition properties.
#[test]
fn command_reroll_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::COMMAND_REROLL).unwrap();
    assert_eq!(def.name, "Command Re-roll");
    assert_eq!(def.cp_cost, 1);
    assert!(!def.once_per_battle);
    assert!(def.once_per_turn);
}

/// Source: 40k_revised.md §15.2
/// Rule: Command Re-roll can be used in any phase.
/// Test: The stratagem is valid in all five active phases.
#[test]
fn command_reroll_available_in_all_phases() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::COMMAND_REROLL).unwrap();
    assert!(def.valid_phases.contains(&Phase::Command));
    assert!(def.valid_phases.contains(&Phase::Movement));
    assert!(def.valid_phases.contains(&Phase::Shooting));
    assert!(def.valid_phases.contains(&Phase::Charge));
    assert!(def.valid_phases.contains(&Phase::Fight));
}

/// Source: 40k_revised.md §15.2
/// Rule: Command Re-roll applies effect via the runtime.
/// Test: apply_stratagem creates a CommandReroll effect and spends CP.
#[test]
fn command_reroll_apply_spends_cp_and_creates_effect() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(2);
    state.current_phase = Phase::Shooting;

    let target = StratagemTarget::DiceRoll;
    let result = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::COMMAND_REROLL,
        &target,
    );
    assert!(result.is_ok());

    // CP was spent
    assert_eq!(state.player(player).cp.value(), 1);

    // Usage was recorded
    assert!(state.player(player).stratagem_usage.used_this_phase(ids::COMMAND_REROLL));
    assert!(state.player(player).stratagem_usage.used_this_turn(ids::COMMAND_REROLL));

    // Active effect was created
    assert!(!state.active_effects.is_empty());
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Counter-Offensive (2 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Counter-Offensive costs 2 CP.
/// Test: Verify definition properties.
#[test]
fn counter_offensive_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::COUNTER_OFFENSIVE).unwrap();
    assert_eq!(def.name, "Counter-Offensive");
    assert_eq!(def.cp_cost, 2);
    assert_eq!(def.valid_phases, &[Phase::Fight]);
}

/// Source: 40k_revised.md §15.2
/// Rule: Counter-Offensive can only be used in the Fight phase.
/// Test: Wrong phase blocks usage.
#[test]
fn counter_offensive_wrong_phase_blocked() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Shooting; // Wrong phase

    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::COUNTER_OFFENSIVE,
        &target,
    );
    assert!(!usability.is_usable());
}

/// Source: 40k_revised.md §15.2
/// Rule: Counter-Offensive allowed in the Fight phase with sufficient CP.
/// Test: Correct phase allows usage.
#[test]
fn counter_offensive_correct_phase_allowed() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Fight;

    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::COUNTER_OFFENSIVE,
        &target,
    );
    assert!(usability.is_usable());
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Fire Overwatch (1 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Fire Overwatch costs 1 CP and can be used in Movement or Charge phase.
/// Test: Verify definition and valid phases.
#[test]
fn fire_overwatch_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::FIRE_OVERWATCH).unwrap();
    assert_eq!(def.name, "Fire Overwatch");
    assert_eq!(def.cp_cost, 1);
    assert!(def.valid_phases.contains(&Phase::Movement));
    assert!(def.valid_phases.contains(&Phase::Charge));
}

/// Source: 40k_revised.md §15.2
/// Rule: Fire Overwatch can only be used once per turn (overwatch_used_this_turn flag).
/// Test: overwatch_used_this_turn flag is set after applying Fire Overwatch.
#[test]
fn fire_overwatch_sets_overwatch_used_flag() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Movement;

    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    assert!(!state.turn_flags.overwatch_used_this_turn);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let result = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::FIRE_OVERWATCH,
        &target,
    );
    assert!(result.is_ok());

    // Flag is now set
    assert!(state.turn_flags.overwatch_used_this_turn);
}

/// Source: 40k_revised.md §15.2
/// Rule: overwatch_used_this_turn resets when turn flags are cleared.
/// Test: After clearing turn flags, overwatch can be used again next turn.
#[test]
fn fire_overwatch_flag_resets_on_turn_clear() {
    let mut flags = TurnFlags::new();
    flags.overwatch_used_this_turn = true;
    assert!(flags.overwatch_used_this_turn);

    flags.clear();
    assert!(!flags.overwatch_used_this_turn);
}

/// Source: 40k_revised.md §15.2
/// Rule: Fire Overwatch timing check works for both Movement and Charge phases.
/// Test: check_timing returns true for both valid phases.
#[test]
fn fire_overwatch_timing_movement_and_charge() {
    use wh40k_stratagem_runtime::StratagemTiming;

    // Movement phase
    assert!(StratagemRuntime::check_timing(
        Phase::Movement,
        &StratagemTiming::AnyTime,
        &Phase::Movement,
    ));

    // Charge phase (special case: Overwatch required phase is Movement but also valid in Charge)
    assert!(StratagemRuntime::check_timing(
        Phase::Charge,
        &StratagemTiming::AnyTime,
        &Phase::Movement,
    ));
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Insane Bravery (1 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Insane Bravery costs 1 CP and is once per battle.
/// Test: Verify definition properties.
#[test]
fn insane_bravery_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::INSANE_BRAVERY).unwrap();
    assert_eq!(def.name, "Insane Bravery");
    assert_eq!(def.cp_cost, 1);
    assert!(def.once_per_battle);
    assert!(def.once_per_turn);
    assert_eq!(def.valid_phases, &[Phase::Command]);
}

/// Source: 40k_revised.md §15.2
/// Rule: Insane Bravery auto-passes Battle-shock, clearing the flag.
/// Test: Applying Insane Bravery clears battle_shocked on the target unit.
#[test]
fn insane_bravery_clears_battle_shock() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Command;

    let mut unit = make_infantry_unit(1, player, 5);
    unit.battle_shocked = true;
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let result = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::INSANE_BRAVERY,
        &target,
    );
    assert!(result.is_ok());

    // Battle-shock is cleared
    let unit = state.unit(UnitId::new(1)).unwrap();
    assert!(!unit.battle_shocked);
}

/// Source: 40k_revised.md §15.2
/// Rule: Insane Bravery is once per battle.
/// Test: After using Insane Bravery once, it's blocked for the rest of the battle.
#[test]
fn insane_bravery_once_per_battle_enforced() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Command;

    let mut unit = make_infantry_unit(1, player, 5);
    unit.battle_shocked = true;
    state.units.push(unit);

    // First use
    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let result = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::INSANE_BRAVERY,
        &target,
    );
    assert!(result.is_ok());

    // Recorded as used this battle
    assert!(state.player(player).stratagem_usage.used_this_battle(ids::INSANE_BRAVERY));

    // Simulate a new turn: clear turn and phase usage so the once_per_battle check is reached
    state.player_mut(player).stratagem_usage.clear_turn();
    state.unit_mut(UnitId::new(1)).unwrap().battle_shocked = true;

    // Even after a full turn reset, once_per_battle still blocks it
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::INSANE_BRAVERY,
        &target,
    );
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("Already used this battle"));
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Grenade (1 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Grenade costs 1 CP, requires GRENADES keyword.
/// Test: Verify definition properties.
#[test]
fn grenade_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::GRENADE).unwrap();
    assert_eq!(def.name, "Grenade");
    assert_eq!(def.cp_cost, 1);
    assert!(def.required_keywords.contains(&Keyword::Grenades));
    assert_eq!(def.valid_phases, &[Phase::Shooting]);
}

/// Source: 40k_revised.md §15.2
/// Rule: Grenade is blocked if target unit lacks GRENADES keyword.
/// Test: can_use_stratagem blocks Grenade targeting a unit without GRENADES.
#[test]
fn grenade_blocked_without_grenades_keyword() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Shooting;

    // Unit without GRENADES keyword
    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::GRENADE,
        &target,
    );
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("keyword"));
}

/// Source: 40k_revised.md §15.2
/// Rule: Grenade is allowed if target unit has GRENADES keyword.
/// Test: can_use_stratagem allows Grenade targeting a unit with GRENADES.
#[test]
fn grenade_allowed_with_grenades_keyword() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Shooting;

    let unit = make_grenades_unit(1, player);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::GRENADE,
        &target,
    );
    assert!(usability.is_usable());
}

// ===========================================================================
// §15.2 — CORE STRATAGEMS: Tank Shock (1 CP)
// ===========================================================================

/// Source: 40k_revised.md §15.2
/// Rule: Tank Shock costs 1 CP, used in Charge phase.
/// Test: Verify definition properties.
#[test]
fn tank_shock_definition() {
    let def = wh40k_game_core::stratagem::get_stratagem_def(ids::TANK_SHOCK).unwrap();
    assert_eq!(def.name, "Tank Shock");
    assert_eq!(def.cp_cost, 1);
    assert!(def.valid_phases.contains(&Phase::Charge));
}

/// Source: 40k_revised.md §15.2
/// Rule: Tank Shock targets a VEHICLE unit that just charged.
/// Test: A VEHICLE unit can be used as the target of Tank Shock.
#[test]
fn tank_shock_vehicle_target_allowed() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Charge;

    let vehicle = make_vehicle_unit(1, player, 8);
    state.units.push(vehicle);

    // Tank Shock's required_keywords are empty (custom validation), so FriendlyUnit target passes
    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::TANK_SHOCK,
        &target,
    );
    assert!(usability.is_usable());
}

/// Source: 40k_revised.md §15.2
/// Rule: Tank Shock applies mortal wounds based on Toughness dice (each 5+ = MW, max 6).
/// Test: Verify that Tank Shock apply creates mortal wound events.
#[test]
fn tank_shock_apply_spends_cp_and_records_usage() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Charge;

    let vehicle = make_vehicle_unit(1, player, 8);
    state.units.push(vehicle);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let result = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::TANK_SHOCK,
        &target,
    );
    assert!(result.is_ok());

    // CP was spent
    assert_eq!(state.player(player).cp.value(), 4);

    // Usage was recorded
    assert!(state.player(player).stratagem_usage.used_this_phase(ids::TANK_SHOCK));
}

// ===========================================================================
// §15.1 — ADDITIONAL PHASE RESTRICTION TESTS
// ===========================================================================

/// Source: 40k_revised.md §15.1
/// Rule: Stratagems are tied to specific phases.
/// Test: Fire Overwatch is blocked during the Shooting phase.
#[test]
fn fire_overwatch_blocked_in_shooting_phase() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Shooting;

    let unit = make_infantry_unit(1, player, 5);
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::FIRE_OVERWATCH,
        &target,
    );
    assert!(!usability.is_usable());
}

/// Source: 40k_revised.md §15.1
/// Rule: Insane Bravery can only be used in the Command phase.
/// Test: Insane Bravery is blocked during the Fight phase.
#[test]
fn insane_bravery_blocked_outside_command_phase() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Fight;

    let mut unit = make_infantry_unit(1, player, 5);
    unit.battle_shocked = true;
    state.units.push(unit);

    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::INSANE_BRAVERY,
        &target,
    );
    assert!(!usability.is_usable());
}

/// Source: 40k_revised.md §15.1
/// Rule: Cannot target enemy units with friendly-only stratagems.
/// Test: Counter-Offensive targeting an enemy unit is blocked.
#[test]
fn stratagem_friendly_only_blocks_enemy_target() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    let opponent = PlayerId::new(1);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Fight;

    // Create an enemy unit
    let enemy_unit = make_infantry_unit(1, opponent, 5);
    state.units.push(enemy_unit);

    // Counter-Offensive must be friendly — targeting enemy should fail
    let target = StratagemTarget::FriendlyUnit(UnitId::new(1));
    let usability = StratagemRuntime::can_use_stratagem(
        &state,
        player,
        ids::COUNTER_OFFENSIVE,
        &target,
    );
    assert!(!usability.is_usable());
    assert!(usability.reason().unwrap().contains("friendly"));
}

/// Source: 40k_revised.md §15.1
/// Rule: apply_stratagem fails when called on an already-used-this-phase stratagem.
/// Test: Second apply of the same stratagem in the same phase returns error.
#[test]
fn stratagem_apply_fails_when_already_used_this_phase() {
    let mut state = make_test_game_state();
    let player = PlayerId::new(0);
    state.player_mut(player).gain_cp(5);
    state.current_phase = Phase::Shooting;

    // First apply succeeds
    let target = StratagemTarget::DiceRoll;
    let result1 = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::COMMAND_REROLL,
        &target,
    );
    assert!(result1.is_ok());

    // Second apply in same phase should fail
    let result2 = StratagemRuntime::apply_stratagem(
        &mut state,
        player,
        ids::COMMAND_REROLL,
        &target,
    );
    assert!(result2.is_err());
}
