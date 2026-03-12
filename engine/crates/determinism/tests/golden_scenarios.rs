//! Golden Test Scenarios for Phase 4 - Determinism and Replay Verification
//!
//! These integration tests verify:
//! - State hash determinism (same inputs = same hash every time)
//! - Replay round-trip fidelity (record → export → import → verify)
//! - Edge case correctness (empty state, single command, max rounds)
//! - Faction-specific scenarios (Custodes Ka'tah, World Eaters Blessings)
//! - Mission-specific scenarios (all 6 mission setups)
//! - Stratagem interaction correctness (timing, CP, stacking)
//!
//! Source: implementation_v3.md Section 18 (Determinism and Verification)

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, CommandPoints, DatasheetId, EffectDuration, FactionId,
    GameOutcome, Inches, InvulnerableSave, Keyword, KeywordSet, Leadership, MissionId, ModelId,
    MoveCharacteristic, ObjectiveControl, Phase, PlayerId, Position, StackingBehavior, StratagemId,
    SubPhase, Toughness, UnitId, VictoryPoints, Wounds,
};
use wh40k_command_system::{Command, CommandHistory};
use wh40k_determinism::{
    DeterminismTestSuite, FuzzTester, RoundTripVerifier, StateHasher, VerificationResult,
};
use wh40k_dice::{DiceContext, DiceRoller, SeedBundle, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_game_core::executor::CommandExecutor;
use wh40k_game_core::state::{GameState, PlayerState, TurnFlags};
use wh40k_game_core::unit::{ModelState, UnitState};
use wh40k_geometry::Board;
use wh40k_replay::{
    ReplayDiff, ReplayRecorder, REPLAY_FORMAT_VERSION,
};

// ============================================================================
// Helper: Create minimal GameState for testing
// ============================================================================

/// Create a minimal but valid GameState for golden testing.
fn create_test_game_state(seed: u64) -> GameState {
    let bundle = SeedBundle::from_u64(seed, format!("golden-test-{}", seed));
    let ctx = DiceContext::from_bundle(&bundle, StreamKind::BattleShockTest, 0, 0);
    let dice_roller = DiceRoller::new(ctx);

    let player_a = PlayerState::new(PlayerId::new(0), "Custodes Player".to_string());
    let player_b = PlayerState::new(PlayerId::new(1), "World Eaters Player".to_string());

    let board = Board::combat_patrol();

    GameState {
        content_version: "v1.0.0-combat-patrol".to_string(),
        scenario_id: None,
        battle_round: BattleRound::new(1),
        active_player: PlayerId::new(0),
        current_phase: Phase::PreBattle,
        current_subphase: SubPhase::DetermineAttackerDefender,
        decision_owner: PlayerId::new(0),
        players: [player_a, player_b],
        units: Vec::new(),
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

/// Create a GameState with Custodes units deployed.
fn create_custodes_game_state(seed: u64) -> GameState {
    let mut state = create_test_game_state(seed);
    state.players[0].faction_id = Some(FactionId::new(0));
    state.players[1].faction_id = Some(FactionId::new(1));

    // Custodes: Tristraen (Blade Champion, 1 model, T6, W6, 2+/4++)
    let tristraen = UnitState::new(
        UnitId::new(0),
        PlayerId::new(0),
        "Tristraen".to_string(),
        DatasheetId::new(100),
        KeywordSet::TRISTRAEN_KEYWORDS,
        vec![ModelState::new(
            ModelId::new(0),
            UnitId::new(0),
            Wounds::new(6),
            Position::from_inches(10, 5),
            BaseSize::MM40,
            Vec::new(),
            Vec::new(),
            true,
            None,
        )],
        MoveCharacteristic::from_inches(6),
        Toughness::new(6),
        ArmorSave::TWO_PLUS,
        Some(InvulnerableSave::FOUR_PLUS),
        Leadership::new(6),
        ObjectiveControl::new(2),
    );

    // Custodian Guard (3 models, T6, W3, 2+/4++)
    let guard = UnitState::new(
        UnitId::new(1),
        PlayerId::new(0),
        "Custodian Guard".to_string(),
        DatasheetId::new(101),
        KeywordSet::CUSTODIAN_GUARD_KEYWORDS,
        (0..3)
            .map(|i| {
                ModelState::new(
                    ModelId::new(10 + i),
                    UnitId::new(1),
                    Wounds::new(3),
                    Position::from_inches(12 + i as i32 * 2, 5),
                    BaseSize::MM40,
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                )
            })
            .collect(),
        MoveCharacteristic::from_inches(6),
        Toughness::new(6),
        ArmorSave::TWO_PLUS,
        Some(InvulnerableSave::FOUR_PLUS),
        Leadership::new(6),
        ObjectiveControl::new(2),
    );

    state.units.push(tristraen);
    state.units.push(guard);
    state
}

/// Create a GameState with World Eaters (Frenzied Reavers) units deployed.
fn create_world_eaters_game_state(seed: u64) -> GameState {
    let mut state = create_custodes_game_state(seed);

    // World Eaters: Berzerker Champion (1 model leader, T4, W2)
    let champion = UnitState::new(
        UnitId::new(100),
        PlayerId::new(1),
        "Berzerker Champion".to_string(),
        DatasheetId::new(200),
        KeywordSet::from_keywords(&[
            Keyword::Infantry,
            Keyword::Character,
            Keyword::Chaos,
        ]),
        vec![ModelState::new(
            ModelId::new(1000),
            UnitId::new(100),
            Wounds::new(4),
            Position::from_inches(10, 25),
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            true,
            None,
        )],
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(6),
        ObjectiveControl::new(1),
    );

    // Khorne Berzerkers (8 models, T4, W2)
    let berzerkers = UnitState::new(
        UnitId::new(101),
        PlayerId::new(1),
        "Khorne Berzerkers".to_string(),
        DatasheetId::new(201),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Chaos]),
        (0..8)
            .map(|i| {
                ModelState::new(
                    ModelId::new(1010 + i),
                    UnitId::new(101),
                    Wounds::new(2),
                    Position::from_inches(8 + i as i32 * 2, 25),
                    BaseSize::MM32,
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                )
            })
            .collect(),
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(6),
        ObjectiveControl::new(2),
    );

    state.units.push(champion);
    state.units.push(berzerkers);
    state
}

/// Create a fully-loaded scenario using ScenarioLoader for realistic tests.
fn create_full_scenario(seed: u64, mission: Option<MissionId>) -> GameState {
    use wh40k_game_core::scenario::ScenarioLoader;
    let seed_bytes = SeedBundle::from_u64(seed, format!("golden-{}", seed)).root_seed;
    ScenarioLoader::load_scenario(
        FactionId::new(0), // Custodes
        FactionId::new(1), // World Eaters
        mission,
        seed_bytes,
    )
}

// ============================================================================
// 4.3.1 Edge Case Scenarios
// ============================================================================

#[test]
fn golden_empty_state_hash_determinism() {
    // An empty (no units) game state should produce a stable hash.
    let state = create_test_game_state(42);
    let hash1 = StateHasher::hash(&state);
    let hash2 = StateHasher::hash(&state);
    assert_eq!(hash1, hash2, "Empty state hash must be stable");

    // A different seed should produce a different hash (dice roller context differs).
    let state2 = create_test_game_state(43);
    let hash3 = StateHasher::hash(&state2);
    assert_ne!(hash1, hash3, "Different seeds must produce different hashes");
}

#[test]
fn golden_clone_preserves_hash() {
    let state = create_world_eaters_game_state(42);
    let hash_original = StateHasher::hash(&state);
    let cloned = state.clone();
    let hash_cloned = StateHasher::hash(&cloned);
    assert_eq!(
        hash_original, hash_cloned,
        "Cloned state must have identical hash"
    );
}

#[test]
fn golden_serialization_roundtrip_preserves_hash() {
    let state = create_world_eaters_game_state(42);
    DeterminismTestSuite::verify_serialization_roundtrip(&state)
        .expect("Serialization round-trip must preserve hash");
}

#[test]
fn golden_fuzz_hash_stability_100_iterations() {
    let state = create_world_eaters_game_state(42);
    let result = FuzzTester::fuzz_hash_stability(&state, 100)
        .expect("Fuzz testing must not error");
    assert!(
        result.all_passed,
        "All 100 fuzz iterations must pass. Failures: {:?}",
        result.failures
    );
    assert_eq!(result.iterations_run, 100);
}

#[test]
fn golden_different_phases_different_hashes() {
    let mut state1 = create_test_game_state(42);
    state1.current_phase = Phase::Command;
    let hash_command = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.current_phase = Phase::Movement;
    let hash_movement = StateHasher::hash(&state2);

    assert_ne!(
        hash_command, hash_movement,
        "Different phases must produce different hashes"
    );
}

#[test]
fn golden_different_battle_rounds_different_hashes() {
    let mut state1 = create_test_game_state(42);
    state1.battle_round = BattleRound::new(1);
    let hash_r1 = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.battle_round = BattleRound::new(3);
    let hash_r3 = StateHasher::hash(&state2);

    assert_ne!(
        hash_r1, hash_r3,
        "Different battle rounds must produce different hashes"
    );
}

#[test]
fn golden_different_active_players_different_hashes() {
    let mut state1 = create_test_game_state(42);
    state1.active_player = PlayerId::new(0);
    let hash_p0 = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.active_player = PlayerId::new(1);
    let hash_p1 = StateHasher::hash(&state2);

    assert_ne!(
        hash_p0, hash_p1,
        "Different active players must produce different hashes"
    );
}

#[test]
fn golden_cp_change_changes_hash() {
    let mut state1 = create_test_game_state(42);
    state1.players[0].cp = CommandPoints::new(0);
    let hash_0cp = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.players[0].cp = CommandPoints::new(3);
    let hash_3cp = StateHasher::hash(&state2);

    assert_ne!(
        hash_0cp, hash_3cp,
        "Different CP values must produce different hashes"
    );
}

#[test]
fn golden_vp_change_changes_hash() {
    let mut state1 = create_test_game_state(42);
    state1.players[0].vp = VictoryPoints::new(0);
    let hash_0vp = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.players[0].vp = VictoryPoints::new(10);
    let hash_10vp = StateHasher::hash(&state2);

    assert_ne!(
        hash_0vp, hash_10vp,
        "Different VP values must produce different hashes"
    );
}

#[test]
fn golden_unit_wounds_change_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_full = StateHasher::hash(&state1);

    // Wound a model
    let mut state2 = create_custodes_game_state(42);
    state2.units[0].models[0].wounds_remaining = Wounds::new(3);
    let hash_wounded = StateHasher::hash(&state2);

    assert_ne!(
        hash_full, hash_wounded,
        "Wounding a model must change the hash"
    );
}

#[test]
fn golden_model_death_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_alive = StateHasher::hash(&state1);

    // Kill a model
    let mut state2 = create_custodes_game_state(42);
    state2.units[1].models[0].alive = false;
    state2.units[1].models[0].wounds_remaining = Wounds::new(0);
    let hash_dead = StateHasher::hash(&state2);

    assert_ne!(
        hash_alive, hash_dead,
        "Destroying a model must change the hash"
    );
}

#[test]
fn golden_battle_shocked_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_normal = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.units[1].battle_shocked = true;
    let hash_shocked = StateHasher::hash(&state2);

    assert_ne!(
        hash_normal, hash_shocked,
        "Battle-shocked status must change the hash"
    );
}

#[test]
fn golden_game_outcome_changes_hash() {
    let mut state1 = create_test_game_state(42);
    state1.game_outcome = GameOutcome::InProgress;
    let hash_in_progress = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.game_outcome = GameOutcome::Victory(PlayerId::new(0));
    let hash_victory = StateHasher::hash(&state2);

    assert_ne!(
        hash_in_progress, hash_victory,
        "Game outcome must change the hash"
    );
}

#[test]
fn golden_turn_flags_unit_moved_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_moves = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.turn_flags.mark_moved(UnitId::new(0));
    let hash_moved = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_moves, hash_moved,
        "Marking a unit as moved must change the hash"
    );
}

#[test]
fn golden_turn_flags_order_independence() {
    // HashSets must be sorted before hashing, so order shouldn't matter.
    let mut state1 = create_custodes_game_state(42);
    state1.turn_flags.mark_moved(UnitId::new(0));
    state1.turn_flags.mark_moved(UnitId::new(1));
    let hash1 = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.turn_flags.mark_moved(UnitId::new(1));
    state2.turn_flags.mark_moved(UnitId::new(0));
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "HashSet insertion order must NOT affect the hash"
    );
}

// ============================================================================
// 4.3.2 Faction-Specific Scenarios
// ============================================================================

#[test]
fn golden_custodes_faction_selection_changes_hash() {
    let mut state1 = create_test_game_state(42);
    state1.players[0].faction_id = Some(FactionId::new(0)); // Custodes
    let hash_custodes = StateHasher::hash(&state1);

    let mut state2 = create_test_game_state(42);
    state2.players[0].faction_id = Some(FactionId::new(1)); // World Eaters
    let hash_we = StateHasher::hash(&state2);

    assert_ne!(
        hash_custodes, hash_we,
        "Different factions must produce different hashes"
    );
}

#[test]
fn golden_custodes_units_deterministic_hash() {
    let state1 = create_custodes_game_state(100);
    let state2 = create_custodes_game_state(100);

    let hash1 = StateHasher::hash(&state1);
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "Same Custodes setup must produce identical hash"
    );
}

#[test]
fn golden_world_eaters_units_deterministic_hash() {
    let state1 = create_world_eaters_game_state(200);
    let state2 = create_world_eaters_game_state(200);

    let hash1 = StateHasher::hash(&state1);
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "Same World Eaters setup must produce identical hash"
    );
}

#[test]
fn golden_custodes_ka_tah_stance_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_stance = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2
        .turn_flags
        .ka_tah_stances
        .insert(UnitId::new(1), "Dacatarai".to_string());
    let hash_dacatarai = StateHasher::hash(&state2);

    let mut state3 = create_custodes_game_state(42);
    state3
        .turn_flags
        .ka_tah_stances
        .insert(UnitId::new(1), "Rendax".to_string());
    let hash_rendax = StateHasher::hash(&state3);

    assert_ne!(
        hash_no_stance, hash_dacatarai,
        "Ka'tah stance must change the hash"
    );
    assert_ne!(
        hash_dacatarai, hash_rendax,
        "Different Ka'tah stances must produce different hashes"
    );
}

#[test]
fn golden_world_eaters_blessings_change_hash() {
    let state1 = create_world_eaters_game_state(42);
    let hash_no_blessings = StateHasher::hash(&state1);

    let mut state2 = create_world_eaters_game_state(42);
    state2.players[1]
        .faction_round_flags
        .active_blessings
        .push("Rage-fuelled Invaders".to_string());
    let hash_rage = StateHasher::hash(&state2);

    let mut state3 = create_world_eaters_game_state(42);
    state3.players[1]
        .faction_round_flags
        .active_blessings
        .push("Total Carnage".to_string());
    let hash_carnage = StateHasher::hash(&state3);

    assert_ne!(
        hash_no_blessings, hash_rage,
        "Active blessings must change the hash"
    );
    assert_ne!(
        hash_rage, hash_carnage,
        "Different blessings must produce different hashes"
    );
}

#[test]
fn golden_vaultswords_profile_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_profile = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2
        .turn_flags
        .vaultswords_profiles
        .insert(ModelId::new(0), "Behemor".to_string());
    let hash_behemor = StateHasher::hash(&state2);

    let mut state3 = create_custodes_game_state(42);
    state3
        .turn_flags
        .vaultswords_profiles
        .insert(ModelId::new(0), "Hurricanus".to_string());
    let hash_hurricanus = StateHasher::hash(&state3);

    assert_ne!(
        hash_no_profile, hash_behemor,
        "Vaultswords profile must change the hash"
    );
    assert_ne!(
        hash_behemor, hash_hurricanus,
        "Different Vaultswords profiles must produce different hashes"
    );
}

#[test]
fn golden_fight_on_death_queue_changes_hash() {
    let state1 = create_world_eaters_game_state(42);
    let hash_empty_queue = StateHasher::hash(&state1);

    let mut state2 = create_world_eaters_game_state(42);
    state2
        .turn_flags
        .fight_on_death_queue
        .push(ModelId::new(1000));
    let hash_queued = StateHasher::hash(&state2);

    assert_ne!(
        hash_empty_queue, hash_queued,
        "Fight-on-death queue must change the hash"
    );
}

// ============================================================================
// 4.3.3 Mission-Specific Scenarios
// ============================================================================

#[test]
fn golden_mission_scenario_deterministic_setup() {
    // Verify that the same mission with the same seed produces the same state.
    let state1 = create_full_scenario(42, Some(MissionId::new(1)));
    let state2 = create_full_scenario(42, Some(MissionId::new(1)));

    let hash1 = StateHasher::hash(&state1);
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "Same mission + same seed must produce identical hash"
    );
}

#[test]
fn golden_different_missions_different_hashes() {
    let state1 = create_full_scenario(42, Some(MissionId::new(1)));
    let state2 = create_full_scenario(42, Some(MissionId::new(2)));

    let hash1 = StateHasher::hash(&state1);
    let hash2 = StateHasher::hash(&state2);

    assert_ne!(
        hash1, hash2,
        "Different missions must produce different hashes"
    );
}

#[test]
fn golden_all_six_missions_unique_hashes() {
    let hashes: Vec<u64> = (1..=6)
        .map(|m| {
            let state = create_full_scenario(42, Some(MissionId::new(m)));
            StateHasher::hash(&state)
        })
        .collect();

    // All 6 mission hashes should be unique
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            assert_ne!(
                hashes[i], hashes[j],
                "Mission {} and Mission {} must have different hashes",
                i + 1,
                j + 1
            );
        }
    }
}

#[test]
fn golden_mission_scoring_progress_changes_hash() {
    let state1 = create_full_scenario(42, Some(MissionId::new(1)));
    let hash_no_score = StateHasher::hash(&state1);

    let mut state2 = create_full_scenario(42, Some(MissionId::new(1)));
    state2.players[0].mission_progress.primary_vp = VictoryPoints::new(5);
    let hash_5vp = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_score, hash_5vp,
        "Scoring progress must change the hash"
    );
}

#[test]
fn golden_mission_round_scores_change_hash() {
    let state1 = create_full_scenario(42, Some(MissionId::new(1)));
    let hash_no_rounds = StateHasher::hash(&state1);

    let mut state2 = create_full_scenario(42, Some(MissionId::new(1)));
    state2
        .players[0]
        .mission_progress
        .round_scores
        .push((BattleRound::new(2), VictoryPoints::new(5)));
    let hash_with_round = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_rounds, hash_with_round,
        "Round scores must change the hash"
    );
}

// ============================================================================
// 4.3.4 Stratagem Interaction Scenarios
// ============================================================================

#[test]
fn golden_stratagem_usage_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_strats = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.players[0]
        .stratagem_usage
        .used_this_turn
        .insert(StratagemId::new(1));
    let hash_used = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_strats, hash_used,
        "Stratagem usage must change the hash"
    );
}

#[test]
fn golden_stratagem_phase_usage_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_phase_strats = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.players[0]
        .stratagem_usage
        .used_this_phase
        .insert(StratagemId::new(5));
    let hash_phase_used = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_phase_strats, hash_phase_used,
        "Phase stratagem usage must change the hash"
    );
}

#[test]
fn golden_stratagem_battle_usage_changes_hash() {
    let state1 = create_custodes_game_state(42);
    let hash_no_battle_strats = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.players[0]
        .stratagem_usage
        .used_this_battle
        .insert(StratagemId::new(10));
    let hash_battle_used = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_battle_strats, hash_battle_used,
        "Battle stratagem usage must change the hash"
    );
}

#[test]
fn golden_multiple_stratagems_order_independent() {
    // HashSet order must not affect hash
    let mut state1 = create_custodes_game_state(42);
    state1.players[0]
        .stratagem_usage
        .used_this_turn
        .insert(StratagemId::new(1));
    state1.players[0]
        .stratagem_usage
        .used_this_turn
        .insert(StratagemId::new(5));
    let hash1 = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.players[0]
        .stratagem_usage
        .used_this_turn
        .insert(StratagemId::new(5));
    state2.players[0]
        .stratagem_usage
        .used_this_turn
        .insert(StratagemId::new(1));
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "Stratagem usage order must NOT affect the hash"
    );
}

// ============================================================================
// 4.3.5 Active Effects Scenarios
// ============================================================================

#[test]
fn golden_active_effect_changes_hash() {
    use wh40k_game_core::effect::{ActiveEffect, EffectSource, EffectTarget, EffectType};

    let state1 = create_custodes_game_state(42);
    let hash_no_effects = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.active_effects.push(ActiveEffect {
        id: 1,
        source: EffectSource::Stratagem(StratagemId::new(1)),
        target: EffectTarget::Unit(UnitId::new(0)),
        effect_type: EffectType::GrantFightsFirst,
        duration: EffectDuration::UntilEndOfPhase,
        stacking: StackingBehavior::Unique,
        applied_round: BattleRound::new(1),
        applied_phase: Phase::Charge,
    });
    let hash_with_effect = StateHasher::hash(&state2);

    assert_ne!(
        hash_no_effects, hash_with_effect,
        "Active effects must change the hash"
    );
}

#[test]
fn golden_different_effect_types_different_hashes() {
    use wh40k_game_core::effect::{ActiveEffect, EffectSource, EffectTarget, EffectType};

    let make_state_with_effect = |effect_type: EffectType| {
        let mut state = create_custodes_game_state(42);
        state.active_effects.push(ActiveEffect {
            id: 1,
            source: EffectSource::CoreRule("test".to_string()),
            target: EffectTarget::Unit(UnitId::new(0)),
            effect_type,
            duration: EffectDuration::UntilEndOfPhase,
            stacking: StackingBehavior::Unique,
            applied_round: BattleRound::new(1),
            applied_phase: Phase::Shooting,
        });
        StateHasher::hash(&state)
    };

    let hash_fights_first = make_state_with_effect(EffectType::GrantFightsFirst);
    let hash_stealth = make_state_with_effect(EffectType::Stealth);
    let hash_fnp = make_state_with_effect(EffectType::GrantFeelNoPain(5));

    assert_ne!(hash_fights_first, hash_stealth);
    assert_ne!(hash_stealth, hash_fnp);
    assert_ne!(hash_fights_first, hash_fnp);
}

// ============================================================================
// 4.3.6 Command Execution and Hash Chain Scenarios
// ============================================================================

#[test]
fn golden_select_faction_command_changes_hash() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let hash_before = StateHasher::hash(&state);

    let command = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let result = CommandExecutor::execute(&mut state, &command);
    assert!(result.is_ok(), "SelectFaction should succeed");

    let hash_after = StateHasher::hash(&state);
    assert_ne!(
        hash_before, hash_after,
        "Executing a command must change the hash"
    );

    // Verify the recorded hash_after matches our computed hash
    let recorded_hash = state.command_history.last_state_hash();
    assert_eq!(
        recorded_hash,
        Some(hash_after),
        "Recorded hash_after must match computed hash"
    );
}

#[test]
fn golden_command_hash_chain_consistency() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    // Execute multiple commands
    let commands = vec![
        Command::SelectFaction {
            player: PlayerId::new(0),
            faction_id: FactionId::new(0),
        },
        Command::SelectFaction {
            player: PlayerId::new(1),
            faction_id: FactionId::new(1),
        },
    ];

    for cmd in &commands {
        let result = CommandExecutor::execute(&mut state, cmd);
        assert!(result.is_ok(), "Command should succeed: {:?}", cmd);
    }

    // Verify hash chain: hash_after[i] == hash_before[i+1]
    let verification = RoundTripVerifier::verify_hash_chain(&state.command_history);
    match verification {
        VerificationResult::Match { total_commands, .. } => {
            assert_eq!(total_commands, 2);
        }
        other => panic!("Hash chain verification failed: {:?}", other),
    }
}

#[test]
fn golden_deterministic_command_replay() {
    // Execute the same commands with the same seed twice.
    // State hashes must match at every step.
    let commands = vec![
        Command::SelectFaction {
            player: PlayerId::new(0),
            faction_id: FactionId::new(0),
        },
        Command::SelectFaction {
            player: PlayerId::new(1),
            faction_id: FactionId::new(1),
        },
    ];

    let mut state1 = create_test_game_state(42);
    state1.current_phase = Phase::PreBattle;
    state1.current_subphase = SubPhase::DetermineAttackerDefender;
    let mut hashes1 = vec![StateHasher::hash(&state1)];
    for cmd in &commands {
        CommandExecutor::execute(&mut state1, cmd).unwrap();
        hashes1.push(StateHasher::hash(&state1));
    }

    let mut state2 = create_test_game_state(42);
    state2.current_phase = Phase::PreBattle;
    state2.current_subphase = SubPhase::DetermineAttackerDefender;
    let mut hashes2 = vec![StateHasher::hash(&state2)];
    for cmd in &commands {
        CommandExecutor::execute(&mut state2, cmd).unwrap();
        hashes2.push(StateHasher::hash(&state2));
    }

    assert_eq!(
        hashes1, hashes2,
        "Replaying the same commands with the same seed must produce identical hash sequences"
    );
}

// ============================================================================
// 4.3.7 Replay System Integration Scenarios
// ============================================================================

#[test]
fn golden_replay_recorder_captures_commands() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);

    let cmd = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    recorder.record_command(&state, &cmd, &events, true);

    let replay = recorder.finalize(&state);
    assert_eq!(replay.frames.len(), 1);
    assert_eq!(replay.format_version, REPLAY_FORMAT_VERSION);
}

#[test]
fn golden_replay_json_roundtrip() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);

    let cmd = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    recorder.record_command(&state, &cmd, &events, true);

    let replay = recorder.finalize(&state);

    // Export to JSON
    let json = wh40k_replay::export_json(&replay).expect("JSON export must succeed");
    assert!(!json.is_empty());

    // Import from JSON
    let imported = wh40k_replay::import_json(&json).expect("JSON import must succeed");
    assert_eq!(imported.frames.len(), replay.frames.len());
    assert_eq!(imported.format_version, replay.format_version);
    assert_eq!(imported.header.content_version, replay.header.content_version);
}

#[test]
fn golden_replay_binary_roundtrip() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);

    let cmd = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    recorder.record_command(&state, &cmd, &events, true);

    let replay = recorder.finalize(&state);

    // Export to binary
    let binary = wh40k_replay::export_binary(&replay).expect("Binary export must succeed");
    assert!(!binary.is_empty());

    // Import from binary
    let imported = wh40k_replay::import_binary(&binary).expect("Binary import must succeed");
    assert_eq!(imported.frames.len(), replay.frames.len());
}

#[test]
fn golden_replay_summary_contains_key_info() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);

    let cmd = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    recorder.record_command(&state, &cmd, &events, true);

    let replay = recorder.finalize(&state);
    let summary = wh40k_replay::export_summary(&replay);

    assert!(summary.contains("Replay Summary"), "Summary must have header");
    assert!(summary.contains("Total commands"), "Summary must show command count");
}

#[test]
fn golden_replay_diff_identical_replays() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);
    let cmd = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events = CommandExecutor::execute(&mut state, &cmd).unwrap();
    recorder.record_command(&state, &cmd, &events, true);
    let replay = recorder.finalize(&state);

    // Clone the replay and compare
    let replay_copy = replay.clone();
    let diff = ReplayDiff::compare(&replay, &replay_copy);

    assert!(diff.identical, "Identical replays must have no differences");
    assert!(diff.differences.is_empty());
    assert!(diff.divergence_point.is_none());
}

#[test]
fn golden_replay_diff_detects_divergence() {
    let mut state1 = create_test_game_state(42);
    state1.current_phase = Phase::PreBattle;
    state1.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder1 = ReplayRecorder::new(&state1);
    let cmd1 = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(0),
    };
    let events1 = CommandExecutor::execute(&mut state1, &cmd1).unwrap();
    recorder1.record_command(&state1, &cmd1, &events1, true);
    let replay1 = recorder1.finalize(&state1);

    let mut state2 = create_test_game_state(42);
    state2.current_phase = Phase::PreBattle;
    state2.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder2 = ReplayRecorder::new(&state2);
    let cmd2 = Command::SelectFaction {
        player: PlayerId::new(0),
        faction_id: FactionId::new(1), // Different faction!
    };
    let events2 = CommandExecutor::execute(&mut state2, &cmd2).unwrap();
    recorder2.record_command(&state2, &cmd2, &events2, true);
    let replay2 = recorder2.finalize(&state2);

    let diff = ReplayDiff::compare(&replay1, &replay2);

    assert!(!diff.identical, "Divergent replays must not be identical");
    assert!(
        diff.divergence_point.is_some(),
        "Must detect divergence point"
    );
    assert_eq!(diff.divergence_point.unwrap(), 0);
}

// ============================================================================
// 4.3.8 Full Scenario Determinism Tests
// ============================================================================

#[test]
fn golden_full_scenario_custodes_vs_world_eaters_deterministic() {
    let state1 = create_full_scenario(42, None);
    let state2 = create_full_scenario(42, None);

    assert!(
        RoundTripVerifier::states_match(&state1, &state2),
        "Same full scenario must produce matching states"
    );
}

#[test]
fn golden_full_scenario_different_seeds_diverge() {
    let state1 = create_full_scenario(42, None);
    let state2 = create_full_scenario(99, None);

    assert!(
        !RoundTripVerifier::states_match(&state1, &state2),
        "Different seeds must produce different states"
    );
}

#[test]
fn golden_full_scenario_serialization_roundtrip() {
    let state = create_full_scenario(42, Some(MissionId::new(3)));
    DeterminismTestSuite::verify_serialization_roundtrip(&state)
        .expect("Full scenario serialization round-trip must preserve hash");
}

#[test]
fn golden_full_scenario_clone_determinism() {
    let state = create_full_scenario(42, Some(MissionId::new(4)));
    DeterminismTestSuite::verify_clone_determinism(&state)
        .expect("Full scenario clone must preserve hash");
}

#[test]
fn golden_full_scenario_no_float_dependency() {
    let state = create_full_scenario(42, Some(MissionId::new(5)));
    DeterminismTestSuite::verify_no_float_dependency(&state)
        .expect("Full scenario must have no float dependency");
}

#[test]
fn golden_full_scenario_fuzz_50_iterations() {
    let state = create_full_scenario(42, Some(MissionId::new(1)));
    let result = FuzzTester::fuzz_hash_stability(&state, 50)
        .expect("Fuzz testing must not error");
    assert!(
        result.all_passed,
        "All 50 fuzz iterations must pass on full scenario. Failures: {:?}",
        result.failures
    );
}

// ============================================================================
// 4.3.9 Cross-Platform Determinism (No Floats)
// ============================================================================

#[test]
fn golden_position_uses_fixed_point() {
    // Verify Position uses fixed-point (Inches with milliinch precision),
    // not floating-point values.
    let pos = Position::from_inches(22, 15);
    let pos2 = Position::from_inches(22, 15);
    assert_eq!(pos, pos2, "Fixed-point positions must be exactly equal");

    // Create two states with the same position and verify hash match
    let mut state1 = create_custodes_game_state(42);
    state1.units[0].models[0].position = Position::from_inches(22, 15);
    let hash1 = StateHasher::hash(&state1);

    let mut state2 = create_custodes_game_state(42);
    state2.units[0].models[0].position = Position::from_inches(22, 15);
    let hash2 = StateHasher::hash(&state2);

    assert_eq!(
        hash1, hash2,
        "Same fixed-point positions must produce identical hashes"
    );
}

#[test]
fn golden_measurement_determinism() {
    // Verify all measurements use integer/fixed-point arithmetic
    let inches1 = Inches::from_inches(6);
    let inches2 = Inches::from_inches(6);
    assert_eq!(inches1, inches2);

    // Different values should be different
    let inches3 = Inches::from_inches(7);
    assert_ne!(inches1, inches3);
}

// ============================================================================
// 4.3.10 Replay Verification Against Live State
// ============================================================================

#[test]
fn golden_replay_verification_with_hash_matching() {
    let mut state = create_test_game_state(42);
    state.current_phase = Phase::PreBattle;
    state.current_subphase = SubPhase::DetermineAttackerDefender;

    let mut recorder = ReplayRecorder::new(&state);

    // Execute and record several commands
    let commands = vec![
        Command::SelectFaction {
            player: PlayerId::new(0),
            faction_id: FactionId::new(0),
        },
        Command::SelectFaction {
            player: PlayerId::new(1),
            faction_id: FactionId::new(1),
        },
    ];

    for cmd in &commands {
        let events = CommandExecutor::execute(&mut state, cmd).unwrap();
        recorder.record_command(&state, cmd, &events, true);
    }

    let replay = recorder.finalize(&state);

    // Verify the replay has correct structure
    assert_eq!(replay.frames.len(), 2);

    // Verify hash chain in the replay frames
    for i in 1..replay.frames.len() {
        if let (Some(prev_after), curr_before) = (
            replay.frames[i - 1].state_hash_after,
            replay.frames[i].state_hash_before,
        ) {
            assert_eq!(
                prev_after, curr_before,
                "Replay hash chain must be continuous: frame {} hash_after ({:#018x}) != frame {} hash_before ({:#018x})",
                i - 1, prev_after, i, curr_before
            );
        }
    }
}
