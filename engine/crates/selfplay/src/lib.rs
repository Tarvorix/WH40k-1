//! WH40K Engine - Self-Play and Training Bridge crate
//!
//! Provides the complete self-play training pipeline:
//!
//! # 8.1 Training Data Export
//! - [`TrainingSample`] - Position data with game outcome labels
//! - [`TrainingShard`] / [`ShardWriter`] / [`ShardReader`] - Shard I/O
//! - [`encode_state`] - GameState to dense feature vector
//! - [`encode_legal_mask`] - Legal action mask over fixed vocabulary
//!
//! # 8.2 Self-Play Runner
//! - [`SelfPlayRunner`] - Orchestrates batches of self-play games
//! - [`MatchResult`] - Single game outcome with diagnostics
//! - [`GameVariation`] - Generates diverse match configurations
//!
//! # 8.3 Gating Harness
//! - [`GatingHarness`] - Evaluates candidate model vs baseline
//! - [`EloRating`] - Elo rating calculations
//! - [`ModelLineage`] - Model generation tracking
//!
//! Source: implementation_v3.md Phase 8, Sections 14, 21.8

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use wh40k_core_types::{
    EnhancementId, FactionId, GameOutcome, MissionId, Phase, PlayerId, SecondaryObjectiveId,
    UnitId,
};
use wh40k_command_system::Command;
use wh40k_eval_features::{
    extract_sparse_features_from_state, Score, SparseFeatureVec, TOTAL_FEATURES,
    FEATURE_SCHEMA_VERSION,
};
use wh40k_eval_heuristic::HeuristicWeights;
use wh40k_eval_nnue::NnueModelArtifact;
use wh40k_game_core::state::GameState;
use wh40k_game_core::{CommandExecutor, ScenarioLoader};
use wh40k_replay::{ReplayFile, ReplayRecorder};
use wh40k_search_abstraction::{ActionGenerator, CandidateSet, MacroAction, TacticalIntent};
use wh40k_search_core::{
    AiWorker, GreedyAi, GreedyAiNnue, IterativeDeepeningSearch, NegamaxSearch, OnePlySearch,
    SearchConfig, SearchStats,
};

// ============================================================================
// Constants
// ============================================================================

/// Current shard format version.
pub const SHARD_FORMAT_VERSION: u32 = 1;

/// Engine version string for training data provenance.
pub const ENGINE_VERSION: &str = "0.1.0";

/// Number of TacticalIntent variants for the action vocabulary.
pub const INTENT_COUNT: usize = 33;

/// Maximum unit slots per player for action vocabulary encoding.
pub const MAX_UNIT_SLOTS: usize = 16;

/// Fixed-size action vocabulary: intent × unit slot.
pub const ACTION_VOCAB_SIZE: usize = INTENT_COUNT * MAX_UNIT_SLOTS;

/// Maximum commands allowed per game to prevent infinite loops.
pub const MAX_COMMANDS_PER_GAME: usize = 10_000;

/// Default shard size (number of samples per shard file).
pub const DEFAULT_SHARD_SIZE: usize = 4096;

/// Default number of gating games.
pub const DEFAULT_GATING_GAMES: usize = 100;

/// Default promotion threshold (win rate for candidate to be promoted).
pub const DEFAULT_PROMOTION_THRESHOLD: f64 = 0.55;

/// Default Elo K-factor for rating updates.
pub const DEFAULT_ELO_K: f64 = 32.0;

/// Default initial Elo rating.
pub const DEFAULT_ELO_INITIAL: f64 = 1500.0;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during self-play operations.
#[derive(Debug)]
pub enum SelfPlayError {
    /// IO error during shard read/write.
    IoError(std::io::Error),
    /// Serialization error.
    SerializationError(String),
    /// Game execution error (command failed).
    ExecutionError(String),
    /// AI returned no actions but game is in progress.
    NoActionsAvailable {
        phase: Phase,
        battle_round: u8,
        decision_owner: PlayerId,
    },
    /// Game exceeded maximum command limit.
    CommandLimitExceeded {
        limit: usize,
        commands_executed: usize,
    },
    /// NNUE model error.
    ModelError(String),
    /// Invalid configuration.
    ConfigError(String),
}

impl std::fmt::Display for SelfPlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelfPlayError::IoError(e) => write!(f, "IO error: {}", e),
            SelfPlayError::SerializationError(s) => write!(f, "Serialization error: {}", s),
            SelfPlayError::ExecutionError(s) => write!(f, "Execution error: {}", s),
            SelfPlayError::NoActionsAvailable {
                phase,
                battle_round,
                decision_owner,
            } => write!(
                f,
                "No actions available: phase={:?}, round={}, player={:?}",
                phase, battle_round, decision_owner
            ),
            SelfPlayError::CommandLimitExceeded {
                limit,
                commands_executed,
            } => write!(
                f,
                "Command limit exceeded: {}/{} commands",
                commands_executed, limit
            ),
            SelfPlayError::ModelError(s) => write!(f, "Model error: {}", s),
            SelfPlayError::ConfigError(s) => write!(f, "Config error: {}", s),
        }
    }
}

impl std::error::Error for SelfPlayError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SelfPlayError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SelfPlayError {
    fn from(e: std::io::Error) -> Self {
        SelfPlayError::IoError(e)
    }
}

// ============================================================================
// 8.1: Action Vocabulary Encoding
// ============================================================================

/// Map a TacticalIntent to its vocabulary index (0..INTENT_COUNT-1).
pub fn intent_to_index(intent: TacticalIntent) -> usize {
    match intent {
        TacticalIntent::HoldObjective => 0,
        TacticalIntent::ContestObjective => 1,
        TacticalIntent::MoveToCover => 2,
        TacticalIntent::StageCharge => 3,
        TacticalIntent::Screen => 4,
        TacticalIntent::Retreat => 5,
        TacticalIntent::LineUpShot => 6,
        TacticalIntent::DenyReserveZone => 7,
        TacticalIntent::HoldPosition => 8,
        TacticalIntent::AdvanceAggressively => 9,
        TacticalIntent::FallBackToShoot => 10,
        TacticalIntent::DeepStrikeArrive => 11,
        TacticalIntent::MaximizeKills => 12,
        TacticalIntent::SoftenChargeTarget => 13,
        TacticalIntent::RemoveScoringUnit => 14,
        TacticalIntent::ForceBattleShock => 15,
        TacticalIntent::BracketHardTarget => 16,
        TacticalIntent::ChargeHighValueTarget => 17,
        TacticalIntent::MultiChargeObjectiveSwing => 18,
        TacticalIntent::ChargeSoftenedTarget => 19,
        TacticalIntent::FightMaxDamage => 20,
        TacticalIntent::FightPreserve => 21,
        TacticalIntent::ChooseStance => 22,
        TacticalIntent::ChooseWeaponProfile => 23,
        TacticalIntent::DefensiveStratagem => 24,
        TacticalIntent::OffensiveStratagem => 25,
        TacticalIntent::MovementReaction => 26,
        TacticalIntent::FightOrderManipulation => 27,
        TacticalIntent::DeclineStratagem => 28,
        TacticalIntent::ScoreObjective => 29,
        TacticalIntent::AllocateBlessings => 30,
        TacticalIntent::PhaseControl => 31,
        TacticalIntent::Generic => 32,
    }
}

/// Map a vocabulary index back to a TacticalIntent.
pub fn index_to_intent(index: usize) -> TacticalIntent {
    match index {
        0 => TacticalIntent::HoldObjective,
        1 => TacticalIntent::ContestObjective,
        2 => TacticalIntent::MoveToCover,
        3 => TacticalIntent::StageCharge,
        4 => TacticalIntent::Screen,
        5 => TacticalIntent::Retreat,
        6 => TacticalIntent::LineUpShot,
        7 => TacticalIntent::DenyReserveZone,
        8 => TacticalIntent::HoldPosition,
        9 => TacticalIntent::AdvanceAggressively,
        10 => TacticalIntent::FallBackToShoot,
        11 => TacticalIntent::DeepStrikeArrive,
        12 => TacticalIntent::MaximizeKills,
        13 => TacticalIntent::SoftenChargeTarget,
        14 => TacticalIntent::RemoveScoringUnit,
        15 => TacticalIntent::ForceBattleShock,
        16 => TacticalIntent::BracketHardTarget,
        17 => TacticalIntent::ChargeHighValueTarget,
        18 => TacticalIntent::MultiChargeObjectiveSwing,
        19 => TacticalIntent::ChargeSoftenedTarget,
        20 => TacticalIntent::FightMaxDamage,
        21 => TacticalIntent::FightPreserve,
        22 => TacticalIntent::ChooseStance,
        23 => TacticalIntent::ChooseWeaponProfile,
        24 => TacticalIntent::DefensiveStratagem,
        25 => TacticalIntent::OffensiveStratagem,
        26 => TacticalIntent::MovementReaction,
        27 => TacticalIntent::FightOrderManipulation,
        28 => TacticalIntent::DeclineStratagem,
        29 => TacticalIntent::ScoreObjective,
        30 => TacticalIntent::AllocateBlessings,
        31 => TacticalIntent::PhaseControl,
        _ => TacticalIntent::Generic,
    }
}

/// Map a unit ID to a slot index (0..MAX_UNIT_SLOTS-1) relative to the player.
/// Units are assigned slots based on their order in the game state's unit list.
pub fn unit_to_slot(unit_id: UnitId, state: &GameState, perspective: PlayerId) -> usize {
    let player_units: Vec<UnitId> = state
        .units
        .iter()
        .filter(|u| u.owner == perspective)
        .map(|u| u.id)
        .collect();
    player_units
        .iter()
        .position(|&id| id == unit_id)
        .unwrap_or(0)
        .min(MAX_UNIT_SLOTS - 1)
}

/// Compute the action vocabulary index for a macro-action.
/// Index = intent_index * MAX_UNIT_SLOTS + unit_slot_index
pub fn action_to_vocab_index(
    action: &MacroAction,
    state: &GameState,
    perspective: PlayerId,
) -> u32 {
    let intent_idx = intent_to_index(action.intent);
    let unit_slot = if let Some(&unit_id) = action.actor_units.first() {
        unit_to_slot(unit_id, state, perspective)
    } else {
        0
    };
    (intent_idx * MAX_UNIT_SLOTS + unit_slot) as u32
}

// ============================================================================
// 8.1: State Encoding
// ============================================================================

/// Encode a game state as a dense f32 feature vector for training.
///
/// Uses the NNUE sparse feature extraction and converts to a dense vector
/// of TOTAL_FEATURES (1203) dimensions. This is the canonical state
/// representation for neural network training.
///
/// # Arguments
/// * `state` - The game state to encode
/// * `perspective` - Which player's perspective to encode from
///
/// # Returns
/// A Vec<f32> of length TOTAL_FEATURES (1203).
pub fn encode_state(state: &GameState, perspective: PlayerId) -> Vec<f32> {
    let sparse = extract_sparse_features_from_state(state, perspective);
    sparse_to_dense(&sparse, TOTAL_FEATURES)
}

/// Convert sparse features to a dense f32 vector.
fn sparse_to_dense(sparse: &SparseFeatureVec, total_features: usize) -> Vec<f32> {
    let mut dense = vec![0.0f32; total_features];
    for feat in &sparse.features {
        let idx = feat.index as usize;
        if idx < total_features {
            dense[idx] = feat.value as f32 / 1000.0; // Normalize from fixed-point
        }
    }
    dense
}

/// Encode a sparse feature vector as (index, value) pairs for efficient storage.
pub fn encode_sparse_features(
    state: &GameState,
    perspective: PlayerId,
) -> Vec<(u16, i16)> {
    let sparse = extract_sparse_features_from_state(state, perspective);
    sparse
        .features
        .iter()
        .map(|f| (f.index, f.value))
        .collect()
}

// ============================================================================
// 8.1: Legal Mask
// ============================================================================

/// A fixed-size boolean mask over the action vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalMask {
    /// Size of the vocabulary.
    pub vocab_size: usize,
    /// Boolean mask: true if the action at that index is legal.
    pub mask: Vec<bool>,
    /// Number of legal actions (number of true values).
    pub action_count: usize,
    /// Mapping from vocabulary index to candidate index in the generated set.
    /// Only populated for legal actions. Vocab index -> candidate index.
    pub vocab_to_candidate: Vec<Option<usize>>,
}

impl LegalMask {
    /// Create an empty mask with no legal actions.
    pub fn empty() -> Self {
        Self {
            vocab_size: ACTION_VOCAB_SIZE,
            mask: vec![false; ACTION_VOCAB_SIZE],
            action_count: 0,
            vocab_to_candidate: vec![None; ACTION_VOCAB_SIZE],
        }
    }

    /// Check if an action at the given vocabulary index is legal.
    pub fn is_legal(&self, vocab_index: usize) -> bool {
        vocab_index < self.vocab_size && self.mask[vocab_index]
    }

    /// Get all legal action vocabulary indices.
    pub fn legal_indices(&self) -> Vec<u32> {
        self.mask
            .iter()
            .enumerate()
            .filter(|(_, &legal)| legal)
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Convert to a dense f32 vector (1.0 for legal, 0.0 for illegal).
    pub fn to_dense_f32(&self) -> Vec<f32> {
        self.mask
            .iter()
            .map(|&legal| if legal { 1.0 } else { 0.0 })
            .collect()
    }
}

/// Encode the legal action mask for the current game state.
///
/// Generates all legal macro-actions and maps each to the fixed-size
/// action vocabulary. The vocabulary index for each action is:
/// `intent_index * MAX_UNIT_SLOTS + unit_slot_index`
///
/// # Arguments
/// * `state` - The current game state
/// * `perspective` - Which player to generate legal actions for
///
/// # Returns
/// A LegalMask with ACTION_VOCAB_SIZE entries.
pub fn encode_legal_mask(state: &GameState, perspective: PlayerId) -> LegalMask {
    let candidates = ActionGenerator::generate(state, perspective);
    encode_legal_mask_from_candidates(&candidates, state, perspective)
}

/// Encode a legal mask from a pre-generated candidate set.
pub fn encode_legal_mask_from_candidates(
    candidates: &CandidateSet,
    state: &GameState,
    perspective: PlayerId,
) -> LegalMask {
    let mut mask = LegalMask::empty();

    for (cand_idx, action) in candidates.candidates.iter().enumerate() {
        let vocab_idx = action_to_vocab_index(action, state, perspective) as usize;
        if vocab_idx < ACTION_VOCAB_SIZE {
            if !mask.mask[vocab_idx] {
                mask.action_count += 1;
            }
            mask.mask[vocab_idx] = true;
            // Store mapping (last candidate at this vocab index wins if there's a collision)
            mask.vocab_to_candidate[vocab_idx] = Some(cand_idx);
        }
    }

    mask
}

// ============================================================================
// 8.1: Training Sample
// ============================================================================

/// A single training sample extracted from a self-play game.
///
/// Contains the position features, legal action mask, chosen action,
/// search evaluation, and the eventual game outcome label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Sparse features as (index, value) pairs for efficient storage.
    /// This is the canonical NNUE input representation.
    pub sparse_features: Vec<(u16, i16)>,

    /// Legal action vocabulary indices that were available at this position.
    pub legal_action_indices: Vec<u32>,

    /// Vocabulary index of the action that was chosen by the search.
    pub chosen_action_index: u32,

    /// The search engine's evaluation score at this position.
    pub search_score: Score,

    /// Game outcome from the perspective of the player making the decision.
    /// +1.0 = win, -1.0 = loss, 0.0 = draw.
    /// Set after the game completes.
    pub outcome: f32,

    /// Which player was making the decision (0 or 1).
    pub perspective: u8,

    /// Game progress at the time of this sample (0.0 = start, 1.0 = end).
    /// Based on battle round / 5.
    pub game_progress: f32,

    /// Deterministic state hash for deduplication.
    pub state_hash: u64,

    /// Battle round when this sample was taken.
    pub battle_round: u8,

    /// Phase when this sample was taken.
    pub phase: u8,
}

impl TrainingSample {
    /// Get the number of sparse features.
    pub fn feature_count(&self) -> usize {
        self.sparse_features.len()
    }

    /// Get the number of legal actions.
    pub fn legal_action_count(&self) -> usize {
        self.legal_action_indices.len()
    }

    /// Convert sparse features to a dense f32 vector.
    pub fn to_dense_features(&self) -> Vec<f32> {
        let mut dense = vec![0.0f32; TOTAL_FEATURES];
        for &(idx, val) in &self.sparse_features {
            let i = idx as usize;
            if i < TOTAL_FEATURES {
                dense[i] = val as f32 / 1000.0;
            }
        }
        dense
    }
}

// ============================================================================
// 8.1: Shard Format
// ============================================================================

/// Metadata header for a training shard file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardHeader {
    /// Shard format version.
    pub version: u32,
    /// Engine version that generated this shard.
    pub engine_version: String,
    /// Feature schema version for compatibility checking.
    pub feature_schema_version: u32,
    /// Number of training samples in this shard.
    pub sample_count: usize,
    /// Number of games contributing to this shard.
    pub game_count: usize,
    /// NNUE model generation that was used during data collection.
    pub model_generation: u32,
    /// Creation timestamp (seconds since Unix epoch).
    pub created_at: u64,
    /// Total features per sample (for validation).
    pub total_features: usize,
    /// Action vocabulary size (for validation).
    pub action_vocab_size: usize,
}

impl ShardHeader {
    /// Create a new shard header with current metadata.
    pub fn new(model_generation: u32) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            version: SHARD_FORMAT_VERSION,
            engine_version: ENGINE_VERSION.to_string(),
            feature_schema_version: FEATURE_SCHEMA_VERSION,
            sample_count: 0,
            game_count: 0,
            model_generation,
            created_at,
            total_features: TOTAL_FEATURES,
            action_vocab_size: ACTION_VOCAB_SIZE,
        }
    }
}

/// A complete training shard containing a header and samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingShard {
    /// Shard metadata.
    pub header: ShardHeader,
    /// Training samples.
    pub samples: Vec<TrainingSample>,
}

impl TrainingShard {
    /// Create a new empty shard.
    pub fn new(model_generation: u32) -> Self {
        Self {
            header: ShardHeader::new(model_generation),
            samples: Vec::new(),
        }
    }

    /// Add a training sample to the shard.
    pub fn add_sample(&mut self, sample: TrainingSample) {
        self.samples.push(sample);
        self.header.sample_count = self.samples.len();
    }

    /// Add all samples from a game, incrementing game count.
    pub fn add_game_samples(&mut self, samples: Vec<TrainingSample>) {
        let count = samples.len();
        self.samples.extend(samples);
        self.header.sample_count = self.samples.len();
        if count > 0 {
            self.header.game_count += 1;
        }
    }

    /// Check if the shard has reached its capacity.
    pub fn is_full(&self, max_samples: usize) -> bool {
        self.samples.len() >= max_samples
    }

    /// Serialize to bincode bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SelfPlayError> {
        bincode::serialize(self)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))
    }

    /// Deserialize from bincode bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SelfPlayError> {
        bincode::deserialize(data)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))
    }

    /// Serialize to JSON string (for debugging/portability).
    pub fn to_json(&self) -> Result<String, SelfPlayError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, SelfPlayError> {
        serde_json::from_str(json)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))
    }
}

// ============================================================================
// 8.1: Shard Writer
// ============================================================================

/// Writes training shards to disk.
///
/// Accumulates samples and writes a shard file when the buffer reaches
/// the configured maximum size. Supports both bincode and JSON formats.
pub struct ShardWriter {
    /// Base directory for shard output.
    output_dir: PathBuf,
    /// Maximum samples per shard file.
    max_samples: usize,
    /// Current shard being filled.
    current_shard: TrainingShard,
    /// Index counter for shard file naming.
    shard_index: usize,
    /// Total samples written across all shards.
    total_samples_written: usize,
    /// Total shards written.
    total_shards_written: usize,
    /// Whether to also write JSON format (for debugging).
    write_json: bool,
}

impl ShardWriter {
    /// Create a new shard writer.
    ///
    /// # Arguments
    /// * `output_dir` - Directory to write shard files
    /// * `max_samples` - Maximum samples per shard file
    /// * `model_generation` - Model generation for header metadata
    pub fn new(
        output_dir: PathBuf,
        max_samples: usize,
        model_generation: u32,
    ) -> Result<Self, SelfPlayError> {
        std::fs::create_dir_all(&output_dir)?;
        Ok(Self {
            output_dir,
            max_samples,
            current_shard: TrainingShard::new(model_generation),
            shard_index: 0,
            total_samples_written: 0,
            total_shards_written: 0,
            write_json: false,
        })
    }

    /// Enable JSON output alongside bincode (for debugging).
    pub fn with_json_output(mut self) -> Self {
        self.write_json = true;
        self
    }

    /// Add samples from a completed game.
    ///
    /// If the current shard is full, it is flushed to disk before adding.
    pub fn add_game_samples(&mut self, samples: Vec<TrainingSample>) -> Result<(), SelfPlayError> {
        if self.current_shard.is_full(self.max_samples) {
            self.flush()?;
        }
        self.current_shard.add_game_samples(samples);
        Ok(())
    }

    /// Add a single sample.
    pub fn add_sample(&mut self, sample: TrainingSample) -> Result<(), SelfPlayError> {
        if self.current_shard.is_full(self.max_samples) {
            self.flush()?;
        }
        self.current_shard.add_sample(sample);
        Ok(())
    }

    /// Flush the current shard to disk and start a new one.
    pub fn flush(&mut self) -> Result<(), SelfPlayError> {
        if self.current_shard.samples.is_empty() {
            return Ok(());
        }

        let shard_name = format!("shard_{:06}.bin", self.shard_index);
        let shard_path = self.output_dir.join(&shard_name);

        let data = self.current_shard.to_bytes()?;
        std::fs::write(&shard_path, data)?;

        if self.write_json {
            let json_name = format!("shard_{:06}.json", self.shard_index);
            let json_path = self.output_dir.join(&json_name);
            let json_data = self.current_shard.to_json()?;
            std::fs::write(&json_path, json_data)?;
        }

        self.total_samples_written += self.current_shard.samples.len();
        self.total_shards_written += 1;
        self.shard_index += 1;

        let model_gen = self.current_shard.header.model_generation;
        self.current_shard = TrainingShard::new(model_gen);

        Ok(())
    }

    /// Finalize: flush any remaining samples and return write statistics.
    pub fn finalize(mut self) -> Result<ShardWriteStats, SelfPlayError> {
        self.flush()?;
        Ok(ShardWriteStats {
            total_samples: self.total_samples_written,
            total_shards: self.total_shards_written,
            output_dir: self.output_dir,
        })
    }

    /// Get the number of samples in the current unflushed shard.
    pub fn pending_samples(&self) -> usize {
        self.current_shard.samples.len()
    }

    /// Get total samples written (not including pending).
    pub fn total_written(&self) -> usize {
        self.total_samples_written
    }
}

/// Statistics from a shard writing session.
#[derive(Debug, Clone)]
pub struct ShardWriteStats {
    /// Total training samples written across all shards.
    pub total_samples: usize,
    /// Total shard files created.
    pub total_shards: usize,
    /// Output directory where shards were written.
    pub output_dir: PathBuf,
}

impl std::fmt::Display for ShardWriteStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ShardWriter: {} samples in {} shards -> {:?}",
            self.total_samples, self.total_shards, self.output_dir
        )
    }
}

// ============================================================================
// 8.1: Shard Reader
// ============================================================================

/// Reads training shards from disk.
pub struct ShardReader;

impl ShardReader {
    /// Read a single shard file (bincode format).
    pub fn read_shard(path: &Path) -> Result<TrainingShard, SelfPlayError> {
        let data = std::fs::read(path)?;
        TrainingShard::from_bytes(&data)
    }

    /// Read a single shard file (JSON format).
    pub fn read_shard_json(path: &Path) -> Result<TrainingShard, SelfPlayError> {
        let data = std::fs::read_to_string(path)?;
        TrainingShard::from_json(&data)
    }

    /// Read all shard files from a directory (bincode format).
    /// Returns shards sorted by filename.
    pub fn read_all_shards(dir: &Path) -> Result<Vec<TrainingShard>, SelfPlayError> {
        let mut shards = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "bin")
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let shard = Self::read_shard(&entry.path())?;
            shards.push(shard);
        }

        Ok(shards)
    }

    /// Read all samples from all shards in a directory.
    /// Returns a flat list of training samples.
    pub fn read_all_samples(dir: &Path) -> Result<Vec<TrainingSample>, SelfPlayError> {
        let shards = Self::read_all_shards(dir)?;
        let mut samples = Vec::new();
        for shard in shards {
            samples.extend(shard.samples);
        }
        Ok(samples)
    }

    /// Count total samples across all shards in a directory.
    pub fn count_samples(dir: &Path) -> Result<usize, SelfPlayError> {
        let shards = Self::read_all_shards(dir)?;
        Ok(shards.iter().map(|s| s.header.sample_count).sum())
    }

    /// Validate shard compatibility with the current engine.
    pub fn validate_shard(shard: &TrainingShard) -> Result<(), SelfPlayError> {
        if shard.header.version != SHARD_FORMAT_VERSION {
            return Err(SelfPlayError::ConfigError(format!(
                "Shard format version mismatch: expected {}, got {}",
                SHARD_FORMAT_VERSION, shard.header.version
            )));
        }
        if shard.header.feature_schema_version != FEATURE_SCHEMA_VERSION {
            return Err(SelfPlayError::ConfigError(format!(
                "Feature schema version mismatch: expected {}, got {}",
                FEATURE_SCHEMA_VERSION, shard.header.feature_schema_version
            )));
        }
        if shard.header.total_features != TOTAL_FEATURES {
            return Err(SelfPlayError::ConfigError(format!(
                "Total features mismatch: expected {}, got {}",
                TOTAL_FEATURES, shard.header.total_features
            )));
        }
        if shard.header.action_vocab_size != ACTION_VOCAB_SIZE {
            return Err(SelfPlayError::ConfigError(format!(
                "Action vocab size mismatch: expected {}, got {}",
                ACTION_VOCAB_SIZE, shard.header.action_vocab_size
            )));
        }
        Ok(())
    }
}

// ============================================================================
// 8.2: AI Type and Player Configuration
// ============================================================================

/// Type of AI to use for a self-play player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiType {
    /// Greedy (depth 0) with heuristic evaluator.
    Greedy,
    /// One-ply search with heuristic evaluator.
    OnePly,
    /// Negamax at specified depth with heuristic evaluator.
    Negamax(u8),
    /// Iterative deepening up to specified max depth.
    IterativeDeepening(u8),
    /// Time-limited iterative deepening (milliseconds per move).
    IterativeDeepeningTimed(u64),
    /// Greedy (depth 0) with NNUE evaluator.
    GreedyNnue,
}

/// Configuration for a single self-play player.
#[derive(Debug, Clone)]
pub struct PlayerConfig {
    /// Name of this player (for diagnostics).
    pub name: String,
    /// Type of AI to use.
    pub ai_type: AiType,
    /// Custom heuristic weights (None = defaults).
    pub weights: Option<HeuristicWeights>,
    /// NNUE model artifact (required for GreedyNnue).
    pub model_artifact: Option<NnueModelArtifact>,
    /// Custom search configuration override.
    pub search_config: Option<SearchConfig>,
}

impl PlayerConfig {
    /// Create a greedy player config.
    pub fn greedy(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::Greedy,
            weights: None,
            model_artifact: None,
            search_config: None,
        }
    }

    /// Create a one-ply player config.
    pub fn one_ply(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::OnePly,
            weights: None,
            model_artifact: None,
            search_config: None,
        }
    }

    /// Create a negamax player config at specified depth.
    pub fn negamax(name: &str, depth: u8) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::Negamax(depth),
            weights: None,
            model_artifact: None,
            search_config: None,
        }
    }

    /// Create an iterative deepening player config.
    pub fn iterative_deepening(name: &str, max_depth: u8) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::IterativeDeepening(max_depth),
            weights: None,
            model_artifact: None,
            search_config: None,
        }
    }

    /// Create a time-limited iterative deepening player config.
    pub fn iterative_deepening_timed(name: &str, time_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::IterativeDeepeningTimed(time_ms),
            weights: None,
            model_artifact: None,
            search_config: None,
        }
    }

    /// Create an NNUE greedy player config with bootstrap model.
    pub fn nnue_bootstrap(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::GreedyNnue,
            weights: None,
            model_artifact: Some(NnueModelArtifact::bootstrap()),
            search_config: None,
        }
    }

    /// Create an NNUE greedy player config with a specific model.
    pub fn nnue_with_model(name: &str, artifact: NnueModelArtifact) -> Self {
        Self {
            name: name.to_string(),
            ai_type: AiType::GreedyNnue,
            weights: None,
            model_artifact: Some(artifact),
            search_config: None,
        }
    }

    /// Set custom heuristic weights.
    pub fn with_weights(mut self, weights: HeuristicWeights) -> Self {
        self.weights = Some(weights);
        self
    }

    /// Set custom search configuration.
    pub fn with_search_config(mut self, config: SearchConfig) -> Self {
        self.search_config = Some(config);
        self
    }
}

/// Create a boxed AiWorker from a PlayerConfig.
pub fn create_ai_worker(config: &PlayerConfig) -> Result<Box<dyn AiWorker>, SelfPlayError> {
    match &config.ai_type {
        AiType::Greedy => {
            let worker: Box<dyn AiWorker> = if let Some(ref weights) = config.weights {
                Box::new(GreedyAi::with_weights(weights.clone()))
            } else {
                Box::new(GreedyAi::new())
            };
            Ok(worker)
        }
        AiType::OnePly => {
            let worker: Box<dyn AiWorker> = if let Some(ref weights) = config.weights {
                Box::new(OnePlySearch::with_weights(weights.clone()))
            } else {
                Box::new(OnePlySearch::new())
            };
            Ok(worker)
        }
        AiType::Negamax(depth) => {
            let search_config = config
                .search_config
                .clone()
                .unwrap_or_else(|| SearchConfig::negamax(*depth));
            let worker: Box<dyn AiWorker> = if let Some(ref weights) = config.weights {
                Box::new(NegamaxSearch::with_weights_and_config(
                    weights.clone(),
                    search_config,
                ))
            } else {
                Box::new(NegamaxSearch::with_config(search_config))
            };
            Ok(worker)
        }
        AiType::IterativeDeepening(max_depth) => {
            let search_config = config
                .search_config
                .clone()
                .unwrap_or_else(|| SearchConfig::iterative_deepening(*max_depth));
            let worker: Box<dyn AiWorker> = if let Some(ref artifact) = config.model_artifact {
                // Use NNUE evaluator with iterative deepening
                Box::new(IterativeDeepeningSearch::with_nnue(artifact, search_config)
                    .map_err(|e| SelfPlayError::ModelError(format!("{}", e)))?)
            } else if let Some(ref weights) = config.weights {
                Box::new(IterativeDeepeningSearch::with_weights_and_config(
                    weights.clone(),
                    search_config,
                ))
            } else {
                Box::new(IterativeDeepeningSearch::with_config(search_config))
            };
            Ok(worker)
        }
        AiType::IterativeDeepeningTimed(time_ms) => {
            let search_config = config
                .search_config
                .clone()
                .unwrap_or_else(|| SearchConfig::iterative_deepening_timed(*time_ms));
            let worker: Box<dyn AiWorker> = if let Some(ref weights) = config.weights {
                Box::new(IterativeDeepeningSearch::with_weights_and_config(
                    weights.clone(),
                    search_config,
                ))
            } else {
                Box::new(IterativeDeepeningSearch::with_config(search_config))
            };
            Ok(worker)
        }
        AiType::GreedyNnue => {
            if let Some(ref artifact) = config.model_artifact {
                let worker = GreedyAiNnue::from_artifact(artifact)
                    .map_err(|e| SelfPlayError::ModelError(format!("{}", e)))?;
                Ok(Box::new(worker))
            } else {
                Ok(Box::new(GreedyAiNnue::bootstrap()))
            }
        }
    }
}

// ============================================================================
// 8.2: Match Configuration
// ============================================================================

/// Configuration for a single self-play game.
#[derive(Debug, Clone)]
pub struct MatchConfig {
    /// Seed for deterministic game execution.
    pub seed: [u8; 32],
    /// Mission to play (None for default).
    pub mission_id: Option<MissionId>,
    /// Faction for player 1 (index 0).
    pub player1_faction: FactionId,
    /// Faction for player 2 (index 1).
    pub player2_faction: FactionId,
    /// Enhancement for player 1.
    pub player1_enhancement: Option<EnhancementId>,
    /// Enhancement for player 2.
    pub player2_enhancement: Option<EnhancementId>,
    /// Secondary objective for player 1.
    pub player1_secondary: Option<SecondaryObjectiveId>,
    /// Secondary objective for player 2.
    pub player2_secondary: Option<SecondaryObjectiveId>,
}

impl MatchConfig {
    /// Create a default match config (Custodes vs World Eaters, mission 0).
    pub fn default_match(seed: [u8; 32]) -> Self {
        Self {
            seed,
            mission_id: Some(MissionId::new(0)),
            player1_faction: FactionId::new(0), // Custodes
            player2_faction: FactionId::new(1), // World Eaters
            player1_enhancement: None,
            player2_enhancement: None,
            player1_secondary: None,
            player2_secondary: None,
        }
    }

    /// Create a match config with swapped factions (player 2 gets first player's faction).
    pub fn swapped(&self) -> Self {
        Self {
            seed: self.seed,
            mission_id: self.mission_id,
            player1_faction: self.player2_faction,
            player2_faction: self.player1_faction,
            player1_enhancement: self.player2_enhancement,
            player2_enhancement: self.player1_enhancement,
            player1_secondary: self.player2_secondary,
            player2_secondary: self.player1_secondary,
        }
    }
}

/// Generate a deterministic seed from a base seed and game index.
pub fn generate_seed(base: u64, game_index: usize) -> [u8; 32] {
    let mut rng = SmallRng::seed_from_u64(base.wrapping_add(game_index as u64));
    let mut seed = [0u8; 32];
    for byte in seed.iter_mut() {
        *byte = rng.gen();
    }
    seed
}

// ============================================================================
// 8.2: Serializable Search Statistics
// ============================================================================

/// Serializable snapshot of search statistics for training data export.
/// Mirrors `SearchStats` from search_core but with Serialize/Deserialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchStatsSnapshot {
    pub nodes_evaluated: u64,
    pub leaf_evaluations: u64,
    pub beta_cutoffs: u64,
    pub tt_cutoffs: u64,
    pub tt_ordering_hits: u64,
    pub max_depth_reached: u8,
    pub root_candidates: usize,
    pub depth_requested: u8,
    pub chance_samples: u64,
    pub iterations_completed: u8,
    pub aspiration_resets: u64,
    pub quiescence_nodes: u64,
    pub extensions: u64,
    pub time_elapsed_ms: u64,
    pub nps: u64,
    pub pv_changes: u64,
    pub seldepth: u8,
}

impl From<&SearchStats> for SearchStatsSnapshot {
    fn from(s: &SearchStats) -> Self {
        Self {
            nodes_evaluated: s.nodes_evaluated,
            leaf_evaluations: s.leaf_evaluations,
            beta_cutoffs: s.beta_cutoffs,
            tt_cutoffs: s.tt_cutoffs,
            tt_ordering_hits: s.tt_ordering_hits,
            max_depth_reached: s.max_depth_reached,
            root_candidates: s.root_candidates,
            depth_requested: s.depth_requested,
            chance_samples: s.chance_samples,
            iterations_completed: s.iterations_completed,
            aspiration_resets: s.aspiration_resets,
            quiescence_nodes: s.quiescence_nodes,
            extensions: s.extensions,
            time_elapsed_ms: s.time_elapsed_ms,
            nps: s.nps,
            pv_changes: s.pv_changes,
            seldepth: s.seldepth,
        }
    }
}

// ============================================================================
// 8.2: Search Diagnostic Entry
// ============================================================================

/// Per-move search diagnostic captured during self-play.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDiagnosticEntry {
    /// Command number in the game (0-based).
    pub move_number: usize,
    /// Battle round (1-5).
    pub battle_round: u8,
    /// Current phase.
    pub phase: Phase,
    /// Player who made this decision.
    pub player: PlayerId,
    /// Search statistics snapshot (serializable).
    pub stats: SearchStatsSnapshot,
    /// Search evaluation score.
    pub score: Score,
    /// Label of the best action.
    pub best_action_label: String,
    /// Tactical intent of the best action.
    pub best_intent: TacticalIntent,
    /// Number of candidates evaluated.
    pub candidates_evaluated: usize,
}

// ============================================================================
// 8.2: Match Result
// ============================================================================

/// Result of a single self-play game.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Game outcome (Victory, Draw, InProgress if terminated early).
    pub outcome: GameOutcome,
    /// Player 1 victory points.
    pub player1_vp: i16,
    /// Player 2 victory points.
    pub player2_vp: i16,
    /// Total battle rounds played.
    pub total_rounds: u8,
    /// Total commands executed.
    pub total_commands: usize,
    /// Per-move search diagnostics.
    pub search_diagnostics: Vec<SearchDiagnosticEntry>,
    /// Training samples collected from this game (if data collection enabled).
    pub training_samples: Vec<TrainingSample>,
    /// Wall-clock duration of the game in milliseconds.
    pub duration_ms: u64,
    /// Seed used for this game.
    pub game_seed: [u8; 32],
    /// Replay file (if replay collection enabled).
    pub replay: Option<ReplayFile>,
    /// Whether the game terminated normally (not due to command limit).
    pub normal_termination: bool,
    /// Error message if the game terminated abnormally.
    pub error_message: Option<String>,
}

impl MatchResult {
    /// Check if player 1 won.
    pub fn player1_won(&self) -> bool {
        matches!(self.outcome, GameOutcome::Victory(p) if p == PlayerId::new(0))
    }

    /// Check if player 2 won.
    pub fn player2_won(&self) -> bool {
        matches!(self.outcome, GameOutcome::Victory(p) if p == PlayerId::new(1))
    }

    /// Check if the game was a draw.
    pub fn is_draw(&self) -> bool {
        matches!(self.outcome, GameOutcome::Draw)
    }
}

// ============================================================================
// 8.2: Self-Play Game Execution
// ============================================================================

/// Play a single game between two AI workers.
///
/// This is the core self-play function. It:
/// 1. Loads the scenario
/// 2. Loops: generate candidates, choose action, execute commands
/// 3. Collects training samples at each decision point
/// 4. Labels samples with the game outcome after completion
/// 5. Optionally records a replay
///
/// # Arguments
/// * `match_config` - Game configuration (seed, factions, mission)
/// * `player1_config` - AI config for player 1
/// * `player2_config` - AI config for player 2
/// * `collect_training_data` - Whether to collect training samples
/// * `collect_replay` - Whether to record a replay
///
/// # Returns
/// A MatchResult with the game outcome, diagnostics, and optional training data.
pub fn play_single_game(
    match_config: &MatchConfig,
    player1_config: &PlayerConfig,
    player2_config: &PlayerConfig,
    collect_training_data: bool,
    collect_replay: bool,
) -> Result<MatchResult, SelfPlayError> {
    let start_time = Instant::now();

    // Create AI workers
    let mut ai1 = create_ai_worker(player1_config)?;
    let mut ai2 = create_ai_worker(player2_config)?;

    let player1_id = PlayerId::new(0);
    let _player2_id = PlayerId::new(1);

    // Load the scenario
    let mut state = ScenarioLoader::load_scenario(
        match_config.player1_faction,
        match_config.player2_faction,
        match_config.mission_id,
        match_config.seed,
    );

    // Set enhancements and secondaries if specified
    if let Some(enh) = match_config.player1_enhancement {
        state.players[0].enhancement_choice = Some(enh);
    }
    if let Some(enh) = match_config.player2_enhancement {
        state.players[1].enhancement_choice = Some(enh);
    }
    if let Some(sec) = match_config.player1_secondary {
        state.players[0].secondary_choice = Some(sec);
    }
    if let Some(sec) = match_config.player2_secondary {
        state.players[1].secondary_choice = Some(sec);
    }

    // Initialize replay recorder if needed
    let mut recorder = if collect_replay {
        Some(ReplayRecorder::new(&state))
    } else {
        None
    };

    // Collect training samples (position features + chosen action, labeled later)
    let mut pending_samples: Vec<TrainingSample> = Vec::new();
    let mut search_diagnostics: Vec<SearchDiagnosticEntry> = Vec::new();
    let mut commands_executed: usize = 0;
    let mut error_message: Option<String> = None;

    // Main game loop
    let mut last_action_label = String::new();
    let mut same_action_count: usize = 0;
    let mut last_state_hash: u64 = 0;
    let mut same_state_count: usize = 0;

    while state.is_in_progress() {
        // Safety: prevent infinite loops
        if commands_executed >= MAX_COMMANDS_PER_GAME {
            error_message = Some(format!(
                "Command limit exceeded: {} commands",
                commands_executed
            ));
            break;
        }

        // Detect stuck state (same game state hash repeating)
        let current_hash = state.compute_hash();
        if current_hash == last_state_hash {
            same_state_count += 1;
            if same_state_count >= 5 {
                error_message = Some(format!(
                    "Game stuck: state unchanged {} iters at BR{} {:?} P{} (cmd {})",
                    same_state_count,
                    state.battle_round.number(),
                    state.current_phase,
                    state.decision_owner.raw(),
                    commands_executed,
                ));
                break;
            }
        } else {
            last_state_hash = current_hash;
            same_state_count = 0;
        }

        let decision_player = state.decision_owner;
        let ai: &mut Box<dyn AiWorker> = if decision_player == player1_id {
            &mut ai1
        } else {
            &mut ai2
        };

        // Choose action
        let search_result = ai.choose_action(&state, decision_player);

        match search_result {
            Some(result) => {
                // Detect repeated identical actions (stuck AI)
                if result.best_action.label == last_action_label {
                    same_action_count += 1;
                    if same_action_count >= 5 {
                        error_message = Some(format!(
                            "AI stuck: '{}' repeated {}x at BR{} {:?} P{} (cmd {})",
                            last_action_label, same_action_count,
                            state.battle_round.number(),
                            state.current_phase,
                            state.decision_owner.raw(),
                            commands_executed,
                        ));
                        break;
                    }
                } else {
                    last_action_label = result.best_action.label.clone();
                    same_action_count = 1;
                }

                // Collect training sample before executing the action
                if collect_training_data {
                    let sparse = encode_sparse_features(&state, decision_player);
                    let legal_mask = encode_legal_mask(&state, decision_player);
                    let chosen_vocab_idx =
                        action_to_vocab_index(&result.best_action, &state, decision_player);
                    let progress = state.battle_round.number() as f32 / 5.0;

                    pending_samples.push(TrainingSample {
                        sparse_features: sparse,
                        legal_action_indices: legal_mask.legal_indices(),
                        chosen_action_index: chosen_vocab_idx,
                        search_score: result.score,
                        outcome: 0.0, // Set after game completes
                        perspective: decision_player.raw() as u8,
                        game_progress: progress,
                        state_hash: state.compute_hash(),
                        battle_round: state.battle_round.number(),
                        phase: state.current_phase as u8,
                    });
                }

                // Record search diagnostics
                search_diagnostics.push(SearchDiagnosticEntry {
                    move_number: commands_executed,
                    battle_round: state.battle_round.number(),
                    phase: state.current_phase,
                    player: decision_player,
                    stats: SearchStatsSnapshot::from(&result.stats),
                    score: result.score,
                    best_action_label: result.best_action.label.clone(),
                    best_intent: result.best_action.intent,
                    candidates_evaluated: result.candidate_scores.len(),
                });

                // Execute each command in the macro-action
                for cmd in result.best_commands() {
                    // Re-validate before executing to catch stale commands
                    let validation = wh40k_game_core::CommandValidator::validate(&state, cmd);
                    if validation.is_illegal() {
                        // Command became invalid — skip it instead of failing
                        eprintln!(
                            "[selfplay] Skipping invalid command at BR{} {:?} P{}: {:?}",
                            state.battle_round.number(),
                            state.current_phase,
                            state.decision_owner.raw(),
                            cmd,
                        );
                        continue;
                    }

                    // Record replay frame before execution
                    if let Some(ref mut rec) = recorder {
                        rec.record_command(
                            &state,
                            cmd,
                            &[], // Events captured by event bus
                            true,
                        );
                    }

                    match CommandExecutor::execute(&mut state, cmd) {
                        Ok(_events) => {
                            commands_executed += 1;
                        }
                        Err(e) => {
                            // Command failed - log but continue
                            // This can happen when macro-action commands become invalid
                            // due to state changes from earlier commands in the same action
                            error_message = Some(format!(
                                "Command execution error at cmd {}: {:?}",
                                commands_executed, e
                            ));
                            commands_executed += 1;
                            break;
                        }
                    }
                }
            }
            None => {
                // No legal actions available
                // This can happen in edge cases (e.g., all units destroyed mid-phase)
                // Try to end the current phase
                let end_phase_cmd = Command::EndPhase {
                    phase: state.current_phase,
                };
                match CommandExecutor::execute(&mut state, &end_phase_cmd) {
                    Ok(_events) => {
                        commands_executed += 1;
                    }
                    Err(_) => {
                        // Can't even end the phase - try PassAction
                        let pass_cmd = Command::PassAction;
                        match CommandExecutor::execute(&mut state, &pass_cmd) {
                            Ok(_events) => {
                                commands_executed += 1;
                            }
                            Err(e) => {
                                error_message = Some(format!(
                                    "Game stuck: no actions and can't end phase/pass: {:?}",
                                    e
                                ));
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Determine outcome
    let outcome = state.game_outcome;

    // Label training samples with game outcome
    if collect_training_data {
        for sample in &mut pending_samples {
            sample.outcome = match outcome {
                GameOutcome::Victory(winner) => {
                    if PlayerId::new(sample.perspective as u32) == winner {
                        1.0
                    } else {
                        -1.0
                    }
                }
                GameOutcome::Draw => 0.0,
                GameOutcome::InProgress => 0.0,
            };
        }
    }

    // Finalize replay
    let replay = recorder.map(|rec| rec.finalize(&state));

    let duration_ms = start_time.elapsed().as_millis() as u64;

    Ok(MatchResult {
        outcome,
        player1_vp: state.players[0].vp.value(),
        player2_vp: state.players[1].vp.value(),
        total_rounds: state.battle_round.number(),
        total_commands: commands_executed,
        search_diagnostics,
        training_samples: pending_samples,
        duration_ms,
        game_seed: match_config.seed,
        replay,
        normal_termination: error_message.is_none(),
        error_message,
    })
}

// ============================================================================
// 8.2: Self-Play Configuration
// ============================================================================

/// Configuration for a self-play training run.
#[derive(Debug, Clone)]
pub struct SelfPlayConfig {
    /// AI configuration for player 1.
    pub player1: PlayerConfig,
    /// AI configuration for player 2.
    pub player2: PlayerConfig,
    /// Number of games to play.
    pub num_games: usize,
    /// Base seed for deterministic game generation.
    pub seed_base: u64,
    /// Missions to cycle through (None entries use default).
    pub missions: Vec<Option<MissionId>>,
    /// Output directory for training shards (None = don't write).
    pub output_dir: Option<PathBuf>,
    /// Maximum samples per shard file.
    pub shard_size: usize,
    /// Whether to collect training data at each decision point.
    pub collect_training_data: bool,
    /// Whether to record replays.
    pub collect_replays: bool,
    /// How often to log progress (every N games).
    pub log_interval: usize,
    /// Whether to alternate which player plays which faction.
    pub alternate_factions: bool,
    /// Model generation (for shard metadata).
    pub model_generation: u32,
}

impl SelfPlayConfig {
    /// Create a default self-play config with greedy AI.
    pub fn default_greedy(num_games: usize) -> Self {
        Self {
            player1: PlayerConfig::greedy("Greedy-1"),
            player2: PlayerConfig::greedy("Greedy-2"),
            num_games,
            seed_base: 42,
            missions: vec![
                Some(MissionId::new(0)),
                Some(MissionId::new(1)),
                Some(MissionId::new(2)),
                Some(MissionId::new(3)),
                Some(MissionId::new(4)),
                Some(MissionId::new(5)),
            ],
            output_dir: None,
            shard_size: DEFAULT_SHARD_SIZE,
            collect_training_data: true,
            collect_replays: false,
            log_interval: 10,
            alternate_factions: true,
            model_generation: 0,
        }
    }

    /// Set the output directory for training shards.
    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = Some(dir);
        self
    }

    /// Enable replay collection.
    pub fn with_replays(mut self) -> Self {
        self.collect_replays = true;
        self
    }

    /// Set the seed base.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed_base = seed;
        self
    }
}

// ============================================================================
// 8.2: Self-Play Report
// ============================================================================

/// Aggregate statistics from a self-play run.
#[derive(Debug, Clone)]
pub struct SelfPlayReport {
    /// Total games completed.
    pub total_games: usize,
    /// Games won by player 1.
    pub player1_wins: usize,
    /// Games won by player 2.
    pub player2_wins: usize,
    /// Games that ended in a draw.
    pub draws: usize,
    /// Games that terminated abnormally.
    pub abnormal_terminations: usize,
    /// Total commands executed across all games.
    pub total_commands: usize,
    /// Total training samples collected.
    pub total_samples: usize,
    /// Total wall-clock time in milliseconds.
    pub total_duration_ms: u64,
    /// Throughput in games per hour.
    pub games_per_hour: f64,
    /// Average commands per game.
    pub avg_commands_per_game: f64,
    /// Average battle rounds per game.
    pub avg_rounds_per_game: f64,
    /// Average duration per game in milliseconds.
    pub avg_duration_ms: f64,
    /// Player 1 win rate (0.0 to 1.0).
    pub player1_win_rate: f64,
    /// Individual game results.
    pub results: Vec<MatchResult>,
}

impl std::fmt::Display for SelfPlayReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SelfPlay Report: {} games | P1 wins: {} ({:.1}%) | P2 wins: {} | Draws: {} | \
             Abnormal: {} | Samples: {} | {:.1} games/hr | Avg {:.0} cmds/game | Avg {:.1} rounds/game",
            self.total_games,
            self.player1_wins,
            self.player1_win_rate * 100.0,
            self.player2_wins,
            self.draws,
            self.abnormal_terminations,
            self.total_samples,
            self.games_per_hour,
            self.avg_commands_per_game,
            self.avg_rounds_per_game,
        )
    }
}

// ============================================================================
// 8.2: Game Variation
// ============================================================================

/// Generates diverse match configurations for self-play.
///
/// Varies seeds, missions, faction assignments, enhancements, and secondaries
/// to produce a diverse training set.
pub struct GameVariation;

impl GameVariation {
    /// Generate match configs for a self-play batch.
    ///
    /// # Arguments
    /// * `num_games` - Number of games to generate configs for
    /// * `seed_base` - Base seed for determinism
    /// * `missions` - Available missions to cycle through
    /// * `alternate_factions` - Whether to swap factions every other game
    ///
    /// # Returns
    /// A Vec of MatchConfig, one per game.
    pub fn generate_configs(
        num_games: usize,
        seed_base: u64,
        missions: &[Option<MissionId>],
        alternate_factions: bool,
    ) -> Vec<MatchConfig> {
        let mut configs = Vec::with_capacity(num_games);
        let mut rng = SmallRng::seed_from_u64(seed_base);

        // Available factions
        let factions = [FactionId::new(0), FactionId::new(1)]; // Custodes, World Eaters

        // Available enhancements per faction
        // Custodes: EnhancementId 0 (Watchman of Terra), 1 (Warrior Exemplar)
        // World Eaters: EnhancementId 2 (Fearsome Presence), 3 (Bane of the Craven)
        let custodes_enhancements = [EnhancementId::new(0), EnhancementId::new(1)];
        let world_eaters_enhancements = [EnhancementId::new(2), EnhancementId::new(3)];

        // Available secondaries per faction
        // Custodes: SecondaryObjectiveId 0 (Raise the Vexillas), 1 (Consecrated Ground)
        // World Eaters: SecondaryObjectiveId 2 (Champions of Khorne), 3 (Skull Takers)
        let custodes_secondaries = [
            SecondaryObjectiveId::new(0),
            SecondaryObjectiveId::new(1),
        ];
        let world_eaters_secondaries = [
            SecondaryObjectiveId::new(2),
            SecondaryObjectiveId::new(3),
        ];

        for game_idx in 0..num_games {
            let seed = generate_seed(seed_base, game_idx);

            // Cycle through missions
            let mission = if !missions.is_empty() {
                missions[game_idx % missions.len()]
            } else {
                Some(MissionId::new(0))
            };

            // Determine faction assignment
            let (p1_faction, p2_faction) = if alternate_factions && game_idx % 2 == 1 {
                (factions[1], factions[0]) // Swap
            } else {
                (factions[0], factions[1]) // Normal
            };

            // Randomly select enhancements
            let p1_enh = if p1_faction == FactionId::new(0) {
                Some(custodes_enhancements[rng.gen_range(0..custodes_enhancements.len())])
            } else {
                Some(world_eaters_enhancements[rng.gen_range(0..world_eaters_enhancements.len())])
            };

            let p2_enh = if p2_faction == FactionId::new(0) {
                Some(custodes_enhancements[rng.gen_range(0..custodes_enhancements.len())])
            } else {
                Some(world_eaters_enhancements[rng.gen_range(0..world_eaters_enhancements.len())])
            };

            // Randomly select secondaries
            let p1_sec = if p1_faction == FactionId::new(0) {
                Some(custodes_secondaries[rng.gen_range(0..custodes_secondaries.len())])
            } else {
                Some(world_eaters_secondaries[rng.gen_range(0..world_eaters_secondaries.len())])
            };

            let p2_sec = if p2_faction == FactionId::new(0) {
                Some(custodes_secondaries[rng.gen_range(0..custodes_secondaries.len())])
            } else {
                Some(world_eaters_secondaries[rng.gen_range(0..world_eaters_secondaries.len())])
            };

            configs.push(MatchConfig {
                seed,
                mission_id: mission,
                player1_faction: p1_faction,
                player2_faction: p2_faction,
                player1_enhancement: p1_enh,
                player2_enhancement: p2_enh,
                player1_secondary: p1_sec,
                player2_secondary: p2_sec,
            });
        }

        configs
    }
}

// ============================================================================
// 8.2: Self-Play Runner
// ============================================================================

/// Orchestrates a batch of self-play games.
///
/// The runner:
/// 1. Generates diverse match configurations
/// 2. Plays each game using the configured AI workers
/// 3. Collects training samples at each decision point
/// 4. Labels samples with game outcomes
/// 5. Writes training shards to disk
/// 6. Compiles aggregate statistics
pub struct SelfPlayRunner {
    config: SelfPlayConfig,
}

impl SelfPlayRunner {
    /// Create a new self-play runner with the given configuration.
    pub fn new(config: SelfPlayConfig) -> Self {
        Self { config }
    }

    /// Run the full self-play batch and return a report.
    pub fn run(&self) -> Result<SelfPlayReport, SelfPlayError> {
        let run_start = Instant::now();

        // Generate match configurations
        let match_configs = GameVariation::generate_configs(
            self.config.num_games,
            self.config.seed_base,
            &self.config.missions,
            self.config.alternate_factions,
        );

        // Initialize shard writer if output directory is configured
        let mut shard_writer = if let Some(ref dir) = self.config.output_dir {
            Some(ShardWriter::new(
                dir.clone(),
                self.config.shard_size,
                self.config.model_generation,
            )?)
        } else {
            None
        };

        let mut results: Vec<MatchResult> = Vec::with_capacity(self.config.num_games);
        let mut total_samples: usize = 0;

        for (game_idx, match_config) in match_configs.iter().enumerate() {
            // Play the game
            let result = play_single_game(
                match_config,
                &self.config.player1,
                &self.config.player2,
                self.config.collect_training_data,
                self.config.collect_replays,
            )?;

            // Write training samples to shard writer
            if let Some(ref mut writer) = shard_writer {
                if !result.training_samples.is_empty() {
                    total_samples += result.training_samples.len();
                    writer.add_game_samples(result.training_samples.clone())?;
                }
            } else {
                total_samples += result.training_samples.len();
            }

            results.push(result);

            // Log progress
            if self.config.log_interval > 0 && (game_idx + 1) % self.config.log_interval == 0 {
                let elapsed_ms = run_start.elapsed().as_millis() as u64;
                let games_done = game_idx + 1;
                let gph = if elapsed_ms > 0 {
                    games_done as f64 / (elapsed_ms as f64 / 3_600_000.0)
                } else {
                    0.0
                };
                // Progress is logged to stderr for non-blocking output
                eprintln!(
                    "[SelfPlay] {}/{} games complete | {:.0} games/hr | {} samples",
                    games_done, self.config.num_games, gph, total_samples
                );
            }
        }

        // Finalize shard writer
        if let Some(writer) = shard_writer {
            let stats = writer.finalize()?;
            eprintln!("[SelfPlay] {}", stats);
        }

        // Compile report
        let total_duration_ms = run_start.elapsed().as_millis() as u64;
        let report = Self::compile_report(results, total_samples, total_duration_ms);

        Ok(report)
    }

    /// Run a single game with the runner's configuration but a specific match config.
    pub fn run_single(&self, match_config: &MatchConfig) -> Result<MatchResult, SelfPlayError> {
        play_single_game(
            match_config,
            &self.config.player1,
            &self.config.player2,
            self.config.collect_training_data,
            self.config.collect_replays,
        )
    }

    /// Compile aggregate statistics from game results.
    fn compile_report(
        results: Vec<MatchResult>,
        total_samples: usize,
        total_duration_ms: u64,
    ) -> SelfPlayReport {
        let total_games = results.len();
        let player1_wins = results.iter().filter(|r| r.player1_won()).count();
        let player2_wins = results.iter().filter(|r| r.player2_won()).count();
        let draws = results.iter().filter(|r| r.is_draw()).count();
        let abnormal = results.iter().filter(|r| !r.normal_termination).count();
        let total_commands: usize = results.iter().map(|r| r.total_commands).sum();
        let total_rounds: usize = results.iter().map(|r| r.total_rounds as usize).sum();

        let games_per_hour = if total_duration_ms > 0 {
            total_games as f64 / (total_duration_ms as f64 / 3_600_000.0)
        } else {
            0.0
        };

        let avg_commands = if total_games > 0 {
            total_commands as f64 / total_games as f64
        } else {
            0.0
        };

        let avg_rounds = if total_games > 0 {
            total_rounds as f64 / total_games as f64
        } else {
            0.0
        };

        let avg_duration = if total_games > 0 {
            total_duration_ms as f64 / total_games as f64
        } else {
            0.0
        };

        let completed_games = player1_wins + player2_wins + draws;
        let p1_wr = if completed_games > 0 {
            player1_wins as f64 / completed_games as f64
        } else {
            0.0
        };

        SelfPlayReport {
            total_games,
            player1_wins,
            player2_wins,
            draws,
            abnormal_terminations: abnormal,
            total_commands,
            total_samples,
            total_duration_ms,
            games_per_hour,
            avg_commands_per_game: avg_commands,
            avg_rounds_per_game: avg_rounds,
            avg_duration_ms: avg_duration,
            player1_win_rate: p1_wr,
            results,
        }
    }
}

// ============================================================================
// 8.3: Elo Rating System
// ============================================================================

/// Elo rating calculation utilities.
///
/// Implements the standard Elo rating system for estimating relative
/// model strength from head-to-head match results.
pub struct EloRating;

impl EloRating {
    /// Calculate expected score for player A against player B.
    ///
    /// Returns a value in [0, 1] representing A's expected win rate.
    ///
    /// Formula: E_A = 1 / (1 + 10^((R_B - R_A) / 400))
    pub fn expected_score(rating_a: f64, rating_b: f64) -> f64 {
        1.0 / (1.0 + 10.0_f64.powf((rating_b - rating_a) / 400.0))
    }

    /// Update a player's rating based on a single result.
    ///
    /// # Arguments
    /// * `rating` - Current rating
    /// * `expected` - Expected score (from expected_score)
    /// * `actual` - Actual score (1.0 = win, 0.5 = draw, 0.0 = loss)
    /// * `k` - K-factor (sensitivity; 32 is standard for new players)
    ///
    /// # Returns
    /// The updated rating.
    pub fn update_rating(rating: f64, expected: f64, actual: f64, k: f64) -> f64 {
        rating + k * (actual - expected)
    }

    /// Estimate the Elo delta from a win rate and number of games.
    ///
    /// Uses the inverse of the expected score formula:
    /// delta = 400 * log10(win_rate / (1 - win_rate))
    ///
    /// Clamps the result to [-800, 800] to avoid extreme values.
    pub fn estimate_elo_delta(win_rate: f64, _num_games: usize) -> i32 {
        if win_rate <= 0.0 {
            return -800;
        }
        if win_rate >= 1.0 {
            return 800;
        }
        let delta = 400.0 * (win_rate / (1.0 - win_rate)).log10();
        delta.round().clamp(-800.0, 800.0) as i32
    }

    /// Calculate confidence interval for the Elo estimate.
    ///
    /// Uses the normal approximation to the binomial distribution.
    /// Returns (lower_delta, upper_delta) at approximately 95% confidence.
    pub fn confidence_interval(win_rate: f64, num_games: usize) -> (i32, i32) {
        if num_games == 0 {
            return (0, 0);
        }
        // Standard error of the win rate
        let se = (win_rate * (1.0 - win_rate) / num_games as f64).sqrt();
        let lower_wr = (win_rate - 1.96 * se).max(0.01);
        let upper_wr = (win_rate + 1.96 * se).min(0.99);
        let lower = Self::estimate_elo_delta(lower_wr, num_games);
        let upper = Self::estimate_elo_delta(upper_wr, num_games);
        (lower, upper)
    }

    /// Calculate Elo ratings from a series of match results between two players.
    ///
    /// # Arguments
    /// * `wins_a` - Number of wins for player A
    /// * `wins_b` - Number of wins for player B
    /// * `draws` - Number of draws
    /// * `initial_a` - Initial rating for player A
    /// * `initial_b` - Initial rating for player B
    /// * `k` - K-factor
    ///
    /// # Returns
    /// (final_rating_a, final_rating_b)
    pub fn calculate_ratings(
        wins_a: usize,
        wins_b: usize,
        draws: usize,
        initial_a: f64,
        initial_b: f64,
        _k: f64,
    ) -> (f64, f64) {
        let total = wins_a + wins_b + draws;
        if total == 0 {
            return (initial_a, initial_b);
        }

        // Calculate A's score fraction: wins count 1.0, draws count 0.5
        let score_a = wins_a as f64 + draws as f64 * 0.5;
        let score_fraction = score_a / total as f64;

        // Compute Elo difference from aggregate win rate using the
        // inverse of the logistic expected-score formula:
        //   E(A) = 1 / (1 + 10^((Rb-Ra)/400))
        //   => Ra - Rb = -400 * log10(1/E - 1)
        // Clamp to avoid log(0) or log(inf)
        let clamped = score_fraction.clamp(0.001, 0.999);
        let elo_delta = -400.0 * (1.0 / clamped - 1.0).log10();

        // Split the Elo delta symmetrically around the initial midpoint
        let midpoint = (initial_a + initial_b) / 2.0;
        let rating_a = midpoint + elo_delta / 2.0;
        let rating_b = midpoint - elo_delta / 2.0;

        (rating_a, rating_b)
    }
}

// ============================================================================
// 8.3: Model Lineage
// ============================================================================

/// A single entry in the model lineage tracking system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEntry {
    /// Model identifier.
    pub model_id: String,
    /// Generation number (0 = bootstrap, 1+ = trained).
    pub generation: u32,
    /// Parent model ID (None for generation 0).
    pub parent_model_id: Option<String>,
    /// Number of training positions used.
    pub training_positions: u64,
    /// Training loss at end of training.
    pub training_loss: f64,
    /// Estimated Elo relative to generation 0.
    pub elo_estimate: i32,
    /// Whether this model was promoted (passed gating).
    pub promoted: bool,
    /// Win rate against baseline during gating (if gated).
    pub gating_win_rate: Option<f64>,
    /// Number of gating games played (if gated).
    pub gating_games: Option<usize>,
    /// Elo delta vs baseline from gating (if gated).
    pub gating_elo_delta: Option<i32>,
    /// Creation timestamp (seconds since Unix epoch).
    pub created_at: u64,
    /// Description / notes.
    pub description: String,
}

impl LineageEntry {
    /// Create a bootstrap (generation 0) lineage entry.
    pub fn bootstrap() -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            model_id: "gen0_bootstrap".to_string(),
            generation: 0,
            parent_model_id: None,
            training_positions: 0,
            training_loss: 0.0,
            elo_estimate: 0,
            promoted: true, // Bootstrap is always "promoted" as the starting point
            gating_win_rate: None,
            gating_games: None,
            gating_elo_delta: None,
            created_at,
            description: "Bootstrap generation 0 model with random weights".to_string(),
        }
    }

    /// Create a lineage entry for a newly trained model.
    pub fn new_trained(
        model_id: String,
        generation: u32,
        parent_model_id: String,
        training_positions: u64,
        training_loss: f64,
    ) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            model_id,
            generation,
            parent_model_id: Some(parent_model_id),
            training_positions,
            training_loss,
            elo_estimate: 0,
            promoted: false,
            gating_win_rate: None,
            gating_games: None,
            gating_elo_delta: None,
            created_at,
            description: format!("Trained generation {} model", generation),
        }
    }

    /// Update this entry with gating results.
    pub fn set_gating_result(
        &mut self,
        win_rate: f64,
        games: usize,
        elo_delta: i32,
        promoted: bool,
    ) {
        self.gating_win_rate = Some(win_rate);
        self.gating_games = Some(games);
        self.gating_elo_delta = Some(elo_delta);
        self.promoted = promoted;
        if promoted {
            self.elo_estimate += elo_delta;
        }
    }
}

/// Tracks the lineage (genealogy) of trained models.
///
/// Records each model generation, its training provenance, gating results,
/// and promotion status. Persisted to disk as JSON for human readability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLineage {
    /// All lineage entries, ordered by generation.
    pub entries: Vec<LineageEntry>,
}

impl ModelLineage {
    /// Create a new empty lineage.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a lineage with a bootstrap entry.
    pub fn with_bootstrap() -> Self {
        let mut lineage = Self::new();
        lineage.add_entry(LineageEntry::bootstrap());
        lineage
    }

    /// Add a lineage entry.
    pub fn add_entry(&mut self, entry: LineageEntry) {
        self.entries.push(entry);
    }

    /// Get the latest (most recent) entry.
    pub fn latest(&self) -> Option<&LineageEntry> {
        self.entries.last()
    }

    /// Get the latest promoted model entry.
    pub fn latest_promoted(&self) -> Option<&LineageEntry> {
        self.entries.iter().rev().find(|e| e.promoted)
    }

    /// Get an entry by generation number.
    pub fn get_by_generation(&self, generation: u32) -> Option<&LineageEntry> {
        self.entries.iter().find(|e| e.generation == generation)
    }

    /// Get an entry by model ID.
    pub fn get_by_id(&self, model_id: &str) -> Option<&LineageEntry> {
        self.entries.iter().find(|e| e.model_id == model_id)
    }

    /// Get all promoted models.
    pub fn promoted_models(&self) -> Vec<&LineageEntry> {
        self.entries.iter().filter(|e| e.promoted).collect()
    }

    /// Get the next generation number.
    pub fn next_generation(&self) -> u32 {
        self.entries
            .iter()
            .map(|e| e.generation)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Get the cumulative Elo estimate for the latest promoted model.
    pub fn current_elo(&self) -> i32 {
        self.latest_promoted()
            .map(|e| e.elo_estimate)
            .unwrap_or(0)
    }

    /// Save the lineage to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), SelfPlayError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load the lineage from a JSON file.
    pub fn load(path: &Path) -> Result<Self, SelfPlayError> {
        let json = std::fs::read_to_string(path)?;
        let lineage: Self = serde_json::from_str(&json)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        Ok(lineage)
    }

    /// Total training positions across all generations.
    pub fn total_training_positions(&self) -> u64 {
        self.entries.iter().map(|e| e.training_positions).sum()
    }

    /// Number of generations that were promoted.
    pub fn total_promotions(&self) -> usize {
        self.entries.iter().filter(|e| e.promoted).count()
    }
}

impl Default for ModelLineage {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ModelLineage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ModelLineage: {} generations, {} promoted, current Elo: {}",
            self.entries.len(),
            self.total_promotions(),
            self.current_elo()
        )
    }
}

// ============================================================================
// 8.3: Gating Configuration and Result
// ============================================================================

/// Configuration for the gating harness.
#[derive(Debug, Clone)]
pub struct GatingConfig {
    /// Number of games to play for evaluation.
    pub num_games: usize,
    /// Win rate threshold for promotion (e.g., 0.55 = 55%).
    pub promotion_threshold: f64,
    /// Base seed for deterministic games.
    pub seed_base: u64,
    /// Missions to use during evaluation.
    pub missions: Vec<Option<MissionId>>,
    /// Search configuration for both players during gating.
    pub search_config: Option<SearchConfig>,
    /// Whether to collect training data during gating games.
    pub collect_training_data: bool,
    /// Whether to alternate faction assignments.
    pub alternate_factions: bool,
}

impl GatingConfig {
    /// Create a default gating config.
    pub fn default_config() -> Self {
        Self {
            num_games: DEFAULT_GATING_GAMES,
            promotion_threshold: DEFAULT_PROMOTION_THRESHOLD,
            seed_base: 12345,
            missions: vec![
                Some(MissionId::new(0)),
                Some(MissionId::new(1)),
                Some(MissionId::new(2)),
                Some(MissionId::new(3)),
                Some(MissionId::new(4)),
                Some(MissionId::new(5)),
            ],
            search_config: None,
            collect_training_data: false,
            alternate_factions: true,
        }
    }

    /// Set the number of gating games.
    pub fn with_num_games(mut self, n: usize) -> Self {
        self.num_games = n;
        self
    }

    /// Set the promotion threshold.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.promotion_threshold = threshold;
        self
    }
}

/// Result of a gating evaluation (candidate vs baseline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatingResult {
    /// Candidate model identifier.
    pub candidate_model_id: String,
    /// Baseline model identifier.
    pub baseline_model_id: String,
    /// Games won by the candidate.
    pub candidate_wins: usize,
    /// Games won by the baseline.
    pub baseline_wins: usize,
    /// Games that ended in a draw.
    pub draws: usize,
    /// Total games played.
    pub total_games: usize,
    /// Candidate win rate (0.0 to 1.0).
    pub candidate_win_rate: f64,
    /// Whether the candidate was promoted (win rate >= threshold).
    pub promoted: bool,
    /// Promotion threshold that was used.
    pub promotion_threshold: f64,
    /// Estimated Elo delta (candidate relative to baseline).
    pub elo_delta: i32,
    /// Candidate Elo rating (absolute).
    pub candidate_elo: f64,
    /// Baseline Elo rating (absolute).
    pub baseline_elo: f64,
    /// 95% confidence interval for the Elo delta.
    pub elo_confidence_interval: (i32, i32),
    /// Wall-clock duration of gating in milliseconds.
    pub duration_ms: u64,
    /// Total training samples collected (if enabled).
    pub total_samples: usize,
}

impl std::fmt::Display for GatingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gating: {} vs {} | {}/{}/{} (W/L/D) | WR: {:.1}% | {} | Elo: {:+} [{:+}, {:+}]",
            self.candidate_model_id,
            self.baseline_model_id,
            self.candidate_wins,
            self.baseline_wins,
            self.draws,
            self.candidate_win_rate * 100.0,
            if self.promoted {
                "PROMOTED"
            } else {
                "REJECTED"
            },
            self.elo_delta,
            self.elo_confidence_interval.0,
            self.elo_confidence_interval.1,
        )
    }
}

// ============================================================================
// 8.3: Gating Harness
// ============================================================================

/// The gating harness evaluates candidate NNUE models against a baseline.
///
/// A candidate model must achieve a win rate above the promotion threshold
/// (default 55%) to be promoted. Gating uses diverse game configurations
/// and alternates faction assignments for fair evaluation.
///
/// The harness:
/// 1. Creates AI workers for candidate and baseline
/// 2. Plays a series of games with alternating colors
/// 3. Calculates win rate, Elo delta, and confidence intervals
/// 4. Decides whether to promote the candidate
pub struct GatingHarness {
    config: GatingConfig,
}

impl GatingHarness {
    /// Create a new gating harness with the given configuration.
    pub fn new(config: GatingConfig) -> Self {
        Self { config }
    }

    /// Evaluate a candidate model against a baseline model.
    ///
    /// # Arguments
    /// * `candidate` - The candidate NNUE model artifact to evaluate
    /// * `baseline` - The baseline NNUE model artifact to evaluate against
    ///
    /// # Returns
    /// A GatingResult with the evaluation outcome.
    pub fn evaluate(
        &self,
        candidate: &NnueModelArtifact,
        baseline: &NnueModelArtifact,
    ) -> Result<GatingResult, SelfPlayError> {
        let start = Instant::now();

        let candidate_config = PlayerConfig::nnue_with_model("Candidate", candidate.clone());
        let baseline_config = PlayerConfig::nnue_with_model("Baseline", baseline.clone());

        let match_configs = GameVariation::generate_configs(
            self.config.num_games,
            self.config.seed_base,
            &self.config.missions,
            self.config.alternate_factions,
        );

        let mut candidate_wins: usize = 0;
        let mut baseline_wins: usize = 0;
        let mut draws: usize = 0;
        let mut total_samples: usize = 0;

        for (game_idx, match_config) in match_configs.iter().enumerate() {
            // Alternate which player slot the candidate occupies
            let (p1_config, p2_config) = if game_idx % 2 == 0 {
                (&candidate_config, &baseline_config)
            } else {
                (&baseline_config, &candidate_config)
            };

            let result = play_single_game(
                match_config,
                p1_config,
                p2_config,
                self.config.collect_training_data,
                false, // No replays during gating
            )?;

            total_samples += result.training_samples.len();

            // Determine who won relative to candidate/baseline
            let candidate_is_p1 = game_idx % 2 == 0;
            match result.outcome {
                GameOutcome::Victory(winner) => {
                    let winner_is_p1 = winner == PlayerId::new(0);
                    if (candidate_is_p1 && winner_is_p1)
                        || (!candidate_is_p1 && !winner_is_p1)
                    {
                        candidate_wins += 1;
                    } else {
                        baseline_wins += 1;
                    }
                }
                GameOutcome::Draw => {
                    draws += 1;
                }
                GameOutcome::InProgress => {
                    // Abnormal termination - count as draw
                    draws += 1;
                }
            }
        }

        let total_games = candidate_wins + baseline_wins + draws;
        let completed_games = candidate_wins + baseline_wins + draws;
        let candidate_win_rate = if completed_games > 0 {
            (candidate_wins as f64 + 0.5 * draws as f64) / completed_games as f64
        } else {
            0.5
        };

        let promoted = candidate_win_rate >= self.config.promotion_threshold;
        let elo_delta = EloRating::estimate_elo_delta(candidate_win_rate, total_games);
        let confidence = EloRating::confidence_interval(candidate_win_rate, total_games);

        let (candidate_elo, baseline_elo) = EloRating::calculate_ratings(
            candidate_wins,
            baseline_wins,
            draws,
            DEFAULT_ELO_INITIAL,
            DEFAULT_ELO_INITIAL,
            DEFAULT_ELO_K,
        );

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(GatingResult {
            candidate_model_id: candidate.metadata.model_id.clone(),
            baseline_model_id: baseline.metadata.model_id.clone(),
            candidate_wins,
            baseline_wins,
            draws,
            total_games,
            candidate_win_rate,
            promoted,
            promotion_threshold: self.config.promotion_threshold,
            elo_delta,
            candidate_elo,
            baseline_elo,
            elo_confidence_interval: confidence,
            duration_ms,
            total_samples,
        })
    }

    /// Run a full training cycle: self-play data collection -> gating.
    ///
    /// This is a convenience method that:
    /// 1. Runs self-play with the baseline model to collect training data
    /// 2. Evaluates the candidate against the baseline
    ///
    /// The caller is responsible for training the candidate model between
    /// data collection and gating (typically done in Python).
    pub fn evaluate_heuristic_vs_nnue(
        &self,
        nnue_artifact: &NnueModelArtifact,
    ) -> Result<GatingResult, SelfPlayError> {
        let start = Instant::now();

        // Use iterative deepening (depth 6) for both sides so the NNUE
        // is tested as Perturabo's evaluator — its actual use case.
        let mut nnue_config = PlayerConfig::nnue_with_model("NNUE", nnue_artifact.clone());
        nnue_config.ai_type = AiType::IterativeDeepening(6);
        let heuristic_config = PlayerConfig::iterative_deepening("Heuristic", 6);

        let match_configs = GameVariation::generate_configs(
            self.config.num_games,
            self.config.seed_base,
            &self.config.missions,
            self.config.alternate_factions,
        );

        let mut nnue_wins: usize = 0;
        let mut heuristic_wins: usize = 0;
        let mut draws: usize = 0;

        for (game_idx, match_config) in match_configs.iter().enumerate() {
            let (p1_config, p2_config) = if game_idx % 2 == 0 {
                (&nnue_config, &heuristic_config)
            } else {
                (&heuristic_config, &nnue_config)
            };

            let result = play_single_game(match_config, p1_config, p2_config, false, false)?;

            let nnue_is_p1 = game_idx % 2 == 0;
            match result.outcome {
                GameOutcome::Victory(winner) => {
                    let winner_is_p1 = winner == PlayerId::new(0);
                    if (nnue_is_p1 && winner_is_p1) || (!nnue_is_p1 && !winner_is_p1) {
                        nnue_wins += 1;
                    } else {
                        heuristic_wins += 1;
                    }
                }
                GameOutcome::Draw => draws += 1,
                GameOutcome::InProgress => draws += 1,
            }
        }

        let total_games = nnue_wins + heuristic_wins + draws;
        let completed = nnue_wins + heuristic_wins + draws;
        let win_rate = if completed > 0 {
            (nnue_wins as f64 + 0.5 * draws as f64) / completed as f64
        } else {
            0.5
        };

        let promoted = win_rate >= self.config.promotion_threshold;
        let elo_delta = EloRating::estimate_elo_delta(win_rate, total_games);
        let confidence = EloRating::confidence_interval(win_rate, total_games);
        let (nnue_elo, heur_elo) = EloRating::calculate_ratings(
            nnue_wins,
            heuristic_wins,
            draws,
            DEFAULT_ELO_INITIAL,
            DEFAULT_ELO_INITIAL,
            DEFAULT_ELO_K,
        );

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(GatingResult {
            candidate_model_id: nnue_artifact.metadata.model_id.clone(),
            baseline_model_id: "heuristic_default".to_string(),
            candidate_wins: nnue_wins,
            baseline_wins: heuristic_wins,
            draws,
            total_games,
            candidate_win_rate: win_rate,
            promoted,
            promotion_threshold: self.config.promotion_threshold,
            elo_delta,
            candidate_elo: nnue_elo,
            baseline_elo: heur_elo,
            elo_confidence_interval: confidence,
            duration_ms,
            total_samples: 0,
        })
    }
}

// ============================================================================
// Policy Target Encoding (Phase 10: AlphaGo Expansion)
// ============================================================================

/// A training sample for policy/value training (dual-head network).
/// Extends TrainingSample with policy target from MCTS visit distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyValueSample {
    /// Sparse features for the position (same as TrainingSample).
    pub sparse_features: Vec<(u16, i16)>,
    /// Legal action mask over ACTION_VOCAB_SIZE (528).
    pub legal_mask: LegalMask,
    /// Policy target: probability distribution over ACTION_VOCAB_SIZE.
    /// From MCTS visit counts (normalized). Zero for illegal actions.
    pub policy_target: Vec<f32>,
    /// Value target: game outcome from this position's perspective.
    /// +1.0 = win, -1.0 = loss, 0.0 = draw.
    pub value_target: f32,
    /// Which player's perspective this sample is from.
    pub perspective: u32,
    /// Game progress (0.0 = start, 1.0 = end).
    pub progress: f32,
    /// Optional search score at this position.
    pub search_score: Option<i32>,
}

/// Encode a policy target from MCTS visit distribution over candidates.
/// Maps candidate-level visit proportions to the fixed 528-action vocabulary.
pub fn encode_policy_target(
    visit_distribution: &[(usize, f32)],
    candidates: &[wh40k_search_abstraction::MacroAction],
    state: &GameState,
    perspective: PlayerId,
) -> Vec<f32> {
    let mut policy = vec![0.0f32; ACTION_VOCAB_SIZE];

    for &(candidate_idx, visit_prob) in visit_distribution {
        if candidate_idx >= candidates.len() {
            continue;
        }
        let action = &candidates[candidate_idx];
        let vocab_idx = action_to_vocab_index(action, state, perspective);
        if (vocab_idx as usize) < ACTION_VOCAB_SIZE {
            policy[vocab_idx as usize] += visit_prob;
        }
    }

    // Ensure it sums to 1.0
    let sum: f32 = policy.iter().sum();
    if sum > 0.0 {
        policy.iter_mut().for_each(|p| *p /= sum);
    }

    policy
}

/// Encode a uniform policy target over legal actions (for initial training).
pub fn encode_uniform_policy_target(
    candidates: &[wh40k_search_abstraction::MacroAction],
    state: &GameState,
    perspective: PlayerId,
) -> Vec<f32> {
    let n = candidates.len();
    if n == 0 {
        return vec![0.0f32; ACTION_VOCAB_SIZE];
    }

    let uniform_prob = 1.0 / n as f32;
    let distribution: Vec<(usize, f32)> = (0..n).map(|i| (i, uniform_prob)).collect();
    encode_policy_target(&distribution, candidates, state, perspective)
}

/// Schema version for the policy/value training data format.
pub const POLICY_VALUE_FORMAT_VERSION: u32 = 1;

/// Shard for policy/value training samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyValueShard {
    pub format_version: u32,
    pub engine_version: String,
    pub samples: Vec<PolicyValueSample>,
}

impl PolicyValueShard {
    /// Create a new shard.
    pub fn new(samples: Vec<PolicyValueSample>) -> Self {
        Self {
            format_version: POLICY_VALUE_FORMAT_VERSION,
            engine_version: ENGINE_VERSION.to_string(),
            samples,
        }
    }

    /// Save shard to file as JSON.
    pub fn save_json(&self, path: &std::path::Path) -> Result<(), SelfPlayError> {
        let json = serde_json::to_string(self)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        std::fs::write(path, json)
            .map_err(SelfPlayError::IoError)?;
        Ok(())
    }

    /// Save shard to file as bincode.
    pub fn save_bincode(&self, path: &std::path::Path) -> Result<(), SelfPlayError> {
        let data = bincode::serialize(self)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        std::fs::write(path, data)
            .map_err(SelfPlayError::IoError)?;
        Ok(())
    }

    /// Load shard from JSON file.
    pub fn load_json(path: &std::path::Path) -> Result<Self, SelfPlayError> {
        let data = std::fs::read_to_string(path)
            .map_err(SelfPlayError::IoError)?;
        let shard: Self = serde_json::from_str(&data)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        Ok(shard)
    }

    /// Load shard from bincode file.
    pub fn load_bincode(path: &std::path::Path) -> Result<Self, SelfPlayError> {
        let data = std::fs::read(path)
            .map_err(SelfPlayError::IoError)?;
        let shard: Self = bincode::deserialize(&data)
            .map_err(|e| SelfPlayError::SerializationError(e.to_string()))?;
        Ok(shard)
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Run a quick self-play benchmark: N greedy games and return throughput.
pub fn benchmark_selfplay_throughput(num_games: usize) -> Result<SelfPlayReport, SelfPlayError> {
    let config = SelfPlayConfig::default_greedy(num_games);
    let runner = SelfPlayRunner::new(config);
    runner.run()
}

/// Run a single test game and return the result.
pub fn run_test_game() -> Result<MatchResult, SelfPlayError> {
    let seed = generate_seed(42, 0);
    let match_config = MatchConfig::default_match(seed);
    let p1 = PlayerConfig::greedy("Test-P1");
    let p2 = PlayerConfig::greedy("Test-P2");
    play_single_game(&match_config, &p1, &p2, false, false)
}

/// Collect training data from N self-play games using greedy AI.
pub fn collect_training_data(
    num_games: usize,
    output_dir: PathBuf,
) -> Result<SelfPlayReport, SelfPlayError> {
    let config = SelfPlayConfig::default_greedy(num_games).with_output_dir(output_dir);
    let runner = SelfPlayRunner::new(config);
    runner.run()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helper ---

    fn test_state() -> GameState {
        ScenarioLoader::load_scenario(
            FactionId::new(0),
            FactionId::new(1),
            Some(MissionId::new(0)),
            [0u8; 32],
        )
    }

    // --- 8.1: Encoding Tests ---

    #[test]
    fn test_intent_to_index_roundtrip() {
        let intents = [
            TacticalIntent::HoldObjective,
            TacticalIntent::ContestObjective,
            TacticalIntent::MoveToCover,
            TacticalIntent::StageCharge,
            TacticalIntent::Screen,
            TacticalIntent::Retreat,
            TacticalIntent::LineUpShot,
            TacticalIntent::DenyReserveZone,
            TacticalIntent::HoldPosition,
            TacticalIntent::AdvanceAggressively,
            TacticalIntent::FallBackToShoot,
            TacticalIntent::DeepStrikeArrive,
            TacticalIntent::MaximizeKills,
            TacticalIntent::SoftenChargeTarget,
            TacticalIntent::RemoveScoringUnit,
            TacticalIntent::ForceBattleShock,
            TacticalIntent::BracketHardTarget,
            TacticalIntent::ChargeHighValueTarget,
            TacticalIntent::MultiChargeObjectiveSwing,
            TacticalIntent::ChargeSoftenedTarget,
            TacticalIntent::FightMaxDamage,
            TacticalIntent::FightPreserve,
            TacticalIntent::ChooseStance,
            TacticalIntent::ChooseWeaponProfile,
            TacticalIntent::DefensiveStratagem,
            TacticalIntent::OffensiveStratagem,
            TacticalIntent::MovementReaction,
            TacticalIntent::FightOrderManipulation,
            TacticalIntent::DeclineStratagem,
            TacticalIntent::ScoreObjective,
            TacticalIntent::AllocateBlessings,
            TacticalIntent::PhaseControl,
            TacticalIntent::Generic,
        ];

        for (i, intent) in intents.iter().enumerate() {
            assert_eq!(intent_to_index(*intent), i, "Failed for {:?}", intent);
            assert_eq!(index_to_intent(i), *intent, "Reverse failed for index {}", i);
        }
    }

    #[test]
    fn test_intent_count_matches() {
        assert_eq!(INTENT_COUNT, 33);
        assert_eq!(ACTION_VOCAB_SIZE, 33 * 16);
    }

    #[test]
    fn test_encode_state_dimensions() {
        let state = test_state();
        let features = encode_state(&state, PlayerId::new(0));
        assert_eq!(features.len(), TOTAL_FEATURES);
    }

    #[test]
    fn test_encode_state_not_all_zeros() {
        let state = test_state();
        let features = encode_state(&state, PlayerId::new(0));
        let non_zero = features.iter().filter(|&&v| v != 0.0).count();
        assert!(
            non_zero > 0,
            "Encoded state should have non-zero features"
        );
    }

    #[test]
    fn test_encode_sparse_features() {
        let state = test_state();
        let sparse = encode_sparse_features(&state, PlayerId::new(0));
        assert!(!sparse.is_empty(), "Should have non-empty sparse features");

        // All indices should be within bounds
        for &(idx, _) in &sparse {
            assert!(
                (idx as usize) < TOTAL_FEATURES,
                "Feature index {} out of bounds",
                idx
            );
        }
    }

    #[test]
    fn test_encode_legal_mask() {
        let state = test_state();
        let mask = encode_legal_mask(&state, PlayerId::new(0));
        assert_eq!(mask.vocab_size, ACTION_VOCAB_SIZE);
        assert!(mask.action_count > 0, "Should have at least one legal action");
        assert_eq!(mask.mask.len(), ACTION_VOCAB_SIZE);

        // Legal indices should match mask
        let legal = mask.legal_indices();
        assert_eq!(legal.len(), mask.action_count);
        for idx in &legal {
            assert!(mask.is_legal(*idx as usize));
        }
    }

    #[test]
    fn test_legal_mask_dense() {
        let state = test_state();
        let mask = encode_legal_mask(&state, PlayerId::new(0));
        let dense = mask.to_dense_f32();
        assert_eq!(dense.len(), ACTION_VOCAB_SIZE);

        let ones: usize = dense.iter().filter(|&&v| v == 1.0).count();
        assert_eq!(ones, mask.action_count);
    }

    #[test]
    fn test_action_to_vocab_index_bounded() {
        let state = test_state();
        let candidates = ActionGenerator::generate(&state, PlayerId::new(0));
        for action in &candidates.candidates {
            let idx = action_to_vocab_index(action, &state, PlayerId::new(0));
            assert!(
                (idx as usize) < ACTION_VOCAB_SIZE,
                "Action vocab index {} out of bounds for action {:?}",
                idx,
                action.intent
            );
        }
    }

    #[test]
    fn test_unit_to_slot_bounded() {
        let state = test_state();
        for unit in &state.units {
            let slot = unit_to_slot(unit.id, &state, unit.owner);
            assert!(
                slot < MAX_UNIT_SLOTS,
                "Unit slot {} out of bounds for unit {:?}",
                slot,
                unit.id
            );
        }
    }

    // --- 8.1: Shard Tests ---

    #[test]
    fn test_training_sample_serialization_roundtrip() {
        let sample = TrainingSample {
            sparse_features: vec![(0, 100), (5, 200), (100, 50)],
            legal_action_indices: vec![0, 5, 10, 31 * 16],
            chosen_action_index: 5,
            search_score: 150,
            outcome: 1.0,
            perspective: 0,
            game_progress: 0.4,
            state_hash: 0xDEADBEEF,
            battle_round: 2,
            phase: 1,
        };

        let bytes = bincode::serialize(&sample).unwrap();
        let restored: TrainingSample = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.sparse_features, sample.sparse_features);
        assert_eq!(restored.chosen_action_index, sample.chosen_action_index);
        assert_eq!(restored.search_score, sample.search_score);
        assert_eq!(restored.outcome, sample.outcome);
        assert_eq!(restored.state_hash, sample.state_hash);
    }

    #[test]
    fn test_training_sample_dense_features() {
        let sample = TrainingSample {
            sparse_features: vec![(0, 1000), (5, 500)],
            legal_action_indices: vec![],
            chosen_action_index: 0,
            search_score: 0,
            outcome: 0.0,
            perspective: 0,
            game_progress: 0.0,
            state_hash: 0,
            battle_round: 1,
            phase: 0,
        };

        let dense = sample.to_dense_features();
        assert_eq!(dense.len(), TOTAL_FEATURES);
        assert!((dense[0] - 1.0).abs() < 0.001);
        assert!((dense[5] - 0.5).abs() < 0.001);
        assert_eq!(dense[1], 0.0);
    }

    #[test]
    fn test_shard_header() {
        let header = ShardHeader::new(0);
        assert_eq!(header.version, SHARD_FORMAT_VERSION);
        assert_eq!(header.feature_schema_version, FEATURE_SCHEMA_VERSION);
        assert_eq!(header.total_features, TOTAL_FEATURES);
        assert_eq!(header.action_vocab_size, ACTION_VOCAB_SIZE);
        assert_eq!(header.sample_count, 0);
        assert_eq!(header.game_count, 0);
    }

    #[test]
    fn test_shard_bincode_roundtrip() {
        let mut shard = TrainingShard::new(0);
        shard.add_sample(TrainingSample {
            sparse_features: vec![(1, 100)],
            legal_action_indices: vec![0, 1],
            chosen_action_index: 0,
            search_score: 50,
            outcome: 1.0,
            perspective: 0,
            game_progress: 0.2,
            state_hash: 42,
            battle_round: 1,
            phase: 0,
        });
        shard.add_sample(TrainingSample {
            sparse_features: vec![(2, 200)],
            legal_action_indices: vec![5],
            chosen_action_index: 5,
            search_score: -30,
            outcome: -1.0,
            perspective: 1,
            game_progress: 0.8,
            state_hash: 99,
            battle_round: 4,
            phase: 2,
        });

        let bytes = shard.to_bytes().unwrap();
        let restored = TrainingShard::from_bytes(&bytes).unwrap();
        assert_eq!(restored.header.sample_count, 2);
        assert_eq!(restored.samples.len(), 2);
        assert_eq!(restored.samples[0].search_score, 50);
        assert_eq!(restored.samples[1].outcome, -1.0);
    }

    #[test]
    fn test_shard_json_roundtrip() {
        let mut shard = TrainingShard::new(1);
        shard.add_sample(TrainingSample {
            sparse_features: vec![(10, 500)],
            legal_action_indices: vec![0],
            chosen_action_index: 0,
            search_score: 100,
            outcome: 0.0,
            perspective: 0,
            game_progress: 0.5,
            state_hash: 1234,
            battle_round: 3,
            phase: 1,
        });

        let json = shard.to_json().unwrap();
        let restored = TrainingShard::from_json(&json).unwrap();
        assert_eq!(restored.header.sample_count, 1);
        assert_eq!(restored.samples[0].search_score, 100);
    }

    #[test]
    fn test_shard_game_samples() {
        let mut shard = TrainingShard::new(0);
        let samples = vec![
            TrainingSample {
                sparse_features: vec![],
                legal_action_indices: vec![],
                chosen_action_index: 0,
                search_score: 0,
                outcome: 1.0,
                perspective: 0,
                game_progress: 0.0,
                state_hash: 0,
                battle_round: 1,
                phase: 0,
            },
            TrainingSample {
                sparse_features: vec![],
                legal_action_indices: vec![],
                chosen_action_index: 0,
                search_score: 0,
                outcome: 1.0,
                perspective: 0,
                game_progress: 0.0,
                state_hash: 0,
                battle_round: 1,
                phase: 0,
            },
        ];

        shard.add_game_samples(samples);
        assert_eq!(shard.header.sample_count, 2);
        assert_eq!(shard.header.game_count, 1);

        shard.add_game_samples(vec![TrainingSample {
            sparse_features: vec![],
            legal_action_indices: vec![],
            chosen_action_index: 0,
            search_score: 0,
            outcome: -1.0,
            perspective: 1,
            game_progress: 0.0,
            state_hash: 0,
            battle_round: 1,
            phase: 0,
        }]);
        assert_eq!(shard.header.sample_count, 3);
        assert_eq!(shard.header.game_count, 2);
    }

    #[test]
    fn test_shard_writer_and_reader() {
        let temp_dir = std::env::temp_dir().join("wh40k_test_shards");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut writer = ShardWriter::new(temp_dir.clone(), 2, 0).unwrap();

        // Add 3 samples (should create 2 shards: one with 2, one with 1)
        for i in 0..3 {
            writer
                .add_sample(TrainingSample {
                    sparse_features: vec![(i as u16, (i * 100) as i16)],
                    legal_action_indices: vec![i as u32],
                    chosen_action_index: i as u32,
                    search_score: i as Score,
                    outcome: 1.0,
                    perspective: 0,
                    game_progress: i as f32 / 3.0,
                    state_hash: i as u64,
                    battle_round: 1,
                    phase: 0,
                })
                .unwrap();
        }

        let stats = writer.finalize().unwrap();
        assert_eq!(stats.total_samples, 3);
        assert_eq!(stats.total_shards, 2);

        // Read shards back
        let shards = ShardReader::read_all_shards(&temp_dir).unwrap();
        assert_eq!(shards.len(), 2);
        assert_eq!(shards[0].samples.len(), 2);
        assert_eq!(shards[1].samples.len(), 1);

        // Read all samples
        let all_samples = ShardReader::read_all_samples(&temp_dir).unwrap();
        assert_eq!(all_samples.len(), 3);

        // Count
        let count = ShardReader::count_samples(&temp_dir).unwrap();
        assert_eq!(count, 3);

        // Validate
        for shard in &shards {
            ShardReader::validate_shard(shard).unwrap();
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    // --- 8.2: Player Config Tests ---

    #[test]
    fn test_player_config_greedy() {
        let config = PlayerConfig::greedy("Test");
        assert_eq!(config.name, "Test");
        assert!(matches!(config.ai_type, AiType::Greedy));
    }

    #[test]
    fn test_player_config_nnue() {
        let config = PlayerConfig::nnue_bootstrap("NnueTest");
        assert!(matches!(config.ai_type, AiType::GreedyNnue));
        assert!(config.model_artifact.is_some());
    }

    #[test]
    fn test_create_ai_worker_greedy() {
        let config = PlayerConfig::greedy("Test");
        let worker = create_ai_worker(&config).unwrap();
        assert_eq!(worker.name(), "GreedyAi");
    }

    #[test]
    fn test_create_ai_worker_one_ply() {
        let config = PlayerConfig::one_ply("Test");
        let worker = create_ai_worker(&config).unwrap();
        assert_eq!(worker.name(), "OnePlySearch");
    }

    #[test]
    fn test_create_ai_worker_negamax() {
        let config = PlayerConfig::negamax("Test", 2);
        let worker = create_ai_worker(&config).unwrap();
        assert_eq!(worker.name(), "NegamaxSearch");
    }

    #[test]
    fn test_create_ai_worker_nnue() {
        let config = PlayerConfig::nnue_bootstrap("Test");
        let worker = create_ai_worker(&config).unwrap();
        assert_eq!(worker.name(), "GreedyAiNnue");
    }

    // --- 8.2: Match Config Tests ---

    #[test]
    fn test_match_config_default() {
        let config = MatchConfig::default_match([42u8; 32]);
        assert_eq!(config.player1_faction, FactionId::new(0));
        assert_eq!(config.player2_faction, FactionId::new(1));
        assert_eq!(config.mission_id, Some(MissionId::new(0)));
    }

    #[test]
    fn test_match_config_swapped() {
        let config = MatchConfig::default_match([0u8; 32]);
        let swapped = config.swapped();
        assert_eq!(swapped.player1_faction, FactionId::new(1));
        assert_eq!(swapped.player2_faction, FactionId::new(0));
    }

    #[test]
    fn test_generate_seed_deterministic() {
        let seed1 = generate_seed(42, 0);
        let seed2 = generate_seed(42, 0);
        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_generate_seed_unique() {
        let seed1 = generate_seed(42, 0);
        let seed2 = generate_seed(42, 1);
        assert_ne!(seed1, seed2);
    }

    // --- 8.2: Game Variation Tests ---

    #[test]
    fn test_game_variation_generates_correct_count() {
        let configs = GameVariation::generate_configs(
            10,
            42,
            &[Some(MissionId::new(0)), Some(MissionId::new(1))],
            true,
        );
        assert_eq!(configs.len(), 10);
    }

    #[test]
    fn test_game_variation_alternates_factions() {
        let configs = GameVariation::generate_configs(
            4,
            42,
            &[Some(MissionId::new(0))],
            true,
        );

        // Even games: Custodes (0) vs World Eaters (1)
        assert_eq!(configs[0].player1_faction, FactionId::new(0));
        assert_eq!(configs[0].player2_faction, FactionId::new(1));

        // Odd games: World Eaters (1) vs Custodes (0) (swapped)
        assert_eq!(configs[1].player1_faction, FactionId::new(1));
        assert_eq!(configs[1].player2_faction, FactionId::new(0));
    }

    #[test]
    fn test_game_variation_cycles_missions() {
        let missions = vec![
            Some(MissionId::new(0)),
            Some(MissionId::new(1)),
            Some(MissionId::new(2)),
        ];
        let configs = GameVariation::generate_configs(6, 42, &missions, false);

        assert_eq!(configs[0].mission_id, Some(MissionId::new(0)));
        assert_eq!(configs[1].mission_id, Some(MissionId::new(1)));
        assert_eq!(configs[2].mission_id, Some(MissionId::new(2)));
        assert_eq!(configs[3].mission_id, Some(MissionId::new(0)));
    }

    #[test]
    fn test_game_variation_has_enhancements() {
        let configs = GameVariation::generate_configs(
            5,
            42,
            &[Some(MissionId::new(0))],
            false,
        );
        for config in &configs {
            assert!(config.player1_enhancement.is_some());
            assert!(config.player2_enhancement.is_some());
            assert!(config.player1_secondary.is_some());
            assert!(config.player2_secondary.is_some());
        }
    }

    // --- 8.2: Self-Play Runner Tests ---

    #[test]
    fn test_selfplay_config_default() {
        let config = SelfPlayConfig::default_greedy(5);
        assert_eq!(config.num_games, 5);
        assert!(config.collect_training_data);
        assert!(!config.collect_replays);
    }

    #[test]
    fn test_run_test_game() {
        let result = run_test_game();
        assert!(result.is_ok(), "Test game should complete: {:?}", result.err());
        let result = result.unwrap();
        assert!(result.total_commands > 0, "Game should have executed commands");
    }

    #[test]
    fn test_play_single_game_with_training_data() {
        let seed = generate_seed(42, 0);
        let match_config = MatchConfig::default_match(seed);
        let p1 = PlayerConfig::greedy("P1");
        let p2 = PlayerConfig::greedy("P2");

        let result = play_single_game(&match_config, &p1, &p2, true, false);
        assert!(result.is_ok(), "Game should complete: {:?}", result.err());

        let result = result.unwrap();
        assert!(
            !result.training_samples.is_empty(),
            "Should have collected training samples"
        );

        // All samples should have outcomes set
        for sample in &result.training_samples {
            assert!(
                sample.outcome == 1.0 || sample.outcome == -1.0 || sample.outcome == 0.0,
                "Outcome should be -1, 0, or +1, got {}",
                sample.outcome
            );
        }
    }

    #[test]
    fn test_play_single_game_with_replay() {
        let seed = generate_seed(99, 0);
        let match_config = MatchConfig::default_match(seed);
        let p1 = PlayerConfig::greedy("P1");
        let p2 = PlayerConfig::greedy("P2");

        let result = play_single_game(&match_config, &p1, &p2, false, true);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.replay.is_some(), "Should have recorded a replay");
    }

    #[test]
    fn test_play_single_game_deterministic() {
        let seed = generate_seed(777, 0);

        let match_config = MatchConfig::default_match(seed);
        let p1 = PlayerConfig::greedy("P1");
        let p2 = PlayerConfig::greedy("P2");

        let result1 = play_single_game(&match_config, &p1, &p2, false, false).unwrap();

        let match_config2 = MatchConfig::default_match(seed);
        let result2 = play_single_game(&match_config2, &p1, &p2, false, false).unwrap();

        assert_eq!(result1.total_commands, result2.total_commands);
        assert_eq!(result1.player1_vp, result2.player1_vp);
        assert_eq!(result1.player2_vp, result2.player2_vp);
    }

    // --- 8.3: Elo Rating Tests ---

    #[test]
    fn test_elo_expected_score_equal() {
        let expected = EloRating::expected_score(1500.0, 1500.0);
        assert!((expected - 0.5).abs() < 0.001, "Equal ratings should give 50% expected score");
    }

    #[test]
    fn test_elo_expected_score_stronger() {
        let expected = EloRating::expected_score(1700.0, 1500.0);
        assert!(expected > 0.5, "Stronger player should have >50% expected score");
        assert!(expected < 1.0);
    }

    #[test]
    fn test_elo_expected_score_weaker() {
        let expected = EloRating::expected_score(1300.0, 1500.0);
        assert!(expected < 0.5, "Weaker player should have <50% expected score");
        assert!(expected > 0.0);
    }

    #[test]
    fn test_elo_expected_score_symmetric() {
        let ea = EloRating::expected_score(1600.0, 1400.0);
        let eb = EloRating::expected_score(1400.0, 1600.0);
        assert!((ea + eb - 1.0).abs() < 0.001, "Expected scores should sum to 1.0");
    }

    #[test]
    fn test_elo_update_win() {
        let rating = EloRating::update_rating(1500.0, 0.5, 1.0, 32.0);
        assert!(rating > 1500.0, "Rating should increase after a win");
    }

    #[test]
    fn test_elo_update_loss() {
        let rating = EloRating::update_rating(1500.0, 0.5, 0.0, 32.0);
        assert!(rating < 1500.0, "Rating should decrease after a loss");
    }

    #[test]
    fn test_elo_update_draw() {
        let rating = EloRating::update_rating(1500.0, 0.5, 0.5, 32.0);
        assert!((rating - 1500.0).abs() < 0.001, "Rating should stay same on expected draw");
    }

    #[test]
    fn test_elo_delta_positive() {
        let delta = EloRating::estimate_elo_delta(0.6, 100);
        assert!(delta > 0, "60% win rate should give positive Elo delta");
    }

    #[test]
    fn test_elo_delta_negative() {
        let delta = EloRating::estimate_elo_delta(0.4, 100);
        assert!(delta < 0, "40% win rate should give negative Elo delta");
    }

    #[test]
    fn test_elo_delta_even() {
        let delta = EloRating::estimate_elo_delta(0.5, 100);
        assert_eq!(delta, 0, "50% win rate should give zero Elo delta");
    }

    #[test]
    fn test_elo_delta_extreme() {
        let delta_high = EloRating::estimate_elo_delta(0.99, 100);
        assert!(delta_high > 0 && delta_high <= 800);

        let delta_low = EloRating::estimate_elo_delta(0.01, 100);
        assert!(delta_low < 0 && delta_low >= -800);
    }

    #[test]
    fn test_elo_confidence_interval() {
        let (lower, upper) = EloRating::confidence_interval(0.55, 100);
        assert!(lower < upper, "Lower bound should be less than upper bound");

        // More games should give tighter confidence
        let (lower2, upper2) = EloRating::confidence_interval(0.55, 1000);
        assert!(
            (upper2 - lower2) < (upper - lower),
            "More games should give tighter confidence interval"
        );
    }

    #[test]
    fn test_elo_calculate_ratings() {
        let (ra, rb) = EloRating::calculate_ratings(60, 40, 0, 1500.0, 1500.0, 32.0);
        assert!(ra > rb, "Player with more wins should have higher rating");
        assert!(ra > 1500.0, "Winner should be above initial");
        assert!(rb < 1500.0, "Loser should be below initial");
    }

    // --- 8.3: Model Lineage Tests ---

    #[test]
    fn test_lineage_bootstrap() {
        let lineage = ModelLineage::with_bootstrap();
        assert_eq!(lineage.entries.len(), 1);
        assert_eq!(lineage.entries[0].generation, 0);
        assert!(lineage.entries[0].promoted);
    }

    #[test]
    fn test_lineage_next_generation() {
        let lineage = ModelLineage::with_bootstrap();
        assert_eq!(lineage.next_generation(), 1);
    }

    #[test]
    fn test_lineage_add_entry() {
        let mut lineage = ModelLineage::with_bootstrap();
        lineage.add_entry(LineageEntry::new_trained(
            "gen1_test".to_string(),
            1,
            "gen0_bootstrap".to_string(),
            10000,
            0.5,
        ));

        assert_eq!(lineage.entries.len(), 2);
        assert_eq!(lineage.next_generation(), 2);
    }

    #[test]
    fn test_lineage_latest_promoted() {
        let mut lineage = ModelLineage::with_bootstrap();

        let mut entry = LineageEntry::new_trained(
            "gen1".to_string(),
            1,
            "gen0_bootstrap".to_string(),
            5000,
            0.3,
        );
        entry.set_gating_result(0.58, 100, 56, true);
        lineage.add_entry(entry);

        let latest = lineage.latest_promoted().unwrap();
        assert_eq!(latest.generation, 1);
        assert!(latest.promoted);
    }

    #[test]
    fn test_lineage_rejected_not_promoted() {
        let mut lineage = ModelLineage::with_bootstrap();

        let mut entry = LineageEntry::new_trained(
            "gen1".to_string(),
            1,
            "gen0_bootstrap".to_string(),
            5000,
            0.3,
        );
        entry.set_gating_result(0.48, 100, -14, false);
        lineage.add_entry(entry);

        let latest_promoted = lineage.latest_promoted().unwrap();
        assert_eq!(latest_promoted.generation, 0, "Only bootstrap should be promoted");
    }

    #[test]
    fn test_lineage_save_load_roundtrip() {
        let temp_path = std::env::temp_dir().join("wh40k_test_lineage.json");

        let mut lineage = ModelLineage::with_bootstrap();
        lineage.add_entry(LineageEntry::new_trained(
            "gen1".to_string(),
            1,
            "gen0_bootstrap".to_string(),
            10000,
            0.25,
        ));

        lineage.save(&temp_path).unwrap();
        let loaded = ModelLineage::load(&temp_path).unwrap();

        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].model_id, "gen0_bootstrap");
        assert_eq!(loaded.entries[1].model_id, "gen1");
        assert_eq!(loaded.entries[1].training_positions, 10000);

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_lineage_current_elo() {
        let mut lineage = ModelLineage::with_bootstrap();

        let mut entry = LineageEntry::new_trained(
            "gen1".to_string(),
            1,
            "gen0_bootstrap".to_string(),
            5000,
            0.3,
        );
        entry.set_gating_result(0.58, 100, 56, true);
        lineage.add_entry(entry);

        assert_eq!(lineage.current_elo(), 56);
    }

    #[test]
    fn test_lineage_display() {
        let lineage = ModelLineage::with_bootstrap();
        let display = format!("{}", lineage);
        assert!(display.contains("1 generations"));
        assert!(display.contains("1 promoted"));
    }

    // --- 8.3: Gating Tests ---

    #[test]
    fn test_gating_config_default() {
        let config = GatingConfig::default_config();
        assert_eq!(config.num_games, DEFAULT_GATING_GAMES);
        assert!((config.promotion_threshold - DEFAULT_PROMOTION_THRESHOLD).abs() < 0.001);
    }

    #[test]
    fn test_gating_result_display() {
        let result = GatingResult {
            candidate_model_id: "gen1_test".to_string(),
            baseline_model_id: "gen0_bootstrap".to_string(),
            candidate_wins: 58,
            baseline_wins: 38,
            draws: 4,
            total_games: 100,
            candidate_win_rate: 0.60,
            promoted: true,
            promotion_threshold: 0.55,
            elo_delta: 70,
            candidate_elo: 1570.0,
            baseline_elo: 1430.0,
            elo_confidence_interval: (30, 110),
            duration_ms: 60000,
            total_samples: 0,
        };

        let display = format!("{}", result);
        assert!(display.contains("PROMOTED"));
        assert!(display.contains("gen1_test"));
        assert!(display.contains("60.0%"));
    }

    // --- Integration Tests ---

    #[test]
    fn test_selfplay_report_compilation() {
        let results = vec![
            MatchResult {
                outcome: GameOutcome::Victory(PlayerId::new(0)),
                player1_vp: 50,
                player2_vp: 30,
                total_rounds: 5,
                total_commands: 200,
                search_diagnostics: vec![],
                training_samples: vec![],
                duration_ms: 1000,
                game_seed: [0u8; 32],
                replay: None,
                normal_termination: true,
                error_message: None,
            },
            MatchResult {
                outcome: GameOutcome::Victory(PlayerId::new(1)),
                player1_vp: 25,
                player2_vp: 45,
                total_rounds: 4,
                total_commands: 180,
                search_diagnostics: vec![],
                training_samples: vec![],
                duration_ms: 900,
                game_seed: [1u8; 32],
                replay: None,
                normal_termination: true,
                error_message: None,
            },
            MatchResult {
                outcome: GameOutcome::Draw,
                player1_vp: 35,
                player2_vp: 35,
                total_rounds: 5,
                total_commands: 220,
                search_diagnostics: vec![],
                training_samples: vec![],
                duration_ms: 1100,
                game_seed: [2u8; 32],
                replay: None,
                normal_termination: true,
                error_message: None,
            },
        ];

        let report = SelfPlayRunner::compile_report(results, 0, 3000);
        assert_eq!(report.total_games, 3);
        assert_eq!(report.player1_wins, 1);
        assert_eq!(report.player2_wins, 1);
        assert_eq!(report.draws, 1);
        assert_eq!(report.total_commands, 600);
        assert!((report.avg_rounds_per_game - 4.666).abs() < 0.01);
        assert!((report.player1_win_rate - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_selfplay_report_display() {
        let report = SelfPlayReport {
            total_games: 10,
            player1_wins: 4,
            player2_wins: 5,
            draws: 1,
            abnormal_terminations: 0,
            total_commands: 2000,
            total_samples: 500,
            total_duration_ms: 10000,
            games_per_hour: 3600.0,
            avg_commands_per_game: 200.0,
            avg_rounds_per_game: 4.5,
            avg_duration_ms: 1000.0,
            player1_win_rate: 0.4,
            results: vec![],
        };

        let display = format!("{}", report);
        assert!(display.contains("10 games"));
        assert!(display.contains("40.0%"));
    }
}
