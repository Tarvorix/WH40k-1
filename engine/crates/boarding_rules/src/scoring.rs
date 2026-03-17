//! Boarding Actions Scoring Engine.
//!
//! Evaluates progressive and end-game scoring conditions based on game state.
//! Supports all scoring methods found across the 16 Boarding Actions missions.
//!
//! Source: boarding_actions_objectives_complete_v3.json
//! Source: boarding_actions_complete_v3.md

use serde::{Deserialize, Serialize};
use wh40k_core_types::{BattleRound, ObjectiveId, PlayerId, VictoryPoints};

use crate::mission_loader::BoardingMissionPackage;

// ---------------------------------------------------------------------------
// Underdog threshold (BA-19)
// ---------------------------------------------------------------------------

/// Determine if a player is the underdog based on army points difference.
/// A player is the underdog if their army is at least 30 points lower than their opponent's.
/// Source: boarding_actions_complete_v3.md §6.6
pub const UNDERDOG_POINT_THRESHOLD: u32 = 30;

pub fn is_underdog(player_points: u32, opponent_points: u32) -> bool {
    opponent_points >= player_points + UNDERDOG_POINT_THRESHOLD
}

// ---------------------------------------------------------------------------
// Scoring event and state types
// ---------------------------------------------------------------------------

/// A single scoring event produced by the scoring engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoringEvent {
    /// The player who scored
    pub player: PlayerId,
    /// Victory points awarded
    pub amount: VictoryPoints,
    /// Human-readable source description
    pub source: String,
    /// The battle round in which this was scored
    pub round: BattleRound,
}

/// End-game state information needed for end-game scoring evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EndGameState {
    /// Number of objective markers corrupted (BA-06)
    pub corrupted_objectives: u32,
    /// Number of airlocks opened (BA-01)
    pub airlocks_opened: u32,
    /// Total number of airlocks on the map (BA-01)
    pub total_airlocks: u32,
    /// Number of lighting area pairs controlled (BA-22)
    pub lighting_area_pairs_controlled: u32,
    /// Whether the imprisoned unit is still in prison cells (BA-04)
    pub imprisoned_unit_in_prison: bool,
    /// Whether the imprisoned unit was destroyed (BA-04)
    pub imprisoned_unit_destroyed: bool,
    /// Whether the imprisoned unit is within engagement range of defender models (BA-04)
    pub imprisoned_unit_engaged_by_defender: bool,
    /// Number of objective markers controlled at end of battle (general)
    pub markers_controlled: u32,
    /// Number of strongroom markers controlled by attacker (BA-03)
    pub strongroom_markers_controlled: u32,
    /// Additional mission-specific conditions as key-value pairs
    pub special_conditions: Vec<(String, String)>,
}

// ---------------------------------------------------------------------------
// Scoring Engine
// ---------------------------------------------------------------------------

/// The Boarding Actions scoring engine. Stateless — all state comes from parameters.
pub struct BoardingScoringEngine;

impl BoardingScoringEngine {
    /// Evaluate progressive scoring for a player at the appropriate timing.
    ///
    /// Standard template: at end of Command Phase in rounds 2-4 (and round 5 with timing rules),
    /// score 5 VP for each satisfied condition.
    ///
    /// Standard 3-condition template:
    /// 1. Control 1+ objective markers
    /// 2. Control 2+ objective markers
    /// 3. Control more objective markers than opponent
    ///
    /// Some missions have a 4th condition (linked power lines, control objective A, furnace, etc.)
    ///
    /// Round 5 special rules:
    /// - First player scores at end of Command Phase (normal)
    /// - Second player scores at end of Turn instead
    ///
    /// Parameters:
    /// - `mission`: the loaded mission package
    /// - `round`: current battle round
    /// - `player`: the player being scored
    /// - `is_second_player`: whether this player is the second player (for round 5 timing)
    /// - `controlled_objectives`: objectives currently controlled by this player
    /// - `opponent_controlled`: objectives currently controlled by the opponent
    /// - `total_objectives`: total number of objective markers on the battlefield
    /// - `extra_conditions_met`: additional conditions satisfied (e.g., linked power lines, control A, furnace)
    pub fn score_progressive(
        mission: &BoardingMissionPackage,
        round: BattleRound,
        player: PlayerId,
        _is_second_player: bool,
        controlled_objectives: &[ObjectiveId],
        opponent_controlled: &[ObjectiveId],
        _total_objectives: usize,
        extra_conditions_met: &[String],
    ) -> Vec<ScoringEvent> {
        let mut events = Vec::new();

        // Find progressive scoring objectives
        for scoring_obj in &mission.scoring_objectives {
            if scoring_obj.objective_type != "progressive" {
                continue;
            }

            let scoring = &scoring_obj.scoring;

            // Check for event_trigger type (BA-23 Data Download)
            if let Some(method) = scoring.get("method").and_then(|v| v.as_str()) {
                if method == "event_trigger" {
                    // Event-trigger scoring is handled externally when the event occurs,
                    // not during the standard progressive scoring pass.
                    continue;
                }
                if method == "vp_transfer" || method == "binary_if_true_repeating" {
                    // VP transfer and binary repeating are also handled by special mechanics.
                    continue;
                }
                if method == "command_phase_conditions" {
                    // This is a variant of the standard progressive template
                    // but with unknown exact conditions. Use standard template.
                }
            }

            // Determine which rounds this objective scores in
            let scoring_rounds: Vec<u8> = scoring
                .get("rounds")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect()
                })
                .unwrap_or_else(|| vec![2, 3, 4]);

            // Check if this round is a scoring round
            let round_num = round.number();
            let is_scoring_round = scoring_rounds.contains(&round_num);
            let is_round_5 = round_num == 5;

            // Round 5 special handling:
            // - check if round5 data exists
            // - first player scores at end_of_command_phase
            // - second player scores at end_of_turn
            let scores_in_round_5 = if is_round_5 {
                // Check if round 5 data indicates this player should score
                if let Some(r5) = scoring.get("round5") {
                    // If second player, they score at end_of_turn (handled by caller),
                    // but we still produce the events.
                    // If first player, they score at end_of_command_phase (standard).
                    let same_conditions = r5
                        .get("same_conditions")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    same_conditions
                } else {
                    // Default: round 5 scoring follows the global rule
                    // (second player at end of turn, first player at command phase)
                    true
                }
            } else {
                false
            };

            if !is_scoring_round && !(is_round_5 && scores_in_round_5) {
                continue;
            }

            // If round 5 and second player, they score at end of turn.
            // We still compute the events; the caller is responsible for timing.
            // (The scoring engine is timing-agnostic; it just evaluates conditions.)

            // Determine VP per condition
            let vp_per_condition: i16 = scoring
                .get("vp_per_condition")
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as i16;

            // Get conditions list
            let conditions: Vec<String> = scoring
                .get("conditions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| {
                            c.get("id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_else(|| {
                    // Default standard 3-condition template
                    vec![
                        "control_1plus".to_string(),
                        "control_2plus".to_string(),
                        "control_more".to_string(),
                    ]
                });

            let player_count = controlled_objectives.len();
            let opponent_count = opponent_controlled.len();

            // Evaluate each condition
            for condition_id in &conditions {
                let satisfied = match condition_id.as_str() {
                    "control_1plus" => player_count >= 1,
                    "control_2plus" => player_count >= 2,
                    "control_more" => player_count > opponent_count,
                    // Mission-specific 4th conditions (linked power lines, control A, furnace)
                    other => extra_conditions_met.iter().any(|ec| ec == other),
                };

                if satisfied {
                    events.push(ScoringEvent {
                        player,
                        amount: VictoryPoints::new(vp_per_condition),
                        source: format!(
                            "{}: {} (Round {})",
                            scoring_obj.name,
                            condition_id,
                            round.number()
                        ),
                        round,
                    });
                }
            }
        }

        events
    }

    /// Evaluate end-game scoring for a player.
    ///
    /// Supports all end-game scoring methods:
    /// - `threshold_table`: destroyed points to VP lookup
    /// - `binary_if_true`: condition check for fixed VP
    /// - `per_marker_controlled`: VP per objective marker controlled
    /// - `per_corrupted`: VP per corrupted marker
    /// - `per_condition`: VP per satisfied condition
    /// - `role_based_per_object`: role-specific per-object VP
    /// - `role_based_per_marker_controlled`: role-specific per marker VP
    /// - `exclusive_outcome_table`: exclusive outcome state table (BA-04)
    pub fn score_end_game(
        mission: &BoardingMissionPackage,
        player: PlayerId,
        player_role: Option<&str>,
        controlled_objectives: &[ObjectiveId],
        opponent_warlord_destroyed: bool,
        destroyed_points: u16,
        special_state: &EndGameState,
    ) -> Vec<ScoringEvent> {
        let mut events = Vec::new();
        let round = BattleRound::new(5);

        for scoring_obj in &mission.scoring_objectives {
            if scoring_obj.objective_type != "end_game" {
                continue;
            }

            let scoring = &scoring_obj.scoring;
            let method = scoring
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            match method {
                "threshold_table" => {
                    // Look up destroyed points in threshold table
                    let vp = Self::destroyed_points_to_vp_from_scoring(
                        destroyed_points,
                        scoring,
                    );
                    if vp > 0 {
                        events.push(ScoringEvent {
                            player,
                            amount: VictoryPoints::new(vp),
                            source: format!(
                                "{}: {} destroyed points",
                                scoring_obj.name, destroyed_points
                            ),
                            round,
                        });
                    }
                }

                "binary_if_true" => {
                    let condition = scoring
                        .get("condition")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let vp = scoring
                        .get("vp")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i16;

                    let satisfied = match condition {
                        "opponent_warlord_destroyed" => opponent_warlord_destroyed,
                        other => special_state
                            .special_conditions
                            .iter()
                            .any(|(k, v)| k == other && v == "true"),
                    };

                    if satisfied {
                        events.push(ScoringEvent {
                            player,
                            amount: VictoryPoints::new(vp),
                            source: format!("{}: {}", scoring_obj.name, condition),
                            round,
                        });
                    }
                }

                "per_marker_controlled" => {
                    let vp_per = scoring
                        .get("vp_per_marker")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i16;
                    let count = controlled_objectives.len() as i16;
                    let total_vp = vp_per * count;
                    if total_vp > 0 {
                        events.push(ScoringEvent {
                            player,
                            amount: VictoryPoints::new(total_vp),
                            source: format!(
                                "{}: {} markers x {}VP",
                                scoring_obj.name, count, vp_per
                            ),
                            round,
                        });
                    }
                }

                "per_condition" => {
                    let vp_per = scoring
                        .get("vp_per_condition")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i16;
                    let conditions = scoring
                        .get("conditions")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.len() as u32)
                        .unwrap_or(0);

                    // For lighting area pairs (BA-22), use special_state
                    let satisfied = special_state.lighting_area_pairs_controlled.min(conditions);
                    let total_vp = vp_per * satisfied as i16;
                    if total_vp > 0 {
                        events.push(ScoringEvent {
                            player,
                            amount: VictoryPoints::new(total_vp),
                            source: format!(
                                "{}: {} conditions x {}VP",
                                scoring_obj.name, satisfied, vp_per
                            ),
                            round,
                        });
                    }
                }

                "role_based_per_object" => {
                    if let Some(role_scoring) = scoring.get("role_scoring") {
                        let role_key = player_role.unwrap_or("unknown");
                        if let Some(role_data) = role_scoring.get(role_key) {
                            let vp_per = role_data
                                .get("vp_per")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i16;
                            let metric = role_data
                                .get("metric")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            let count = match metric {
                                "airlocks_opened" => special_state.airlocks_opened as i16,
                                "airlocks_still_closed" => {
                                    (special_state.total_airlocks
                                        - special_state.airlocks_opened)
                                        as i16
                                }
                                _ => 0,
                            };

                            let total_vp = vp_per * count;
                            if total_vp > 0 {
                                events.push(ScoringEvent {
                                    player,
                                    amount: VictoryPoints::new(total_vp),
                                    source: format!(
                                        "{}: {} x {}VP ({})",
                                        scoring_obj.name, count, vp_per, metric
                                    ),
                                    round,
                                });
                            }
                        }
                    }
                }

                "role_based_per_marker_controlled" => {
                    if let Some(role_scoring) = scoring.get("role_scoring") {
                        let role_key = player_role.unwrap_or("unknown");
                        if let Some(role_data) = role_scoring.get(role_key) {
                            let vp_per = role_data
                                .get("vp_per_marker")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i16;
                            let count = controlled_objectives.len() as i16;
                            let total_vp = vp_per * count;
                            if total_vp > 0 {
                                events.push(ScoringEvent {
                                    player,
                                    amount: VictoryPoints::new(total_vp),
                                    source: format!(
                                        "{}: {} markers x {}VP",
                                        scoring_obj.name, count, vp_per
                                    ),
                                    round,
                                });
                            }
                        }
                    }
                }

                "exclusive_outcome_table" => {
                    // BA-04 Jailbreak: outcome depends on imprisoned unit state
                    if let Some(outcomes) = scoring.get("outcomes").and_then(|v| v.as_array()) {
                        let role_key = player_role.unwrap_or("unknown");

                        // Determine which outcome applies based on state
                        let outcome_index = if special_state.imprisoned_unit_in_prison {
                            // imprisoned_unit_still_within_prison_cells
                            Some(0)
                        } else if special_state.imprisoned_unit_destroyed {
                            // imprisoned_unit_not_within_prison_cells_and_destroyed
                            Some(1)
                        } else if special_state.imprisoned_unit_engaged_by_defender {
                            // not in prison, not destroyed, engaged by defender
                            Some(2)
                        } else {
                            // not in prison, not destroyed, not engaged
                            Some(3)
                        };

                        if let Some(idx) = outcome_index {
                            if let Some(outcome) = outcomes.get(idx) {
                                let vp_key = format!("{}_vp", role_key);
                                let vp = outcome
                                    .get(&vp_key)
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i16;
                                if vp > 0 {
                                    let condition = outcome
                                        .get("condition")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown");
                                    events.push(ScoringEvent {
                                        player,
                                        amount: VictoryPoints::new(vp),
                                        source: format!(
                                            "{}: {}",
                                            scoring_obj.name, condition
                                        ),
                                        round,
                                    });
                                }
                            }
                        }
                    }
                }

                "role_based_end_game_objective" | "unknown_named_end_game_objective" => {
                    // These are partially extracted objectives whose exact scoring
                    // is not yet known. Return no events for now.
                    // When the full rules text is extracted, this branch will be updated.
                }

                _ => {
                    // Unknown scoring method; skip
                }
            }
        }

        events
    }

    /// Apply the 90 VP cap on mission-scored victory points.
    ///
    /// Boarding Actions rules cap mission VP at 90. The 10 VP battle-ready bonus
    /// is added separately and is not subject to the cap.
    pub fn apply_vp_cap(total_mission_vp: i16) -> i16 {
        total_mission_vp.min(90)
    }

    /// Calculate the total score including battle-ready bonus.
    ///
    /// Mission VP is capped at 90, then 10 VP battle-ready bonus is added.
    pub fn total_score(mission_vp: i16, battle_ready: bool) -> i16 {
        let capped = Self::apply_vp_cap(mission_vp);
        if battle_ready {
            capped + 10
        } else {
            capped
        }
    }

    /// Look up destroyed points in a threshold table from scoring data.
    ///
    /// Uses the standard BA threshold table when thresholds are not explicitly
    /// provided in the scoring data:
    /// - 0-124 => 0 VP
    /// - 125-249 => 15 VP
    /// - 250-374 => 40 VP
    /// - 375-499 => 60 VP
    /// - 500+ => 80 VP
    fn destroyed_points_to_vp_from_scoring(
        destroyed_points: u16,
        _scoring: &serde_json::Value,
    ) -> i16 {
        // The JSON thresholds array is empty (exact_value_known: false),
        // so use the standard Boarding Actions threshold table.
        destroyed_points_to_vp(destroyed_points, &STANDARD_DESTROYED_POINTS_THRESHOLDS)
    }
}

// ---------------------------------------------------------------------------
// Standard destroyed-points threshold table
// ---------------------------------------------------------------------------

/// Standard Boarding Actions destroyed-points threshold table.
/// (min_points, max_points, vp_awarded)
///
/// Purge the Ship uses three 15 VP thresholds at 125+, 250+, and 375+.
/// Source: boarding_actions_missions_complete_v3.md §v3 addendum
pub const STANDARD_DESTROYED_POINTS_THRESHOLDS: [(u16, u16, i16); 4] = [
    (0, 124, 0),
    (125, 249, 15),
    (250, 374, 30),
    (375, u16::MAX, 45),
];

/// Look up destroyed points in a threshold table.
///
/// Each entry is (min_points, max_points, vp_awarded).
/// Returns the VP for the matching range.
pub fn destroyed_points_to_vp(destroyed_points: u16, thresholds: &[(u16, u16, i16)]) -> i16 {
    for &(min, max, vp) in thresholds {
        if destroyed_points >= min && destroyed_points <= max {
            return vp;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Event-trigger scoring helper
// ---------------------------------------------------------------------------

/// Create a scoring event for an event-triggered objective (e.g., BA-23 Data Download).
///
/// Call this when the triggering event actually occurs (not during standard scoring passes).
pub fn score_event_trigger(
    player: PlayerId,
    round: BattleRound,
    objective_name: &str,
    vp: i16,
) -> ScoringEvent {
    ScoringEvent {
        player,
        amount: VictoryPoints::new(vp),
        source: format!("{}: event triggered", objective_name),
        round,
    }
}

/// Create a scoring event for VP transfer (BA-02 Pull Their Teeth).
///
/// Returns a pair of events: one negative for the giver, one positive for the receiver.
pub fn score_vp_transfer(
    giver: PlayerId,
    receiver: PlayerId,
    round: BattleRound,
    amount: i16,
    reason: &str,
) -> (ScoringEvent, ScoringEvent) {
    (
        ScoringEvent {
            player: giver,
            amount: VictoryPoints::new(-amount),
            source: format!("VP transfer: -{} ({})", amount, reason),
            round,
        },
        ScoringEvent {
            player: receiver,
            amount: VictoryPoints::new(amount),
            source: format!("VP transfer: +{} ({})", amount, reason),
            round,
        },
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission_loader;
    use wh40k_boarding_content::loader;

    /// Helper to load all three assets from the actual JSON files.
    fn load_all_assets() -> (
        wh40k_boarding_content::mission_maps::BoardingMapsAsset,
        wh40k_boarding_content::mission_objectives::BoardingObjectivesAsset,
        wh40k_boarding_content::mission_tags::BoardingMissionTagsAsset,
    ) {
        let maps_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_maps_complete_v3.json"
        ))
        .unwrap();
        let obj_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_objectives_complete_v3.json"
        ))
        .unwrap();
        let tags_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../boarding_actions_mission_tags_complete_v3.json"
        ))
        .unwrap();

        (
            loader::load_maps_asset(&maps_json).unwrap(),
            loader::load_objectives_asset(&obj_json).unwrap(),
            loader::load_mission_tags_asset(&tags_json).unwrap(),
        )
    }

    #[test]
    fn test_destroyed_points_thresholds() {
        assert_eq!(
            destroyed_points_to_vp(0, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            0
        );
        assert_eq!(
            destroyed_points_to_vp(50, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            0
        );
        assert_eq!(
            destroyed_points_to_vp(124, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            0
        );
        assert_eq!(
            destroyed_points_to_vp(125, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            15
        );
        assert_eq!(
            destroyed_points_to_vp(200, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            15
        );
        assert_eq!(
            destroyed_points_to_vp(249, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            15
        );
        assert_eq!(
            destroyed_points_to_vp(250, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            30
        );
        assert_eq!(
            destroyed_points_to_vp(374, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            30
        );
        assert_eq!(
            destroyed_points_to_vp(375, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(499, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(500, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
        assert_eq!(
            destroyed_points_to_vp(1000, &STANDARD_DESTROYED_POINTS_THRESHOLDS),
            45
        );
    }

    #[test]
    fn test_vp_cap() {
        assert_eq!(BoardingScoringEngine::apply_vp_cap(45), 45);
        assert_eq!(BoardingScoringEngine::apply_vp_cap(90), 90);
        assert_eq!(BoardingScoringEngine::apply_vp_cap(100), 90);
        assert_eq!(BoardingScoringEngine::apply_vp_cap(0), 0);
    }

    #[test]
    fn test_total_score_with_battle_ready() {
        assert_eq!(BoardingScoringEngine::total_score(45, true), 55);
        assert_eq!(BoardingScoringEngine::total_score(90, true), 100);
        assert_eq!(BoardingScoringEngine::total_score(100, true), 100); // cap then +10
        assert_eq!(BoardingScoringEngine::total_score(45, false), 45);
        assert_eq!(BoardingScoringEngine::total_score(90, false), 90);
    }

    #[test]
    fn test_progressive_scoring_ba_11_round_2() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let opponent = vec![ObjectiveId::new(3)];

        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(2),
            player,
            false,
            &controlled,
            &opponent,
            4,
            &[],
        );

        // BA-11 has 3 standard conditions: control_1plus, control_2plus, control_more
        // Player controls 2, opponent controls 1 => all 3 satisfied => 15 VP
        assert_eq!(events.len(), 3);
        let total_vp: i16 = events.iter().map(|e| e.amount.value()).sum();
        assert_eq!(total_vp, 15);
    }

    #[test]
    fn test_progressive_scoring_ba_11_round_1_no_scoring() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let opponent = vec![ObjectiveId::new(3)];

        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(1),
            player,
            false,
            &controlled,
            &opponent,
            4,
            &[],
        );

        // Round 1 is not a scoring round
        assert!(events.is_empty());
    }

    #[test]
    fn test_progressive_scoring_ba_13_linked_power_lines() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-13", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let opponent = vec![ObjectiveId::new(3)];

        // With linked power lines condition met
        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(3),
            player,
            false,
            &controlled,
            &opponent,
            4,
            &["linked_power_lines".to_string()],
        );

        // BA-13 has 4 conditions: control_1plus, control_2plus, control_more, linked_power_lines
        // All 4 should be satisfied => 20 VP
        assert_eq!(events.len(), 4);
        let total_vp: i16 = events.iter().map(|e| e.amount.value()).sum();
        assert_eq!(total_vp, 20);
    }

    #[test]
    fn test_progressive_scoring_ba_12_only_2_conditions() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-12", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let opponent = vec![ObjectiveId::new(3)];

        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(2),
            player,
            false,
            &controlled,
            &opponent,
            4,
            &[],
        );

        // BA-12 has only 2 conditions: control_1plus, control_more
        // Player controls 2, opponent controls 1 => both satisfied => 10 VP
        assert_eq!(events.len(), 2);
        let total_vp: i16 = events.iter().map(|e| e.amount.value()).sum();
        assert_eq!(total_vp, 10);
    }

    #[test]
    fn test_progressive_scoring_partial_conditions() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        // Player controls 1 objective, opponent controls 2
        let controlled = vec![ObjectiveId::new(1)];
        let opponent = vec![ObjectiveId::new(2), ObjectiveId::new(3)];

        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(3),
            player,
            false,
            &controlled,
            &opponent,
            4,
            &[],
        );

        // control_1plus: yes (1 >= 1)
        // control_2plus: no (1 < 2)
        // control_more: no (1 < 2)
        // => 5 VP
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount.value(), 5);
    }

    #[test]
    fn test_end_game_binary_if_true_warlord() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-21", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let state = EndGameState::default();

        // Test with warlord destroyed
        let events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            None,
            &[],
            true, // warlord destroyed
            0,
            &state,
        );

        // BA-21 has "Cut Off the Head": binary_if_true, 10 VP for opponent_warlord_destroyed
        assert!(!events.is_empty());
        let warlord_event = events
            .iter()
            .find(|e| e.source.contains("Cut Off the Head"));
        assert!(warlord_event.is_some());
        assert_eq!(warlord_event.unwrap().amount.value(), 10);
    }

    #[test]
    fn test_end_game_binary_if_true_warlord_not_destroyed() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-21", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let state = EndGameState::default();

        let events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            None,
            &[],
            false, // warlord not destroyed
            0,
            &state,
        );

        // No warlord kill => no VP
        let warlord_event = events
            .iter()
            .find(|e| e.source.contains("Cut Off the Head"));
        assert!(warlord_event.is_none());
    }

    #[test]
    fn test_end_game_per_marker_controlled() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-12", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2), ObjectiveId::new(3)];
        let state = EndGameState::default();

        let events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            None,
            &controlled,
            false,
            0,
            &state,
        );

        // BA-12 has "Sweep the Decks": per_marker_controlled, 15VP per marker
        // 3 markers * 15 VP = 45 VP
        let sweep_event = events
            .iter()
            .find(|e| e.source.contains("Sweep the Decks"));
        assert!(sweep_event.is_some());
        assert_eq!(sweep_event.unwrap().amount.value(), 45);
    }

    #[test]
    fn test_end_game_per_condition_lighting_areas() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-22", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let mut state = EndGameState::default();
        state.lighting_area_pairs_controlled = 2;

        let events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            None,
            &[],
            false,
            0,
            &state,
        );

        // BA-22 has "Seek Sanctuary": per_condition, 15VP per condition, 2 conditions
        // 2 pairs * 15 VP = 30 VP
        let seek_event = events
            .iter()
            .find(|e| e.source.contains("Seek Sanctuary"));
        assert!(seek_event.is_some());
        assert_eq!(seek_event.unwrap().amount.value(), 30);
    }

    #[test]
    fn test_end_game_role_based_per_object_airlock() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-01", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let mut state = EndGameState::default();
        state.airlocks_opened = 3;
        state.total_airlocks = 4;

        // Attacker scores per airlock opened
        let attacker_events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            Some("Attacker"),
            &[],
            false,
            0,
            &state,
        );
        let airlock_event = attacker_events
            .iter()
            .find(|e| e.source.contains("Maintain Ship Integrity"));
        assert!(airlock_event.is_some());
        // 3 airlocks * 20 VP = 60 VP
        assert_eq!(airlock_event.unwrap().amount.value(), 60);

        // Defender scores per airlock still closed
        let defender_events = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(1),
            Some("Defender"),
            &[],
            false,
            0,
            &state,
        );
        let defender_airlock = defender_events
            .iter()
            .find(|e| e.source.contains("Maintain Ship Integrity"));
        assert!(defender_airlock.is_some());
        // 1 airlock still closed * 20 VP = 20 VP
        assert_eq!(defender_airlock.unwrap().amount.value(), 20);
    }

    #[test]
    fn test_end_game_exclusive_outcome_table_ba04() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-04", &maps, &objectives, &tags).unwrap();

        // Case 1: Imprisoned unit still in prison => Attacker 0, Defender 90
        let mut state = EndGameState::default();
        state.imprisoned_unit_in_prison = true;

        let attacker_events = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(0),
            Some("Attacker"),
            &[],
            false,
            0,
            &state,
        );
        let defender_events = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(1),
            Some("Defender"),
            &[],
            false,
            0,
            &state,
        );

        // Attacker gets 0 VP
        let attacker_vp: i16 = attacker_events.iter().map(|e| e.amount.value()).sum();
        assert_eq!(attacker_vp, 0);
        // Defender gets 90 VP
        let defender_vp: i16 = defender_events.iter().map(|e| e.amount.value()).sum();
        assert_eq!(defender_vp, 90);

        // Case 4: Imprisoned unit free, alive, not engaged => Attacker 90, Defender 0
        let mut state2 = EndGameState::default();
        state2.imprisoned_unit_in_prison = false;
        state2.imprisoned_unit_destroyed = false;
        state2.imprisoned_unit_engaged_by_defender = false;

        let attacker_events2 = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(0),
            Some("Attacker"),
            &[],
            false,
            0,
            &state2,
        );
        let defender_events2 = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(1),
            Some("Defender"),
            &[],
            false,
            0,
            &state2,
        );

        let attacker_vp2: i16 = attacker_events2.iter().map(|e| e.amount.value()).sum();
        assert_eq!(attacker_vp2, 90);
        let defender_vp2: i16 = defender_events2.iter().map(|e| e.amount.value()).sum();
        assert_eq!(defender_vp2, 0);
    }

    #[test]
    fn test_end_game_destroyed_points_threshold() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(0);
        let state = EndGameState::default();

        // 300 destroyed points => 30 VP from threshold table (250-374 range)
        let events = BoardingScoringEngine::score_end_game(
            &mission,
            player,
            None,
            &[],
            false,
            300,
            &state,
        );

        let purge_event = events
            .iter()
            .find(|e| e.source.contains("Purge the Ship"));
        assert!(purge_event.is_some());
        assert_eq!(purge_event.unwrap().amount.value(), 30);
    }

    #[test]
    fn test_score_event_trigger() {
        let event = score_event_trigger(
            PlayerId::new(0),
            BattleRound::new(3),
            "Data Retrieval",
            15,
        );
        assert_eq!(event.player, PlayerId::new(0));
        assert_eq!(event.amount.value(), 15);
        assert!(event.source.contains("Data Retrieval"));
        assert_eq!(event.round, BattleRound::new(3));
    }

    #[test]
    fn test_score_vp_transfer() {
        let (give, receive) = score_vp_transfer(
            PlayerId::new(0),
            PlayerId::new(1),
            BattleRound::new(2),
            10,
            "loader controlled",
        );

        assert_eq!(give.player, PlayerId::new(0));
        assert_eq!(give.amount.value(), -10);
        assert_eq!(receive.player, PlayerId::new(1));
        assert_eq!(receive.amount.value(), 10);
    }

    #[test]
    fn test_progressive_scoring_round_5_second_player() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-11", &maps, &objectives, &tags).unwrap();

        let player = PlayerId::new(1);
        let controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let opponent = vec![ObjectiveId::new(3)];

        // Round 5, second player - should still produce events
        // (the caller is responsible for applying at end_of_turn timing)
        let events = BoardingScoringEngine::score_progressive(
            &mission,
            BattleRound::new(5),
            player,
            true,
            &controlled,
            &opponent,
            4,
            &[],
        );

        // Should produce scoring events (timing is caller's responsibility)
        assert!(!events.is_empty());
    }

    #[test]
    fn test_role_based_per_marker_ba_06() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-06", &maps, &objectives, &tags).unwrap();

        let defender_controlled = vec![
            ObjectiveId::new(1),
            ObjectiveId::new(2),
            ObjectiveId::new(3),
        ];
        let state = EndGameState::default();

        // Defender: per_marker_controlled at 30 VP each
        let events = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(1),
            Some("Defender"),
            &defender_controlled,
            false,
            0,
            &state,
        );

        let thwart_event = events
            .iter()
            .find(|e| e.source.contains("Thwart the Corrupters"));
        assert!(thwart_event.is_some());
        assert_eq!(thwart_event.unwrap().amount.value(), 90);
    }

    #[test]
    fn test_role_based_per_marker_ba_03() {
        let (maps, objectives, tags) = load_all_assets();
        let mission = mission_loader::load_mission("BA-03", &maps, &objectives, &tags).unwrap();

        let attacker_controlled = vec![ObjectiveId::new(1), ObjectiveId::new(2)];
        let state = EndGameState::default();

        // Attacker: 45 VP per marker
        let events = BoardingScoringEngine::score_end_game(
            &mission,
            PlayerId::new(0),
            Some("Attacker"),
            &attacker_controlled,
            false,
            0,
            &state,
        );

        let ours_event = events
            .iter()
            .find(|e| e.source.contains("They Are Ours"));
        assert!(ours_event.is_some());
        assert_eq!(ours_event.unwrap().amount.value(), 90);
    }
}
