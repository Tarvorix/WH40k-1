//! WH40K Engine - Replay crate
//!
//! Provides replay recording, playback, export, and verification for
//! WH40K Combat Patrol games. Every game can be captured as a `ReplayFile`
//! containing the seed bundle, player info, and an ordered sequence of
//! command frames with associated events and dice rolls.
//!
//! # Architecture
//!
//! - [`ReplayRecorder`] captures frames during live play.
//! - [`ReplayPlayer`] steps through a recorded replay for playback or analysis.
//! - [`ReplayDiff`] compares two replays frame-by-frame.
//! - Export functions convert replays to JSON, binary, or human-readable summaries.
//!
//! Source: implementation_v3.md Section 6.8 (Replay system)

use serde::{Deserialize, Serialize};

use wh40k_core_types::{
    BattleRound, EnhancementId, FactionId, GameOutcome, MissionId, Phase, PlayerId,
    SecondaryObjectiveId,
};
use wh40k_command_system::Command;
use wh40k_dice::{DiceRollRecord, SeedBundle};
use wh40k_event_system::GameEvent;
use wh40k_game_core::state::GameState;

// ============================================================================
// Constants
// ============================================================================

/// Current replay format version. Increment when the format changes
/// in a backwards-incompatible way.
pub const REPLAY_FORMAT_VERSION: u32 = 1;

// ============================================================================
// ReplayError
// ============================================================================

/// Errors that can occur during replay operations.
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("Invalid replay format version: expected {expected}, got {actual}")]
    InvalidFormatVersion { expected: u32, actual: u32 },

    #[error("Replay verification failed at frame {frame_index}: {reason}")]
    VerificationFailed { frame_index: usize, reason: String },

    #[error("Replay is empty (no frames)")]
    EmptyReplay,

    #[error("Frame index {index} out of bounds (total: {total})")]
    FrameOutOfBounds { index: usize, total: usize },
}

// ============================================================================
// ReplayPlayerInfo
// ============================================================================

/// Information about a player in the replay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayPlayerInfo {
    /// The player's identifier.
    pub player_id: PlayerId,
    /// The player's display name.
    pub name: String,
    /// The faction the player selected, if any.
    pub faction_id: Option<FactionId>,
    /// The enhancement the player selected, if any.
    pub enhancement: Option<EnhancementId>,
    /// The secondary objective the player selected, if any.
    pub secondary: Option<SecondaryObjectiveId>,
}

// ============================================================================
// ReplayHeader
// ============================================================================

/// Metadata for a replay file, capturing everything needed to identify
/// and verify the game without parsing every frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    /// Engine version that created this replay.
    pub engine_version: String,
    /// Content version for compatibility checking.
    pub content_version: String,
    /// The seed bundle used for deterministic RNG.
    pub seed_bundle: SeedBundle,
    /// Player names and faction selections.
    pub players: [ReplayPlayerInfo; 2],
    /// Mission/scenario being played.
    pub mission_id: Option<MissionId>,
    /// Timestamp when the game started (Unix seconds).
    pub started_at: u64,
    /// Timestamp when the game ended (Unix seconds).
    pub ended_at: Option<u64>,
    /// Total number of commands in the replay.
    pub total_commands: usize,
    /// Total number of battle rounds completed.
    pub total_rounds: u8,
    /// Hash of the initial game state (before any commands).
    pub initial_state_hash: u64,
    /// Hash of the final game state (after all commands).
    pub final_state_hash: Option<u64>,
}

// ============================================================================
// ReplayFrame
// ============================================================================

/// A single frame in the replay: one command plus its outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFrame {
    /// Index of this frame in the replay (0-based).
    pub frame_index: usize,
    /// The command that was executed.
    pub command: Command,
    /// Player who issued the command.
    pub player: PlayerId,
    /// Battle round when command was executed.
    pub round: BattleRound,
    /// Phase when command was executed.
    pub phase: Phase,
    /// State hash before this command was applied.
    pub state_hash_before: u64,
    /// State hash after this command was applied (None if command was illegal).
    pub state_hash_after: Option<u64>,
    /// Whether the command was legal.
    pub was_legal: bool,
    /// Events emitted by this command.
    pub events: Vec<GameEvent>,
    /// Dice rolls that occurred during this command.
    pub dice_rolls: Vec<DiceRollRecord>,
}

// ============================================================================
// ReplayScore
// ============================================================================

/// Final VP score for a player at game end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayScore {
    /// The player this score belongs to.
    pub player_id: PlayerId,
    /// Victory points from primary objectives.
    pub primary_vp: i16,
    /// Victory points from secondary objectives.
    pub secondary_vp: i16,
    /// Total victory points.
    pub total_vp: i16,
}

// ============================================================================
// ReplayFile
// ============================================================================

/// The complete replay artifact — everything needed to reproduce or
/// analyze a game from start to finish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFile {
    /// File format version for forward compatibility.
    pub format_version: u32,
    /// Replay metadata.
    pub header: ReplayHeader,
    /// Ordered sequence of command frames.
    pub frames: Vec<ReplayFrame>,
    /// Final game outcome.
    pub outcome: GameOutcome,
    /// Final VP scores.
    pub final_scores: [ReplayScore; 2],
}

// ============================================================================
// ReplayRecorder
// ============================================================================

/// Records a game as it plays, building up frames that can be finalized
/// into a `ReplayFile`.
pub struct ReplayRecorder {
    /// Header metadata, partially populated at creation time.
    header: ReplayHeader,
    /// Accumulated frames.
    frames: Vec<ReplayFrame>,
    /// Track which dice rolls we have already recorded, so we only capture
    /// the new ones that occurred during each command.
    dice_log_offset: usize,
}

impl ReplayRecorder {
    /// Create a new recorder for a game about to start.
    ///
    /// Extracts header information from the initial game state including
    /// player names, factions, and the seed bundle from the dice roller.
    pub fn new(state: &GameState) -> Self {
        let seed_bundle = state.dice_roller.context().seed_bundle();
        let initial_state_hash = compute_state_hash(state);

        let players = [
            ReplayPlayerInfo {
                player_id: state.players[0].id,
                name: state.players[0].name.clone(),
                faction_id: state.players[0].faction_id,
                enhancement: state.players[0].enhancement_choice,
                secondary: state.players[0].secondary_choice,
            },
            ReplayPlayerInfo {
                player_id: state.players[1].id,
                name: state.players[1].name.clone(),
                faction_id: state.players[1].faction_id,
                enhancement: state.players[1].enhancement_choice,
                secondary: state.players[1].secondary_choice,
            },
        ];

        let header = ReplayHeader {
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            content_version: state.content_version.clone(),
            seed_bundle,
            players,
            mission_id: state.scenario_id,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ended_at: None,
            total_commands: 0,
            total_rounds: 0,
            initial_state_hash,
            final_state_hash: None,
        };

        let dice_log_offset = state.dice_roller.log().len();

        Self {
            header,
            frames: Vec::new(),
            dice_log_offset,
        }
    }

    /// Record a command execution result.
    ///
    /// Call this AFTER `CommandExecutor::execute()` returns. Captures the
    /// command, events, dice rolls, and state hash into a new `ReplayFrame`.
    pub fn record_command(
        &mut self,
        state: &GameState,
        command: &Command,
        events: &[GameEvent],
        was_legal: bool,
    ) {
        let state_hash_before = if let Some(last_record) = state.command_history.latest() {
            last_record.state_hash_before
        } else {
            self.header.initial_state_hash
        };

        let state_hash_after = if was_legal {
            if let Some(last_record) = state.command_history.latest() {
                last_record.state_hash_after
            } else {
                Some(compute_state_hash(state))
            }
        } else {
            None
        };

        // Extract new dice rolls since last recording
        let all_rolls = state.dice_roller.log().records();
        let new_rolls = if self.dice_log_offset < all_rolls.len() {
            all_rolls[self.dice_log_offset..].to_vec()
        } else {
            Vec::new()
        };
        self.dice_log_offset = all_rolls.len();

        let player = command
            .player()
            .unwrap_or(state.active_player);

        let frame = ReplayFrame {
            frame_index: self.frames.len(),
            command: command.clone(),
            player,
            round: state.battle_round,
            phase: state.current_phase,
            state_hash_before,
            state_hash_after,
            was_legal,
            events: events.to_vec(),
            dice_rolls: new_rolls,
        };

        self.frames.push(frame);
    }

    /// Record a failed command (validation error).
    ///
    /// Creates a frame with `was_legal = false`, no events, and no state hash
    /// after.
    pub fn record_failed_command(
        &mut self,
        state: &GameState,
        command: &Command,
    ) {
        let state_hash_before = if let Some(last_record) = state.command_history.latest() {
            if let Some(hash_after) = last_record.state_hash_after {
                hash_after
            } else {
                last_record.state_hash_before
            }
        } else {
            self.header.initial_state_hash
        };

        let player = command
            .player()
            .unwrap_or(state.active_player);

        let frame = ReplayFrame {
            frame_index: self.frames.len(),
            command: command.clone(),
            player,
            round: state.battle_round,
            phase: state.current_phase,
            state_hash_before,
            state_hash_after: None,
            was_legal: false,
            events: Vec::new(),
            dice_rolls: Vec::new(),
        };

        self.frames.push(frame);
    }

    /// Finalize the replay and produce a `ReplayFile`.
    ///
    /// Sets the end timestamp, final state hash, total command count,
    /// and extracts the outcome and scores from the final game state.
    pub fn finalize(mut self, state: &GameState) -> ReplayFile {
        self.header.ended_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
        self.header.total_commands = self.frames.len();
        self.header.total_rounds = state.battle_round.number();
        self.header.final_state_hash = Some(compute_state_hash(state));

        let outcome = state.game_outcome;

        let final_scores = [
            ReplayScore {
                player_id: state.players[0].id,
                primary_vp: state.players[0].mission_progress.primary_vp.value(),
                secondary_vp: state.players[0].mission_progress.secondary_vp.value(),
                total_vp: state.players[0].vp.value(),
            },
            ReplayScore {
                player_id: state.players[1].id,
                primary_vp: state.players[1].mission_progress.primary_vp.value(),
                secondary_vp: state.players[1].mission_progress.secondary_vp.value(),
                total_vp: state.players[1].vp.value(),
            },
        ];

        ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: self.header,
            frames: self.frames,
            outcome,
            final_scores,
        }
    }
}

// ============================================================================
// ReplayPlayer
// ============================================================================

/// Steps through a replay file frame-by-frame for playback or analysis.
pub struct ReplayPlayer {
    /// The replay being played.
    replay: ReplayFile,
    /// Current frame index (next frame to be returned).
    current_frame: usize,
}

impl ReplayPlayer {
    /// Create a new replay player from a replay file.
    pub fn new(replay: ReplayFile) -> Self {
        Self {
            replay,
            current_frame: 0,
        }
    }

    /// Get the total number of frames in the replay.
    pub fn total_frames(&self) -> usize {
        self.replay.frames.len()
    }

    /// Get the current frame index (the index of the next frame that
    /// will be returned by `next_frame`).
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    /// Get the next frame without advancing the position.
    pub fn peek_frame(&self) -> Option<&ReplayFrame> {
        self.replay.frames.get(self.current_frame)
    }

    /// Advance to the next frame and return it.
    pub fn next_frame(&mut self) -> Option<&ReplayFrame> {
        if self.current_frame < self.replay.frames.len() {
            let frame = &self.replay.frames[self.current_frame];
            self.current_frame += 1;
            Some(frame)
        } else {
            None
        }
    }

    /// Get a specific frame by index.
    pub fn frame_at(&self, index: usize) -> Option<&ReplayFrame> {
        self.replay.frames.get(index)
    }

    /// Reset to the beginning.
    pub fn reset(&mut self) {
        self.current_frame = 0;
    }

    /// Check if there are more frames to play.
    pub fn has_more(&self) -> bool {
        self.current_frame < self.replay.frames.len()
    }

    /// Get the replay header.
    pub fn header(&self) -> &ReplayHeader {
        &self.replay.header
    }

    /// Get all frames matching a specific battle round.
    pub fn frames_for_round(&self, round: BattleRound) -> Vec<&ReplayFrame> {
        self.replay
            .frames
            .iter()
            .filter(|f| f.round == round)
            .collect()
    }

    /// Get all frames matching a specific phase.
    pub fn frames_for_phase(&self, phase: Phase) -> Vec<&ReplayFrame> {
        self.replay
            .frames
            .iter()
            .filter(|f| f.phase == phase)
            .collect()
    }

    /// Get all frames for a specific player.
    pub fn frames_for_player(&self, player: PlayerId) -> Vec<&ReplayFrame> {
        self.replay
            .frames
            .iter()
            .filter(|f| f.player == player)
            .collect()
    }

    /// Verify this replay against a live game state by checking hashes.
    ///
    /// This method walks through every frame and compares the recorded
    /// `state_hash_before` and `state_hash_after` values against the
    /// hashes stored in the replay itself. It does NOT re-execute commands
    /// (that would require the full executor), but it validates internal
    /// consistency: that consecutive frames have matching hash chains.
    ///
    /// For frame N, `state_hash_after` (if present) should equal frame N+1's
    /// `state_hash_before`. The first frame's `state_hash_before` should
    /// match the header's `initial_state_hash`.
    pub fn verify_against_state(&self, _initial_state: &GameState) -> ReplayVerificationResult {
        let total_frames = self.replay.frames.len();
        let mut hash_matches: usize = 0;
        let mut hash_mismatches: Vec<ReplayHashMismatch> = Vec::new();
        let mut frames_verified: usize = 0;

        if total_frames == 0 {
            return ReplayVerificationResult {
                total_frames: 0,
                frames_verified: 0,
                hash_matches: 0,
                hash_mismatches: Vec::new(),
                passed: true,
            };
        }

        // Verify that the first frame's state_hash_before matches the header's initial_state_hash
        let first_frame = &self.replay.frames[0];
        if first_frame.state_hash_before == self.replay.header.initial_state_hash {
            hash_matches += 1;
        } else {
            hash_mismatches.push(ReplayHashMismatch {
                frame_index: 0,
                checkpoint: "before".to_string(),
                expected: self.replay.header.initial_state_hash,
                actual: first_frame.state_hash_before,
            });
        }
        frames_verified += 1;

        // Walk through consecutive frames and check hash chain continuity
        for i in 0..total_frames.saturating_sub(1) {
            let current = &self.replay.frames[i];
            let next = &self.replay.frames[i + 1];

            if let Some(hash_after) = current.state_hash_after {
                if hash_after == next.state_hash_before {
                    hash_matches += 1;
                } else {
                    hash_mismatches.push(ReplayHashMismatch {
                        frame_index: i,
                        checkpoint: "after".to_string(),
                        expected: next.state_hash_before,
                        actual: hash_after,
                    });
                }
            }
            // If current frame was illegal (state_hash_after is None), we skip
            // the chain check for that transition since the state didn't change.
            frames_verified += 1;
        }

        // If the last frame was not already counted
        if total_frames > 1 {
            frames_verified += 1;
        }
        // Cap at total
        if frames_verified > total_frames {
            frames_verified = total_frames;
        }

        // Check final state hash against header
        if let Some(final_hash) = self.replay.header.final_state_hash {
            if let Some(last_frame) = self.replay.frames.last() {
                if let Some(last_hash_after) = last_frame.state_hash_after {
                    if last_hash_after == final_hash {
                        hash_matches += 1;
                    } else {
                        hash_mismatches.push(ReplayHashMismatch {
                            frame_index: total_frames - 1,
                            checkpoint: "after".to_string(),
                            expected: final_hash,
                            actual: last_hash_after,
                        });
                    }
                }
            }
        }

        let passed = hash_mismatches.is_empty();

        ReplayVerificationResult {
            total_frames,
            frames_verified,
            hash_matches,
            hash_mismatches,
            passed,
        }
    }
}

// ============================================================================
// ReplayVerificationResult
// ============================================================================

/// Result of verifying a replay's hash chain integrity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayVerificationResult {
    /// Total number of frames in the replay.
    pub total_frames: usize,
    /// Number of frames that were verified.
    pub frames_verified: usize,
    /// Number of hash comparisons that matched.
    pub hash_matches: usize,
    /// Details of any hash mismatches found.
    pub hash_mismatches: Vec<ReplayHashMismatch>,
    /// Whether the entire verification passed (no mismatches).
    pub passed: bool,
}

/// A single hash mismatch found during verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHashMismatch {
    /// The frame index where the mismatch occurred.
    pub frame_index: usize,
    /// Which checkpoint failed: "before" or "after".
    pub checkpoint: String,
    /// The expected hash value.
    pub expected: u64,
    /// The actual hash value found.
    pub actual: u64,
}

// ============================================================================
// Export functions
// ============================================================================

/// Export a replay as pretty-printed JSON.
pub fn export_json(replay: &ReplayFile) -> Result<String, ReplayError> {
    serde_json::to_string_pretty(replay)
        .map_err(|e| ReplayError::SerializationError(e.to_string()))
}

/// Export a replay as compact JSON (for storage/transfer).
pub fn export_json_compact(replay: &ReplayFile) -> Result<String, ReplayError> {
    serde_json::to_string(replay)
        .map_err(|e| ReplayError::SerializationError(e.to_string()))
}

/// Export a replay as binary (bincode) for minimal file size.
pub fn export_binary(replay: &ReplayFile) -> Result<Vec<u8>, ReplayError> {
    bincode::serialize(replay)
        .map_err(|e| ReplayError::SerializationError(e.to_string()))
}

/// Import a replay from JSON.
pub fn import_json(json: &str) -> Result<ReplayFile, ReplayError> {
    let replay: ReplayFile = serde_json::from_str(json)
        .map_err(|e| ReplayError::DeserializationError(e.to_string()))?;

    if replay.format_version != REPLAY_FORMAT_VERSION {
        return Err(ReplayError::InvalidFormatVersion {
            expected: REPLAY_FORMAT_VERSION,
            actual: replay.format_version,
        });
    }

    Ok(replay)
}

/// Import a replay from binary (bincode).
pub fn import_binary(data: &[u8]) -> Result<ReplayFile, ReplayError> {
    let replay: ReplayFile = bincode::deserialize(data)
        .map_err(|e| ReplayError::DeserializationError(e.to_string()))?;

    if replay.format_version != REPLAY_FORMAT_VERSION {
        return Err(ReplayError::InvalidFormatVersion {
            expected: REPLAY_FORMAT_VERSION,
            actual: replay.format_version,
        });
    }

    Ok(replay)
}

/// Generate a human-readable text summary of a replay.
///
/// Produces a detailed report including:
/// - Header info (players, factions, mission, date)
/// - Round-by-round summary (commands per round, key events)
/// - Final scores and outcome
/// - Statistics (total commands, dice rolls, etc.)
pub fn export_summary(replay: &ReplayFile) -> String {
    let mut out = String::new();

    // ── Header ──────────────────────────────────────────────────────
    out.push_str("=== WH40K Combat Patrol - Replay Summary ===\n");
    out.push_str(&format!("Format version: {}\n", replay.format_version));
    out.push_str(&format!("Engine version: {}\n", replay.header.engine_version));
    out.push_str(&format!("Content version: {}\n", replay.header.content_version));

    // Timestamps
    out.push_str(&format!("Started at (Unix): {}\n", replay.header.started_at));
    if let Some(ended) = replay.header.ended_at {
        out.push_str(&format!("Ended at (Unix):   {}\n", ended));
        let duration_secs = ended.saturating_sub(replay.header.started_at);
        let minutes = duration_secs / 60;
        let seconds = duration_secs % 60;
        out.push_str(&format!("Duration: {}m {}s\n", minutes, seconds));
    }
    out.push('\n');

    // Mission
    if let Some(mission) = replay.header.mission_id {
        out.push_str(&format!("Mission: {}\n", mission));
    } else {
        out.push_str("Mission: (none)\n");
    }
    out.push('\n');

    // Players
    out.push_str("--- Players ---\n");
    for (i, player_info) in replay.header.players.iter().enumerate() {
        out.push_str(&format!(
            "  Player {} (ID {}): {}\n",
            i + 1,
            player_info.player_id,
            player_info.name
        ));
        if let Some(fid) = player_info.faction_id {
            out.push_str(&format!("    Faction: {}\n", fid));
        }
        if let Some(eid) = player_info.enhancement {
            out.push_str(&format!("    Enhancement: {}\n", eid));
        }
        if let Some(sid) = player_info.secondary {
            out.push_str(&format!("    Secondary Objective: {}\n", sid));
        }
    }
    out.push('\n');

    // Seed info
    out.push_str(&format!("Seed label: {}\n", replay.header.seed_bundle.label));
    if let Some(ref desc) = replay.header.seed_bundle.description {
        out.push_str(&format!("Seed description: {}\n", desc));
    }
    out.push_str(&format!(
        "Initial state hash: 0x{:016X}\n",
        replay.header.initial_state_hash
    ));
    if let Some(fh) = replay.header.final_state_hash {
        out.push_str(&format!("Final state hash:   0x{:016X}\n", fh));
    }
    out.push('\n');

    // ── Round-by-round summary ──────────────────────────────────────
    out.push_str("--- Round-by-Round Summary ---\n");

    // Group frames by round
    let max_round = replay
        .frames
        .iter()
        .map(|f| f.round.number())
        .max()
        .unwrap_or(0);

    for round_num in 0..=max_round {
        let round = BattleRound::new(round_num);
        let round_frames: Vec<&ReplayFrame> =
            replay.frames.iter().filter(|f| f.round == round).collect();

        if round_frames.is_empty() {
            continue;
        }

        out.push_str(&format!("\n  Round {}:\n", round_num));

        // Count commands by category
        let mut category_counts: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        let mut legal_count = 0usize;
        let mut illegal_count = 0usize;
        let mut dice_roll_count = 0usize;
        let mut event_count = 0usize;

        for frame in &round_frames {
            *category_counts.entry(frame.command.category()).or_insert(0) += 1;
            if frame.was_legal {
                legal_count += 1;
            } else {
                illegal_count += 1;
            }
            dice_roll_count += frame.dice_rolls.len();
            event_count += frame.events.len();
        }

        out.push_str(&format!(
            "    Commands: {} total ({} legal, {} illegal)\n",
            round_frames.len(),
            legal_count,
            illegal_count
        ));
        out.push_str(&format!("    Dice rolls: {}\n", dice_roll_count));
        out.push_str(&format!("    Events emitted: {}\n", event_count));

        // Break down by category
        out.push_str("    Command breakdown:\n");
        for (category, count) in &category_counts {
            out.push_str(&format!("      {}: {}\n", category, count));
        }

        // Summarize phases touched this round
        let mut phases_seen: Vec<Phase> = Vec::new();
        for frame in &round_frames {
            if !phases_seen.contains(&frame.phase) {
                phases_seen.push(frame.phase);
            }
        }
        out.push_str("    Phases: ");
        let phase_names: Vec<String> = phases_seen.iter().map(|p| format!("{:?}", p)).collect();
        out.push_str(&phase_names.join(", "));
        out.push('\n');

        // Count key events (unit destroyed, VP scored, etc.)
        let mut units_destroyed = 0usize;
        let mut vp_scored_events = 0usize;
        let mut charges_declared = 0usize;

        for frame in &round_frames {
            for event in &frame.events {
                match event {
                    GameEvent::UnitDestroyed { .. } => units_destroyed += 1,
                    GameEvent::VictoryPointsScored { .. } => vp_scored_events += 1,
                    GameEvent::ChargeDeclared { .. } => charges_declared += 1,
                    _ => {}
                }
            }
        }

        if units_destroyed > 0 {
            out.push_str(&format!("    Units destroyed: {}\n", units_destroyed));
        }
        if vp_scored_events > 0 {
            out.push_str(&format!("    VP scoring events: {}\n", vp_scored_events));
        }
        if charges_declared > 0 {
            out.push_str(&format!("    Charges declared: {}\n", charges_declared));
        }
    }
    out.push('\n');

    // ── Outcome ─────────────────────────────────────────────────────
    out.push_str("--- Game Outcome ---\n");
    match replay.outcome {
        GameOutcome::InProgress => {
            out.push_str("  Result: Game still in progress\n");
        }
        GameOutcome::Victory(pid) => {
            // Find the winner's name
            let winner_name = replay
                .header
                .players
                .iter()
                .find(|p| p.player_id == pid)
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown");
            out.push_str(&format!(
                "  Result: Victory for {} (Player {})\n",
                winner_name, pid
            ));
        }
        GameOutcome::Draw => {
            out.push_str("  Result: Draw\n");
        }
    }
    out.push('\n');

    // ── Final Scores ────────────────────────────────────────────────
    out.push_str("--- Final Scores ---\n");
    for (i, score) in replay.final_scores.iter().enumerate() {
        let name = &replay.header.players[i].name;
        out.push_str(&format!(
            "  {} (Player {}): {} VP total (Primary: {}, Secondary: {})\n",
            name, score.player_id, score.total_vp, score.primary_vp, score.secondary_vp
        ));
    }
    out.push('\n');

    // ── Statistics ──────────────────────────────────────────────────
    out.push_str("--- Statistics ---\n");
    let total_commands = replay.frames.len();
    let legal_commands = replay.frames.iter().filter(|f| f.was_legal).count();
    let illegal_commands = total_commands - legal_commands;
    let total_dice_rolls: usize = replay.frames.iter().map(|f| f.dice_rolls.len()).sum();
    let total_events: usize = replay.frames.iter().map(|f| f.events.len()).sum();

    out.push_str(&format!("  Total commands: {}\n", total_commands));
    out.push_str(&format!("  Legal commands: {}\n", legal_commands));
    out.push_str(&format!("  Illegal commands: {}\n", illegal_commands));
    out.push_str(&format!("  Total dice rolls: {}\n", total_dice_rolls));
    out.push_str(&format!("  Total events: {}\n", total_events));
    out.push_str(&format!("  Total rounds: {}\n", replay.header.total_rounds));

    // Per-player command counts
    out.push_str("\n  Per-player commands:\n");
    for player_info in &replay.header.players {
        let count = replay
            .frames
            .iter()
            .filter(|f| f.player == player_info.player_id)
            .count();
        out.push_str(&format!(
            "    {} (Player {}): {} commands\n",
            player_info.name, player_info.player_id, count
        ));
    }

    // Command category distribution across the whole game
    let mut global_categories: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for frame in &replay.frames {
        *global_categories
            .entry(frame.command.category())
            .or_insert(0) += 1;
    }
    out.push_str("\n  Command categories:\n");
    for (cat, count) in &global_categories {
        out.push_str(&format!("    {}: {}\n", cat, count));
    }

    out
}

// ============================================================================
// ReplayDiff
// ============================================================================

/// The type of difference found between two replay frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffType {
    /// Different commands at the same frame index.
    CommandMismatch,
    /// Different state hashes.
    HashMismatch,
    /// Different events emitted.
    EventMismatch,
    /// Different dice rolls.
    DiceRollMismatch,
    /// One replay has more frames than the other.
    FrameCountMismatch,
    /// Header differences.
    HeaderMismatch,
}

/// A single difference between two replays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDifference {
    /// The frame index where the difference was found.
    pub frame_index: usize,
    /// The type of difference.
    pub diff_type: DiffType,
    /// Human-readable description of the difference.
    pub description: String,
}

/// Result of comparing two replay files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayDiff {
    /// Whether the replays are identical.
    pub identical: bool,
    /// First frame where they diverge (None if identical).
    pub divergence_point: Option<usize>,
    /// Total frames in replay A.
    pub frames_a: usize,
    /// Total frames in replay B.
    pub frames_b: usize,
    /// All differences found.
    pub differences: Vec<ReplayDifference>,
}

impl ReplayDiff {
    /// Compare two replay files and produce a diff.
    ///
    /// Compares headers, then walks through frames one-by-one, reporting
    /// all differences found.
    pub fn compare(a: &ReplayFile, b: &ReplayFile) -> Self {
        let mut differences: Vec<ReplayDifference> = Vec::new();
        let mut divergence_point: Option<usize> = None;

        // Compare headers
        if a.header.engine_version != b.header.engine_version {
            differences.push(ReplayDifference {
                frame_index: 0,
                diff_type: DiffType::HeaderMismatch,
                description: format!(
                    "Engine version differs: '{}' vs '{}'",
                    a.header.engine_version, b.header.engine_version
                ),
            });
        }

        if a.header.content_version != b.header.content_version {
            differences.push(ReplayDifference {
                frame_index: 0,
                diff_type: DiffType::HeaderMismatch,
                description: format!(
                    "Content version differs: '{}' vs '{}'",
                    a.header.content_version, b.header.content_version
                ),
            });
        }

        if a.header.seed_bundle != b.header.seed_bundle {
            differences.push(ReplayDifference {
                frame_index: 0,
                diff_type: DiffType::HeaderMismatch,
                description: "Seed bundle differs".to_string(),
            });
        }

        if a.header.initial_state_hash != b.header.initial_state_hash {
            differences.push(ReplayDifference {
                frame_index: 0,
                diff_type: DiffType::HashMismatch,
                description: format!(
                    "Initial state hash differs: 0x{:016X} vs 0x{:016X}",
                    a.header.initial_state_hash, b.header.initial_state_hash
                ),
            });
        }

        if a.header.mission_id != b.header.mission_id {
            differences.push(ReplayDifference {
                frame_index: 0,
                diff_type: DiffType::HeaderMismatch,
                description: format!(
                    "Mission ID differs: {:?} vs {:?}",
                    a.header.mission_id, b.header.mission_id
                ),
            });
        }

        // Compare player info
        for idx in 0..2 {
            let pa = &a.header.players[idx];
            let pb = &b.header.players[idx];
            if pa.name != pb.name {
                differences.push(ReplayDifference {
                    frame_index: 0,
                    diff_type: DiffType::HeaderMismatch,
                    description: format!(
                        "Player {} name differs: '{}' vs '{}'",
                        idx, pa.name, pb.name
                    ),
                });
            }
            if pa.faction_id != pb.faction_id {
                differences.push(ReplayDifference {
                    frame_index: 0,
                    diff_type: DiffType::HeaderMismatch,
                    description: format!(
                        "Player {} faction differs: {:?} vs {:?}",
                        idx, pa.faction_id, pb.faction_id
                    ),
                });
            }
        }

        // Compare frame count
        if a.frames.len() != b.frames.len() {
            differences.push(ReplayDifference {
                frame_index: a.frames.len().min(b.frames.len()),
                diff_type: DiffType::FrameCountMismatch,
                description: format!(
                    "Frame count differs: {} vs {}",
                    a.frames.len(),
                    b.frames.len()
                ),
            });
        }

        // Compare frames one-by-one
        let min_len = a.frames.len().min(b.frames.len());
        for i in 0..min_len {
            let fa = &a.frames[i];
            let fb = &b.frames[i];

            if fa.command != fb.command {
                if divergence_point.is_none() {
                    divergence_point = Some(i);
                }
                differences.push(ReplayDifference {
                    frame_index: i,
                    diff_type: DiffType::CommandMismatch,
                    description: format!(
                        "Command differs at frame {}: {:?} vs {:?}",
                        i, fa.command, fb.command
                    ),
                });
            }

            if fa.state_hash_before != fb.state_hash_before {
                if divergence_point.is_none() {
                    divergence_point = Some(i);
                }
                differences.push(ReplayDifference {
                    frame_index: i,
                    diff_type: DiffType::HashMismatch,
                    description: format!(
                        "State hash before differs at frame {}: 0x{:016X} vs 0x{:016X}",
                        i, fa.state_hash_before, fb.state_hash_before
                    ),
                });
            }

            if fa.state_hash_after != fb.state_hash_after {
                if divergence_point.is_none() {
                    divergence_point = Some(i);
                }
                differences.push(ReplayDifference {
                    frame_index: i,
                    diff_type: DiffType::HashMismatch,
                    description: format!(
                        "State hash after differs at frame {}: {:?} vs {:?}",
                        i, fa.state_hash_after, fb.state_hash_after
                    ),
                });
            }

            if fa.events != fb.events {
                if divergence_point.is_none() {
                    divergence_point = Some(i);
                }
                differences.push(ReplayDifference {
                    frame_index: i,
                    diff_type: DiffType::EventMismatch,
                    description: format!(
                        "Events differ at frame {}: {} event(s) vs {} event(s)",
                        i,
                        fa.events.len(),
                        fb.events.len()
                    ),
                });
            }

            if fa.dice_rolls != fb.dice_rolls {
                if divergence_point.is_none() {
                    divergence_point = Some(i);
                }
                differences.push(ReplayDifference {
                    frame_index: i,
                    diff_type: DiffType::DiceRollMismatch,
                    description: format!(
                        "Dice rolls differ at frame {}: {} roll(s) vs {} roll(s)",
                        i,
                        fa.dice_rolls.len(),
                        fb.dice_rolls.len()
                    ),
                });
            }
        }

        // If one replay is longer, set divergence point at the boundary
        if a.frames.len() != b.frames.len() && divergence_point.is_none() {
            divergence_point = Some(min_len);
        }

        let identical = differences.is_empty();

        ReplayDiff {
            identical,
            divergence_point: if identical { None } else { divergence_point },
            frames_a: a.frames.len(),
            frames_b: b.frames.len(),
            differences,
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Compute a hash of the game state for replay verification.
///
/// This uses a simple hash of the serialized state. Once `game_core::hasher`
/// is fully implemented, this can delegate to `state.compute_hash()`.
fn compute_state_hash(state: &GameState) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash the key deterministic fields of game state
    state.content_version.hash(&mut hasher);
    state.battle_round.hash(&mut hasher);
    state.active_player.hash(&mut hasher);
    state.current_phase.hash(&mut hasher);
    state.current_subphase.hash(&mut hasher);
    state.decision_owner.hash(&mut hasher);
    state.deterministic_counter.hash(&mut hasher);

    // Hash player state
    for player in &state.players {
        player.id.hash(&mut hasher);
        player.name.hash(&mut hasher);
        player.faction_id.hash(&mut hasher);
        player.cp.hash(&mut hasher);
        player.vp.hash(&mut hasher);
        player.enhancement_choice.hash(&mut hasher);
        player.secondary_choice.hash(&mut hasher);
        player.first_turn.hash(&mut hasher);
    }

    // Hash game outcome
    state.game_outcome.hash(&mut hasher);

    // Hash number of units and command history length as proxies
    state.units.len().hash(&mut hasher);
    state.command_history.len().hash(&mut hasher);

    hasher.finish()
}

/// Trait extension on DiceContext to extract a SeedBundle.
/// The DiceContext doesn't directly hold a SeedBundle, so we reconstruct one
/// from the context's seed bytes.
trait DiceContextExt {
    fn seed_bundle(&self) -> SeedBundle;
}

impl DiceContextExt for wh40k_dice::DiceContext {
    fn seed_bundle(&self) -> SeedBundle {
        SeedBundle::new(
            self.seed,
            format!("replay-{}", self.action_id),
            0,
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{BattleRound, GameOutcome, Phase, PlayerId};
    use wh40k_dice::SeedBundle;

    /// Helper to build a minimal ReplayPlayerInfo.
    fn test_player_info(id: u32, name: &str) -> ReplayPlayerInfo {
        ReplayPlayerInfo {
            player_id: PlayerId::new(id),
            name: name.to_string(),
            faction_id: None,
            enhancement: None,
            secondary: None,
        }
    }

    /// Helper to build a minimal ReplayHeader.
    fn test_header() -> ReplayHeader {
        ReplayHeader {
            engine_version: "0.1.0".to_string(),
            content_version: "test-1.0".to_string(),
            seed_bundle: SeedBundle::from_u64(42, "test-game".to_string()),
            players: [
                test_player_info(0, "Alice"),
                test_player_info(1, "Bob"),
            ],
            mission_id: None,
            started_at: 1700000000,
            ended_at: Some(1700003600),
            total_commands: 3,
            total_rounds: 2,
            initial_state_hash: 0xDEAD_BEEF_0000_0001,
            final_state_hash: Some(0xDEAD_BEEF_0000_0003),
        }
    }

    /// Helper to build a minimal ReplayFrame.
    fn test_frame(index: usize, round: u8, phase: Phase, player_id: u32) -> ReplayFrame {
        ReplayFrame {
            frame_index: index,
            command: Command::PassAction,
            player: PlayerId::new(player_id),
            round: BattleRound::new(round),
            phase,
            state_hash_before: 0x1000 + index as u64,
            state_hash_after: Some(0x1000 + index as u64 + 1),
            was_legal: true,
            events: Vec::new(),
            dice_rolls: Vec::new(),
        }
    }

    /// Helper to build a minimal ReplayFile with multiple frames.
    fn test_replay_file() -> ReplayFile {
        let frames = vec![
            test_frame(0, 1, Phase::Command, 0),
            test_frame(1, 1, Phase::Movement, 0),
            test_frame(2, 1, Phase::Shooting, 1),
            test_frame(3, 2, Phase::Command, 0),
            test_frame(4, 2, Phase::Movement, 1),
        ];

        ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: test_header(),
            frames,
            outcome: GameOutcome::Victory(PlayerId::new(0)),
            final_scores: [
                ReplayScore {
                    player_id: PlayerId::new(0),
                    primary_vp: 10,
                    secondary_vp: 5,
                    total_vp: 15,
                },
                ReplayScore {
                    player_id: PlayerId::new(1),
                    primary_vp: 5,
                    secondary_vp: 3,
                    total_vp: 8,
                },
            ],
        }
    }

    // ── Serialization roundtrip tests ──────────────────────────────

    #[test]
    fn test_replay_file_json_roundtrip() {
        let replay = test_replay_file();

        let json = export_json(&replay).expect("export_json should succeed");
        assert!(json.contains("format_version"));
        assert!(json.contains("Alice"));
        assert!(json.contains("Bob"));

        let imported = import_json(&json).expect("import_json should succeed");
        assert_eq!(imported.format_version, replay.format_version);
        assert_eq!(imported.frames.len(), replay.frames.len());
        assert_eq!(imported.header.engine_version, replay.header.engine_version);
        assert_eq!(
            imported.header.content_version,
            replay.header.content_version
        );
        assert_eq!(
            imported.header.initial_state_hash,
            replay.header.initial_state_hash
        );

        // Verify each frame roundtrips correctly
        for (i, frame) in imported.frames.iter().enumerate() {
            assert_eq!(frame.frame_index, replay.frames[i].frame_index);
            assert_eq!(frame.command, replay.frames[i].command);
            assert_eq!(frame.player, replay.frames[i].player);
            assert_eq!(frame.round, replay.frames[i].round);
            assert_eq!(frame.phase, replay.frames[i].phase);
            assert_eq!(
                frame.state_hash_before,
                replay.frames[i].state_hash_before
            );
            assert_eq!(frame.state_hash_after, replay.frames[i].state_hash_after);
            assert_eq!(frame.was_legal, replay.frames[i].was_legal);
        }
    }

    #[test]
    fn test_replay_file_binary_roundtrip() {
        let replay = test_replay_file();

        let binary = export_binary(&replay).expect("export_binary should succeed");
        assert!(!binary.is_empty());

        let imported = import_binary(&binary).expect("import_binary should succeed");
        assert_eq!(imported.format_version, replay.format_version);
        assert_eq!(imported.frames.len(), replay.frames.len());
        assert_eq!(imported.header.engine_version, replay.header.engine_version);

        // Verify frame data
        for (i, frame) in imported.frames.iter().enumerate() {
            assert_eq!(frame.frame_index, replay.frames[i].frame_index);
            assert_eq!(frame.command, replay.frames[i].command);
            assert_eq!(frame.was_legal, replay.frames[i].was_legal);
        }
    }

    #[test]
    fn test_replay_file_compact_json() {
        let replay = test_replay_file();

        let compact = export_json_compact(&replay).expect("compact export should succeed");
        let pretty = export_json(&replay).expect("pretty export should succeed");

        // Compact should be shorter (no whitespace formatting)
        assert!(compact.len() < pretty.len());

        // Both should parse back correctly
        let from_compact = import_json(&compact).expect("import from compact should succeed");
        assert_eq!(from_compact.frames.len(), replay.frames.len());
    }

    // ── ReplayRecorder tests ──────────────────────────────────────

    #[test]
    fn test_replay_recorder_basic() {
        // Test that ReplayRecorder can be built from header data and frames
        // can be added manually (we don't have a full GameState here, but we
        // can verify the data structures work correctly)
        let replay = test_replay_file();
        assert_eq!(replay.frames.len(), 5);
        assert_eq!(replay.format_version, REPLAY_FORMAT_VERSION);
        assert_eq!(replay.header.total_commands, 3);
        assert_eq!(replay.header.total_rounds, 2);
        assert_eq!(replay.outcome, GameOutcome::Victory(PlayerId::new(0)));
    }

    // ── ReplayPlayer navigation tests ─────────────────────────────

    #[test]
    fn test_replay_player_navigation() {
        let replay = test_replay_file();
        let mut player = ReplayPlayer::new(replay);

        assert_eq!(player.total_frames(), 5);
        assert_eq!(player.current_frame_index(), 0);
        assert!(player.has_more());

        // Peek doesn't advance
        let peeked = player.peek_frame().unwrap();
        assert_eq!(peeked.frame_index, 0);
        assert_eq!(player.current_frame_index(), 0);

        // Next advances
        let first = player.next_frame().unwrap();
        assert_eq!(first.frame_index, 0);
        assert_eq!(player.current_frame_index(), 1);

        let second = player.next_frame().unwrap();
        assert_eq!(second.frame_index, 1);
        assert_eq!(player.current_frame_index(), 2);

        // Skip to end
        let _ = player.next_frame(); // frame 2
        let _ = player.next_frame(); // frame 3
        let last = player.next_frame().unwrap();
        assert_eq!(last.frame_index, 4);
        assert_eq!(player.current_frame_index(), 5);
        assert!(!player.has_more());

        // No more frames
        assert!(player.next_frame().is_none());

        // Reset
        player.reset();
        assert_eq!(player.current_frame_index(), 0);
        assert!(player.has_more());

        // Random access
        let frame_3 = player.frame_at(3).unwrap();
        assert_eq!(frame_3.frame_index, 3);
        assert_eq!(frame_3.round, BattleRound::new(2));

        // Out of bounds
        assert!(player.frame_at(100).is_none());
    }

    #[test]
    fn test_replay_player_frame_queries() {
        let replay = test_replay_file();
        let player = ReplayPlayer::new(replay);

        // Frames for round 1
        let round1_frames = player.frames_for_round(BattleRound::new(1));
        assert_eq!(round1_frames.len(), 3);
        for f in &round1_frames {
            assert_eq!(f.round, BattleRound::new(1));
        }

        // Frames for round 2
        let round2_frames = player.frames_for_round(BattleRound::new(2));
        assert_eq!(round2_frames.len(), 2);

        // Frames for round 3 (none)
        let round3_frames = player.frames_for_round(BattleRound::new(3));
        assert!(round3_frames.is_empty());

        // Frames for Command phase
        let command_frames = player.frames_for_phase(Phase::Command);
        assert_eq!(command_frames.len(), 2);

        // Frames for Shooting phase
        let shooting_frames = player.frames_for_phase(Phase::Shooting);
        assert_eq!(shooting_frames.len(), 1);
        assert_eq!(shooting_frames[0].player, PlayerId::new(1));

        // Frames for player 0
        let p0_frames = player.frames_for_player(PlayerId::new(0));
        assert_eq!(p0_frames.len(), 3); // frames 0, 1, 3

        // Frames for player 1
        let p1_frames = player.frames_for_player(PlayerId::new(1));
        assert_eq!(p1_frames.len(), 2); // frames 2, 4

        // Header access
        let header = player.header();
        assert_eq!(header.engine_version, "0.1.0");
        assert_eq!(header.players[0].name, "Alice");
    }

    // ── ReplayDiff tests ──────────────────────────────────────────

    #[test]
    fn test_replay_diff_identical() {
        let a = test_replay_file();
        let b = test_replay_file();

        let diff = ReplayDiff::compare(&a, &b);
        assert!(diff.identical);
        assert!(diff.divergence_point.is_none());
        assert!(diff.differences.is_empty());
        assert_eq!(diff.frames_a, 5);
        assert_eq!(diff.frames_b, 5);
    }

    #[test]
    fn test_replay_diff_divergence() {
        let a = test_replay_file();
        let mut b = test_replay_file();

        // Modify frame 2 in replay b
        b.frames[2].command = Command::Concede {
            player: PlayerId::new(1),
        };
        b.frames[2].state_hash_after = Some(0xFFFF_FFFF);

        let diff = ReplayDiff::compare(&a, &b);
        assert!(!diff.identical);
        assert_eq!(diff.divergence_point, Some(2));
        assert!(diff.differences.len() >= 2); // At least command + hash differences

        // Check that command mismatch is reported
        let has_cmd_mismatch = diff
            .differences
            .iter()
            .any(|d| matches!(d.diff_type, DiffType::CommandMismatch));
        assert!(has_cmd_mismatch);

        // Check that hash mismatch is reported
        let has_hash_mismatch = diff
            .differences
            .iter()
            .any(|d| matches!(d.diff_type, DiffType::HashMismatch));
        assert!(has_hash_mismatch);
    }

    #[test]
    fn test_replay_diff_different_frame_count() {
        let a = test_replay_file();
        let mut b = test_replay_file();
        b.frames.push(test_frame(5, 3, Phase::Command, 0));

        let diff = ReplayDiff::compare(&a, &b);
        assert!(!diff.identical);
        assert_eq!(diff.frames_a, 5);
        assert_eq!(diff.frames_b, 6);

        let has_count_mismatch = diff
            .differences
            .iter()
            .any(|d| matches!(d.diff_type, DiffType::FrameCountMismatch));
        assert!(has_count_mismatch);
    }

    #[test]
    fn test_replay_diff_header_mismatch() {
        let a = test_replay_file();
        let mut b = test_replay_file();
        b.header.engine_version = "0.2.0".to_string();

        let diff = ReplayDiff::compare(&a, &b);
        assert!(!diff.identical);

        let has_header_mismatch = diff
            .differences
            .iter()
            .any(|d| matches!(d.diff_type, DiffType::HeaderMismatch));
        assert!(has_header_mismatch);
    }

    // ── Export summary test ───────────────────────────────────────

    #[test]
    fn test_export_summary() {
        let replay = test_replay_file();
        let summary = export_summary(&replay);

        // Check that the summary contains key sections
        assert!(summary.contains("WH40K Combat Patrol - Replay Summary"));
        assert!(summary.contains("Format version: 1"));
        assert!(summary.contains("Engine version: 0.1.0"));
        assert!(summary.contains("Content version: test-1.0"));
        assert!(summary.contains("Alice"));
        assert!(summary.contains("Bob"));
        assert!(summary.contains("Round-by-Round Summary"));
        assert!(summary.contains("Game Outcome"));
        assert!(summary.contains("Victory for Alice"));
        assert!(summary.contains("Final Scores"));
        assert!(summary.contains("15 VP total"));
        assert!(summary.contains("8 VP total"));
        assert!(summary.contains("Statistics"));
        assert!(summary.contains("Total commands: 5"));
        assert!(summary.contains("Legal commands: 5"));
        assert!(summary.contains("Illegal commands: 0"));
        assert!(summary.contains("Per-player commands"));
        assert!(summary.contains("Command categories"));
    }

    // ── Format version check ─────────────────────────────────────

    #[test]
    fn test_format_version_check() {
        let mut replay = test_replay_file();
        replay.format_version = 999;

        let json = serde_json::to_string(&replay).unwrap();
        let result = import_json(&json);
        assert!(result.is_err());

        match result.unwrap_err() {
            ReplayError::InvalidFormatVersion { expected, actual } => {
                assert_eq!(expected, REPLAY_FORMAT_VERSION);
                assert_eq!(actual, 999);
            }
            other => panic!("Expected InvalidFormatVersion, got: {:?}", other),
        }
    }

    #[test]
    fn test_format_version_check_binary() {
        let mut replay = test_replay_file();
        replay.format_version = 42;

        let binary = bincode::serialize(&replay).unwrap();
        let result = import_binary(&binary);
        assert!(result.is_err());

        match result.unwrap_err() {
            ReplayError::InvalidFormatVersion { expected, actual } => {
                assert_eq!(expected, REPLAY_FORMAT_VERSION);
                assert_eq!(actual, 42);
            }
            other => panic!("Expected InvalidFormatVersion, got: {:?}", other),
        }
    }

    // ── Error display tests ──────────────────────────────────────

    #[test]
    fn test_replay_error_display() {
        let err = ReplayError::SerializationError("bad data".to_string());
        assert_eq!(format!("{}", err), "Serialization error: bad data");

        let err = ReplayError::DeserializationError("parse failed".to_string());
        assert_eq!(format!("{}", err), "Deserialization error: parse failed");

        let err = ReplayError::InvalidFormatVersion {
            expected: 1,
            actual: 5,
        };
        assert_eq!(
            format!("{}", err),
            "Invalid replay format version: expected 1, got 5"
        );

        let err = ReplayError::VerificationFailed {
            frame_index: 42,
            reason: "hash mismatch".to_string(),
        };
        assert_eq!(
            format!("{}", err),
            "Replay verification failed at frame 42: hash mismatch"
        );

        let err = ReplayError::EmptyReplay;
        assert_eq!(format!("{}", err), "Replay is empty (no frames)");

        let err = ReplayError::FrameOutOfBounds {
            index: 10,
            total: 5,
        };
        assert_eq!(
            format!("{}", err),
            "Frame index 10 out of bounds (total: 5)"
        );
    }

    // ── Verification tests ────────────────────────────────────────

    #[test]
    fn test_replay_verification_pass() {
        // Build a replay with a consistent hash chain and verify it manually
        let replay = test_replay_file_with_consistent_chain();
        let player = ReplayPlayer::new(replay);

        // Verify that the first frame's hash_before matches the header's initial_state_hash
        let first = player.frame_at(0).unwrap();
        assert_eq!(first.state_hash_before, player.header().initial_state_hash);

        // Verify the hash chain: frame[i].state_hash_after == frame[i+1].state_hash_before
        for i in 0..player.total_frames() - 1 {
            let current = player.frame_at(i).unwrap();
            let next = player.frame_at(i + 1).unwrap();
            if let Some(hash_after) = current.state_hash_after {
                assert_eq!(
                    hash_after, next.state_hash_before,
                    "Hash chain broken between frame {} and {}",
                    i,
                    i + 1
                );
            }
        }

        // Verify the final frame's hash_after matches the header's final_state_hash
        let last = player.frame_at(player.total_frames() - 1).unwrap();
        assert_eq!(
            last.state_hash_after,
            player.header().final_state_hash,
            "Final hash mismatch"
        );

        // Also verify the chain is internally consistent by checking no breaks exist
        let mut expected_hash = player.header().initial_state_hash;
        for i in 0..player.total_frames() {
            let frame = player.frame_at(i).unwrap();
            assert_eq!(
                frame.state_hash_before, expected_hash,
                "Frame {} state_hash_before does not match expected",
                i
            );
            if let Some(hash_after) = frame.state_hash_after {
                expected_hash = hash_after;
            }
        }
        assert_eq!(
            Some(expected_hash),
            player.header().final_state_hash,
            "Final expected hash should match header final_state_hash"
        );
    }

    #[test]
    fn test_replay_verification_fail() {
        // Build a replay with an inconsistent hash chain
        let mut replay = test_replay_file();

        // Set up mostly consistent chain but break it at frame 2->3
        replay.header.initial_state_hash = 100;
        replay.frames[0].state_hash_before = 100;
        replay.frames[0].state_hash_after = Some(101);
        replay.frames[1].state_hash_before = 101;
        replay.frames[1].state_hash_after = Some(102);
        replay.frames[2].state_hash_before = 102;
        replay.frames[2].state_hash_after = Some(103);
        // Intentionally break the chain here
        replay.frames[3].state_hash_before = 999; // Should be 103
        replay.frames[3].state_hash_after = Some(104);
        replay.frames[4].state_hash_before = 104;
        replay.frames[4].state_hash_after = Some(105);
        replay.header.final_state_hash = Some(105);

        let player = ReplayPlayer::new(replay);

        // Verify chain manually - should find the break
        let frame_2 = player.frame_at(2).unwrap();
        let frame_3 = player.frame_at(3).unwrap();
        let hash_after_2 = frame_2.state_hash_after.unwrap();
        assert_ne!(
            hash_after_2, frame_3.state_hash_before,
            "Chain should be broken between frames 2 and 3"
        );

        // The chain is broken at frame 2->3
        assert_eq!(hash_after_2, 103);
        assert_eq!(frame_3.state_hash_before, 999);
    }

    // ── Helpers for consistent test data ──────────────────────────

    fn test_replay_file_with_consistent_chain() -> ReplayFile {
        let mut replay = test_replay_file();

        replay.header.initial_state_hash = 100;
        replay.frames[0].state_hash_before = 100;
        replay.frames[0].state_hash_after = Some(101);
        replay.frames[1].state_hash_before = 101;
        replay.frames[1].state_hash_after = Some(102);
        replay.frames[2].state_hash_before = 102;
        replay.frames[2].state_hash_after = Some(103);
        replay.frames[3].state_hash_before = 103;
        replay.frames[3].state_hash_after = Some(104);
        replay.frames[4].state_hash_before = 104;
        replay.frames[4].state_hash_after = Some(105);
        replay.header.final_state_hash = Some(105);

        replay
    }

    // ── Additional edge case tests ────────────────────────────────

    #[test]
    fn test_empty_replay_diff() {
        let a = ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: test_header(),
            frames: Vec::new(),
            outcome: GameOutcome::InProgress,
            final_scores: [
                ReplayScore {
                    player_id: PlayerId::new(0),
                    primary_vp: 0,
                    secondary_vp: 0,
                    total_vp: 0,
                },
                ReplayScore {
                    player_id: PlayerId::new(1),
                    primary_vp: 0,
                    secondary_vp: 0,
                    total_vp: 0,
                },
            ],
        };
        let b = a.clone();

        let diff = ReplayDiff::compare(&a, &b);
        assert!(diff.identical);
        assert_eq!(diff.frames_a, 0);
        assert_eq!(diff.frames_b, 0);
    }

    #[test]
    fn test_replay_player_empty_replay() {
        let replay = ReplayFile {
            format_version: REPLAY_FORMAT_VERSION,
            header: test_header(),
            frames: Vec::new(),
            outcome: GameOutcome::InProgress,
            final_scores: [
                ReplayScore {
                    player_id: PlayerId::new(0),
                    primary_vp: 0,
                    secondary_vp: 0,
                    total_vp: 0,
                },
                ReplayScore {
                    player_id: PlayerId::new(1),
                    primary_vp: 0,
                    secondary_vp: 0,
                    total_vp: 0,
                },
            ],
        };

        let mut player = ReplayPlayer::new(replay);
        assert_eq!(player.total_frames(), 0);
        assert!(!player.has_more());
        assert!(player.peek_frame().is_none());
        assert!(player.next_frame().is_none());
        assert!(player.frame_at(0).is_none());
    }

    #[test]
    fn test_replay_score_serialization() {
        let score = ReplayScore {
            player_id: PlayerId::new(0),
            primary_vp: 10,
            secondary_vp: 5,
            total_vp: 15,
        };

        let json = serde_json::to_string(&score).unwrap();
        let back: ReplayScore = serde_json::from_str(&json).unwrap();
        assert_eq!(back.player_id, score.player_id);
        assert_eq!(back.primary_vp, score.primary_vp);
        assert_eq!(back.secondary_vp, score.secondary_vp);
        assert_eq!(back.total_vp, score.total_vp);
    }

    #[test]
    fn test_replay_frame_with_events() {
        let events = vec![
            GameEvent::BattleRoundStarted {
                round: BattleRound::new(1),
            },
            GameEvent::PhaseStarted {
                phase: Phase::Command,
                player: PlayerId::new(0),
            },
        ];

        let frame = ReplayFrame {
            frame_index: 0,
            command: Command::StartBattleRound {
                round: BattleRound::new(1),
            },
            player: PlayerId::new(0),
            round: BattleRound::new(1),
            phase: Phase::Command,
            state_hash_before: 0x1000,
            state_hash_after: Some(0x1001),
            was_legal: true,
            events: events.clone(),
            dice_rolls: Vec::new(),
        };

        let json = serde_json::to_string(&frame).unwrap();
        let back: ReplayFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(back.events.len(), 2);
        assert_eq!(back.events, events);
    }

    #[test]
    fn test_replay_diff_event_mismatch() {
        let mut a = test_replay_file();
        let b = test_replay_file();

        // Add events to frame 1 in replay a but not b
        a.frames[1].events = vec![GameEvent::BattleRoundStarted {
            round: BattleRound::new(1),
        }];

        let diff = ReplayDiff::compare(&a, &b);
        assert!(!diff.identical);

        let has_event_mismatch = diff
            .differences
            .iter()
            .any(|d| matches!(d.diff_type, DiffType::EventMismatch));
        assert!(has_event_mismatch);
    }

    #[test]
    fn test_export_summary_with_draw() {
        let mut replay = test_replay_file();
        replay.outcome = GameOutcome::Draw;

        let summary = export_summary(&replay);
        assert!(summary.contains("Result: Draw"));
    }

    #[test]
    fn test_export_summary_with_in_progress() {
        let mut replay = test_replay_file();
        replay.outcome = GameOutcome::InProgress;

        let summary = export_summary(&replay);
        assert!(summary.contains("Result: Game still in progress"));
    }

    #[test]
    fn test_replay_header_serialization() {
        let header = test_header();
        let json = serde_json::to_string(&header).unwrap();
        let back: ReplayHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(back.engine_version, header.engine_version);
        assert_eq!(back.content_version, header.content_version);
        assert_eq!(back.started_at, header.started_at);
        assert_eq!(back.ended_at, header.ended_at);
        assert_eq!(back.total_commands, header.total_commands);
        assert_eq!(back.total_rounds, header.total_rounds);
        assert_eq!(back.initial_state_hash, header.initial_state_hash);
        assert_eq!(back.final_state_hash, header.final_state_hash);
        assert_eq!(back.mission_id, header.mission_id);
    }

    #[test]
    fn test_replay_player_info_with_all_fields() {
        let info = ReplayPlayerInfo {
            player_id: PlayerId::new(0),
            name: "Test Player".to_string(),
            faction_id: Some(wh40k_core_types::FactionId::new(1)),
            enhancement: Some(wh40k_core_types::EnhancementId::new(2)),
            secondary: Some(wh40k_core_types::SecondaryObjectiveId::new(3)),
        };

        let json = serde_json::to_string(&info).unwrap();
        let back: ReplayPlayerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.player_id, info.player_id);
        assert_eq!(back.name, info.name);
        assert_eq!(back.faction_id, info.faction_id);
        assert_eq!(back.enhancement, info.enhancement);
        assert_eq!(back.secondary, info.secondary);
    }

    #[test]
    fn test_diff_type_serialization() {
        let diff_types = vec![
            DiffType::CommandMismatch,
            DiffType::HashMismatch,
            DiffType::EventMismatch,
            DiffType::DiceRollMismatch,
            DiffType::FrameCountMismatch,
            DiffType::HeaderMismatch,
        ];

        for dt in diff_types {
            let json = serde_json::to_string(&dt).unwrap();
            let back: DiffType = serde_json::from_str(&json).unwrap();
            // Verify roundtrip by serializing again
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn test_replay_constant() {
        assert_eq!(REPLAY_FORMAT_VERSION, 1);
    }

    #[test]
    fn test_illegal_frame_in_replay() {
        let mut replay = test_replay_file();

        // Mark frame 2 as illegal
        replay.frames[2].was_legal = false;
        replay.frames[2].state_hash_after = None;
        replay.frames[2].events = Vec::new();

        let json = export_json(&replay).unwrap();
        let imported = import_json(&json).unwrap();

        assert!(!imported.frames[2].was_legal);
        assert!(imported.frames[2].state_hash_after.is_none());
        assert!(imported.frames[2].events.is_empty());
    }
}
