//! Source-linked rules tests for CP_Rules.md Section 2: Pre-Battle Sequence.
//!
//! Tests Combat Patrol setup rules and constraints:
//!   - Board dimensions: 44" x 30"
//!   - Maximum 5 battle rounds
//!   - Starting CP: 0 (Combat Patrol specific)
//!   - CP gain per round: 1
//!   - Battle-ready bonus: 10 VP
//!   - Reserve timing (Round 1 too early, Round 2+ allowed, Round 4+ too late)
//!   - Patrol squad validation
//!   - ScenarioLoader creates valid game states with correct initial conditions
//!
//! Source: CP_Rules.md Section 2 — "PRE-BATTLE SEQUENCE"

use wh40k_core_types::{
    BattleRound, BoardDimensions, FactionId, GameMode, GameOutcome,
    Inches, MissionId, Phase, PlayerId, VictoryPoints,
};
use wh40k_combat_patrol_rules::{CombatPatrolRules, ReserveTimingResult};
use wh40k_game_core::ScenarioLoader;

// ===========================================================================
// Board Dimensions — 44" x 30"
// ===========================================================================

/// Source: CP_Rules.md — "44\" x 30\" battlefield"
/// Board dimensions should be exactly 44" wide and 30" tall.
#[test]
fn cp_board_dimensions_44x30() {
    let dims = CombatPatrolRules::board_dimensions();
    assert_eq!(dims.width, Inches::from_inches(44), "Board width should be 44\"");
    assert_eq!(dims.height, Inches::from_inches(30), "Board height should be 30\"");
}

/// Source: CP_Rules.md — Board dimensions match the COMBAT_PATROL constant.
#[test]
fn cp_board_dimensions_match_constant() {
    let dims = CombatPatrolRules::board_dimensions();
    assert_eq!(dims, BoardDimensions::COMBAT_PATROL,
        "board_dimensions() should return the COMBAT_PATROL constant");
}

// ===========================================================================
// Maximum Battle Rounds — 5
// ===========================================================================

/// Source: CP_Rules.md — "The battle lasts five battle rounds"
#[test]
fn cp_max_battle_rounds_is_5() {
    assert_eq!(CombatPatrolRules::max_battle_rounds(), 5,
        "Combat Patrol should have exactly 5 battle rounds");
}

// ===========================================================================
// Starting CP — 0
// ===========================================================================

/// Source: CP_Rules.md — "you start the battle with 0 Command points"
#[test]
fn cp_starting_cp_is_zero() {
    let starting_cp = CombatPatrolRules::starting_cp();
    assert_eq!(starting_cp.value(), 0,
        "Combat Patrol starting CP should be 0");
}

// ===========================================================================
// CP Gain Per Round — 1
// ===========================================================================

/// Source: CP_Rules.md — "gain 1 Command point at the start of your Command phase"
#[test]
fn cp_gain_per_round_is_one() {
    let gain = CombatPatrolRules::max_cp_gain_per_round();
    assert_eq!(gain.value(), 1,
        "Standard CP gain per Command phase should be 1");
}

// ===========================================================================
// Battle-Ready Bonus — 10 VP
// ===========================================================================

/// Source: CP_Rules.md — "Battle Ready bonus of 10 Victory Points"
#[test]
fn cp_battle_ready_bonus_is_10vp() {
    let bonus = CombatPatrolRules::battle_ready_bonus();
    assert_eq!(bonus, VictoryPoints::new(10),
        "Battle-ready bonus should be exactly 10 VP");
}

// ===========================================================================
// Reserve Timing
// ===========================================================================

/// Source: CP_Rules.md — Reserves cannot arrive in battle round 1.
#[test]
fn cp_reserve_timing_round_1_too_early() {
    let result = CombatPatrolRules::check_reserve_timing(BattleRound::new(1));
    assert_eq!(result, ReserveTimingResult::TooEarly,
        "Reserves should not be allowed in BR1");
}

/// Source: CP_Rules.md — Reserves can arrive in battle round 2.
#[test]
fn cp_reserve_timing_round_2_allowed() {
    let result = CombatPatrolRules::check_reserve_timing(BattleRound::new(2));
    assert_eq!(result, ReserveTimingResult::Allowed,
        "Reserves should be allowed in BR2");
}

/// Source: CP_Rules.md — Reserves can arrive in battle round 3.
#[test]
fn cp_reserve_timing_round_3_allowed() {
    let result = CombatPatrolRules::check_reserve_timing(BattleRound::new(3));
    assert_eq!(result, ReserveTimingResult::Allowed,
        "Reserves should be allowed in BR3");
}

/// Source: CP_Rules.md — Reserves that haven't arrived by end of BR3 are destroyed.
/// Rounds 4 and 5 return TooLate.
#[test]
fn cp_reserve_timing_round_4_and_5_too_late() {
    let result_r4 = CombatPatrolRules::check_reserve_timing(BattleRound::new(4));
    assert_eq!(result_r4, ReserveTimingResult::TooLate,
        "Reserves should be TooLate in BR4");

    let result_r5 = CombatPatrolRules::check_reserve_timing(BattleRound::new(5));
    assert_eq!(result_r5, ReserveTimingResult::TooLate,
        "Reserves should be TooLate in BR5");
}

/// Source: CP_Rules.md — Invalid battle round numbers (0, 6+) return InvalidRound.
#[test]
fn cp_reserve_timing_invalid_rounds() {
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(0)),
        ReserveTimingResult::InvalidRound,
        "Round 0 should be invalid"
    );
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(6)),
        ReserveTimingResult::InvalidRound,
        "Round 6 should be invalid"
    );
    assert_eq!(
        CombatPatrolRules::check_reserve_timing(BattleRound::new(255)),
        ReserveTimingResult::InvalidRound,
        "Round 255 should be invalid"
    );
}

/// Source: CP_Rules.md — Reserve arrival rules struct has correct defaults.
#[test]
fn cp_reserve_arrival_rules_defaults() {
    let rules = CombatPatrolRules::reserve_arrival_rules();
    assert!(rules.no_arrival_round_1, "no_arrival_round_1 should be true");
    assert_eq!(rules.must_arrive_by_round, 3, "Reserves must arrive by BR3");
    assert_eq!(rules.min_distance_from_enemies_mils, 9000,
        "Minimum distance from enemies should be 9000 mils (9 inches)");
}

// ===========================================================================
// Patrol Squad Validation
// ===========================================================================

/// Source: CP_Rules.md — Patrol squad choices: index 0 or 1 are valid.
#[test]
fn cp_patrol_squad_choice_valid_indices() {
    assert!(
        CombatPatrolRules::is_patrol_squad_choice_valid(FactionId::new(0), 0),
        "Squad index 0 should be valid"
    );
    assert!(
        CombatPatrolRules::is_patrol_squad_choice_valid(FactionId::new(0), 1),
        "Squad index 1 should be valid"
    );
}

/// Source: CP_Rules.md — Patrol squad choice index 2+ is invalid.
#[test]
fn cp_patrol_squad_choice_invalid_index() {
    assert!(
        !CombatPatrolRules::is_patrol_squad_choice_valid(FactionId::new(0), 2),
        "Squad index 2 should be invalid"
    );
    assert!(
        !CombatPatrolRules::is_patrol_squad_choice_valid(FactionId::new(0), 99),
        "Squad index 99 should be invalid"
    );
}

// ===========================================================================
// ScenarioLoader — Initial Game State Validation
// ===========================================================================

/// Source: CP_Rules.md — ScenarioLoader produces a game state starting at BR1
/// in PreBattle phase with 0 CP for both players.
#[test]
fn scenario_loader_initial_state_br1_prebattle_0cp() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0), // Custodes
        FactionId::new(1), // World Eaters
        Some(MissionId::new(1)),
        [42u8; 32],
    );

    assert_eq!(state.battle_round, BattleRound::new(1),
        "Game should start at battle round 1");
    assert_eq!(state.current_phase, Phase::PreBattle,
        "Game should start in PreBattle phase");
    assert_eq!(state.players[0].cp.value(), 0,
        "Player A should start with 0 CP");
    assert_eq!(state.players[1].cp.value(), 0,
        "Player B should start with 0 CP");
    assert_eq!(state.game_outcome, GameOutcome::InProgress,
        "Game should be in progress");
    assert_eq!(state.game_mode, GameMode::CombatPatrol,
        "Game mode should be CombatPatrol");
}

/// Source: CP_Rules.md — ScenarioLoader assigns faction IDs to players.
#[test]
fn scenario_loader_assigns_faction_ids() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [0u8; 32],
    );

    assert_eq!(state.players[0].faction_id, Some(FactionId::new(0)),
        "Player A should have faction 0 (Custodes)");
    assert_eq!(state.players[1].faction_id, Some(FactionId::new(1)),
        "Player B should have faction 1 (World Eaters)");
}

/// Source: CP_Rules.md — ScenarioLoader creates units for both factions.
/// Both players should have at least one unit.
#[test]
fn scenario_loader_creates_units_for_both_players() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [0u8; 32],
    );

    let p0_units: Vec<_> = state.units.iter().filter(|u| u.owner == PlayerId::new(0)).collect();
    let p1_units: Vec<_> = state.units.iter().filter(|u| u.owner == PlayerId::new(1)).collect();

    assert!(!p0_units.is_empty(), "Player A should have units");
    assert!(!p1_units.is_empty(), "Player B should have units");
}

/// Source: CP_Rules.md — ScenarioLoader places 4 objectives for mission 1.
/// Board should have objectives after scenario loading.
#[test]
fn scenario_loader_places_objectives_for_mission() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [0u8; 32],
    );

    assert_eq!(state.board.objectives.len(), 4,
        "Mission 1 should have 4 objectives on the board");
}

/// Source: CP_Rules.md — ScenarioLoader sets up deployment config.
#[test]
fn scenario_loader_creates_deployment_config() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [0u8; 32],
    );

    assert!(state.deployment_config.is_some(),
        "Deployment config should be set after scenario loading");
}

/// Source: CP_Rules.md — ScenarioLoader stores the mission ID in scenario_id.
#[test]
fn scenario_loader_stores_mission_id() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(3)),
        [0u8; 32],
    );

    assert_eq!(state.scenario_id, Some(MissionId::new(3)),
        "scenario_id should match the requested mission");
}

/// Source: CP_Rules.md — ScenarioLoader initial VP for both players is 0.
#[test]
fn scenario_loader_initial_vp_zero() {
    let state = ScenarioLoader::load_scenario(
        FactionId::new(0),
        FactionId::new(1),
        Some(MissionId::new(1)),
        [0u8; 32],
    );

    assert_eq!(state.players[0].vp, VictoryPoints::ZERO,
        "Player A should start with 0 VP");
    assert_eq!(state.players[1].vp, VictoryPoints::ZERO,
        "Player B should start with 0 VP");
}

// ===========================================================================
// Step 5: Battle Formations — declare patrol squads
// ===========================================================================

/// Source: CP_Rules.md Section 2 — Step 5 (SelectPatrolSquad)
/// Rule: "Players declare their patrol squad choice during the pre-battle sequence."
/// Test: Verify the CombatPatrolSetup tracks patrol squad selection for both
///       players and advances to the Deployment step after both choose.
#[test]
fn cp_step5_battle_formations_declare_patrol_squads() {
    use wh40k_combat_patrol_rules::{CombatPatrolSetup, PreBattleStep};

    let mut setup = CombatPatrolSetup::new();

    // Steps 1-3: advance to SelectPatrolSquad
    setup.determine_attacker_defender(5, 3).unwrap();
    setup.select_enhancement(PlayerId::new(0), 0).unwrap();
    setup.select_enhancement(PlayerId::new(1), 0).unwrap();
    setup.select_secondary(PlayerId::new(0), 0).unwrap();
    setup.select_secondary(PlayerId::new(1), 0).unwrap();

    assert_eq!(
        setup.current_step(),
        PreBattleStep::SelectPatrolSquad,
        "After secondaries, should be at SelectPatrolSquad step"
    );

    // Player 0 selects squad index 0
    setup.select_patrol_squad(PlayerId::new(0), 0).unwrap();
    // Still waiting for player 1
    assert_eq!(
        setup.current_step(),
        PreBattleStep::SelectPatrolSquad,
        "Should still be at SelectPatrolSquad until both players choose"
    );

    // Player 1 selects squad index 1
    setup.select_patrol_squad(PlayerId::new(1), 1).unwrap();

    // Both chose => should advance to Deployment
    assert_eq!(
        setup.current_step(),
        PreBattleStep::Deployment,
        "After both players select patrol squads, should advance to Deployment"
    );

    // Verify the choices are recorded
    assert_eq!(setup.patrol_squad_choices[0], Some(0));
    assert_eq!(setup.patrol_squad_choices[1], Some(1));
}

// ===========================================================================
// Step 7: First turn determined by roll-off
// ===========================================================================

/// Source: CP_Rules.md Section 2 — Step 7 (DetermineFirstTurn)
/// Rule: "After deployment, players roll off. The winner chooses who takes
///        the first turn."
/// Test: Verify the CombatPatrolSetup determines first turn via roll-off and
///       supports overriding the choice.
#[test]
fn cp_step7_first_turn_determined_by_rolloff() {
    use wh40k_combat_patrol_rules::{CombatPatrolSetup, PreBattleStep};

    let mut setup = CombatPatrolSetup::new();

    // Run through all preceding steps
    setup.determine_attacker_defender(5, 3).unwrap();
    setup.select_enhancement(PlayerId::new(0), 0).unwrap();
    setup.select_enhancement(PlayerId::new(1), 0).unwrap();
    setup.select_secondary(PlayerId::new(0), 0).unwrap();
    setup.select_secondary(PlayerId::new(1), 0).unwrap();
    setup.select_patrol_squad(PlayerId::new(0), 0).unwrap();
    setup.select_patrol_squad(PlayerId::new(1), 0).unwrap();
    setup.finish_deployment().unwrap();

    assert_eq!(
        setup.current_step(),
        PreBattleStep::DetermineFirstTurn,
        "After deployment, should be at DetermineFirstTurn step"
    );

    // Player 1 wins the roll-off (6 vs 2)
    setup.determine_first_turn(2, 6).unwrap();

    assert!(setup.is_complete(), "Setup should be complete after first turn determination");
    assert_eq!(
        setup.first_turn_player,
        Some(PlayerId::new(1)),
        "Player 1 should go first (won the roll-off)"
    );

    // The winner can choose to let the opponent go first
    setup.set_first_turn_player(PlayerId::new(0));
    assert_eq!(
        setup.first_turn_player,
        Some(PlayerId::new(0)),
        "First turn player should be overridable"
    );

    // Tied roll-off should fail (must re-roll)
    let mut setup2 = CombatPatrolSetup::new();
    setup2.determine_attacker_defender(5, 3).unwrap();
    setup2.select_enhancement(PlayerId::new(0), 0).unwrap();
    setup2.select_enhancement(PlayerId::new(1), 0).unwrap();
    setup2.select_secondary(PlayerId::new(0), 0).unwrap();
    setup2.select_secondary(PlayerId::new(1), 0).unwrap();
    setup2.select_patrol_squad(PlayerId::new(0), 0).unwrap();
    setup2.select_patrol_squad(PlayerId::new(1), 0).unwrap();
    setup2.finish_deployment().unwrap();

    let tied_result = setup2.determine_first_turn(4, 4);
    assert!(
        tied_result.is_err(),
        "Tied roll-off should return an error (must re-roll)"
    );
}

// ===========================================================================
// Step 6: Deployment within deployment zone
// ===========================================================================

/// Source: CP_Rules.md Section 2 — Step 6 (Deployment)
/// Rule: "Players deploy their units alternately, starting with the attacker.
///        Units must be deployed within their deployment zone."
/// Test: Verify the DeploymentAlternation tracker correctly alternates
///       between players and the attacker deploys first.
#[test]
fn cp_step6_deployment_within_zone() {
    use wh40k_combat_patrol_rules::{CombatPatrolSetup, PreBattleStep};

    let mut setup = CombatPatrolSetup::new();

    // Advance to Deployment step; player 0 is attacker (rolled higher)
    setup.determine_attacker_defender(6, 2).unwrap();
    setup.select_enhancement(PlayerId::new(0), 0).unwrap();
    setup.select_enhancement(PlayerId::new(1), 0).unwrap();
    setup.select_secondary(PlayerId::new(0), 0).unwrap();
    setup.select_secondary(PlayerId::new(1), 0).unwrap();
    setup.select_patrol_squad(PlayerId::new(0), 0).unwrap();
    setup.select_patrol_squad(PlayerId::new(1), 0).unwrap();

    assert_eq!(
        setup.current_step(),
        PreBattleStep::Deployment,
        "Should be at Deployment step"
    );

    // Attacker (player 0) deploys first
    let deployer = setup.deployment_alternation().unwrap().next_deployer();
    assert_eq!(
        deployer,
        PlayerId::new(0),
        "Attacker should deploy first"
    );

    // After player 0 deploys, player 1 deploys next
    setup
        .deployment_alternation_mut()
        .unwrap()
        .record_deployment(PlayerId::new(0));
    let next_deployer = setup.deployment_alternation().unwrap().next_deployer();
    assert_eq!(
        next_deployer,
        PlayerId::new(1),
        "Defender should deploy second"
    );

    // After player 1 deploys, back to player 0
    setup
        .deployment_alternation_mut()
        .unwrap()
        .record_deployment(PlayerId::new(1));
    let next_next = setup.deployment_alternation().unwrap().next_deployer();
    assert_eq!(
        next_next,
        PlayerId::new(0),
        "Deployment should alternate back to attacker"
    );

    assert_eq!(
        setup.deployment_alternation().unwrap().total_deployed(),
        2,
        "Two units should have been deployed so far"
    );
}

// ===========================================================================
// ScenarioLoader: creates 4 objectives for standard missions
// ===========================================================================

/// Source: CP_Rules.md Section 2 — "Standard missions use 4 objective markers."
/// Test: Verify ScenarioLoader creates exactly 4 objectives for multiple
///       standard mission IDs, not just mission 1.
#[test]
fn scenario_loader_creates_4_objectives_for_standard_missions() {
    // Test multiple mission IDs to ensure 4 objectives is consistent
    for mission_id in [1, 2, 3] {
        let state = ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            Some(MissionId::new(mission_id)),
            [0u8; 32],
        );

        assert_eq!(
            state.board.objectives.len(),
            4,
            "Mission {} should have exactly 4 objectives",
            mission_id
        );

        // Verify objectives have distinct positions
        let positions: Vec<_> = state.board.objectives.iter().map(|o| o.position).collect();
        for i in 0..positions.len() {
            for j in (i + 1)..positions.len() {
                assert_ne!(
                    positions[i], positions[j],
                    "Mission {}: objectives {} and {} should have distinct positions",
                    mission_id, i, j
                );
            }
        }
    }
}
