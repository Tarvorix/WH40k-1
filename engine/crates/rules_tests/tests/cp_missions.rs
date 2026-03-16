//! Source-linked rules tests for CP_Rules.md Section 13: Combat Patrol Missions.
//!
//! Tests all 6 Combat Patrol missions and their scoring mechanics:
//!   Mission 1: Clash of Patrols (Retrieve Intelligence + Take and Hold)
//!   Mission 2: Archeotech Recovery (Irradiated Power Cells + Recover Archeotech)
//!   Mission 3: Forward Outpost (Sabotage Enemy Comms + Vital Ground)
//!   Mission 4: Scorched Earth (Raze and Ruin)
//!   Mission 5: Sweeping Raid (Supply Lines + Priority Targets)
//!   Mission 6: Display of Might (Break Their Spirit + Claim Sites + Symbolic Sites)
//!
//! Validates VP award values, scoring schedules, battle-ready bonus (10 VP),
//! VP scoring via score_vp method, and mission structure from the mission_runtime
//! and combat_patrol_rules crates.
//!
//! Source: CP_Rules.md Section 13 — "COMBAT PATROL MISSIONS"

use wh40k_core_types::{
    ArmorSave, BaseSize, BattleRound, DatasheetId, GameMode, GameOutcome,
    Keyword, KeywordSet, Leadership, MissionId, ModelId,
    MoveCharacteristic, ObjectiveControl, Phase, PlayerId, Position,
    SubPhase, Toughness, UnitId, UnitStatus, VictoryPoints, Wounds,
};
use wh40k_content_schema::ObjectiveZone;
use wh40k_combat_patrol_rules::CombatPatrolRules;
use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
use wh40k_event_system::EventBus;
use wh40k_command_system::CommandHistory;
use wh40k_game_core::state::{GameState, PlayerState, TurnFlags};
use wh40k_game_core::unit::{ModelState, UnitState};
use wh40k_geometry::Board;
use wh40k_mission_runtime::{MissionRuntime, ScoringPhase};

// ===========================================================================
// Helper factories
// ===========================================================================

/// Build a basic ModelState at a given position.
fn make_model(model_id: u32, unit_id: UnitId, position: Position) -> ModelState {
    ModelState::new(
        ModelId::new(model_id),
        unit_id,
        Wounds::new(3),
        position,
        BaseSize::MM32,
        Vec::new(),
        Vec::new(),
        false,
        None,
    )
}

/// Build a basic UnitState placed on the battlefield at a given position with given OC.
fn make_unit_at(
    id: u32,
    owner: PlayerId,
    pos: Position,
    oc: u8,
) -> UnitState {
    let unit_id = UnitId::new(id);
    let model = make_model(id * 100, unit_id, pos);

    let mut unit = UnitState::new(
        unit_id,
        owner,
        format!("Test Unit {}", id),
        DatasheetId::new(id),
        KeywordSet::from_keywords(&[Keyword::Infantry, Keyword::Battleline]),
        vec![model],
        MoveCharacteristic::from_inches(6),
        Toughness::new(4),
        ArmorSave::THREE_PLUS,
        None,
        Leadership::new(7),
        ObjectiveControl::new(oc),
    );
    unit.status = UnitStatus::OnBattlefield;
    unit
}

/// Build a minimal GameState for scoring tests. Defaults to Command phase,
/// battle round 2, GameMode::CombatPatrol.
fn make_scoring_game_state(
    units: Vec<UnitState>,
    round: u8,
    phase: Phase,
) -> GameState {
    let seed = [0u8; 32];
    let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
    let dice_roller = DiceRoller::new(ctx);

    GameState {
        content_version: "test-missions-1.0".to_string(),
        scenario_id: None,
        battle_round: BattleRound::new(round),
        active_player: PlayerId::new(0),
        current_phase: phase,
        current_subphase: SubPhase::CommandPhaseStart,
        decision_owner: PlayerId::new(0),
        players: [
            PlayerState::new(PlayerId::new(0), "Player A".to_string()),
            PlayerState::new(PlayerId::new(1), "Player B".to_string()),
        ],
        units,
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

// ===========================================================================
// Mission 1: Clash of Patrols — Retrieve Intelligence + Take and Hold
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 1
/// Clash of Patrols has 4 objectives (A in Attacker zone, B in Defender zone,
/// C and D in No Man's Land) and uses Take and Hold primary scoring.
#[test]
fn m1_clash_of_patrols_has_correct_structure() {
    let mission = MissionRuntime::create_clash_of_patrols();
    assert_eq!(mission.mission_id, MissionId::new(1));
    assert_eq!(mission.name, "Clash of Patrols");
    assert_eq!(mission.objectives.len(), 4);
    assert_eq!(mission.rounds, 5);
}

/// Source: CP_Rules.md §13, Mission 1
/// Take and Hold scoring: 5VP per objective controlled at end of Command Phase
/// from BR2 through BR5.
#[test]
fn m1_clash_of_patrols_scoring_schedule_br2_to_br5() {
    let mission = MissionRuntime::create_clash_of_patrols();
    assert_eq!(mission.scoring_schedule.len(), 1);
    let sched = &mission.scoring_schedule[0];
    assert_eq!(sched.from_round, 2, "Scoring starts at BR2");
    assert_eq!(sched.to_round, 5, "Scoring ends at BR5");
    assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);
    assert_eq!(sched.vp_per_objective, 5, "5VP per objective controlled");
}

/// Source: CP_Rules.md §13, Mission 1
/// Objective zones: A=Attacker, B=Defender, C=NML, D=NML
#[test]
fn m1_clash_of_patrols_objective_zones() {
    let mission = MissionRuntime::create_clash_of_patrols();
    assert_eq!(mission.objectives[0].zone, ObjectiveZone::AttackerZone);
    assert_eq!(mission.objectives[1].zone, ObjectiveZone::DefenderZone);
    assert_eq!(mission.objectives[2].zone, ObjectiveZone::NoMansLand);
    assert_eq!(mission.objectives[3].zone, ObjectiveZone::NoMansLand);
}

/// Source: CP_Rules.md §13, Mission 1
/// Retrieve Intelligence mission rule is present.
#[test]
fn m1_clash_of_patrols_has_retrieve_intelligence_rule() {
    let mission = MissionRuntime::create_clash_of_patrols();
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Retrieve Intelligence")),
        "Mission 1 should have Retrieve Intelligence mission rule"
    );
}

// ===========================================================================
// Mission 2: Archeotech Recovery — Irradiated Power Cells + Recover Archeotech
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 2
/// Archeotech Recovery has 4 objectives, cross deployment (map_id 2), and
/// Irradiated Power Cells special rule (NML objectives removed rounds 3-5).
#[test]
fn m2_archeotech_recovery_structure_and_deployment() {
    let mission = MissionRuntime::create_archeotech_recovery();
    assert_eq!(mission.mission_id, MissionId::new(2));
    assert_eq!(mission.name, "Archeotech Recovery");
    assert_eq!(mission.objectives.len(), 4);
    assert_eq!(mission.rounds, 5);
    assert_eq!(mission.deployment_config.map_id, 2, "Cross deployment map");
}

/// Source: CP_Rules.md §13, Mission 2
/// Recover Archeotech scoring: 5VP per objective at EndOfCommandPhase BR2-5.
/// +10VP end-of-battle bonus for controlling last NML objective.
#[test]
fn m2_archeotech_recovery_scoring_and_bonus() {
    let mission = MissionRuntime::create_archeotech_recovery();
    let sched = &mission.scoring_schedule[0];
    assert_eq!(sched.from_round, 2);
    assert_eq!(sched.to_round, 5);
    assert_eq!(sched.vp_per_objective, 5);

    // Must have Irradiated Power Cells and +10VP bonus rules
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Irradiated Power Cells")),
        "Should have Irradiated Power Cells rule"
    );
    assert!(
        mission.special_rules.iter().any(|r| r.contains("+10VP")),
        "Should have +10VP end-of-battle bonus rule"
    );
}

// ===========================================================================
// Mission 3: Forward Outpost — Sabotage Enemy Comms + Vital Ground
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 3
/// Vital Ground scoring: 5VP per NML objective, 10VP for enemy DZ objective (max 15VP).
/// Sabotage Enemy Comms blocks opponent's Command Re-roll.
#[test]
fn m3_forward_outpost_vital_ground_scoring() {
    let mission = MissionRuntime::create_forward_outpost();
    assert_eq!(mission.mission_id, MissionId::new(3));
    assert_eq!(mission.name, "Forward Outpost");
    assert_eq!(mission.objectives.len(), 4);

    // Vital Ground rule mentions 10VP for enemy DZ
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Vital Ground") && r.contains("10VP")),
        "Should have Vital Ground rule with 10VP for enemy DZ objective"
    );
}

/// Source: CP_Rules.md §13, Mission 3
/// Sabotage Enemy Comms: controls opponent's DZ objective -> blocks Command Re-roll.
#[test]
fn m3_forward_outpost_sabotage_enemy_comms_rule() {
    let mission = MissionRuntime::create_forward_outpost();
    assert!(
        mission.special_rules.iter().any(|r|
            r.contains("Sabotage Enemy Comms") && r.contains("Command Re-roll")
        ),
        "Should have Sabotage Enemy Comms rule referencing Command Re-roll"
    );
}

// ===========================================================================
// Mission 4: Scorched Earth — Raze and Ruin
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 4
/// Raze and Ruin: 5VP control 1+, 5VP control more than opponent, 10VP razed objective.
/// Attacker cannot raze A, Defender cannot raze B.
#[test]
fn m4_scorched_earth_raze_and_ruin_rules() {
    let mission = MissionRuntime::create_scorched_earth();
    assert_eq!(mission.mission_id, MissionId::new(4));
    assert_eq!(mission.name, "Scorched Earth");

    // Must have Raze and Ruin rule with razing restrictions
    assert!(
        mission.special_rules.iter().any(|r|
            r.contains("Raze and Ruin")
                && r.contains("Attacker cannot raze objective A")
                && r.contains("Defender cannot raze objective B")
        ),
        "Should have Raze and Ruin rule with restrictions on home objectives"
    );
}

/// Source: CP_Rules.md §13, Mission 4
/// Objectives A and B have per-objective special rules preventing razing by their owner.
#[test]
fn m4_scorched_earth_objective_raze_restrictions() {
    let mission = MissionRuntime::create_scorched_earth();

    // Objective A: Attacker cannot raze
    assert!(
        mission.objectives[0].special_rules.iter().any(|r| r.contains("Attacker cannot raze")),
        "Objective A should have Attacker-cannot-raze restriction"
    );
    // Objective B: Defender cannot raze
    assert!(
        mission.objectives[1].special_rules.iter().any(|r| r.contains("Defender cannot raze")),
        "Objective B should have Defender-cannot-raze restriction"
    );
    // Objectives C and D: no restrictions
    assert!(mission.objectives[2].special_rules.is_empty(), "Objective C should have no restrictions");
    assert!(mission.objectives[3].special_rules.is_empty(), "Objective D should have no restrictions");
}

// ===========================================================================
// Mission 5: Sweeping Raid — Supply Lines + Priority Targets
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 5
/// Priority Targets scoring: 5VP per objective at EndOfCommandPhase BR2-4 only.
/// End-of-battle bonus: Attacker 5VP for C, 10VP for D; Defender 5VP for B, 10VP for A.
#[test]
fn m5_sweeping_raid_scoring_br2_to_br4() {
    let mission = MissionRuntime::create_sweeping_raid();
    assert_eq!(mission.mission_id, MissionId::new(5));
    assert_eq!(mission.name, "Sweeping Raid");

    let sched = &mission.scoring_schedule[0];
    assert_eq!(sched.from_round, 2, "Priority Targets scoring starts at BR2");
    assert_eq!(sched.to_round, 4, "Priority Targets scoring ends at BR4 (not BR5)");
    assert_eq!(sched.vp_per_objective, 5);
}

/// Source: CP_Rules.md §13, Mission 5
/// End-of-battle bonus values are encoded in objective special_rules.
#[test]
fn m5_sweeping_raid_end_of_battle_bonus_values() {
    let mission = MissionRuntime::create_sweeping_raid();

    // A: Defender scores 10VP, B: Defender scores 5VP
    // C: Attacker scores 5VP, D: Attacker scores 10VP
    assert!(
        mission.objectives[0].special_rules.iter().any(|r| r.contains("Defender scores 10VP")),
        "Objective A should give Defender 10VP at end of battle"
    );
    assert!(
        mission.objectives[1].special_rules.iter().any(|r| r.contains("Defender scores 5VP")),
        "Objective B should give Defender 5VP at end of battle"
    );
    assert!(
        mission.objectives[2].special_rules.iter().any(|r| r.contains("Attacker scores 5VP")),
        "Objective C should give Attacker 5VP at end of battle"
    );
    assert!(
        mission.objectives[3].special_rules.iter().any(|r| r.contains("Attacker scores 10VP")),
        "Objective D should give Attacker 10VP at end of battle"
    );
}

/// Source: CP_Rules.md §13, Mission 5
/// Supply Lines: control your DZ objective -> roll D6, on 4+ gain 1CP.
#[test]
fn m5_sweeping_raid_supply_lines_rule() {
    let mission = MissionRuntime::create_sweeping_raid();
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Supply Lines") && r.contains("4+") && r.contains("1CP")),
        "Should have Supply Lines rule (4+ for 1CP)"
    );
}

// ===========================================================================
// Mission 6: Display of Might — Break Their Spirit + Claim Sites + Symbolic Sites
// ===========================================================================

/// Source: CP_Rules.md §13, Mission 6
/// Display of Might: NML objectives are symbolic sites. CHARACTERs claim them.
/// Scoring: 5VP each for multiple conditions (control 1+, control 2+, claimed site,
/// same model claimed 2+ consecutive turns). Max 20VP per scoring.
#[test]
fn m6_display_of_might_structure() {
    let mission = MissionRuntime::create_display_of_might();
    assert_eq!(mission.mission_id, MissionId::new(6));
    assert_eq!(mission.name, "Display of Might");
    assert_eq!(mission.objectives.len(), 4);
    assert_eq!(mission.rounds, 5);
}

/// Source: CP_Rules.md §13, Mission 6
/// NML objectives (C, D) are marked as symbolic sites. DZ objectives (A, B) are not.
#[test]
fn m6_display_of_might_symbolic_sites() {
    let mission = MissionRuntime::create_display_of_might();

    // C and D are symbolic sites (NoMansLand)
    assert!(
        mission.objectives[2].special_rules.contains(&"Symbolic site".to_string()),
        "Objective C should be a symbolic site"
    );
    assert!(
        mission.objectives[3].special_rules.contains(&"Symbolic site".to_string()),
        "Objective D should be a symbolic site"
    );
    // A and B are NOT symbolic sites
    assert!(mission.objectives[0].special_rules.is_empty(), "Objective A should not be a symbolic site");
    assert!(mission.objectives[1].special_rules.is_empty(), "Objective B should not be a symbolic site");
}

/// Source: CP_Rules.md §13, Mission 6
/// Break Their Spirit: Insane Bravery limited to within 6" of WARLORD.
/// Claim Sites: CHARACTER models claim symbolic sites.
#[test]
fn m6_display_of_might_special_rules() {
    let mission = MissionRuntime::create_display_of_might();
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Break Their Spirit") && r.contains("Insane Bravery") && r.contains("6\"")),
        "Should have Break Their Spirit rule"
    );
    assert!(
        mission.special_rules.iter().any(|r| r.contains("Claim Sites") && r.contains("CHARACTER")),
        "Should have Claim Sites rule referencing CHARACTER"
    );
}

// ===========================================================================
// Cross-mission: All 6 missions via create_mission lookup
// ===========================================================================

/// Source: CP_Rules.md §13
/// All 6 missions can be created by ID and have consistent structure.
#[test]
fn all_six_missions_created_via_lookup() {
    let expected_names = [
        "Clash of Patrols",
        "Archeotech Recovery",
        "Forward Outpost",
        "Scorched Earth",
        "Sweeping Raid",
        "Display of Might",
    ];

    for (idx, expected_name) in expected_names.iter().enumerate() {
        let id = (idx + 1) as u32;
        let mission = MissionRuntime::create_mission(MissionId::new(id));
        assert!(mission.is_some(), "Mission {} should exist", id);
        let m = mission.unwrap();
        assert_eq!(m.mission_id, MissionId::new(id));
        assert_eq!(m.name, *expected_name, "Mission {} name mismatch", id);
        assert_eq!(m.rounds, 5, "Mission {} should have 5 rounds", id);
        assert_eq!(m.objectives.len(), 4, "Mission {} should have 4 objectives", id);
    }

    // Non-existent missions return None
    assert!(MissionRuntime::create_mission(MissionId::new(0)).is_none());
    assert!(MissionRuntime::create_mission(MissionId::new(7)).is_none());
}

/// Source: CP_Rules.md §13
/// All 6 missions score at EndOfCommandPhase with 5VP per objective.
#[test]
fn all_missions_score_at_end_of_command_phase() {
    for id in 1..=6u32 {
        let mission = MissionRuntime::create_mission(MissionId::new(id)).unwrap();
        for sched in &mission.scoring_schedule {
            assert_eq!(
                sched.timing,
                ScoringPhase::EndOfCommandPhase,
                "Mission {} should score at EndOfCommandPhase",
                mission.name
            );
            assert_eq!(
                sched.vp_per_objective, 5,
                "Mission {} should award 5VP per objective",
                mission.name
            );
        }
    }
}

// ===========================================================================
// Battle-Ready Bonus (10 VP)
// ===========================================================================

/// Source: CP_Rules.md — "Battle Ready bonus of 10 Victory Points"
/// Both players receive 10VP battle-ready bonus.
#[test]
fn battle_ready_bonus_awards_10vp_to_both_players() {
    let state = make_scoring_game_state(Vec::new(), 1, Phase::PreBattle);
    let events = MissionRuntime::apply_battle_ready_bonus(&state);

    assert_eq!(events.len(), 2, "Both players should receive battle-ready bonus");
    assert_eq!(events[0].player, PlayerId::new(0));
    assert_eq!(events[0].amount, VictoryPoints::new(10));
    assert_eq!(events[1].player, PlayerId::new(1));
    assert_eq!(events[1].amount, VictoryPoints::new(10));
}

/// Source: CP_Rules.md — Battle-ready bonus value matches CombatPatrolRules constant.
#[test]
fn battle_ready_bonus_matches_combat_patrol_rules_constant() {
    let bonus = CombatPatrolRules::battle_ready_bonus();
    assert_eq!(bonus.value(), 10, "CombatPatrolRules::battle_ready_bonus should be 10VP");

    let state = make_scoring_game_state(Vec::new(), 1, Phase::PreBattle);
    let events = MissionRuntime::apply_battle_ready_bonus(&state);
    assert_eq!(
        events[0].amount.value(),
        bonus.value(),
        "MissionRuntime bonus should match CombatPatrolRules constant"
    );
}

// ===========================================================================
// VP Scoring via score_vp method
// ===========================================================================

/// Source: CP_Rules.md — VP accumulation via PlayerState::score_vp.
/// VP adds cumulatively across multiple scoring events.
#[test]
fn score_vp_accumulates_correctly() {
    let mut player = PlayerState::new(PlayerId::new(0), "Test".to_string());
    assert_eq!(player.vp.value(), 0, "Initial VP should be 0");

    // Score primary VP: 5VP for controlling one objective
    player.score_vp(5);
    assert_eq!(player.vp.value(), 5);

    // Score again: 10VP for controlling two objectives
    player.score_vp(10);
    assert_eq!(player.vp.value(), 15);

    // Battle-ready bonus
    player.score_vp(10);
    assert_eq!(player.vp.value(), 25);
}

/// Source: CP_Rules.md — MissionProgress tracks primary and secondary VP separately.
#[test]
fn mission_progress_tracks_primary_and_secondary_vp() {
    let mut player = PlayerState::new(PlayerId::new(0), "Test".to_string());

    player.mission_progress.primary_vp = VictoryPoints::new(15);
    player.mission_progress.secondary_vp = VictoryPoints::new(8);

    assert_eq!(player.mission_progress.total_vp().value(), 23,
        "Total VP should be primary + secondary");
}

/// Source: CP_Rules.md — MissionProgress records round scores for history.
#[test]
fn mission_progress_records_round_scores() {
    let mut player = PlayerState::new(PlayerId::new(0), "Test".to_string());

    player.mission_progress.record_round_score(BattleRound::new(2), VictoryPoints::new(10));
    player.mission_progress.record_round_score(BattleRound::new(3), VictoryPoints::new(15));

    assert_eq!(player.mission_progress.round_scores.len(), 2);
    assert_eq!(player.mission_progress.round_scores[0].0, BattleRound::new(2));
    assert_eq!(player.mission_progress.round_scores[0].1, VictoryPoints::new(10));
    assert_eq!(player.mission_progress.round_scores[1].0, BattleRound::new(3));
    assert_eq!(player.mission_progress.round_scores[1].1, VictoryPoints::new(15));
}

// ===========================================================================
// Primary Scoring — score_primary with units on objectives
// ===========================================================================

/// Source: CP_Rules.md §13 — Primary scoring awards VP based on objective control.
/// A player controlling an objective within range gets 5VP.
#[test]
fn score_primary_awards_5vp_per_controlled_objective() {
    let mission = MissionRuntime::create_clash_of_patrols();

    // Place Player 0 unit on objective A position (22, 9) with OC 2
    let unit_p0 = make_unit_at(1, PlayerId::new(0), Position::from_inches(22, 9), 2);
    let state = make_scoring_game_state(vec![unit_p0], 2, Phase::Command);

    let events = MissionRuntime::score_primary(&state, &mission, BattleRound::new(2));

    // Player 0 should score for controlling objective A
    let p0_vp: i16 = events
        .iter()
        .filter(|e| e.player == PlayerId::new(0))
        .map(|e| e.amount.value())
        .sum();
    assert_eq!(p0_vp, 5, "Controlling 1 objective should award 5VP");
}

/// Source: CP_Rules.md §13 — No scoring in battle round 1 (scoring starts at BR2).
#[test]
fn score_primary_no_scoring_in_round_1() {
    let mission = MissionRuntime::create_clash_of_patrols();

    let unit_p0 = make_unit_at(1, PlayerId::new(0), Position::from_inches(22, 9), 2);
    let state = make_scoring_game_state(vec![unit_p0], 1, Phase::Command);

    let events = MissionRuntime::score_primary(&state, &mission, BattleRound::new(1));
    assert!(events.is_empty(), "No primary scoring should occur in BR1");
}

/// Source: CP_Rules.md §13 — OC tiebreaker: equal OC = no one controls.
#[test]
fn score_primary_tied_oc_means_no_control() {
    let mission = MissionRuntime::create_clash_of_patrols();

    // Both players have a unit on objective A (22, 9) with equal OC
    let unit_p0 = make_unit_at(1, PlayerId::new(0), Position::from_inches(22, 9), 2);
    let unit_p1 = make_unit_at(2, PlayerId::new(1), Position::from_inches(22, 9), 2);
    let state = make_scoring_game_state(vec![unit_p0, unit_p1], 2, Phase::Command);

    let events = MissionRuntime::score_primary(&state, &mission, BattleRound::new(2));

    // Tied OC = no one controls objective A, so no VP for that objective
    let a_events: Vec<_> = events.iter().filter(|e| e.source.contains("A")).collect();
    assert!(a_events.is_empty(), "Tied OC on objective A should mean no controller");
}

// ===========================================================================
// End-of-game determination
// ===========================================================================

/// Source: CP_Rules.md — Game ends after 5 rounds with VP comparison.
/// Higher VP wins.
#[test]
fn check_end_game_vp_winner() {
    let mut state = make_scoring_game_state(Vec::new(), 5, Phase::GameEnd);
    state.players[0].score_vp(30);
    state.players[1].score_vp(20);

    let mission = MissionRuntime::create_clash_of_patrols();
    let result = MissionRuntime::check_end_game(&state, &mission);
    assert_eq!(
        result,
        Some(GameOutcome::Victory(PlayerId::new(0))),
        "Player with more VP should win"
    );
}

/// Source: CP_Rules.md — Equal VP at end of game = draw.
#[test]
fn check_end_game_draw_on_equal_vp() {
    let mut state = make_scoring_game_state(Vec::new(), 5, Phase::GameEnd);
    state.players[0].score_vp(25);
    state.players[1].score_vp(25);

    let mission = MissionRuntime::create_clash_of_patrols();
    let result = MissionRuntime::check_end_game(&state, &mission);
    assert_eq!(result, Some(GameOutcome::Draw), "Equal VP should result in draw");
}

/// Source: CP_Rules.md — If one player is tabled, the other wins.
#[test]
fn check_end_game_tabled_player_loses() {
    // Player 0 has units, Player 1 has none (tabled)
    let unit = make_unit_at(1, PlayerId::new(0), Position::from_inches(10, 10), 2);
    let state = make_scoring_game_state(vec![unit], 3, Phase::Command);

    let mission = MissionRuntime::create_clash_of_patrols();
    let result = MissionRuntime::check_end_game(&state, &mission);
    assert_eq!(
        result,
        Some(GameOutcome::Victory(PlayerId::new(0))),
        "Tabled player should lose"
    );
}
