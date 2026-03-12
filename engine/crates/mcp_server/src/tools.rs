//! MCP Tool handler implementations.
//!
//! Each tool corresponds to one MCP tool definition and takes
//! parsed arguments, a reference to the SessionManager, and returns
//! a ToolCallResult.
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

use serde::Serialize;

use wh40k_core_types::*;
use wh40k_game_core::GameState;
use wh40k_search_core::{SearchRoot, AiLevel};
use wh40k_search_abstraction::ActionGenerator;

use crate::error::McpError;
use crate::observation::{build_observation, ViewMode};
use crate::protocol::*;
use crate::session::*;

// ============================================================================
// Tool Dispatcher
// ============================================================================

/// Dispatch a tool call to the appropriate handler.
pub fn dispatch_tool(
    name: &str,
    arguments: Option<serde_json::Value>,
    sessions: &mut SessionManager,
) -> ToolCallResult {
    let args = arguments.unwrap_or(serde_json::Value::Null);

    let result = match name {
        "game.create_session" => handle_create_session(args, sessions),
        "game.load_scenario" => handle_load_scenario(args, sessions),
        "game.get_observation" => handle_get_observation(args, sessions),
        "game.list_legal_actions" => handle_list_legal_actions(args, sessions),
        "game.apply_action" => handle_apply_action(args, sessions),
        "game.step_until_decision" => handle_step_until_decision(args, sessions),
        "game.get_replay_log" => handle_get_replay_log(args, sessions),
        "game.get_score" => handle_get_score(args, sessions),
        "game.reset" => handle_reset(args, sessions),
        "game.end_session" => handle_end_session(args, sessions),
        _ => Err(McpError::InvalidArguments(format!("Unknown tool: {}", name))),
    };

    match result {
        Ok(r) => r,
        Err(e) => e.to_tool_error(),
    }
}

// ============================================================================
// Tool Handlers
// ============================================================================

/// game.create_session: Create a new Combat Patrol game session.
fn handle_create_session(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: CreateSessionArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let faction_a = parse_faction(&parsed.faction_a)?;
    let faction_b = parse_faction(&parsed.faction_b)?;
    let mission = parse_mission(parsed.mission)?;

    let seed = match parsed.seed {
        Some(ref s) => parse_seed(s)?,
        None => random_seed(),
    };

    // Parse optional enhancement/secondary IDs
    let enhancement_a = parsed.enhancement_a.as_ref()
        .map(|s| parse_enhancement_id(s, faction_a))
        .transpose()?;
    let enhancement_b = parsed.enhancement_b.as_ref()
        .map(|s| parse_enhancement_id(s, faction_b))
        .transpose()?;
    let secondary_a = parsed.secondary_a.as_ref()
        .map(|s| parse_secondary_id(s, faction_a))
        .transpose()?;
    let secondary_b = parsed.secondary_b.as_ref()
        .map(|s| parse_secondary_id(s, faction_b))
        .transpose()?;

    let config = SessionConfig {
        faction_a,
        faction_b,
        mission,
        seed,
        enhancement_a,
        enhancement_b,
        secondary_a,
        secondary_b,
    };

    let session_id = sessions.create_session(config)?;

    let session = sessions.get(&session_id)?;
    let observation = build_observation(&session.state, ViewMode::Public);

    #[derive(Serialize)]
    struct CreateResult {
        session_id: String,
        seed: String,
        observation: crate::observation::GameObservation,
    }

    let result = CreateResult {
        session_id: session_id.clone(),
        seed: seed_to_hex(&seed),
        observation,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.load_scenario: Load a game state from serialized JSON.
fn handle_load_scenario(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: LoadScenarioArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let state: GameState = serde_json::from_str(&parsed.state_json)
        .map_err(|e| McpError::SerializationError(format!("Failed to parse state JSON: {}", e)))?;

    let session_id = if let Some(ref id) = parsed.session_id {
        // Load into existing session
        let session = sessions.get_mut(id)?;
        session.state = state;
        session.cached_actions = None;
        id.clone()
    } else {
        // Create new session
        sessions.create_session_from_state(state)
    };

    let session = sessions.get(&session_id)?;
    let observation = build_observation(&session.state, ViewMode::Public);

    #[derive(Serialize)]
    struct LoadResult {
        session_id: String,
        observation: crate::observation::GameObservation,
    }

    let result = LoadResult {
        session_id,
        observation,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.get_observation: Get current game state observation.
fn handle_get_observation(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: GetObservationArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let view_mode = ViewMode::from_str(&parsed.view_mode)
        .ok_or_else(|| McpError::InvalidViewMode(parsed.view_mode.clone()))?;

    let session = sessions.get(&parsed.session_id)?;
    let observation = build_observation(&session.state, view_mode);

    ToolCallResult::json(&observation).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.list_legal_actions: List all legal actions at the current decision point.
fn handle_list_legal_actions(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: ListLegalActionsArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let session = sessions.get_mut(&parsed.session_id)?;

    if !session.state.is_in_progress() {
        return Err(McpError::GameAlreadyEnded);
    }

    let candidates = session.get_legal_actions();

    #[derive(Serialize)]
    struct ActionInfo {
        index: usize,
        label: String,
        intent: String,
        commands_count: usize,
        actor_units: Vec<u32>,
        priority: i32,
    }

    #[derive(Serialize)]
    struct LegalActionsResult {
        session_id: String,
        decision_owner: u32,
        phase: String,
        battle_round: u8,
        action_count: usize,
        actions: Vec<ActionInfo>,
    }

    let actions: Vec<ActionInfo> = candidates.candidates.iter().enumerate().map(|(i, action)| {
        ActionInfo {
            index: i,
            label: action.label.clone(),
            intent: format!("{:?}", action.intent),
            commands_count: action.commands.len(),
            actor_units: action.actor_units.iter().map(|u| u.raw()).collect(),
            priority: action.priority_hint,
        }
    }).collect();

    let result = LegalActionsResult {
        session_id: parsed.session_id.clone(),
        decision_owner: session.state.decision_owner.raw(),
        phase: format!("{:?}", session.state.current_phase),
        battle_round: session.state.battle_round.number(),
        action_count: actions.len(),
        actions,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.apply_action: Apply a legal action by index.
fn handle_apply_action(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: ApplyActionArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let session = sessions.get_mut(&parsed.session_id)?;

    if !session.state.is_in_progress() {
        return Err(McpError::GameAlreadyEnded);
    }

    let apply_result = session.apply_action(parsed.action_index)?;

    // Get updated observation
    let observation = build_observation(&session.state, ViewMode::Public);

    #[derive(Serialize)]
    struct ActionResult {
        applied: ApplyResult,
        observation: crate::observation::GameObservation,
    }

    let result = ActionResult {
        applied: apply_result,
        observation,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.step_until_decision: Auto-advance using AI until next decision point.
fn handle_step_until_decision(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: StepUntilDecisionArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let ai_level = parse_ai_level(&parsed.ai_level)?;

    let session = sessions.get_mut(&parsed.session_id)?;

    if !session.state.is_in_progress() {
        return Err(McpError::GameAlreadyEnded);
    }

    let mut steps_taken: u32 = 0;
    let mut actions_applied = Vec::new();

    while session.state.is_in_progress() && steps_taken < parsed.max_steps {
        // Generate candidates
        let perspective = session.state.decision_owner;
        let candidates = ActionGenerator::generate(&session.state, perspective);

        if candidates.candidates.is_empty() {
            break;
        }

        // Use the search engine to pick the best action
        let mut search_root = SearchRoot::new(ai_level);
        let search_result = search_root.search(&session.state, perspective);

        match search_result {
            Some(result) => {
                // Execute the best action's commands
                let action = &result.best_action;
                let label = action.label.clone();

                for command in &action.commands {
                    let validation = wh40k_game_core::CommandValidator::validate(
                        &session.state, command,
                    );
                    if !validation.is_legal() {
                        // Skip invalid commands during auto-stepping
                        break;
                    }
                    match wh40k_game_core::CommandExecutor::execute(
                        &mut session.state, command,
                    ) {
                        Ok(events) => {
                            session.recorder.record_command(&session.state, command, &events, true);
                        }
                        Err(_) => break,
                    }
                }

                actions_applied.push(label);
                session.cached_actions = None;
                steps_taken += 1;
            }
            None => break,
        }
    }

    let observation = build_observation(&session.state, ViewMode::Public);

    #[derive(Serialize)]
    struct StepResult {
        session_id: String,
        steps_taken: u32,
        actions_applied: Vec<String>,
        game_over: bool,
        observation: crate::observation::GameObservation,
    }

    let result = StepResult {
        session_id: parsed.session_id.clone(),
        steps_taken,
        actions_applied,
        game_over: !session.state.is_in_progress(),
        observation,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.get_replay_log: Get the replay log for the session.
fn handle_get_replay_log(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: GetReplayLogArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let session = sessions.get(&parsed.session_id)?;
    let all_frames = session.recorder.frames();
    let total_frame_count = all_frames.len();

    match parsed.format.as_str() {
        "json" => {
            let frames = if let Some(last_n) = parsed.last_n {
                let start = if total_frame_count > last_n {
                    total_frame_count - last_n
                } else {
                    0
                };
                &all_frames[start..]
            } else {
                all_frames
            };

            #[derive(Serialize)]
            struct ReplayLogResult {
                session_id: String,
                frame_count: usize,
                frames: Vec<ReplayFrameSummary>,
            }

            #[derive(Serialize)]
            struct ReplayFrameSummary {
                index: usize,
                command: String,
                events_count: usize,
                battle_round: u8,
                phase: String,
            }

            let frame_summaries: Vec<ReplayFrameSummary> = frames.iter().enumerate().map(|(i, frame)| {
                ReplayFrameSummary {
                    index: i,
                    command: format!("{:?}", frame.command),
                    events_count: frame.events.len(),
                    battle_round: frame.round.number(),
                    phase: format!("{:?}", frame.phase),
                }
            }).collect();

            let result = ReplayLogResult {
                session_id: parsed.session_id.clone(),
                frame_count: total_frame_count,
                frames: frame_summaries,
            };

            ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
        }
        "summary" | _ => {
            let mut summary = String::new();
            summary.push_str(&format!(
                "Replay Log for session {}\n",
                parsed.session_id
            ));
            summary.push_str(&format!("Total frames: {}\n", total_frame_count));
            summary.push_str("---\n");

            let frames = if let Some(last_n) = parsed.last_n {
                let start = if total_frame_count > last_n {
                    total_frame_count - last_n
                } else {
                    0
                };
                &all_frames[start..]
            } else {
                all_frames
            };

            for (i, frame) in frames.iter().enumerate() {
                summary.push_str(&format!(
                    "Frame {}: BR{} {:?} - {:?} ({} events)\n",
                    i,
                    frame.round.number(),
                    frame.phase,
                    frame.command,
                    frame.events.len(),
                ));
            }

            Ok(ToolCallResult::text(summary))
        }
    }
}

/// game.get_score: Get current scores and game status.
fn handle_get_score(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: GetScoreArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let session = sessions.get(&parsed.session_id)?;
    let state = &session.state;

    #[derive(Serialize)]
    struct ScoreResult {
        session_id: String,
        battle_round: u8,
        phase: String,
        game_outcome: String,
        is_in_progress: bool,
        player_0: PlayerScore,
        player_1: PlayerScore,
        action_count: u64,
    }

    #[derive(Serialize)]
    struct PlayerScore {
        id: u32,
        name: String,
        faction: String,
        victory_points: i16,
        command_points: i8,
        units_alive: usize,
        models_alive: usize,
    }

    let p0_units_alive = state.alive_units_for_player(PlayerId::new(0)).len();
    let p1_units_alive = state.alive_units_for_player(PlayerId::new(1)).len();
    let p0_models_alive: usize = state.alive_units_for_player(PlayerId::new(0))
        .iter()
        .map(|u| u.models.iter().filter(|m| m.alive).count())
        .sum();
    let p1_models_alive: usize = state.alive_units_for_player(PlayerId::new(1))
        .iter()
        .map(|u| u.models.iter().filter(|m| m.alive).count())
        .sum();

    let faction_name = |pid: PlayerId| -> String {
        state.player(pid).faction_id.map(|fid| match fid.raw() {
            0 => "Custodes".to_string(),
            1 => "World Eaters".to_string(),
            _ => format!("Faction({})", fid.raw()),
        }).unwrap_or_else(|| "Unknown".to_string())
    };

    let result = ScoreResult {
        session_id: parsed.session_id.clone(),
        battle_round: state.battle_round.number(),
        phase: format!("{:?}", state.current_phase),
        game_outcome: format!("{:?}", state.game_outcome),
        is_in_progress: state.is_in_progress(),
        player_0: PlayerScore {
            id: 0,
            name: state.player(PlayerId::new(0)).name.clone(),
            faction: faction_name(PlayerId::new(0)),
            victory_points: state.player(PlayerId::new(0)).vp.value(),
            command_points: state.player(PlayerId::new(0)).cp.value(),
            units_alive: p0_units_alive,
            models_alive: p0_models_alive,
        },
        player_1: PlayerScore {
            id: 1,
            name: state.player(PlayerId::new(1)).name.clone(),
            faction: faction_name(PlayerId::new(1)),
            victory_points: state.player(PlayerId::new(1)).vp.value(),
            command_points: state.player(PlayerId::new(1)).cp.value(),
            units_alive: p1_units_alive,
            models_alive: p1_models_alive,
        },
        action_count: session.action_count,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.reset: Reset session to initial state.
fn handle_reset(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: ResetArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    let session = sessions.get_mut(&parsed.session_id)?;
    session.reset()?;

    let observation = build_observation(&session.state, ViewMode::Public);

    #[derive(Serialize)]
    struct ResetResult {
        session_id: String,
        status: String,
        observation: crate::observation::GameObservation,
    }

    let result = ResetResult {
        session_id: parsed.session_id.clone(),
        status: "reset_complete".to_string(),
        observation,
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

/// game.end_session: End and clean up a session.
fn handle_end_session(
    args: serde_json::Value,
    sessions: &mut SessionManager,
) -> Result<ToolCallResult, McpError> {
    let parsed: EndSessionArgs = serde_json::from_value(args)
        .map_err(|e| McpError::InvalidArguments(e.to_string()))?;

    sessions.end_session(&parsed.session_id)?;

    #[derive(Serialize)]
    struct EndResult {
        session_id: String,
        status: String,
        remaining_sessions: usize,
    }

    let result = EndResult {
        session_id: parsed.session_id,
        status: "ended".to_string(),
        remaining_sessions: sessions.session_count(),
    };

    ToolCallResult::json(&result).map_err(|e| McpError::SerializationError(e.to_string()))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse AI level string into AiLevel enum.
fn parse_ai_level(level: &str) -> Result<AiLevel, McpError> {
    match level.to_lowercase().as_str() {
        "greedy" => Ok(AiLevel::Greedy),
        "one_ply" | "oneply" => Ok(AiLevel::OnePly),
        "negamax" => Ok(AiLevel::Negamax),
        _ => Err(McpError::InvalidAiLevel(level.to_string())),
    }
}

/// Parse enhancement ID from string name.
fn parse_enhancement_id(name: &str, faction: FactionId) -> Result<EnhancementId, McpError> {
    match (name.to_lowercase().as_str(), faction.raw()) {
        ("fearsome_presence" | "fearsome presence", 1) => Ok(EnhancementId::new(0)),
        ("bane_of_the_craven" | "bane of the craven", 1) => Ok(EnhancementId::new(1)),
        ("watchman_of_terra" | "watchman of terra", 0) => Ok(EnhancementId::new(0)),
        ("warrior_exemplar" | "warrior exemplar", 0) => Ok(EnhancementId::new(1)),
        _ => Err(McpError::InvalidArguments(format!(
            "Unknown enhancement '{}' for faction {}",
            name,
            if faction.raw() == 0 { "Custodes" } else { "World Eaters" }
        ))),
    }
}

/// Parse secondary objective ID from string name.
fn parse_secondary_id(name: &str, faction: FactionId) -> Result<SecondaryObjectiveId, McpError> {
    match (name.to_lowercase().as_str(), faction.raw()) {
        ("champions_of_khorne" | "champions of khorne", 1) => Ok(SecondaryObjectiveId::new(0)),
        ("skull_takers" | "skull takers", 1) => Ok(SecondaryObjectiveId::new(1)),
        ("raise_the_vexillas" | "raise the vexillas", 0) => Ok(SecondaryObjectiveId::new(0)),
        ("consecrated_ground" | "consecrated ground", 0) => Ok(SecondaryObjectiveId::new(1)),
        _ => Err(McpError::InvalidArguments(format!(
            "Unknown secondary objective '{}' for faction {}",
            name,
            if faction.raw() == 0 { "Custodes" } else { "World Eaters" }
        ))),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session_manager() -> SessionManager {
        SessionManager::new()
    }

    #[test]
    fn test_dispatch_unknown_tool() {
        let mut sessions = create_test_session_manager();
        let result = dispatch_tool("unknown.tool", None, &mut sessions);
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_create_session() {
        let mut sessions = create_test_session_manager();
        let args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });

        let result = dispatch_tool("game.create_session", Some(args), &mut sessions);
        assert!(result.is_error.is_none());
        assert_eq!(sessions.session_count(), 1);
    }

    #[test]
    fn test_create_session_with_seed() {
        let mut sessions = create_test_session_manager();
        let args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 3,
            "seed": "deadbeef"
        });

        let result = dispatch_tool("game.create_session", Some(args), &mut sessions);
        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_create_session_invalid_faction() {
        let mut sessions = create_test_session_manager();
        let args = serde_json::json!({
            "faction_a": "orks",
            "faction_b": "world_eaters",
            "mission": 1
        });

        let result = dispatch_tool("game.create_session", Some(args), &mut sessions);
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_get_observation() {
        let mut sessions = create_test_session_manager();

        // Create session first
        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        // Get observation
        let obs_args = serde_json::json!({
            "session_id": session_id,
            "view_mode": "public"
        });
        let result = dispatch_tool("game.get_observation", Some(obs_args), &mut sessions);
        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_get_observation_debug() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        let obs_args = serde_json::json!({
            "session_id": session_id,
            "view_mode": "debug"
        });
        let result = dispatch_tool("game.get_observation", Some(obs_args), &mut sessions);
        assert!(result.is_error.is_none());
        // Debug view should contain state hash
        assert!(result.content[0].text.contains("state_hash"));
    }

    #[test]
    fn test_list_legal_actions() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        let actions_args = serde_json::json!({
            "session_id": session_id
        });
        let result = dispatch_tool("game.list_legal_actions", Some(actions_args), &mut sessions);
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("action_count"));
    }

    #[test]
    fn test_get_score() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        let score_args = serde_json::json!({
            "session_id": session_id
        });
        let result = dispatch_tool("game.get_score", Some(score_args), &mut sessions);
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("victory_points"));
    }

    #[test]
    fn test_reset_session() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        let reset_args = serde_json::json!({
            "session_id": session_id
        });
        let result = dispatch_tool("game.reset", Some(reset_args), &mut sessions);
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("reset_complete"));
    }

    #[test]
    fn test_end_session() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();
        assert_eq!(sessions.session_count(), 1);

        let end_args = serde_json::json!({
            "session_id": session_id
        });
        let result = dispatch_tool("game.end_session", Some(end_args), &mut sessions);
        assert!(result.is_error.is_none());
        assert_eq!(sessions.session_count(), 0);
    }

    #[test]
    fn test_session_not_found() {
        let mut sessions = create_test_session_manager();

        let args = serde_json::json!({
            "session_id": "nonexistent"
        });
        let result = dispatch_tool("game.get_score", Some(args), &mut sessions);
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_get_replay_log_summary() {
        let mut sessions = create_test_session_manager();

        let create_args = serde_json::json!({
            "faction_a": "custodes",
            "faction_b": "world_eaters",
            "mission": 1
        });
        dispatch_tool("game.create_session", Some(create_args), &mut sessions);

        let session_id = sessions.list_sessions()[0].to_string();

        let replay_args = serde_json::json!({
            "session_id": session_id,
            "format": "summary"
        });
        let result = dispatch_tool("game.get_replay_log", Some(replay_args), &mut sessions);
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("Replay Log"));
    }

    #[test]
    fn test_parse_ai_level() {
        assert!(matches!(parse_ai_level("greedy"), Ok(AiLevel::Greedy)));
        assert!(matches!(parse_ai_level("one_ply"), Ok(AiLevel::OnePly)));
        assert!(matches!(parse_ai_level("negamax"), Ok(AiLevel::Negamax)));
        assert!(parse_ai_level("invalid").is_err());
    }
}
