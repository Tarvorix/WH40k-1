//! Observation model for MCP game state views.
//!
//! Provides three view levels:
//! - Public: information any legal player is allowed to know
//! - Player(N): information visible to a specific player
//! - Debug: complete internal state for testing/debugging
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

use serde::Serialize;
use wh40k_core_types::*;
use wh40k_game_core::state::GameState;

// ============================================================================
// View Mode
// ============================================================================

/// Controls what information is included in the observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Public information visible to all players.
    Public,
    /// Information visible to a specific player (includes their private data).
    Player(PlayerId),
    /// Complete internal state for debugging/testing.
    Debug,
}

impl ViewMode {
    /// Parse a view mode from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(ViewMode::Public),
            "player_0" => Some(ViewMode::Player(PlayerId::new(0))),
            "player_1" => Some(ViewMode::Player(PlayerId::new(1))),
            "debug" => Some(ViewMode::Debug),
            _ => None,
        }
    }
}

// ============================================================================
// Observation Types
// ============================================================================

/// Top-level game observation returned to MCP clients.
#[derive(Debug, Clone, Serialize)]
pub struct GameObservation {
    /// Current battle round (1-5).
    pub battle_round: u8,
    /// Current phase name.
    pub phase: String,
    /// Current sub-phase name.
    pub subphase: String,
    /// Active player ID (whose turn it is).
    pub active_player: u32,
    /// Decision owner ID (who must act next).
    pub decision_owner: u32,
    /// Game outcome status.
    pub game_outcome: String,
    /// Whether game is still in progress.
    pub is_in_progress: bool,
    /// Player observations.
    pub players: [PlayerObservation; 2],
    /// Unit observations (filtered by view mode).
    pub units: Vec<UnitObservation>,
    /// Objective marker observations.
    pub objectives: Vec<ObjectiveObservation>,
    /// Board dimensions.
    pub board: BoardObservation,
    /// Recent events (last 20).
    pub recent_events: Vec<String>,
    /// Active reaction windows (if any).
    pub reaction_windows: Vec<ReactionWindowObservation>,
    /// View mode used to generate this observation.
    pub view_mode: String,
    /// Debug-only fields (only populated in debug mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<DebugObservation>,
}

/// Observation of a single player's state.
#[derive(Debug, Clone, Serialize)]
pub struct PlayerObservation {
    pub id: u32,
    pub name: String,
    pub faction: String,
    pub command_points: i8,
    pub victory_points: i16,
    pub units_alive: usize,
    pub units_total: usize,
    pub models_alive: usize,
    pub models_total: usize,
    /// Enhancement name (if public or own player).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enhancement: Option<String>,
    /// Secondary objective name (if public or own player).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_objective: Option<String>,
}

/// Observation of a single unit.
#[derive(Debug, Clone, Serialize)]
pub struct UnitObservation {
    pub id: u32,
    pub name: String,
    pub owner: u32,
    pub status: String,
    pub models_alive: usize,
    pub models_total: usize,
    pub wounds_current: u8,
    pub wounds_max: u8,
    pub position: Option<PositionObservation>,
    pub is_engaged: bool,
    pub is_battle_shocked: bool,
    pub has_moved: bool,
    pub has_shot: bool,
    pub has_charged: bool,
    pub has_fought: bool,
    pub objective_control: u8,
    pub movement_action: Option<String>,
    /// Keywords on this unit.
    pub keywords: Vec<String>,
    /// Models in this unit with their individual status.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelObservation>,
}

/// Observation of a single model within a unit.
#[derive(Debug, Clone, Serialize)]
pub struct ModelObservation {
    pub id: u32,
    pub name: String,
    pub wounds_current: u8,
    pub wounds_max: u8,
    pub is_alive: bool,
    pub position: Option<PositionObservation>,
}

/// Position in game coordinates.
#[derive(Debug, Clone, Serialize)]
pub struct PositionObservation {
    pub x: f64,
    pub y: f64,
}

/// Observation of an objective marker.
#[derive(Debug, Clone, Serialize)]
pub struct ObjectiveObservation {
    pub id: u32,
    pub position: PositionObservation,
    pub controller: Option<u32>,
    pub player_0_oc: u8,
    pub player_1_oc: u8,
}

/// Board dimensions and deployment zones.
#[derive(Debug, Clone, Serialize)]
pub struct BoardObservation {
    pub width_inches: f64,
    pub height_inches: f64,
    pub terrain_count: usize,
}

/// Reaction window observation.
#[derive(Debug, Clone, Serialize)]
pub struct ReactionWindowObservation {
    pub window_type: String,
    pub triggering_player: u32,
    pub responding_player: u32,
}

/// Debug-only observation with complete internal state.
#[derive(Debug, Clone, Serialize)]
pub struct DebugObservation {
    pub command_history_len: usize,
    pub event_log_len: usize,
    pub active_effects_count: usize,
    pub deterministic_counter: u64,
    pub state_hash: u64,
    /// Full serialized GameState JSON (debug mode only).
    pub full_state_json: String,
}

// ============================================================================
// Observation Builder
// ============================================================================

/// Build a GameObservation from a GameState and ViewMode.
pub fn build_observation(state: &GameState, view_mode: ViewMode) -> GameObservation {
    let players = [
        build_player_observation(state, PlayerId::new(0), view_mode),
        build_player_observation(state, PlayerId::new(1), view_mode),
    ];

    let units = build_unit_observations(state, view_mode);
    let objectives = build_objective_observations(state);
    let board = build_board_observation(state);
    let recent_events = build_recent_events(state);
    let reaction_windows = build_reaction_windows(state);

    let debug = if view_mode == ViewMode::Debug {
        Some(build_debug_observation(state))
    } else {
        None
    };

    let view_mode_str = match view_mode {
        ViewMode::Public => "public".to_string(),
        ViewMode::Player(pid) => format!("player_{}", pid.raw()),
        ViewMode::Debug => "debug".to_string(),
    };

    GameObservation {
        battle_round: state.battle_round.number(),
        phase: format!("{:?}", state.current_phase),
        subphase: format!("{:?}", state.current_subphase),
        active_player: state.active_player.raw(),
        decision_owner: state.decision_owner.raw(),
        game_outcome: format!("{:?}", state.game_outcome),
        is_in_progress: state.is_in_progress(),
        players,
        units,
        objectives,
        board,
        recent_events,
        reaction_windows,
        view_mode: view_mode_str,
        debug,
    }
}

fn build_player_observation(
    state: &GameState,
    player_id: PlayerId,
    view_mode: ViewMode,
) -> PlayerObservation {
    let player = state.player(player_id);

    let units_for_player: Vec<_> = state.units.iter()
        .filter(|u| u.owner == player_id)
        .collect();

    let units_alive = units_for_player.iter()
        .filter(|u| u.status == UnitStatus::OnBattlefield || u.status == UnitStatus::InStrategicReserves)
        .count();

    let models_alive: usize = units_for_player.iter()
        .flat_map(|u| u.models.iter())
        .filter(|m| m.alive)
        .count();

    let models_total: usize = units_for_player.iter()
        .map(|u| u.models.len())
        .sum();

    // Enhancement and secondary are visible to own player, or in debug mode, or always in CP
    // (Combat Patrol has no hidden information for enhancements/secondaries)
    let show_private = match view_mode {
        ViewMode::Public => true, // In CP, enhancements/secondaries are public
        ViewMode::Player(_) => true,
        ViewMode::Debug => true,
    };

    let enhancement = if show_private {
        player.enhancement_choice.map(|eid| format!("Enhancement({})", eid.raw()))
    } else {
        None
    };

    let secondary_objective = if show_private {
        player.secondary_choice.map(|sid| format!("Secondary({})", sid.raw()))
    } else {
        None
    };

    let faction_name = player.faction_id
        .map(|fid| match fid.raw() {
            0 => "Custodes".to_string(),
            1 => "World Eaters".to_string(),
            _ => format!("Faction({})", fid.raw()),
        })
        .unwrap_or_else(|| "Unknown".to_string());

    PlayerObservation {
        id: player_id.raw(),
        name: player.name.clone(),
        faction: faction_name,
        command_points: player.cp.value(),
        victory_points: player.vp.value(),
        units_alive,
        units_total: units_for_player.len(),
        models_alive,
        models_total,
        enhancement,
        secondary_objective,
    }
}

fn build_unit_observations(state: &GameState, view_mode: ViewMode) -> Vec<UnitObservation> {
    state.units.iter().map(|unit| {
        let models_alive = unit.models.iter().filter(|m| m.alive).count();
        let models_total = unit.models.len();

        // Aggregate wounds
        let wounds_current: u8 = unit.models.iter()
            .filter(|m| m.alive)
            .map(|m| m.wounds_remaining.value())
            .sum();
        let wounds_max: u8 = unit.models.iter()
            .map(|m| m.wounds_max.value())
            .sum();

        // Position: use centroid of alive models
        let position = compute_unit_centroid(unit);

        // UnitState does not have a movement_action field; leave as None
        let movement_action: Option<String> = None;

        let keywords: Vec<String> = unit.keywords.iter()
            .map(|k| format!("{:?}", k))
            .collect();

        let models = if view_mode == ViewMode::Debug {
            unit.models.iter().map(|m| {
                ModelObservation {
                    id: m.id.raw(),
                    name: format!("Model({})", m.id.raw()),
                    wounds_current: m.wounds_remaining.value(),
                    wounds_max: m.wounds_max.value(),
                    is_alive: m.alive,
                    position: Some(PositionObservation {
                        x: m.position.x.as_f64(),
                        y: m.position.y.as_f64(),
                    }),
                }
            }).collect()
        } else {
            Vec::new()
        };

        UnitObservation {
            id: unit.id.raw(),
            name: unit.name.clone(),
            owner: unit.owner.raw(),
            status: format!("{:?}", unit.status),
            models_alive,
            models_total,
            wounds_current,
            wounds_max,
            position,
            is_engaged: unit.engagement_status == EngagementStatus::Engaged,
            is_battle_shocked: unit.battle_shocked,
            has_moved: state.turn_flags.has_moved(unit.id),
            has_shot: state.turn_flags.has_shot(unit.id),
            has_charged: state.turn_flags.has_charged(unit.id),
            has_fought: state.turn_flags.has_fought(unit.id),
            objective_control: unit.effective_oc().value(),
            movement_action,
            keywords,
            models,
        }
    }).collect()
}

fn compute_unit_centroid(unit: &wh40k_game_core::UnitState) -> Option<PositionObservation> {
    let alive_positions: Vec<_> = unit.models.iter()
        .filter(|m| m.alive)
        .map(|m| m.position)
        .collect();

    if alive_positions.is_empty() {
        return None;
    }

    let count = alive_positions.len() as f64;
    let sum_x: f64 = alive_positions.iter().map(|p| p.x.as_f64()).sum();
    let sum_y: f64 = alive_positions.iter().map(|p| p.y.as_f64()).sum();

    Some(PositionObservation {
        x: sum_x / count,
        y: sum_y / count,
    })
}

fn build_objective_observations(state: &GameState) -> Vec<ObjectiveObservation> {
    state.board.objectives.iter().map(|obj| {
        // Calculate OC for each player
        let mut p0_oc: u8 = 0;
        let mut p1_oc: u8 = 0;

        for unit in &state.units {
            if unit.status != UnitStatus::OnBattlefield || unit.battle_shocked {
                continue;
            }
            // Check if unit is within range of objective (3" in 40k)
            if let Some(pos) = compute_unit_centroid(unit) {
                let obj_x = obj.position.x.as_f64();
                let obj_y = obj.position.y.as_f64();
                let dx = pos.x - obj_x;
                let dy = pos.y - obj_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= 3.0 {
                    let oc = unit.effective_oc().value();
                    if unit.owner == PlayerId::new(0) {
                        p0_oc = p0_oc.saturating_add(oc);
                    } else {
                        p1_oc = p1_oc.saturating_add(oc);
                    }
                }
            }
        }

        let controller = if p0_oc > 0 && p0_oc > p1_oc {
            Some(0u32)
        } else if p1_oc > 0 && p1_oc > p0_oc {
            Some(1u32)
        } else {
            None
        };

        ObjectiveObservation {
            id: obj.id.raw(),
            position: PositionObservation {
                x: obj.position.x.as_f64(),
                y: obj.position.y.as_f64(),
            },
            controller,
            player_0_oc: p0_oc,
            player_1_oc: p1_oc,
        }
    }).collect()
}

fn build_board_observation(state: &GameState) -> BoardObservation {
    BoardObservation {
        width_inches: state.board.width().as_f64(),
        height_inches: state.board.height().as_f64(),
        terrain_count: state.board.terrain.len(),
    }
}

fn build_recent_events(state: &GameState) -> Vec<String> {
    let log = state.event_bus.log();
    let start = if log.len() > 20 { log.len() - 20 } else { 0 };
    log[start..].iter()
        .map(|entry| format!("{:?}", entry.event))
        .collect()
}

fn build_reaction_windows(state: &GameState) -> Vec<ReactionWindowObservation> {
    state.reaction_windows.iter().map(|rw| {
        // ReactionWindow has `owner` (the player who can respond).
        // The triggering player is the opponent of the owner.
        let responding = rw.owner;
        let triggering = state.opponent_id(rw.owner);
        ReactionWindowObservation {
            window_type: format!("{:?}", rw.window_type),
            triggering_player: triggering.raw(),
            responding_player: responding.raw(),
        }
    }).collect()
}

fn build_debug_observation(state: &GameState) -> DebugObservation {
    let state_hash = state.compute_hash();
    let full_state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));

    DebugObservation {
        command_history_len: state.command_history.len(),
        event_log_len: state.event_bus.log().len(),
        active_effects_count: state.active_effects.len(),
        deterministic_counter: state.deterministic_counter,
        state_hash,
        full_state_json,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_from_str() {
        assert_eq!(ViewMode::from_str("public"), Some(ViewMode::Public));
        assert_eq!(ViewMode::from_str("player_0"), Some(ViewMode::Player(PlayerId::new(0))));
        assert_eq!(ViewMode::from_str("player_1"), Some(ViewMode::Player(PlayerId::new(1))));
        assert_eq!(ViewMode::from_str("debug"), Some(ViewMode::Debug));
        assert_eq!(ViewMode::from_str("invalid"), None);
    }

    #[test]
    fn test_view_mode_equality() {
        assert_eq!(ViewMode::Public, ViewMode::Public);
        assert_ne!(ViewMode::Public, ViewMode::Debug);
        assert_ne!(
            ViewMode::Player(PlayerId::new(0)),
            ViewMode::Player(PlayerId::new(1))
        );
    }

    #[test]
    fn test_observation_serializes() {
        let obs = GameObservation {
            battle_round: 1,
            phase: "Command".to_string(),
            subphase: "GainCommandPoints".to_string(),
            active_player: 0,
            decision_owner: 0,
            game_outcome: "InProgress".to_string(),
            is_in_progress: true,
            players: [
                PlayerObservation {
                    id: 0,
                    name: "Player 1".to_string(),
                    faction: "Custodes".to_string(),
                    command_points: 0,
                    victory_points: 0,
                    units_alive: 3,
                    units_total: 3,
                    models_alive: 7,
                    models_total: 7,
                    enhancement: Some("Watchman of Terra".to_string()),
                    secondary_objective: None,
                },
                PlayerObservation {
                    id: 1,
                    name: "Player 2".to_string(),
                    faction: "World Eaters".to_string(),
                    command_points: 0,
                    victory_points: 0,
                    units_alive: 4,
                    units_total: 4,
                    models_alive: 22,
                    models_total: 22,
                    enhancement: None,
                    secondary_objective: None,
                },
            ],
            units: Vec::new(),
            objectives: Vec::new(),
            board: BoardObservation {
                width_inches: 44.0,
                height_inches: 30.0,
                terrain_count: 6,
            },
            recent_events: Vec::new(),
            reaction_windows: Vec::new(),
            view_mode: "public".to_string(),
            debug: None,
        };

        let json = serde_json::to_string_pretty(&obs).unwrap();
        assert!(json.contains("Custodes"));
        assert!(json.contains("World Eaters"));
        assert!(json.contains("\"battle_round\": 1"));
    }
}
