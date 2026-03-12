//! MCP Session management.
//!
//! Each session holds a GameState, ReplayRecorder, cached decision surface,
//! and original configuration for reset support.
//!
//! Source: implementation_v3.md Phase 10, mcp_api.md

use std::collections::HashMap;
use std::time::Instant;

use wh40k_core_types::*;
use wh40k_command_system::Command;
use wh40k_game_core::{GameState, ScenarioLoader, CommandExecutor, CommandValidator};
use wh40k_replay::ReplayRecorder;
use wh40k_search_abstraction::{ActionGenerator, CandidateSet};

use crate::error::McpError;

// ============================================================================
// Session
// ============================================================================

/// Configuration used to create a session, preserved for reset.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub faction_a: FactionId,
    pub faction_b: FactionId,
    pub mission: MissionId,
    pub seed: [u8; 32],
    pub enhancement_a: Option<EnhancementId>,
    pub enhancement_b: Option<EnhancementId>,
    pub secondary_a: Option<SecondaryObjectiveId>,
    pub secondary_b: Option<SecondaryObjectiveId>,
}

/// A single game session managed by the MCP server.
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// Current game state.
    pub state: GameState,
    /// Replay recorder capturing all commands/events.
    pub recorder: ReplayRecorder,
    /// Cached legal actions for the current decision point.
    pub cached_actions: Option<CandidateSet>,
    /// Original configuration for reset support.
    pub config: SessionConfig,
    /// Session creation time.
    pub created_at: Instant,
    /// Number of actions applied in this session.
    pub action_count: u64,
}

impl Session {
    /// Create a new session from configuration.
    pub fn new(id: String, config: SessionConfig) -> Result<Self, McpError> {
        let state = Self::load_state_from_config(&config)?;
        let recorder = ReplayRecorder::new(&state);

        Ok(Self {
            id,
            state,
            recorder,
            cached_actions: None,
            config,
            created_at: Instant::now(),
            action_count: 0,
        })
    }

    /// Create a session from a pre-existing GameState.
    pub fn from_state(id: String, state: GameState) -> Self {
        let recorder = ReplayRecorder::new(&state);
        let config = SessionConfig {
            faction_a: state.players[0].faction_id.unwrap_or(FactionId::new(0)),
            faction_b: state.players[1].faction_id.unwrap_or(FactionId::new(1)),
            mission: state.scenario_id.unwrap_or(MissionId::new(1)),
            seed: [0u8; 32],
            enhancement_a: state.players[0].enhancement_choice,
            enhancement_b: state.players[1].enhancement_choice,
            secondary_a: state.players[0].secondary_choice,
            secondary_b: state.players[1].secondary_choice,
        };

        Self {
            id,
            state,
            recorder,
            cached_actions: None,
            config,
            created_at: Instant::now(),
            action_count: 0,
        }
    }

    /// Reset the session to its initial state.
    pub fn reset(&mut self) -> Result<(), McpError> {
        self.state = Self::load_state_from_config(&self.config)?;
        self.recorder = ReplayRecorder::new(&self.state);
        self.cached_actions = None;
        self.action_count = 0;
        Ok(())
    }

    /// Get or generate the current legal actions.
    pub fn get_legal_actions(&mut self) -> &CandidateSet {
        if self.cached_actions.is_none() {
            let perspective = self.state.decision_owner;
            let candidates = ActionGenerator::generate(&self.state, perspective);
            self.cached_actions = Some(candidates);
        }
        self.cached_actions.as_ref().unwrap()
    }

    /// Apply a macro action by index, executing all its commands.
    pub fn apply_action(&mut self, index: usize) -> Result<ApplyResult, McpError> {
        // Get the action from the cached candidates
        let action = {
            let candidates = self.get_legal_actions();
            if index >= candidates.candidates.len() {
                return Err(McpError::ActionIndexOutOfRange {
                    index,
                    available: candidates.candidates.len(),
                });
            }
            candidates.candidates[index].clone()
        };

        let mut events_generated = Vec::new();
        let mut commands_applied = Vec::new();

        // Execute each command in the macro action
        for command in &action.commands {
            // Validate
            let validation = CommandValidator::validate(&self.state, command);
            if !validation.is_legal() {
                return Err(McpError::EngineError(format!(
                    "Command validation failed: {:?} -> {:?}",
                    command, validation
                )));
            }

            // Execute
            match CommandExecutor::execute(&mut self.state, command) {
                Ok(events) => {
                    // Record in replay
                    self.recorder.record_command(&self.state, command, &events, true);
                    events_generated.extend(events);
                    commands_applied.push(command.clone());
                }
                Err(e) => {
                    return Err(McpError::EngineError(format!(
                        "Command execution failed: {:?} -> {}",
                        command, e
                    )));
                }
            }
        }

        self.action_count += 1;
        // Invalidate the action cache since state changed
        self.cached_actions = None;

        Ok(ApplyResult {
            action_label: action.label.clone(),
            intent: format!("{:?}", action.intent),
            commands_count: commands_applied.len(),
            events_count: events_generated.len(),
            game_over: !self.state.is_in_progress(),
        })
    }

    /// Apply a single raw command directly.
    pub fn apply_command(&mut self, command: &Command) -> Result<Vec<wh40k_event_system::GameEvent>, McpError> {
        let validation = CommandValidator::validate(&self.state, command);
        if !validation.is_legal() {
            return Err(McpError::EngineError(format!(
                "Command validation failed: {:?}",
                validation
            )));
        }

        match CommandExecutor::execute(&mut self.state, command) {
            Ok(events) => {
                self.recorder.record_command(&self.state, command, &events, true);
                self.cached_actions = None;
                Ok(events)
            }
            Err(e) => Err(McpError::EngineError(format!("Execution failed: {}", e))),
        }
    }

    /// Load a GameState from a SessionConfig.
    fn load_state_from_config(config: &SessionConfig) -> Result<GameState, McpError> {
        let state = ScenarioLoader::load_scenario(
            config.faction_a,
            config.faction_b,
            Some(config.mission),
            config.seed,
        );
        Ok(state)
    }
}

/// Result of applying an action.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApplyResult {
    pub action_label: String,
    pub intent: String,
    pub commands_count: usize,
    pub events_count: usize,
    pub game_over: bool,
}

// ============================================================================
// Session Manager
// ============================================================================

/// Manages multiple concurrent game sessions.
pub struct SessionManager {
    sessions: HashMap<String, Session>,
    next_id: u64,
}

impl SessionManager {
    /// Create a new empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    /// Generate a new unique session ID.
    pub fn generate_id(&mut self) -> String {
        let id = format!("session_{}", self.next_id);
        self.next_id += 1;
        id
    }

    /// Create a new session and return its ID.
    pub fn create_session(&mut self, config: SessionConfig) -> Result<String, McpError> {
        let id = self.generate_id();
        let session = Session::new(id.clone(), config)?;
        self.sessions.insert(id.clone(), session);
        Ok(id)
    }

    /// Create a session from a pre-existing GameState.
    pub fn create_session_from_state(&mut self, state: GameState) -> String {
        let id = self.generate_id();
        let session = Session::from_state(id.clone(), state);
        self.sessions.insert(id.clone(), session);
        id
    }

    /// Get a reference to a session.
    pub fn get(&self, id: &str) -> Result<&Session, McpError> {
        self.sessions.get(id).ok_or_else(|| McpError::SessionNotFound(id.to_string()))
    }

    /// Get a mutable reference to a session.
    pub fn get_mut(&mut self, id: &str) -> Result<&mut Session, McpError> {
        self.sessions.get_mut(id).ok_or_else(|| McpError::SessionNotFound(id.to_string()))
    }

    /// End and remove a session.
    pub fn end_session(&mut self, id: &str) -> Result<(), McpError> {
        self.sessions.remove(id).ok_or_else(|| McpError::SessionNotFound(id.to_string()))?;
        Ok(())
    }

    /// List all active session IDs.
    pub fn list_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|k| k.as_str()).collect()
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper: Parse faction string to FactionId
// ============================================================================

/// Parse a faction name string into a FactionId.
pub fn parse_faction(name: &str) -> Result<FactionId, McpError> {
    match name.to_lowercase().as_str() {
        "custodes" | "adeptus_custodes" | "adeptus custodes" => Ok(FactionId::new(0)),
        "world_eaters" | "world eaters" | "frenzied_reavers" | "frenzied reavers" => Ok(FactionId::new(1)),
        _ => Err(McpError::InvalidFaction(name.to_string())),
    }
}

/// Parse a mission number into a MissionId.
pub fn parse_mission(mission: u32) -> Result<MissionId, McpError> {
    if mission >= 1 && mission <= 6 {
        Ok(MissionId::new(mission))
    } else {
        Err(McpError::InvalidMission(mission))
    }
}

/// Parse a hex seed string into a 32-byte array.
pub fn parse_seed(seed_str: &str) -> Result<[u8; 32], McpError> {
    let hex = seed_str.trim_start_matches("0x").trim_start_matches("0X");
    if hex.len() > 64 {
        return Err(McpError::InvalidSeed("Seed too long (max 64 hex chars)".to_string()));
    }

    // Pad to 64 chars
    let padded = format!("{:0>64}", hex);
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&padded[i * 2..i * 2 + 2], 16)
            .map_err(|_| McpError::InvalidSeed(format!("Invalid hex at position {}", i * 2)))?;
    }
    Ok(bytes)
}

/// Generate a random seed.
pub fn random_seed() -> [u8; 32] {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    seed
}

/// Format a seed as a hex string.
pub fn seed_to_hex(seed: &[u8; 32]) -> String {
    seed.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_faction_custodes() {
        assert_eq!(parse_faction("custodes").unwrap(), FactionId::new(0));
        assert_eq!(parse_faction("Custodes").unwrap(), FactionId::new(0));
        assert_eq!(parse_faction("adeptus_custodes").unwrap(), FactionId::new(0));
    }

    #[test]
    fn test_parse_faction_world_eaters() {
        assert_eq!(parse_faction("world_eaters").unwrap(), FactionId::new(1));
        assert_eq!(parse_faction("World Eaters").unwrap(), FactionId::new(1));
        assert_eq!(parse_faction("frenzied_reavers").unwrap(), FactionId::new(1));
    }

    #[test]
    fn test_parse_faction_invalid() {
        assert!(parse_faction("orks").is_err());
        assert!(parse_faction("").is_err());
    }

    #[test]
    fn test_parse_mission_valid() {
        for i in 1..=6 {
            assert_eq!(parse_mission(i).unwrap(), MissionId::new(i));
        }
    }

    #[test]
    fn test_parse_mission_invalid() {
        assert!(parse_mission(0).is_err());
        assert!(parse_mission(7).is_err());
    }

    #[test]
    fn test_parse_seed() {
        let seed_str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let seed = parse_seed(seed_str).unwrap();
        assert_eq!(seed[0], 0x01);
        assert_eq!(seed[1], 0x23);
        assert_eq!(seed[15], 0xef);
    }

    #[test]
    fn test_parse_seed_short() {
        let seed = parse_seed("ff").unwrap();
        assert_eq!(seed[31], 0xff);
        assert_eq!(seed[0], 0x00);
    }

    #[test]
    fn test_seed_roundtrip() {
        let original = random_seed();
        let hex = seed_to_hex(&original);
        let parsed = parse_seed(&hex).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_session_manager_new() {
        let manager = SessionManager::new();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_manager_generate_id() {
        let mut manager = SessionManager::new();
        let id1 = manager.generate_id();
        let id2 = manager.generate_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("session_"));
    }

    #[test]
    fn test_session_manager_create_and_get() {
        let mut manager = SessionManager::new();
        let config = SessionConfig {
            faction_a: FactionId::new(0),
            faction_b: FactionId::new(1),
            mission: MissionId::new(1),
            seed: [42u8; 32],
            enhancement_a: None,
            enhancement_b: None,
            secondary_a: None,
            secondary_b: None,
        };

        let id = manager.create_session(config).unwrap();
        assert_eq!(manager.session_count(), 1);
        assert!(manager.get(&id).is_ok());
        assert!(manager.get("nonexistent").is_err());
    }

    #[test]
    fn test_session_manager_end_session() {
        let mut manager = SessionManager::new();
        let config = SessionConfig {
            faction_a: FactionId::new(0),
            faction_b: FactionId::new(1),
            mission: MissionId::new(1),
            seed: [0u8; 32],
            enhancement_a: None,
            enhancement_b: None,
            secondary_a: None,
            secondary_b: None,
        };

        let id = manager.create_session(config).unwrap();
        assert_eq!(manager.session_count(), 1);
        manager.end_session(&id).unwrap();
        assert_eq!(manager.session_count(), 0);
        assert!(manager.end_session(&id).is_err());
    }

    #[test]
    fn test_session_reset() {
        let config = SessionConfig {
            faction_a: FactionId::new(0),
            faction_b: FactionId::new(1),
            mission: MissionId::new(1),
            seed: [42u8; 32],
            enhancement_a: None,
            enhancement_b: None,
            secondary_a: None,
            secondary_b: None,
        };

        let mut session = Session::new("test".to_string(), config).unwrap();
        let hash_before = session.state.compute_hash();
        session.reset().unwrap();
        let hash_after = session.state.compute_hash();
        assert_eq!(hash_before, hash_after);
    }

    #[test]
    fn test_session_get_legal_actions() {
        let config = SessionConfig {
            faction_a: FactionId::new(0),
            faction_b: FactionId::new(1),
            mission: MissionId::new(1),
            seed: [0u8; 32],
            enhancement_a: None,
            enhancement_b: None,
            secondary_a: None,
            secondary_b: None,
        };

        let mut session = Session::new("test".to_string(), config).unwrap();
        let actions = session.get_legal_actions();
        // Should have at least some candidates in the initial state
        assert!(actions.candidates.len() > 0 || !session.state.is_in_progress());
    }
}
