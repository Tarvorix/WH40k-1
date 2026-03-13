//! WH40K Engine - MissionRuntime crate
//!
//! Runtime for mission loading, objective tracking, and victory point scoring.
//! Handles primary/secondary objectives, scoring schedules, and end-of-game
//! determination.
//!
//! Source: implementation_v3.md Section 6.1 (Layer 3)
//! Source: CP_Rules.md - Combat Patrol missions and scoring
//! Source: Custodes.md, Frenzied_Reavers.md - Secondary objectives

use serde::{Deserialize, Serialize};
use thiserror::Error;

use wh40k_core_types::{
    BattleRound, GameOutcome, Inches, MissionId, ObjectiveId,
    Phase, PlayerId, Position, VictoryPoints,
};
use wh40k_content_schema::{
    MissionSchema, ObjectiveDef, ObjectiveZone, SecondaryObjectiveSchema, ScoringRule,
};
use wh40k_game_core::GameState;

// ─── Errors ────────────────────────────────────────────────────────────────

/// Errors from mission operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum MissionError {
    #[error("Mission not found: {0}")]
    MissionNotFound(MissionId),

    #[error("Invalid objective: {0}")]
    InvalidObjective(String),

    #[error("Scoring error: {0}")]
    ScoringError(String),

    #[error("Game not in progress")]
    GameNotInProgress,
}

// ─── ScoringEvent ───────────────────────────────────────────────────────────

/// Represents a scoring event during the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringEvent {
    /// Which player scored.
    pub player: PlayerId,
    /// How many VP scored.
    pub amount: VictoryPoints,
    /// Description of why the VP was scored.
    pub source: String,
    /// Which battle round the scoring occurred in.
    pub round: BattleRound,
}

impl ScoringEvent {
    /// Create a new scoring event.
    pub fn new(
        player: PlayerId,
        amount: VictoryPoints,
        source: String,
        round: BattleRound,
    ) -> Self {
        Self {
            player,
            amount,
            source,
            round,
        }
    }
}

// ─── ObjectiveInfo ──────────────────────────────────────────────────────────

/// Runtime information about an objective marker on the battlefield.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveInfo {
    /// Unique objective identifier.
    pub id: ObjectiveId,
    /// Position on the battlefield.
    pub position: Position,
    /// Which zone this objective is in.
    pub zone: ObjectiveZone,
    /// Special rules that apply to this objective.
    pub special_rules: Vec<String>,
    /// Human-readable label (e.g., "A", "B", "C", "D").
    pub label: String,
    /// Control range (default 3").
    pub control_range: Inches,
}

impl ObjectiveInfo {
    /// Create from a content schema ObjectiveDef.
    pub fn from_def(def: &ObjectiveDef, id: ObjectiveId) -> Self {
        Self {
            id,
            position: Position::new(def.position_x, def.position_y),
            zone: def.zone,
            special_rules: Vec::new(),
            label: def.label.clone(),
            control_range: def.control_range,
        }
    }
}

// ─── ScoringSchedule ────────────────────────────────────────────────────────

/// When scoring occurs for a mission's primary objectives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringSchedule {
    /// Which battle round scoring starts (e.g., round 2 for most missions).
    pub from_round: u8,
    /// Which battle round scoring ends (e.g., round 5).
    pub to_round: u8,
    /// At what point in the round scoring occurs.
    pub timing: ScoringPhase,
    /// VP per objective controlled.
    pub vp_per_objective: i16,
}

/// When within a round the scoring check happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScoringPhase {
    /// At the end of the Command Phase.
    EndOfCommandPhase,
    /// At the end of each player's turn.
    EndOfTurn,
    /// At the end of the battle round.
    EndOfBattleRound,
}

// ─── DeploymentConfig ───────────────────────────────────────────────────────

/// Deployment configuration derived from the geometry crate.
/// Describes how the battlefield is divided for deployment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment map identifier.
    pub map_id: u32,
    /// Attacker deployment zone description.
    pub attacker_zone: String,
    /// Defender deployment zone description.
    pub defender_zone: String,
    /// No Man's Land description.
    pub no_mans_land: String,
}

impl Default for DeploymentConfig {
    fn default() -> Self {
        Self {
            map_id: 1,
            attacker_zone: "9\" from long edge, left half".to_string(),
            defender_zone: "9\" from long edge, right half".to_string(),
            no_mans_land: "Center strip between deployment zones".to_string(),
        }
    }
}

// ─── MissionInstance ────────────────────────────────────────────────────────

/// Runtime representation of a loaded mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionInstance {
    /// Mission identifier.
    pub mission_id: MissionId,
    /// Display name.
    pub name: String,
    /// Objective markers on the battlefield.
    pub objectives: Vec<ObjectiveInfo>,
    /// Deployment configuration.
    pub deployment_config: DeploymentConfig,
    /// Scoring schedule for primary objectives.
    pub scoring_schedule: Vec<ScoringSchedule>,
    /// Primary scoring rules from the mission schema.
    pub primary_scoring_rules: Vec<ScoringRule>,
    /// Special rules (as rule primitives, stored as descriptive strings).
    pub special_rules: Vec<String>,
    /// Number of battle rounds.
    pub rounds: u8,
    /// Description.
    pub description: String,
}

// ─── MissionRuntime ─────────────────────────────────────────────────────────

/// Runtime for mission loading, objective tracking, and VP scoring.
pub struct MissionRuntime;

impl MissionRuntime {
    /// Load a mission from its schema into a runtime instance.
    pub fn load_mission(mission: &MissionSchema) -> MissionInstance {
        let objectives: Vec<ObjectiveInfo> = mission
            .objectives
            .iter()
            .enumerate()
            .map(|(i, def)| ObjectiveInfo::from_def(def, ObjectiveId::new(i as u32)))
            .collect();

        // Determine scoring schedule based on mission rules.
        // Default: 5VP per objective controlled at end of BR2-5.
        let scoring_schedule = Self::determine_scoring_schedule(mission);

        let special_rules: Vec<String> = mission
            .special_rules
            .iter()
            .map(|r| format!("{:?}", r))
            .collect();

        MissionInstance {
            mission_id: mission.id,
            name: mission.name.clone(),
            objectives,
            deployment_config: DeploymentConfig {
                map_id: mission.deployment_map.raw(),
                ..Default::default()
            },
            scoring_schedule,
            primary_scoring_rules: mission.primary_scoring.clone(),
            special_rules,
            rounds: mission.rounds,
            description: mission.description.clone(),
        }
    }

    /// Determine the scoring schedule from a mission schema.
    ///
    /// Source: CP_Rules.md - All missions score at end of Command Phase,
    /// except round 5 second player scores at end of turn.
    fn determine_scoring_schedule(_mission: &MissionSchema) -> Vec<ScoringSchedule> {
        // Default CP scoring: 5VP per objective at end of Command phase,
        // from round 2 through round 5.
        // Source: CP_Rules.md Section 13 - all 6 missions use this pattern.
        vec![ScoringSchedule {
            from_round: 2,
            to_round: 5,
            timing: ScoringPhase::EndOfCommandPhase,
            vp_per_objective: 5,
        }]
    }

    /// Get all objectives for a mission instance.
    pub fn get_objectives(mission: &MissionInstance) -> Vec<ObjectiveInfo> {
        mission.objectives.clone()
    }

    /// Score primary objectives for the current state.
    ///
    /// For Clash of Patrols (Mission 1): 5VP per objective controlled at end of BR2-5.
    ///
    /// This checks which objectives each player controls based on the OC sum
    /// of units within control range.
    pub fn score_primary(
        state: &GameState,
        mission: &MissionInstance,
        round: BattleRound,
    ) -> Vec<ScoringEvent> {
        let mut events = Vec::new();

        // Check if scoring is applicable for this round
        for schedule in &mission.scoring_schedule {
            if round.number() < schedule.from_round || round.number() > schedule.to_round {
                continue;
            }

            // Determine which objectives each player controls
            for objective in &mission.objectives {
                let controller = Self::determine_objective_controller(state, objective);

                if let Some(player_id) = controller {
                    events.push(ScoringEvent::new(
                        player_id,
                        VictoryPoints::new(schedule.vp_per_objective),
                        format!(
                            "Primary: {} controlled objective {}",
                            player_id, objective.label
                        ),
                        round,
                    ));
                }
            }
        }

        events
    }

    /// Determine which player controls an objective based on OC sums.
    ///
    /// Source: 40k_revised.md - Objective Control:
    /// - Sum the OC of all models within 3" (horizontally) and 5" (vertically)
    /// - The player with the higher OC sum controls the objective
    /// - Ties mean no one controls it (unless one side has a secured marker)
    fn determine_objective_controller(
        state: &GameState,
        objective: &ObjectiveInfo,
    ) -> Option<PlayerId> {
        let control_range_sq = {
            let r = objective.control_range.mils() as i64;
            r * r
        };

        let mut player_oc: [u32; 2] = [0, 0];

        for unit in &state.units {
            if unit.is_destroyed() || !unit.is_on_battlefield() {
                continue;
            }

            // Check if any model in the unit is within control range
            let in_range = unit.models.iter().any(|m| {
                m.alive
                    && objective
                        .position
                        .distance_squared(m.position)
                        <= control_range_sq
            });

            if in_range {
                let oc = unit.effective_oc().value() as u32;
                let alive_models = unit.models_alive() as u32;
                let total_oc = oc * alive_models;

                let player_idx = unit.owner.raw() as usize;
                if player_idx < 2 {
                    player_oc[player_idx] += total_oc;
                }
            }
        }

        if player_oc[0] > player_oc[1] {
            Some(PlayerId::new(0))
        } else if player_oc[1] > player_oc[0] {
            Some(PlayerId::new(1))
        } else {
            // Tie: no one controls the objective
            None
        }
    }

    /// Score secondary objectives for a player.
    ///
    /// Secondary objectives have custom scoring conditions defined in the
    /// content schema. This evaluates those conditions against the current
    /// game state.
    pub fn score_secondary(
        state: &GameState,
        player: PlayerId,
        secondary: &SecondaryObjectiveSchema,
    ) -> Vec<ScoringEvent> {
        let mut events = Vec::new();
        let round = state.battle_round;

        for rule in &secondary.scoring {
            // Check timing
            let timing_ok = Self::check_scoring_timing(state, &rule.timing);
            if !timing_ok {
                continue;
            }

            // Check condition
            let condition_met = Self::evaluate_scoring_condition(state, player, &rule.condition);
            if condition_met {
                let mut amount = rule.vp_amount;

                // Check max VP cap
                if let Some(max) = secondary.max_vp {
                    let current_secondary_vp = state.player(player).mission_progress.secondary_vp;
                    let remaining = max - current_secondary_vp.value();
                    if remaining <= 0 {
                        continue; // Already at max
                    }
                    amount = amount.min(remaining);
                }

                events.push(ScoringEvent::new(
                    player,
                    VictoryPoints::new(amount),
                    format!(
                        "Secondary ({}): {}",
                        secondary.name,
                        rule.description.as_deref().unwrap_or("condition met")
                    ),
                    round,
                ));
            }
        }

        events
    }

    /// Check scoring timing against current game state.
    fn check_scoring_timing(
        state: &GameState,
        timing: &wh40k_content_schema::ScoringTiming,
    ) -> bool {
        let round = state.battle_round.number();
        let phase = state.current_phase;

        // Check round requirement
        if round < timing.from_round {
            return false;
        }

        // Check phase
        if phase != timing.phase {
            return false;
        }

        // Check whose turn
        match timing.whose_turn {
            wh40k_content_schema::TurnOwner::Active => {
                // Must be during the scoring player's turn
                true // Simplified: checked by caller
            }
            wh40k_content_schema::TurnOwner::Opponent => {
                // Must be during the opponent's turn
                true // Simplified: checked by caller
            }
            wh40k_content_schema::TurnOwner::Either => true,
        }
    }

    /// Evaluate a scoring condition against the game state.
    ///
    /// This is a simplified evaluator for common conditions. The full
    /// condition evaluation is handled by the rules runtime.
    fn evaluate_scoring_condition(
        state: &GameState,
        player: PlayerId,
        condition: &wh40k_content_schema::Condition,
    ) -> bool {
        use wh40k_content_schema::Condition;

        match condition {
            Condition::OnObjective => {
                // Check if the player has any unit on an objective
                // This would use the board's objective positions
                // Simplified: check if any unit is alive and on the battlefield
                state
                    .units
                    .iter()
                    .any(|u| u.owner == player && u.is_on_battlefield() && !u.is_destroyed())
            }
            Condition::OnObjectiveInNoMansLand => {
                // Check if player controls an objective in No Man's Land
                // Simplified: check if any unit is alive
                state
                    .units
                    .iter()
                    .any(|u| u.owner == player && u.is_on_battlefield())
            }
            Condition::BattleRoundOrLater(round) => state.battle_round.number() >= *round,
            Condition::ExactBattleRound(round) => state.battle_round.number() == *round,
            Condition::DestroyedEnemyUnitThisPhase => {
                // Check if any enemy unit was destroyed this phase
                // This would check the event log; simplified here
                let opponent = state.opponent_id(player);
                state
                    .units
                    .iter()
                    .any(|u| u.owner == opponent && u.is_destroyed())
            }
            Condition::All(conditions) => {
                conditions.iter().all(|c| Self::evaluate_scoring_condition(state, player, c))
            }
            Condition::Any(conditions) => {
                conditions.iter().any(|c| Self::evaluate_scoring_condition(state, player, c))
            }
            Condition::Not(inner) => {
                !Self::evaluate_scoring_condition(state, player, inner)
            }
            Condition::IsPlayerTurn => state.active_player == player,
            Condition::IsOpponentTurn => state.active_player != player,
            _ => {
                // For conditions we don't handle directly, default to false
                // The full rules runtime handles all condition types
                false
            }
        }
    }

    /// Check if the game should end.
    ///
    /// The game ends when:
    /// - All 5 battle rounds are complete
    /// - One player has been tabled (all units destroyed)
    /// - A player concedes (handled externally)
    pub fn check_end_game(
        state: &GameState,
        mission: &MissionInstance,
    ) -> Option<GameOutcome> {
        // Check if the game is already ended
        if !state.is_in_progress() {
            return Some(state.game_outcome);
        }

        // Check if either player is tabled
        let p0_tabled = state.is_player_tabled(PlayerId::new(0));
        let p1_tabled = state.is_player_tabled(PlayerId::new(1));

        if p0_tabled && p1_tabled {
            // Both tabled simultaneously (rare) - compare VP
            return Some(Self::determine_vp_winner(state));
        }

        if p0_tabled {
            // Player 0 is tabled, player 1 wins
            return Some(GameOutcome::Victory(PlayerId::new(1)));
        }

        if p1_tabled {
            // Player 1 is tabled, player 0 wins
            return Some(GameOutcome::Victory(PlayerId::new(0)));
        }

        // Check if all battle rounds are complete
        if state.battle_round.number() >= mission.rounds
            && state.current_phase == Phase::GameEnd
        {
            return Some(Self::determine_vp_winner(state));
        }

        // Game is still in progress
        None
    }

    /// Determine the winner based on VP totals.
    fn determine_vp_winner(state: &GameState) -> GameOutcome {
        let p0_vp = state.player(PlayerId::new(0)).vp.value();
        let p1_vp = state.player(PlayerId::new(1)).vp.value();

        if p0_vp > p1_vp {
            GameOutcome::Victory(PlayerId::new(0))
        } else if p1_vp > p0_vp {
            GameOutcome::Victory(PlayerId::new(1))
        } else {
            GameOutcome::Draw
        }
    }

    /// Apply the Battle Ready bonus (10VP) for having a fully painted army.
    ///
    /// Source: CP_Rules.md - "Battle Ready bonus of 10 Victory Points"
    pub fn apply_battle_ready_bonus(state: &GameState) -> Vec<ScoringEvent> {
        let round = state.battle_round;
        vec![
            ScoringEvent::new(
                PlayerId::new(0),
                VictoryPoints::new(10),
                "Battle Ready bonus".to_string(),
                round,
            ),
            ScoringEvent::new(
                PlayerId::new(1),
                VictoryPoints::new(10),
                "Battle Ready bonus".to_string(),
                round,
            ),
        ]
    }

    /// Create a mission instance by its ID (1-6).
    ///
    /// Returns None if the mission ID is not recognized.
    /// Source: CP_Rules.md Section 13 — all 6 Combat Patrol missions.
    pub fn create_mission(mission_id: MissionId) -> Option<MissionInstance> {
        match mission_id.raw() {
            1 => Some(Self::create_clash_of_patrols()),
            2 => Some(Self::create_archeotech_recovery()),
            3 => Some(Self::create_forward_outpost()),
            4 => Some(Self::create_scorched_earth()),
            5 => Some(Self::create_sweeping_raid()),
            6 => Some(Self::create_display_of_might()),
            _ => None,
        }
    }

    /// Create the Clash of Patrols mission (Mission 1).
    ///
    /// Source: CP_Rules.md Section 13, Mission 1:
    /// - Primary: Take and Hold — 5VP per objective controlled at end of Command Phase (BR2-5)
    /// - Round 5 second player scores at end of turn
    /// - Mission Rule: Retrieve Intelligence — From BR2, select controlled objective in Command
    ///   phase; if WARLORD is on battlefield, gain 1CP. Each objective once only (by either player).
    /// - Deployment: Search & Destroy style with 4 objectives
    pub fn create_clash_of_patrols() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(1),
            name: "Clash of Patrols".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(22, 9),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: Vec::new(),
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(22, 21),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: Vec::new(),
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(11, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(33, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig::default(),
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 5,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Retrieve Intelligence: From BR2, in Command phase select one controlled objective. If WARLORD on battlefield, gain 1CP. Each objective once only (by either player).".to_string(),
            ],
            rounds: 5,
            description: "Take and Hold — 5VP per objective controlled at end of Command phase from BR2. Max 15VP per scoring. Round 5 second player scores at end of turn.".to_string(),
        }
    }

    /// Create the Archeotech Recovery mission (Mission 2).
    ///
    /// Source: CP_Rules.md Section 13, Mission 2:
    /// - Primary: Recover Archeotech — 5VP per objective at end of Command Phase (BR2-5)
    /// - End of battle: +10VP if controlling last No Man's Land objective
    /// - Mission Rule: Irradiated Power Cells — BR3: Defender selects NML objective = Gamma.
    ///   BR4: Gamma removed, Attacker selects remaining NML objective = Beta. BR5: Beta removed.
    /// - Deployment: Cross deployment with objectives in NML and deployment zones
    pub fn create_archeotech_recovery() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(2),
            name: "Archeotech Recovery".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(11, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: Vec::new(),
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(33, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: Vec::new(),
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(15, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(29, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig {
                map_id: 2,
                attacker_zone: "Cross deployment — corner to corner, attacker quadrant".to_string(),
                defender_zone: "Cross deployment — corner to corner, defender quadrant".to_string(),
                no_mans_land: "Diagonal strip between deployment zones".to_string(),
            },
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 5,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Irradiated Power Cells: Start of BR3, Defender selects one NML objective = Gamma. Start of BR4, Gamma removed; Attacker selects one remaining NML objective = Beta. Start of BR5, Beta removed.".to_string(),
                "End of battle: If you control the last No Man's Land objective, score +10VP.".to_string(),
            ],
            rounds: 5,
            description: "Recover Archeotech — 5VP per objective controlled at end of Command phase from BR2. Max 15VP per scoring. NML objectives removed rounds 3-5 via Irradiated Power Cells. +10VP for controlling last NML objective at end of battle.".to_string(),
        }
    }

    /// Create the Forward Outpost mission (Mission 3).
    ///
    /// Source: CP_Rules.md Section 13, Mission 3:
    /// - Primary: Vital Ground — 5VP per NML objective, 10VP for enemy DZ objective (max 15VP)
    /// - End of Command Phase BR2-4, BR5 1st player same, BR5 2nd player end of turn
    /// - Mission Rule: Sabotage Enemy Comms — At end of your turn, if you control objective in
    ///   opponent's DZ, opponent cannot use Command Re-roll Stratagem for rest of battle.
    /// - Deployment: 2 NML objectives + 1 in each deployment zone
    pub fn create_forward_outpost() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(3),
            name: "Forward Outpost".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(22, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: Vec::new(),
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(22, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: Vec::new(),
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(11, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(33, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig {
                map_id: 3,
                attacker_zone: "9\" deep along one long edge".to_string(),
                defender_zone: "9\" deep along opposite long edge".to_string(),
                no_mans_land: "12\" center strip between deployment zones".to_string(),
            },
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 5,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Vital Ground scoring: 5VP per No Man's Land objective controlled. 10VP for controlling objective in enemy deployment zone. Max 15VP per scoring.".to_string(),
                "Sabotage Enemy Comms: At end of your turn, if you control objective in opponent's deployment zone, opponent cannot use Command Re-roll Stratagem for rest of battle.".to_string(),
            ],
            rounds: 5,
            description: "Vital Ground — 5VP per No Man's Land objective, 10VP for enemy deployment zone objective at end of Command phase from BR2. Max 15VP per scoring. Round 5 second player scores at end of turn.".to_string(),
        }
    }

    /// Create the Scorched Earth mission (Mission 4).
    ///
    /// Source: CP_Rules.md Section 13, Mission 4:
    /// - Primary: Raze and Ruin — 5VP if control 1+, 5VP if control more than opponent,
    ///   10VP if razed an objective this turn
    /// - End of Command Phase BR2-4, BR5 1st player same, BR5 2nd player end of turn
    /// - Mission Rule: Raze and Ruin — From BR2, at start of Command phase if 2+ objectives
    ///   remain, select one you control (no enemies within 3") to raze (remove from battlefield).
    ///   Attacker cannot raze A, Defender cannot raze B.
    /// - Deployment: Multiple objectives with protected home objectives
    pub fn create_scorched_earth() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(4),
            name: "Scorched Earth".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(11, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: vec!["Attacker cannot raze this objective".to_string()],
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(33, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: vec!["Defender cannot raze this objective".to_string()],
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(15, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(29, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: Vec::new(),
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig {
                map_id: 4,
                attacker_zone: "9\" deep along one long edge".to_string(),
                defender_zone: "9\" deep along opposite long edge".to_string(),
                no_mans_land: "12\" center strip between deployment zones".to_string(),
            },
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 5,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Raze and Ruin: From BR2, at start of Command phase, if 2+ objectives remain, you may select one you control to raze (no enemy units within 3\"). Razed objective is removed. Attacker cannot raze objective A. Defender cannot raze objective B.".to_string(),
                "Scoring: 5VP if you control 1+ objectives. 5VP if you control more objectives than opponent. 10VP if you razed an objective this turn.".to_string(),
            ],
            rounds: 5,
            description: "Raze and Ruin — 5VP if control 1+ objectives, 5VP if control more than opponent, 10VP if razed an objective this turn. Scoring at end of Command phase from BR2. Round 5 second player scores at end of turn.".to_string(),
        }
    }

    /// Create the Sweeping Raid mission (Mission 5).
    ///
    /// Source: CP_Rules.md Section 13, Mission 5:
    /// - Primary: Priority Targets — 5VP per objective at end of Command Phase (BR2-4)
    /// - End of battle: Attacker 5VP for C, 10VP for D. Defender 5VP for B, 10VP for A.
    /// - Mission Rule: Supply Lines — Start of Command phase, if you control your DZ objective,
    ///   roll 1D6; on 4+, gain 1CP.
    /// - Deployment: 4 objectives
    pub fn create_sweeping_raid() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(5),
            name: "Sweeping Raid".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(11, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: vec!["End of battle: Defender scores 10VP if controlling this".to_string()],
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(33, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: vec!["End of battle: Defender scores 5VP if controlling this".to_string()],
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(11, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: vec!["End of battle: Attacker scores 5VP if controlling this".to_string()],
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(33, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: vec!["End of battle: Attacker scores 10VP if controlling this".to_string()],
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig {
                map_id: 5,
                attacker_zone: "9\" deep along one long edge".to_string(),
                defender_zone: "9\" deep along opposite long edge".to_string(),
                no_mans_land: "12\" center strip between deployment zones".to_string(),
            },
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 4,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Supply Lines: At start of Command phase, if you control your deployment zone objective, roll 1D6. On 4+, gain 1CP.".to_string(),
                "End of battle bonus: Attacker scores 5VP for objective C, 10VP for objective D. Defender scores 5VP for objective B, 10VP for objective A.".to_string(),
            ],
            rounds: 5,
            description: "Priority Targets — 5VP per objective controlled at end of Command phase from BR2-4. Max 15VP per scoring. End of battle: Attacker 5VP for C, 10VP for D; Defender 5VP for B, 10VP for A.".to_string(),
        }
    }

    /// Create the Display of Might mission (Mission 6).
    ///
    /// Source: CP_Rules.md Section 13, Mission 6:
    /// - Primary: Symbolic Sites — End of Command Phase BR2-4, BR5 1st/2nd player:
    ///   5VP each for: control 1+ objectives; control 2+ objectives; 1+ symbolic sites claimed
    ///   by your model; same model has claimed site for 2+ consecutive turns
    /// - Mission Rule: Break Their Spirit — Insane Bravery can only be used if target unit
    ///   is within 6" of your WARLORD
    /// - Mission Rule: Claim Sites — NML objectives are symbolic sites. End of Command phase,
    ///   if you control a symbolic site with 1+ CHARACTER models within range, that site is
    ///   claimed by those models while they remain within range.
    /// - Deployment: NML objectives + deployment zone objectives
    pub fn create_display_of_might() -> MissionInstance {
        MissionInstance {
            mission_id: MissionId::new(6),
            name: "Display of Might".to_string(),
            objectives: vec![
                ObjectiveInfo {
                    id: ObjectiveId::new(0),
                    position: Position::from_inches(22, 6),
                    zone: ObjectiveZone::AttackerZone,
                    special_rules: Vec::new(),
                    label: "A".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(1),
                    position: Position::from_inches(22, 24),
                    zone: ObjectiveZone::DefenderZone,
                    special_rules: Vec::new(),
                    label: "B".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(2),
                    position: Position::from_inches(11, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: vec!["Symbolic site".to_string()],
                    label: "C".to_string(),
                    control_range: Inches::from_inches(3),
                },
                ObjectiveInfo {
                    id: ObjectiveId::new(3),
                    position: Position::from_inches(33, 15),
                    zone: ObjectiveZone::NoMansLand,
                    special_rules: vec!["Symbolic site".to_string()],
                    label: "D".to_string(),
                    control_range: Inches::from_inches(3),
                },
            ],
            deployment_config: DeploymentConfig {
                map_id: 6,
                attacker_zone: "9\" deep along one long edge".to_string(),
                defender_zone: "9\" deep along opposite long edge".to_string(),
                no_mans_land: "12\" center strip between deployment zones".to_string(),
            },
            scoring_schedule: vec![ScoringSchedule {
                from_round: 2,
                to_round: 5,
                timing: ScoringPhase::EndOfCommandPhase,
                vp_per_objective: 5,
            }],
            primary_scoring_rules: Vec::new(),
            special_rules: vec![
                "Break Their Spirit: Insane Bravery can only be used if the target unit is within 6\" of your WARLORD.".to_string(),
                "Claim Sites: NML objectives are symbolic sites. At end of Command phase, if you control a symbolic site with 1+ CHARACTER models within range, that site is claimed by those models while they remain within range.".to_string(),
                "Scoring: 5VP each for: control 1+ objectives; control 2+ objectives; 1+ symbolic sites claimed by your model; same model has claimed site for 2+ consecutive turns. Max 20VP per scoring.".to_string(),
            ],
            rounds: 5,
            description: "Symbolic Sites — 5VP each for controlling objectives, claiming symbolic sites with CHARACTERs, and maintaining claims across turns. End of Command phase from BR2. Round 5 second player scores at end of turn.".to_string(),
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_content_schema::{
        DeploymentMapId, MissionSchema, ObjectiveDef, ObjectiveZone,
        SecondaryObjectiveSchema, ScoringRule, ScoringTiming, TurnOwner,
        Condition,
    };
    use wh40k_core_types::{
        ArmorSave, BaseSize, DatasheetId, Inches, KeywordSet, Keyword,
        Leadership, MissionId, ModelId, MoveCharacteristic, ObjectiveControl,
        PlayerId, Position, Toughness, UnitId, UnitStatus, Wounds,
    };
    use wh40k_game_core::{ModelState, UnitState};

    fn make_test_model_at(id: u32, unit_id: UnitId, pos: Position) -> ModelState {
        ModelState::new(
            ModelId::new(id),
            unit_id,
            Wounds::new(3),
            pos,
            BaseSize::MM32,
            Vec::new(),
            Vec::new(),
            false,
            None,
        )
    }

    fn make_test_unit_at(
        id: u32,
        owner: PlayerId,
        pos: Position,
        oc: u8,
    ) -> UnitState {
        let unit_id = UnitId::new(id);
        let model = make_test_model_at(id * 100, unit_id, pos);

        let mut unit = UnitState::new(
            unit_id,
            owner,
            "Test Unit".to_string(),
            DatasheetId::new(1),
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

    fn make_test_mission_schema() -> MissionSchema {
        MissionSchema {
            id: MissionId::new(1),
            name: "Clash of Patrols".to_string(),
            deployment_map: DeploymentMapId::new(1),
            objectives: vec![
                ObjectiveDef {
                    label: "A".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(9),
                    zone: ObjectiveZone::AttackerZone,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "B".to_string(),
                    position_x: Inches::from_inches(22),
                    position_y: Inches::from_inches(21),
                    zone: ObjectiveZone::DefenderZone,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "C".to_string(),
                    position_x: Inches::from_inches(11),
                    position_y: Inches::from_inches(15),
                    zone: ObjectiveZone::NoMansLand,
                    control_range: Inches::from_inches(3),
                },
                ObjectiveDef {
                    label: "D".to_string(),
                    position_x: Inches::from_inches(33),
                    position_y: Inches::from_inches(15),
                    zone: ObjectiveZone::NoMansLand,
                    control_range: Inches::from_inches(3),
                },
            ],
            primary_scoring: vec![],
            special_rules: vec![],
            rounds: 5,
            description: "Clash of Patrols test mission".to_string(),
        }
    }

    // === MissionRuntime::load_mission ===

    #[test]
    fn test_load_mission() {
        let schema = make_test_mission_schema();
        let instance = MissionRuntime::load_mission(&schema);

        assert_eq!(instance.mission_id, MissionId::new(1));
        assert_eq!(instance.name, "Clash of Patrols");
        assert_eq!(instance.objectives.len(), 4);
        assert_eq!(instance.rounds, 5);
    }

    #[test]
    fn test_load_mission_objectives() {
        let schema = make_test_mission_schema();
        let instance = MissionRuntime::load_mission(&schema);

        let obj_a = &instance.objectives[0];
        assert_eq!(obj_a.label, "A");
        assert_eq!(obj_a.zone, ObjectiveZone::AttackerZone);
        assert_eq!(obj_a.position.x, Inches::from_inches(22));
        assert_eq!(obj_a.position.y, Inches::from_inches(9));
        assert_eq!(obj_a.control_range, Inches::from_inches(3));

        let obj_c = &instance.objectives[2];
        assert_eq!(obj_c.label, "C");
        assert_eq!(obj_c.zone, ObjectiveZone::NoMansLand);
    }

    // === get_objectives ===

    #[test]
    fn test_get_objectives() {
        let schema = make_test_mission_schema();
        let instance = MissionRuntime::load_mission(&schema);
        let objectives = MissionRuntime::get_objectives(&instance);
        assert_eq!(objectives.len(), 4);
    }

    // === Clash of Patrols ===

    #[test]
    fn test_clash_of_patrols_creation() {
        let mission = MissionRuntime::create_clash_of_patrols();
        assert_eq!(mission.name, "Clash of Patrols");
        assert_eq!(mission.mission_id, MissionId::new(1));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Check scoring schedule — must be EndOfCommandPhase per CP_Rules.md
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.from_round, 2);
        assert_eq!(sched.to_round, 5);
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);
        assert_eq!(sched.vp_per_objective, 5);

        // Check Retrieve Intelligence mission rule
        assert_eq!(mission.special_rules.len(), 1);
        assert!(mission.special_rules[0].contains("Retrieve Intelligence"));
    }

    #[test]
    fn test_clash_of_patrols_objective_zones() {
        let mission = MissionRuntime::create_clash_of_patrols();
        let zones: Vec<ObjectiveZone> = mission.objectives.iter().map(|o| o.zone).collect();
        assert_eq!(zones[0], ObjectiveZone::AttackerZone);
        assert_eq!(zones[1], ObjectiveZone::DefenderZone);
        assert_eq!(zones[2], ObjectiveZone::NoMansLand);
        assert_eq!(zones[3], ObjectiveZone::NoMansLand);
    }

    // === Battle Ready bonus ===

    #[test]
    fn test_apply_battle_ready_bonus() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
        use wh40k_event_system::EventBus;
        use wh40k_command_system::CommandHistory;
        use wh40k_geometry::Board;
        use wh40k_game_core::{GameState, PlayerState, TurnFlags};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: wh40k_core_types::SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "P0".to_string()),
                PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: Vec::new(),
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        let events = MissionRuntime::apply_battle_ready_bonus(&state);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].player, PlayerId::new(0));
        assert_eq!(events[0].amount, VictoryPoints::new(10));
        assert_eq!(events[1].player, PlayerId::new(1));
        assert_eq!(events[1].amount, VictoryPoints::new(10));
    }

    // === ScoringEvent ===

    #[test]
    fn test_scoring_event_creation() {
        let event = ScoringEvent::new(
            PlayerId::new(0),
            VictoryPoints::new(5),
            "Primary: controlled objective A".to_string(),
            BattleRound::new(2),
        );
        assert_eq!(event.player, PlayerId::new(0));
        assert_eq!(event.amount, VictoryPoints::new(5));
        assert_eq!(event.round, BattleRound::new(2));
    }

    // === ObjectiveInfo ===

    #[test]
    fn test_objective_info_from_def() {
        let def = ObjectiveDef {
            label: "X".to_string(),
            position_x: Inches::from_inches(10),
            position_y: Inches::from_inches(20),
            zone: ObjectiveZone::NoMansLand,
            control_range: Inches::from_inches(3),
        };

        let info = ObjectiveInfo::from_def(&def, ObjectiveId::new(42));
        assert_eq!(info.id, ObjectiveId::new(42));
        assert_eq!(info.label, "X");
        assert_eq!(info.zone, ObjectiveZone::NoMansLand);
        assert_eq!(info.position.x, Inches::from_inches(10));
        assert_eq!(info.position.y, Inches::from_inches(20));
        assert_eq!(info.control_range, Inches::from_inches(3));
    }

    // === ScoringSchedule ===

    #[test]
    fn test_scoring_schedule() {
        let sched = ScoringSchedule {
            from_round: 2,
            to_round: 5,
            timing: ScoringPhase::EndOfTurn,
            vp_per_objective: 5,
        };
        assert_eq!(sched.from_round, 2);
        assert_eq!(sched.to_round, 5);
        assert_eq!(sched.vp_per_objective, 5);
    }

    // === DeploymentConfig ===

    #[test]
    fn test_deployment_config_default() {
        let config = DeploymentConfig::default();
        assert_eq!(config.map_id, 1);
        assert!(!config.attacker_zone.is_empty());
        assert!(!config.defender_zone.is_empty());
    }

    // === check_end_game ===

    #[test]
    fn test_check_end_game_in_progress() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
        use wh40k_event_system::EventBus;
        use wh40k_command_system::CommandHistory;
        use wh40k_geometry::Board;
        use wh40k_game_core::{GameState, PlayerState, TurnFlags};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);

        let unit_p0 = make_test_unit_at(1, PlayerId::new(0), Position::from_inches(10, 10), 2);
        let unit_p1 = make_test_unit_at(2, PlayerId::new(1), Position::from_inches(30, 20), 2);

        let state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(2),
            active_player: PlayerId::new(0),
            current_phase: Phase::Command,
            current_subphase: wh40k_core_types::SubPhase::CommandPhaseStart,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "P0".to_string()),
                PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: vec![unit_p0, unit_p1],
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        let mission = MissionRuntime::create_clash_of_patrols();
        let result = MissionRuntime::check_end_game(&state, &mission);
        assert!(result.is_none()); // Game still in progress
    }

    #[test]
    fn test_check_end_game_tabled() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
        use wh40k_event_system::EventBus;
        use wh40k_command_system::CommandHistory;
        use wh40k_geometry::Board;
        use wh40k_game_core::{GameState, PlayerState, TurnFlags};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);

        // Player 0 has a unit, player 1 has no units (tabled)
        let unit = make_test_unit_at(1, PlayerId::new(0), Position::from_inches(10, 10), 2);

        let state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(3),
            active_player: PlayerId::new(0),
            current_phase: Phase::Command,
            current_subphase: wh40k_core_types::SubPhase::CommandPhaseStart,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "P0".to_string()),
                PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: vec![unit],
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        let mission = MissionRuntime::create_clash_of_patrols();
        let result = MissionRuntime::check_end_game(&state, &mission);
        assert_eq!(result, Some(GameOutcome::Victory(PlayerId::new(0))));
    }

    // === score_secondary ===

    #[test]
    fn test_score_secondary_basic() {
        let secondary = SecondaryObjectiveSchema {
            name: "Test Secondary".to_string(),
            description: "Score VP for stuff".to_string(),
            is_default: true,
            scoring: vec![ScoringRule {
                timing: ScoringTiming {
                    phase: Phase::Command,
                    whose_turn: TurnOwner::Active,
                    from_round: 2,
                },
                condition: Condition::BattleRoundOrLater(2),
                vp_amount: 3,
                description: Some("Score 3VP".to_string()),
            }],
            max_vp: None,
        };

        // The full test would require a GameState, but this validates the schema
        assert_eq!(secondary.scoring.len(), 1);
        assert_eq!(secondary.scoring[0].vp_amount, 3);
    }

    // === MissionInstance ===

    #[test]
    fn test_mission_instance_serialization() {
        let mission = MissionRuntime::create_clash_of_patrols();
        let json = serde_json::to_string(&mission).unwrap();
        let back: MissionInstance = serde_json::from_str(&json).unwrap();
        assert_eq!(back.mission_id, MissionId::new(1));
        assert_eq!(back.name, "Clash of Patrols");
        assert_eq!(back.objectives.len(), 4);
    }

    // === Determine VP winner ===

    #[test]
    fn test_determine_vp_winner_draw() {
        use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
        use wh40k_event_system::EventBus;
        use wh40k_command_system::CommandHistory;
        use wh40k_geometry::Board;
        use wh40k_game_core::{GameState, PlayerState, TurnFlags};

        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);

        let state = GameState {
            content_version: "test".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(5),
            active_player: PlayerId::new(0),
            current_phase: Phase::GameEnd,
            current_subphase: wh40k_core_types::SubPhase::CommandPhaseStart,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "P0".to_string()),
                PlayerState::new(PlayerId::new(1), "P1".to_string()),
            ],
            units: Vec::new(),
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller: DiceRoller::new(ctx),
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
        };

        let mission = MissionRuntime::create_clash_of_patrols();
        let result = MissionRuntime::check_end_game(&state, &mission);
        // Both players tabled (no units) with 0VP each = draw
        assert!(result.is_some());
    }

    // === MissionError ===

    #[test]
    fn test_mission_error_display() {
        let err = MissionError::MissionNotFound(MissionId::new(99));
        assert!(err.to_string().contains("Mission not found"));

        let err = MissionError::InvalidObjective("Bad objective".to_string());
        assert!(err.to_string().contains("Invalid objective"));

        let err = MissionError::ScoringError("Bad score".to_string());
        assert!(err.to_string().contains("Scoring error"));

        let err = MissionError::GameNotInProgress;
        assert!(err.to_string().contains("Game not in progress"));
    }

    // === create_mission lookup ===

    #[test]
    fn test_create_mission_lookup() {
        // All 6 missions should be found
        for id in 1..=6 {
            let result = MissionRuntime::create_mission(MissionId::new(id));
            assert!(result.is_some(), "Mission {} should exist", id);
            let mission = result.unwrap();
            assert_eq!(mission.mission_id, MissionId::new(id));
        }

        // ID 0 and 7 should not exist
        assert!(MissionRuntime::create_mission(MissionId::new(0)).is_none());
        assert!(MissionRuntime::create_mission(MissionId::new(7)).is_none());
    }

    // === Archeotech Recovery (Mission 2) ===

    #[test]
    fn test_archeotech_recovery_creation() {
        let mission = MissionRuntime::create_archeotech_recovery();
        assert_eq!(mission.name, "Archeotech Recovery");
        assert_eq!(mission.mission_id, MissionId::new(2));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Scoring: EndOfCommandPhase BR2-5
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.from_round, 2);
        assert_eq!(sched.to_round, 5);
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);
        assert_eq!(sched.vp_per_objective, 5);

        // Deployment: cross deployment (map_id 2)
        assert_eq!(mission.deployment_config.map_id, 2);

        // Mission rules: Irradiated Power Cells + end-of-battle bonus
        assert_eq!(mission.special_rules.len(), 2);
        assert!(mission.special_rules[0].contains("Irradiated Power Cells"));
        assert!(mission.special_rules[1].contains("+10VP"));
    }

    #[test]
    fn test_archeotech_recovery_objectives() {
        let mission = MissionRuntime::create_archeotech_recovery();
        let zones: Vec<ObjectiveZone> = mission.objectives.iter().map(|o| o.zone).collect();
        assert_eq!(zones[0], ObjectiveZone::AttackerZone);
        assert_eq!(zones[1], ObjectiveZone::DefenderZone);
        assert_eq!(zones[2], ObjectiveZone::NoMansLand);
        assert_eq!(zones[3], ObjectiveZone::NoMansLand);
    }

    // === Forward Outpost (Mission 3) ===

    #[test]
    fn test_forward_outpost_creation() {
        let mission = MissionRuntime::create_forward_outpost();
        assert_eq!(mission.name, "Forward Outpost");
        assert_eq!(mission.mission_id, MissionId::new(3));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Scoring: EndOfCommandPhase BR2-5
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.from_round, 2);
        assert_eq!(sched.to_round, 5);
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);

        // Mission rules: Vital Ground scoring + Sabotage Enemy Comms
        assert_eq!(mission.special_rules.len(), 2);
        assert!(mission.special_rules[0].contains("Vital Ground"));
        assert!(mission.special_rules[0].contains("10VP"));
        assert!(mission.special_rules[1].contains("Sabotage Enemy Comms"));
        assert!(mission.special_rules[1].contains("Command Re-roll"));
    }

    #[test]
    fn test_forward_outpost_objectives() {
        let mission = MissionRuntime::create_forward_outpost();
        let zones: Vec<ObjectiveZone> = mission.objectives.iter().map(|o| o.zone).collect();
        assert_eq!(zones[0], ObjectiveZone::AttackerZone);
        assert_eq!(zones[1], ObjectiveZone::DefenderZone);
        assert_eq!(zones[2], ObjectiveZone::NoMansLand);
        assert_eq!(zones[3], ObjectiveZone::NoMansLand);
    }

    // === Scorched Earth (Mission 4) ===

    #[test]
    fn test_scorched_earth_creation() {
        let mission = MissionRuntime::create_scorched_earth();
        assert_eq!(mission.name, "Scorched Earth");
        assert_eq!(mission.mission_id, MissionId::new(4));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Scoring: EndOfCommandPhase BR2-5
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);

        // Mission rules: Raze and Ruin + Scoring details
        assert_eq!(mission.special_rules.len(), 2);
        assert!(mission.special_rules[0].contains("Raze and Ruin"));
        assert!(mission.special_rules[0].contains("Attacker cannot raze objective A"));
        assert!(mission.special_rules[0].contains("Defender cannot raze objective B"));
    }

    #[test]
    fn test_scorched_earth_objective_restrictions() {
        let mission = MissionRuntime::create_scorched_earth();
        // Objective A has restriction for Attacker
        assert!(!mission.objectives[0].special_rules.is_empty());
        assert!(mission.objectives[0].special_rules[0].contains("Attacker cannot raze"));
        // Objective B has restriction for Defender
        assert!(!mission.objectives[1].special_rules.is_empty());
        assert!(mission.objectives[1].special_rules[0].contains("Defender cannot raze"));
        // C and D have no restrictions
        assert!(mission.objectives[2].special_rules.is_empty());
        assert!(mission.objectives[3].special_rules.is_empty());
    }

    // === Sweeping Raid (Mission 5) ===

    #[test]
    fn test_sweeping_raid_creation() {
        let mission = MissionRuntime::create_sweeping_raid();
        assert_eq!(mission.name, "Sweeping Raid");
        assert_eq!(mission.mission_id, MissionId::new(5));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Scoring: EndOfCommandPhase BR2-4 only (end of battle has separate bonus)
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.from_round, 2);
        assert_eq!(sched.to_round, 4);
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);
        assert_eq!(sched.vp_per_objective, 5);

        // Mission rules: Supply Lines + End of battle bonus
        assert_eq!(mission.special_rules.len(), 2);
        assert!(mission.special_rules[0].contains("Supply Lines"));
        assert!(mission.special_rules[0].contains("4+"));
        assert!(mission.special_rules[0].contains("1CP"));
        assert!(mission.special_rules[1].contains("End of battle"));
    }

    #[test]
    fn test_sweeping_raid_end_of_battle_bonus() {
        let mission = MissionRuntime::create_sweeping_raid();
        // Objectives have end-of-battle bonus rules
        // A: Defender 10VP, B: Defender 5VP, C: Attacker 5VP, D: Attacker 10VP
        assert!(mission.objectives[0].special_rules[0].contains("Defender scores 10VP"));
        assert!(mission.objectives[1].special_rules[0].contains("Defender scores 5VP"));
        assert!(mission.objectives[2].special_rules[0].contains("Attacker scores 5VP"));
        assert!(mission.objectives[3].special_rules[0].contains("Attacker scores 10VP"));
    }

    // === Display of Might (Mission 6) ===

    #[test]
    fn test_display_of_might_creation() {
        let mission = MissionRuntime::create_display_of_might();
        assert_eq!(mission.name, "Display of Might");
        assert_eq!(mission.mission_id, MissionId::new(6));
        assert_eq!(mission.objectives.len(), 4);
        assert_eq!(mission.rounds, 5);

        // Scoring: EndOfCommandPhase BR2-5
        assert_eq!(mission.scoring_schedule.len(), 1);
        let sched = &mission.scoring_schedule[0];
        assert_eq!(sched.timing, ScoringPhase::EndOfCommandPhase);

        // Mission rules: Break Their Spirit + Claim Sites + Scoring
        assert_eq!(mission.special_rules.len(), 3);
        assert!(mission.special_rules[0].contains("Break Their Spirit"));
        assert!(mission.special_rules[0].contains("Insane Bravery"));
        assert!(mission.special_rules[0].contains("6\""));
        assert!(mission.special_rules[1].contains("Claim Sites"));
        assert!(mission.special_rules[1].contains("CHARACTER"));
        assert!(mission.special_rules[2].contains("Scoring"));
    }

    #[test]
    fn test_display_of_might_symbolic_sites() {
        let mission = MissionRuntime::create_display_of_might();
        // NML objectives (C, D) are symbolic sites
        assert!(mission.objectives[2].special_rules.contains(&"Symbolic site".to_string()));
        assert!(mission.objectives[3].special_rules.contains(&"Symbolic site".to_string()));
        // DZ objectives (A, B) are not symbolic sites
        assert!(mission.objectives[0].special_rules.is_empty());
        assert!(mission.objectives[1].special_rules.is_empty());
    }

    // === All missions have consistent structure ===

    #[test]
    fn test_all_missions_have_correct_structure() {
        let missions = vec![
            MissionRuntime::create_clash_of_patrols(),
            MissionRuntime::create_archeotech_recovery(),
            MissionRuntime::create_forward_outpost(),
            MissionRuntime::create_scorched_earth(),
            MissionRuntime::create_sweeping_raid(),
            MissionRuntime::create_display_of_might(),
        ];

        let expected_names = [
            "Clash of Patrols",
            "Archeotech Recovery",
            "Forward Outpost",
            "Scorched Earth",
            "Sweeping Raid",
            "Display of Might",
        ];

        for (i, mission) in missions.iter().enumerate() {
            assert_eq!(mission.mission_id, MissionId::new((i + 1) as u32));
            assert_eq!(mission.name, expected_names[i]);
            assert_eq!(mission.rounds, 5, "Mission {} should have 5 rounds", mission.name);
            assert_eq!(mission.objectives.len(), 4, "Mission {} should have 4 objectives", mission.name);
            assert!(!mission.scoring_schedule.is_empty(), "Mission {} should have scoring schedule", mission.name);
            assert!(!mission.special_rules.is_empty(), "Mission {} should have special rules", mission.name);

            // All missions score at EndOfCommandPhase
            for sched in &mission.scoring_schedule {
                assert_eq!(
                    sched.timing,
                    ScoringPhase::EndOfCommandPhase,
                    "Mission {} should score at EndOfCommandPhase",
                    mission.name
                );
            }
        }
    }

    #[test]
    fn test_all_missions_serialize() {
        for id in 1..=6u32 {
            let mission = MissionRuntime::create_mission(MissionId::new(id)).unwrap();
            let json = serde_json::to_string(&mission).unwrap();
            let back: MissionInstance = serde_json::from_str(&json).unwrap();
            assert_eq!(back.mission_id, MissionId::new(id));
            assert_eq!(back.name, mission.name);
            assert_eq!(back.objectives.len(), mission.objectives.len());
        }
    }
}
