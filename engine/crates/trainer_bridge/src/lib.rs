//! WH40K Engine - Python Training Bridge (Phase 8.4)
//!
//! PyO3-based FFI bridge exposing the Rust engine to Python for NNUE training.
//!
//! Provides the complete API specified in implementation_v3.md Section 14.2:
//! - `reset(seed, scenario_config) -> state`
//! - `legal_actions(state) -> action_ids`
//! - `step(state, action_id) -> transition`
//! - `encode_state(state) -> tensors`
//! - `encode_legal_mask(state) -> mask`
//! - `terminal_result(state) -> outcome`
//!
//! Plus training-specific functionality:
//! - Shard loading and inspection
//! - Weight import/export for NNUE model
//! - Self-play batch orchestration from Python
//! - Gating evaluation from Python
//!
//! Source: implementation_v3.md Phase 8.4, Sections 2.1, 14.2

use pyo3::exceptions::{PyIOError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use std::path::Path;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use wh40k_core_types::{
    FactionId, GameMode, GameOutcome, MissionId, PlayerId,
};
use wh40k_eval_features::{
    NnueFeatureSchema, TOTAL_FEATURES, FEATURE_SCHEMA_VERSION,
};
use wh40k_eval_nnue::{
    NnueDimensions, NnueModelArtifact, QuantizedWeights, ModelMetadata,
};
use wh40k_game_core::state::GameState;
use wh40k_game_core::{CommandExecutor, ScenarioLoader};
use wh40k_search_abstraction::{ActionGenerator, MacroAction};
use wh40k_selfplay::{
    encode_state, encode_legal_mask, encode_sparse_features,
    action_to_vocab_index,
    play_single_game, collect_training_data, benchmark_selfplay_throughput,
    MatchConfig, MatchResult as RustMatchResult,
    PlayerConfig, AiType,
    GatingConfig, GatingHarness,
    ModelLineage, LineageEntry, EloRating,
    ShardReader,
    ACTION_VOCAB_SIZE, INTENT_COUNT, MAX_UNIT_SLOTS,
    SHARD_FORMAT_VERSION,
};

// ============================================================================
// Constants exposed to Python
// ============================================================================

/// Total number of input features for the NNUE.
const PY_TOTAL_FEATURES: usize = TOTAL_FEATURES;

/// Fixed action vocabulary size (33 intents * 16 unit slots).
const PY_ACTION_VOCAB_SIZE: usize = ACTION_VOCAB_SIZE;

// ============================================================================
// PyGameState: Wraps the Rust GameState for Python
// ============================================================================

/// A WH40K game state accessible from Python.
///
/// This wraps the Rust `GameState` and provides methods matching
/// the API specified in implementation_v3.md Section 14.2.
#[pyclass(name = "GameState")]
struct PyGameState {
    state: GameState,
    /// Cached candidate actions for the current decision point.
    /// Regenerated whenever the state changes.
    cached_actions: Option<Vec<MacroAction>>,
}

#[pymethods]
impl PyGameState {
    /// Create a new game state from scenario parameters.
    ///
    /// Args:
    ///     faction_a: Faction ID for player 1 (0 = Custodes, 1 = World Eaters)
    ///     faction_b: Faction ID for player 2
    ///     mission: Optional mission ID (0-5). None for random.
    ///     seed: 32-byte seed for deterministic replay. None for random.
    #[new]
    #[pyo3(signature = (faction_a=0, faction_b=1, mission=None, seed=None))]
    fn new(
        faction_a: u32,
        faction_b: u32,
        mission: Option<u32>,
        seed: Option<Vec<u8>>,
    ) -> PyResult<Self> {
        let seed_bytes = make_seed(seed)?;
        let mission_id = mission.map(MissionId::new);
        let state = ScenarioLoader::load_scenario(
            FactionId::new(faction_a),
            FactionId::new(faction_b),
            mission_id,
            seed_bytes,
        );

        Ok(PyGameState {
            state,
            cached_actions: None,
        })
    }

    /// Whether the game is still in progress.
    #[getter]
    fn is_in_progress(&self) -> bool {
        self.state.is_in_progress()
    }

    /// Current game outcome as a string: "in_progress", "victory_p0", "victory_p1", "draw".
    #[getter]
    fn outcome(&self) -> String {
        match self.state.game_outcome {
            GameOutcome::InProgress => "in_progress".to_string(),
            GameOutcome::Victory(winner) => format!("victory_p{}", winner.0),
            GameOutcome::Draw => "draw".to_string(),
        }
    }

    /// Winner player ID (0 or 1), or -1 if no winner yet.
    #[getter]
    fn winner(&self) -> i32 {
        match self.state.game_outcome {
            GameOutcome::Victory(winner) => winner.0 as i32,
            _ => -1,
        }
    }

    /// Current battle round (1-5).
    #[getter]
    fn battle_round(&self) -> u8 {
        self.state.battle_round.number()
    }

    /// Current phase name.
    #[getter]
    fn current_phase(&self) -> String {
        format!("{:?}", self.state.current_phase)
    }

    /// ID of the player who must make the next decision.
    #[getter]
    fn decision_owner(&self) -> u32 {
        self.state.decision_owner.0
    }

    /// Player 1 victory points.
    #[getter]
    fn player1_vp(&self) -> i16 {
        self.state.players[0].vp.value()
    }

    /// Player 2 victory points.
    #[getter]
    fn player2_vp(&self) -> i16 {
        self.state.players[1].vp.value()
    }

    /// Deterministic hash of the current state.
    #[getter]
    fn state_hash(&self) -> u64 {
        self.state.compute_hash()
    }

    /// Number of legal actions available at the current decision point.
    fn num_legal_actions(&mut self) -> usize {
        self.ensure_cached_actions();
        self.cached_actions.as_ref().map_or(0, |a| a.len())
    }

    /// Get labels for all legal actions at the current decision point.
    /// Returns a list of (index, label, intent_name) tuples.
    fn legal_action_labels(&mut self) -> Vec<(usize, String, String)> {
        self.ensure_cached_actions();
        match &self.cached_actions {
            Some(actions) => {
                actions
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (i, a.label.clone(), format!("{:?}", a.intent)))
                    .collect()
            }
            None => vec![],
        }
    }

    /// Get the vocabulary indices for all legal actions.
    /// Returns a list of u32 indices into the fixed ACTION_VOCAB_SIZE vocabulary.
    fn legal_action_vocab_indices(&mut self) -> Vec<u32> {
        self.ensure_cached_actions();
        let perspective = self.state.decision_owner;
        match &self.cached_actions {
            Some(actions) => {
                actions
                    .iter()
                    .map(|a| action_to_vocab_index(a, &self.state, perspective))
                    .collect()
            }
            None => vec![],
        }
    }

    /// Encode the current state as a dense f32 feature vector.
    ///
    /// Args:
    ///     perspective: Player perspective (0 or 1). Defaults to decision_owner.
    ///
    /// Returns:
    ///     List of f32 values of length TOTAL_FEATURES (1209).
    #[pyo3(signature = (perspective=None))]
    fn encode_state_dense(&self, perspective: Option<u32>) -> Vec<f32> {
        let p = perspective
            .map(PlayerId::new)
            .unwrap_or(self.state.decision_owner);
        encode_state(&self.state, p)
    }

    /// Encode the current state as sparse features.
    ///
    /// Returns:
    ///     List of (index, value) tuples for non-zero features.
    #[pyo3(signature = (perspective=None))]
    fn encode_state_sparse(&self, perspective: Option<u32>) -> Vec<(u16, i16)> {
        let p = perspective
            .map(PlayerId::new)
            .unwrap_or(self.state.decision_owner);
        encode_sparse_features(&self.state, p)
    }

    /// Encode the legal action mask over the fixed vocabulary.
    ///
    /// Returns:
    ///     List of bool of length ACTION_VOCAB_SIZE (640).
    #[pyo3(signature = (perspective=None))]
    fn encode_legal_mask(&mut self, perspective: Option<u32>) -> Vec<bool> {
        let p = perspective
            .map(PlayerId::new)
            .unwrap_or(self.state.decision_owner);
        let mask = encode_legal_mask(&self.state, p);
        mask.mask
    }

    /// Execute an action by index (into the cached legal actions list).
    ///
    /// This corresponds to `step(state, action_id) -> transition` from
    /// implementation_v3.md Section 14.2.
    ///
    /// Args:
    ///     action_index: Index into the legal actions list (0-based).
    ///
    /// Returns:
    ///     Dict with keys: "reward" (f32), "done" (bool), "info" (str).
    fn step(&mut self, py: Python, action_index: usize) -> PyResult<PyObject> {
        self.ensure_cached_actions();

        let actions = self.cached_actions.take().ok_or_else(|| {
            PyRuntimeError::new_err("No legal actions available")
        })?;

        let num_actions = actions.len();
        if action_index >= num_actions {
            self.cached_actions = Some(actions);
            return Err(PyValueError::new_err(format!(
                "Action index {} out of range [0, {})",
                action_index,
                num_actions
            )));
        }

        let chosen = &actions[action_index];
        let mut error_info = String::new();

        // Execute each command in the macro-action
        for cmd in &chosen.commands {
            match CommandExecutor::execute(&mut self.state, cmd) {
                Ok(_events) => {}
                Err(e) => {
                    error_info = format!("Command error: {:?}", e);
                    break;
                }
            }
        }

        // Invalidate cached actions (state has changed)
        self.cached_actions = None;

        // Build result dict
        let result = PyDict::new(py);

        let done = !self.state.is_in_progress();
        let reward: f32 = match self.state.game_outcome {
            GameOutcome::Victory(winner) => {
                if winner.0 == 0 { 1.0 } else { -1.0 }
            }
            GameOutcome::Draw => 0.0,
            GameOutcome::InProgress => 0.0,
        };

        result.set_item("reward", reward)?;
        result.set_item("done", done)?;
        result.set_item("info", if error_info.is_empty() { "ok".to_string() } else { error_info })?;

        Ok(result.into())
    }

    /// Execute an action by vocabulary index.
    ///
    /// Args:
    ///     vocab_index: Index into the fixed ACTION_VOCAB_SIZE vocabulary.
    ///
    /// Returns:
    ///     Dict with keys: "reward" (f32), "done" (bool), "info" (str).
    fn step_by_vocab_index(&mut self, py: Python, vocab_index: u32) -> PyResult<PyObject> {
        self.ensure_cached_actions();

        let perspective = self.state.decision_owner;
        let actions = self.cached_actions.as_ref().ok_or_else(|| {
            PyRuntimeError::new_err("No legal actions available")
        })?;

        // Find the action with the matching vocab index
        let action_index = actions
            .iter()
            .position(|a| action_to_vocab_index(a, &self.state, perspective) == vocab_index)
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Vocab index {} does not match any legal action",
                    vocab_index
                ))
            })?;

        self.step(py, action_index)
    }

    /// Reset the game state with a new scenario.
    #[pyo3(signature = (faction_a=0, faction_b=1, mission=None, seed=None))]
    fn reset(
        &mut self,
        faction_a: u32,
        faction_b: u32,
        mission: Option<u32>,
        seed: Option<Vec<u8>>,
    ) -> PyResult<()> {
        let seed_bytes = make_seed(seed)?;
        let mission_id = mission.map(MissionId::new);
        self.state = ScenarioLoader::load_scenario(
            FactionId::new(faction_a),
            FactionId::new(faction_b),
            mission_id,
            seed_bytes,
        );
        self.cached_actions = None;
        Ok(())
    }

    /// Get terminal result as a dict.
    ///
    /// Returns None if game is still in progress.
    /// Otherwise returns dict with "outcome", "winner", "p1_vp", "p2_vp".
    fn terminal_result(&self, py: Python) -> PyResult<Option<PyObject>> {
        if self.state.is_in_progress() {
            return Ok(None);
        }

        let result = PyDict::new(py);
        result.set_item("outcome", self.outcome())?;
        result.set_item("winner", self.winner())?;
        result.set_item("p1_vp", self.player1_vp())?;
        result.set_item("p2_vp", self.player2_vp())?;
        result.set_item("rounds", self.battle_round())?;

        Ok(Some(result.into()))
    }

    /// Get a string representation.
    fn __repr__(&self) -> String {
        format!(
            "GameState(round={}, phase={:?}, outcome={:?}, p1_vp={}, p2_vp={})",
            self.state.battle_round.number(),
            self.state.current_phase,
            self.state.game_outcome,
            self.state.players[0].vp.value(),
            self.state.players[1].vp.value(),
        )
    }
}

impl PyGameState {
    fn ensure_cached_actions(&mut self) {
        if self.cached_actions.is_none() && self.state.is_in_progress() {
            let candidates = ActionGenerator::generate(
                &self.state,
                self.state.decision_owner,
            );
            self.cached_actions = Some(candidates.candidates);
        }
    }
}

/// Helper: produce a 32-byte seed from an optional Python input.
fn make_seed(seed: Option<Vec<u8>>) -> PyResult<[u8; 32]> {
    match seed {
        Some(s) => {
            if s.len() != 32 {
                return Err(PyValueError::new_err(
                    format!("Seed must be exactly 32 bytes, got {}", s.len()),
                ));
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&s);
            Ok(arr)
        }
        None => {
            let mut rng = SmallRng::from_entropy();
            let mut arr = [0u8; 32];
            rng.fill(&mut arr);
            Ok(arr)
        }
    }
}

// ============================================================================
// PyTrainingSample: Training data exposed to Python
// ============================================================================

/// A single training sample from self-play.
#[pyclass(name = "TrainingSample")]
#[derive(Clone)]
struct PyTrainingSample {
    /// Sparse features as (index, value) pairs.
    #[pyo3(get)]
    sparse_features: Vec<(u16, i16)>,
    /// Legal action vocabulary indices.
    #[pyo3(get)]
    legal_action_indices: Vec<u32>,
    /// Chosen action vocabulary index.
    #[pyo3(get)]
    chosen_action_index: u32,
    /// Search evaluation score.
    #[pyo3(get)]
    search_score: i32,
    /// Game outcome from this sample's perspective: +1.0 win, -1.0 loss, 0.0 draw.
    #[pyo3(get)]
    outcome: f32,
    /// Player perspective (0 or 1).
    #[pyo3(get)]
    perspective: u8,
    /// Game progress fraction (0.0 to 1.0).
    #[pyo3(get)]
    game_progress: f32,
    /// State hash for deduplication.
    #[pyo3(get)]
    state_hash: u64,
    /// Battle round (1-5).
    #[pyo3(get)]
    battle_round: u8,
}

#[pymethods]
impl PyTrainingSample {
    fn __repr__(&self) -> String {
        format!(
            "TrainingSample(features={}, legal={}, chosen={}, score={}, outcome={:.1}, perspective={}, round={})",
            self.sparse_features.len(),
            self.legal_action_indices.len(),
            self.chosen_action_index,
            self.search_score,
            self.outcome,
            self.perspective,
            self.battle_round,
        )
    }
}

// ============================================================================
// PyTrainingShard: Shard I/O exposed to Python
// ============================================================================

/// A training data shard containing multiple samples.
#[pyclass(name = "TrainingShard")]
struct PyTrainingShard {
    /// Number of samples in this shard.
    #[pyo3(get)]
    num_samples: usize,
    /// Format version.
    #[pyo3(get)]
    format_version: u32,
    /// Engine version string.
    #[pyo3(get)]
    engine_version: String,
    /// Feature schema version.
    #[pyo3(get)]
    schema_version: u32,
    /// Samples.
    samples: Vec<PyTrainingSample>,
}

#[pymethods]
impl PyTrainingShard {
    /// Get all samples in this shard.
    fn samples(&self) -> Vec<PyTrainingSample> {
        self.samples.clone()
    }

    /// Get a specific sample by index.
    fn sample(&self, index: usize) -> PyResult<PyTrainingSample> {
        self.samples.get(index).cloned().ok_or_else(|| {
            PyValueError::new_err(format!(
                "Sample index {} out of range [0, {})",
                index,
                self.samples.len()
            ))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "TrainingShard(samples={}, version={}, schema={})",
            self.num_samples, self.format_version, self.schema_version,
        )
    }

    fn __len__(&self) -> usize {
        self.num_samples
    }
}

// ============================================================================
// PyNnueWeights: NNUE weight import/export for Python training
// ============================================================================

/// NNUE network weights accessible from Python.
///
/// Holds all quantized weights for the 4-layer NNUE:
/// Sparse Features (1209) -> Accumulator (128) -> Hidden1 (32) -> Hidden2 (32) -> Output (1)
///
/// Quantization:
/// - Feature weights: i16
/// - Feature biases: i32
/// - Hidden weights: i8
/// - Hidden biases: i32
/// - Output weights: i16
/// - Output bias: i32
#[pyclass(name = "NnueWeights")]
struct PyNnueWeights {
    /// Feature -> Accumulator weights [input_size * accumulator_size], i16.
    #[pyo3(get)]
    feature_weights: Vec<i16>,
    /// Feature biases [accumulator_size], i32.
    #[pyo3(get)]
    feature_biases: Vec<i32>,
    /// Accumulator -> Hidden1 weights [accumulator_size * hidden1_size], i8.
    #[pyo3(get)]
    hidden1_weights: Vec<i8>,
    /// Hidden1 biases [hidden1_size], i32.
    #[pyo3(get)]
    hidden1_biases: Vec<i32>,
    /// Hidden1 -> Hidden2 weights [hidden1_size * hidden2_size], i8.
    #[pyo3(get)]
    hidden2_weights: Vec<i8>,
    /// Hidden2 biases [hidden2_size], i32.
    #[pyo3(get)]
    hidden2_biases: Vec<i32>,
    /// Hidden2 -> Output weights [hidden2_size], i16.
    #[pyo3(get)]
    output_weights: Vec<i16>,
    /// Output bias, i32.
    #[pyo3(get)]
    output_bias: i32,
}

#[pymethods]
impl PyNnueWeights {
    /// Create new weights with specified values.
    #[new]
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        feature_weights,
        feature_biases,
        hidden1_weights,
        hidden1_biases,
        hidden2_weights,
        hidden2_biases,
        output_weights,
        output_bias
    ))]
    fn new(
        feature_weights: Vec<i16>,
        feature_biases: Vec<i32>,
        hidden1_weights: Vec<i8>,
        hidden1_biases: Vec<i32>,
        hidden2_weights: Vec<i8>,
        hidden2_biases: Vec<i32>,
        output_weights: Vec<i16>,
        output_bias: i32,
    ) -> Self {
        PyNnueWeights {
            feature_weights,
            feature_biases,
            hidden1_weights,
            hidden1_biases,
            hidden2_weights,
            hidden2_biases,
            output_weights,
            output_bias,
        }
    }

    /// Load weights from a .nnue model artifact file.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let artifact = NnueModelArtifact::load_from_file(Path::new(path))
            .map_err(|e| PyIOError::new_err(format!("Failed to load model: {:?}", e)))?;

        Ok(PyNnueWeights {
            feature_weights: artifact.weights.feature_weights,
            feature_biases: artifact.weights.feature_biases,
            hidden1_weights: artifact.weights.hidden1_weights,
            hidden1_biases: artifact.weights.hidden1_biases,
            hidden2_weights: artifact.weights.hidden2_weights,
            hidden2_biases: artifact.weights.hidden2_biases,
            output_weights: artifact.weights.output_weights,
            output_bias: artifact.weights.output_bias,
        })
    }

    /// Save weights to a .nnue model artifact file.
    ///
    /// Args:
    ///     path: Output file path.
    ///     model_id: Unique model identifier string.
    ///     generation: Training generation number.
    ///     description: Human-readable model description.
    #[pyo3(signature = (path, model_id="trained_model", generation=0, description="Python-trained NNUE"))]
    fn save(
        &self,
        path: &str,
        model_id: &str,
        generation: u32,
        description: &str,
    ) -> PyResult<()> {
        let dims = NnueDimensions::DEFAULT;

        let weights = QuantizedWeights {
            feature_weights: self.feature_weights.clone(),
            feature_biases: self.feature_biases.clone(),
            hidden1_weights: self.hidden1_weights.clone(),
            hidden1_biases: self.hidden1_biases.clone(),
            hidden2_weights: self.hidden2_weights.clone(),
            hidden2_biases: self.hidden2_biases.clone(),
            output_weights: self.output_weights.clone(),
            output_bias: self.output_bias,
        };

        // Validate dimensions
        weights
            .validate(&dims)
            .map_err(|e| PyValueError::new_err(format!("Weight dimensions invalid: {:?}", e)))?;

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let metadata = ModelMetadata {
            model_id: model_id.to_string(),
            generation,
            schema_version: FEATURE_SCHEMA_VERSION,
            dimensions: dims,
            description: description.to_string(),
            parent_model_id: None,
            training_positions: 0,
            training_loss: 0.0,
            elo_estimate: 0,
            created_at: now_secs,
            total_parameters: dims.total_parameters(),
        };

        let schema = NnueFeatureSchema::current();

        let artifact = NnueModelArtifact::new(metadata, schema, weights);
        artifact
            .save_to_file(Path::new(path))
            .map_err(|e| PyIOError::new_err(format!("Failed to save model: {:?}", e)))?;

        Ok(())
    }

    /// Create zero-initialized weights with default NNUE dimensions.
    #[staticmethod]
    fn zeros() -> Self {
        let dims = NnueDimensions::DEFAULT;
        PyNnueWeights {
            feature_weights: vec![0i16; dims.input_size * dims.accumulator_size],
            feature_biases: vec![0i32; dims.accumulator_size],
            hidden1_weights: vec![0i8; dims.accumulator_size * dims.hidden1_size],
            hidden1_biases: vec![0i32; dims.hidden1_size],
            hidden2_weights: vec![0i8; dims.hidden1_size * dims.hidden2_size],
            hidden2_biases: vec![0i32; dims.hidden2_size],
            output_weights: vec![0i16; dims.hidden2_size],
            output_bias: 0,
        }
    }

    /// Get the expected dimensions.
    #[staticmethod]
    fn dimensions() -> (usize, usize, usize, usize, usize) {
        let d = NnueDimensions::DEFAULT;
        (d.input_size, d.accumulator_size, d.hidden1_size, d.hidden2_size, d.output_size)
    }

    fn __repr__(&self) -> String {
        format!(
            "NnueWeights(features={}xi16, h1={}xi8, h2={}xi8, out={}xi16, bias={})",
            self.feature_weights.len(),
            self.hidden1_weights.len(),
            self.hidden2_weights.len(),
            self.output_weights.len(),
            self.output_bias,
        )
    }
}

// ============================================================================
// PyMatchResult: Self-play match result
// ============================================================================

/// Result of a single self-play game.
#[pyclass(name = "MatchResult")]
struct PyMatchResult {
    #[pyo3(get)]
    outcome: String,
    #[pyo3(get)]
    winner: i32,
    #[pyo3(get)]
    player1_vp: i16,
    #[pyo3(get)]
    player2_vp: i16,
    #[pyo3(get)]
    total_rounds: u8,
    #[pyo3(get)]
    total_commands: usize,
    #[pyo3(get)]
    duration_ms: u64,
    #[pyo3(get)]
    num_training_samples: usize,
    #[pyo3(get)]
    error: Option<String>,
    /// Training samples collected during this game.
    training_samples: Vec<PyTrainingSample>,
}

#[pymethods]
impl PyMatchResult {
    /// Get all training samples from this game.
    fn samples(&self) -> Vec<PyTrainingSample> {
        self.training_samples.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "MatchResult(outcome={}, vp={}:{}, rounds={}, cmds={}, samples={}, {}ms)",
            self.outcome,
            self.player1_vp,
            self.player2_vp,
            self.total_rounds,
            self.total_commands,
            self.num_training_samples,
            self.duration_ms,
        )
    }
}

impl From<RustMatchResult> for PyMatchResult {
    fn from(r: RustMatchResult) -> Self {
        let outcome_str = match r.outcome {
            GameOutcome::Victory(winner) => format!("victory_p{}", winner.0),
            GameOutcome::Draw => "draw".to_string(),
            GameOutcome::InProgress => "in_progress".to_string(),
        };
        let winner = match r.outcome {
            GameOutcome::Victory(w) => w.0 as i32,
            _ => -1,
        };

        let training_samples: Vec<PyTrainingSample> = r
            .training_samples
            .iter()
            .map(|s| PyTrainingSample {
                sparse_features: s.sparse_features.clone(),
                legal_action_indices: s.legal_action_indices.clone(),
                chosen_action_index: s.chosen_action_index,
                search_score: s.search_score,
                outcome: s.outcome,
                perspective: s.perspective,
                game_progress: s.game_progress,
                state_hash: s.state_hash,
                battle_round: s.battle_round,
            })
            .collect();

        PyMatchResult {
            outcome: outcome_str,
            winner,
            player1_vp: r.player1_vp,
            player2_vp: r.player2_vp,
            total_rounds: r.total_rounds,
            total_commands: r.total_commands,
            duration_ms: r.duration_ms,
            num_training_samples: training_samples.len(),
            error: r.error_message,
            training_samples,
        }
    }
}

// ============================================================================
// PyGatingResult: Gating evaluation result
// ============================================================================

/// Result of a gating evaluation (candidate vs baseline).
#[pyclass(name = "GatingResult")]
struct PyGatingResult {
    #[pyo3(get)]
    candidate_wins: usize,
    #[pyo3(get)]
    baseline_wins: usize,
    #[pyo3(get)]
    draws: usize,
    #[pyo3(get)]
    total_games: usize,
    #[pyo3(get)]
    win_rate: f64,
    #[pyo3(get)]
    promoted: bool,
    #[pyo3(get)]
    elo_delta: i32,
    #[pyo3(get)]
    confidence_lower: i32,
    #[pyo3(get)]
    confidence_upper: i32,
}

#[pymethods]
impl PyGatingResult {
    fn __repr__(&self) -> String {
        format!(
            "GatingResult(wins={}/{}, draws={}, wr={:.1}%, elo={:+}, promoted={})",
            self.candidate_wins,
            self.total_games,
            self.draws,
            self.win_rate * 100.0,
            self.elo_delta,
            self.promoted,
        )
    }
}

// ============================================================================
// PyModelLineage: Model generation tracking
// ============================================================================

/// Model lineage tracker for training generations.
#[pyclass(name = "ModelLineage")]
struct PyModelLineage {
    lineage: ModelLineage,
}

#[pymethods]
impl PyModelLineage {
    /// Create a new lineage with a bootstrap entry.
    #[new]
    fn new() -> Self {
        PyModelLineage {
            lineage: ModelLineage::with_bootstrap(),
        }
    }

    /// Load lineage from a JSON file.
    #[staticmethod]
    fn load(path: &str) -> PyResult<Self> {
        let lineage = ModelLineage::load(Path::new(path))
            .map_err(|e| PyIOError::new_err(format!("Failed to load lineage: {}", e)))?;
        Ok(PyModelLineage { lineage })
    }

    /// Save lineage to a JSON file.
    fn save(&self, path: &str) -> PyResult<()> {
        self.lineage
            .save(Path::new(path))
            .map_err(|e| PyIOError::new_err(format!("Failed to save lineage: {}", e)))?;
        Ok(())
    }

    /// Get the next generation number.
    fn next_generation(&self) -> u32 {
        self.lineage.next_generation()
    }

    /// Get current best Elo rating.
    fn current_elo(&self) -> f64 {
        self.lineage.current_elo() as f64
    }

    /// Get the latest promoted model ID.
    fn latest_promoted_id(&self) -> Option<String> {
        self.lineage.latest_promoted().map(|e| e.model_id.clone())
    }

    /// Add a new lineage entry.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (
        model_id,
        generation,
        parent_id=None,
        training_positions=0,
        training_loss=0.0,
        elo_estimate=0,
        promoted=false,
        gating_win_rate=None,
        gating_games=None,
        gating_elo_delta=None,
        description="",
    ))]
    fn add_entry(
        &mut self,
        model_id: String,
        generation: u32,
        parent_id: Option<String>,
        training_positions: u64,
        training_loss: f64,
        elo_estimate: i32,
        promoted: bool,
        gating_win_rate: Option<f64>,
        gating_games: Option<usize>,
        gating_elo_delta: Option<i32>,
        description: &str,
    ) {
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.lineage.add_entry(LineageEntry {
            model_id,
            generation,
            parent_model_id: parent_id,
            training_positions,
            training_loss,
            elo_estimate,
            promoted,
            gating_win_rate,
            gating_games,
            gating_elo_delta,
            created_at: now_secs,
            description: description.to_string(),
        });
    }

    /// Number of entries in the lineage.
    fn __len__(&self) -> usize {
        self.lineage.entries.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "ModelLineage(entries={}, latest_gen={}, elo={:.0})",
            self.lineage.entries.len(),
            self.lineage.next_generation().saturating_sub(1),
            self.lineage.current_elo(),
        )
    }
}

// ============================================================================
// Module-level functions
// ============================================================================

/// Load a training shard from a bincode file.
#[pyfunction]
fn load_shard(path: &str) -> PyResult<PyTrainingShard> {
    let shard = ShardReader::read_shard(Path::new(path))
        .map_err(|e| PyIOError::new_err(format!("Failed to read shard: {:?}", e)))?;

    let samples: Vec<PyTrainingSample> = shard
        .samples
        .iter()
        .map(|s| PyTrainingSample {
            sparse_features: s.sparse_features.clone(),
            legal_action_indices: s.legal_action_indices.clone(),
            chosen_action_index: s.chosen_action_index,
            search_score: s.search_score,
            outcome: s.outcome,
            perspective: s.perspective,
            game_progress: s.game_progress,
            state_hash: s.state_hash,
            battle_round: s.battle_round,
        })
        .collect();

    Ok(PyTrainingShard {
        num_samples: samples.len(),
        format_version: shard.header.version,
        engine_version: shard.header.engine_version.clone(),
        schema_version: shard.header.feature_schema_version,
        samples,
    })
}

/// Load all training shards from a directory.
#[pyfunction]
fn load_all_shards(dir_path: &str) -> PyResult<Vec<PyTrainingShard>> {
    let shards = ShardReader::read_all_shards(Path::new(dir_path))
        .map_err(|e| PyIOError::new_err(format!("Failed to read shards: {:?}", e)))?;

    let mut result = Vec::with_capacity(shards.len());
    for shard in &shards {
        let samples: Vec<PyTrainingSample> = shard
            .samples
            .iter()
            .map(|s| PyTrainingSample {
                sparse_features: s.sparse_features.clone(),
                legal_action_indices: s.legal_action_indices.clone(),
                chosen_action_index: s.chosen_action_index,
                search_score: s.search_score,
                outcome: s.outcome,
                perspective: s.perspective,
                game_progress: s.game_progress,
                state_hash: s.state_hash,
                battle_round: s.battle_round,
            })
            .collect();

        result.push(PyTrainingShard {
            num_samples: samples.len(),
            format_version: shard.header.version,
            engine_version: shard.header.engine_version.clone(),
            schema_version: shard.header.feature_schema_version,
            samples,
        });
    }

    Ok(result)
}

/// Count total training samples in a directory of shards.
#[pyfunction]
fn count_samples(dir_path: &str) -> PyResult<usize> {
    ShardReader::count_samples(Path::new(dir_path))
        .map_err(|e| PyIOError::new_err(format!("Failed to count samples: {:?}", e)))
}

/// Run a single self-play game with greedy AI vs greedy AI.
///
/// Args:
///     faction_a: Faction ID for player 1 (default 0 = Custodes).
///     faction_b: Faction ID for player 2 (default 1 = World Eaters).
///     mission: Optional mission ID (0-5).
///     seed: Optional 32-byte seed.
///     collect_samples: Whether to collect training samples.
///
/// Returns:
///     MatchResult with game outcome and optional training samples.
#[pyfunction]
#[pyo3(signature = (faction_a=0, faction_b=1, mission=None, seed=None, collect_samples=true))]
fn play_game(
    faction_a: u32,
    faction_b: u32,
    mission: Option<u32>,
    seed: Option<Vec<u8>>,
    collect_samples: bool,
) -> PyResult<PyMatchResult> {
    let seed_bytes = make_seed(seed)?;

    let match_config = MatchConfig {
        seed: seed_bytes,
        game_mode: GameMode::CombatPatrol,
        mission_id: mission.map(MissionId::new),
        player1_faction: FactionId::new(faction_a),
        player2_faction: FactionId::new(faction_b),
        player1_enhancement: None,
        player2_enhancement: None,
        player1_secondary: None,
        player2_secondary: None,
    };

    let p1_config = PlayerConfig::greedy("Player1");
    let p2_config = PlayerConfig::greedy("Player2");

    let result = play_single_game(&match_config, &p1_config, &p2_config, collect_samples, false)
        .map_err(|e| PyRuntimeError::new_err(format!("Game error: {:?}", e)))?;

    Ok(PyMatchResult::from(result))
}

/// Run a batch of self-play games and collect training data.
///
/// Args:
///     num_games: Number of games to play.
///     output_dir: Directory to write training shards to.
///     ai_type: AI type string — "greedy", "one_ply", "negamax", "iterative_deepening" (default).
///     max_depth: Search depth (default 4). Only used for negamax/iterative_deepening.
///     game_mode: "CombatPatrol" (default) or "BoardingActions".
///     model_path: Path to .nnue model file for NNUE-based search evaluation.
///     model_generation: Generation number for shard metadata (default 0).
///
/// Returns:
///     Dict with batch statistics.
#[pyfunction]
#[pyo3(signature = (num_games, output_dir, ai_type=None, max_depth=None, game_mode=None, model_path=None, model_generation=None))]
fn run_selfplay_batch(
    py: Python,
    num_games: usize,
    output_dir: &str,
    ai_type: Option<&str>,
    max_depth: Option<u8>,
    game_mode: Option<&str>,
    model_path: Option<&str>,
    model_generation: Option<u32>,
) -> PyResult<PyObject> {
    // Convert ai_type string to AiType enum
    let resolved_ai = match ai_type {
        Some("greedy") => Some(AiType::Greedy),
        Some("one_ply") => Some(AiType::OnePly),
        Some("negamax") => Some(AiType::Negamax(max_depth.unwrap_or(3))),
        Some("iterative_deepening") => Some(AiType::IterativeDeepening(max_depth.unwrap_or(4))),
        Some(other) => return Err(PyRuntimeError::new_err(format!(
            "Unknown ai_type '{}'. Valid: greedy, one_ply, negamax, iterative_deepening", other
        ))),
        None => None, // collect_training_data defaults to IterativeDeepening(4)
    };

    // Convert game_mode string
    let resolved_mode = match game_mode {
        Some("BoardingActions") | Some("boarding_actions") | Some("ba") =>
            Some(GameMode::BoardingActions),
        Some("CombatPatrol") | Some("combat_patrol") | Some("cp") =>
            Some(GameMode::CombatPatrol),
        Some(other) => return Err(PyRuntimeError::new_err(format!(
            "Unknown game_mode '{}'. Valid: CombatPatrol, BoardingActions", other
        ))),
        None => None, // defaults to CombatPatrol
    };

    let resolved_model_path = model_path.map(std::path::PathBuf::from);

    let report = collect_training_data(
        num_games,
        output_dir.into(),
        resolved_ai,
        resolved_mode,
        resolved_model_path,
        model_generation,
    ).map_err(|e| PyRuntimeError::new_err(format!("Self-play error: {:?}", e)))?;

    let result = PyDict::new(py);
    result.set_item("total_games", report.total_games)?;
    result.set_item("player1_wins", report.player1_wins)?;
    result.set_item("player2_wins", report.player2_wins)?;
    result.set_item("draws", report.draws)?;
    result.set_item("avg_rounds", report.avg_rounds_per_game)?;
    result.set_item("avg_commands", report.avg_commands_per_game)?;
    result.set_item("avg_duration_ms", report.avg_duration_ms)?;
    result.set_item("total_samples", report.total_samples)?;
    result.set_item("games_per_hour", report.games_per_hour)?;

    Ok(result.into())
}

/// Run a throughput benchmark.
///
/// Args:
///     num_games: Number of games to play (default 10).
///
/// Returns:
///     Dict with benchmark results including games_per_hour.
#[pyfunction]
#[pyo3(signature = (num_games=10))]
fn benchmark(py: Python, num_games: usize) -> PyResult<PyObject> {
    let report = benchmark_selfplay_throughput(num_games)
        .map_err(|e| PyRuntimeError::new_err(format!("Benchmark error: {:?}", e)))?;

    let result = PyDict::new(py);
    result.set_item("total_games", report.total_games)?;
    result.set_item("games_per_hour", report.games_per_hour)?;
    result.set_item("avg_rounds", report.avg_rounds_per_game)?;
    result.set_item("avg_commands", report.avg_commands_per_game)?;
    result.set_item("avg_duration_ms", report.avg_duration_ms)?;
    result.set_item("total_samples", report.total_samples)?;

    Ok(result.into())
}

/// Evaluate a candidate NNUE model against the heuristic baseline.
///
/// Args:
///     candidate_weights_path: Path to the candidate .nnue artifact file.
///     num_games: Number of evaluation games (default 100).
///     promotion_threshold: Win rate threshold for promotion (default 0.55).
///
/// Returns:
///     GatingResult with evaluation metrics.
#[pyfunction]
#[pyo3(signature = (candidate_weights_path, num_games=100, promotion_threshold=0.55))]
fn evaluate_candidate(
    candidate_weights_path: &str,
    num_games: usize,
    promotion_threshold: f64,
) -> PyResult<PyGatingResult> {
    let config = GatingConfig {
        num_games,
        promotion_threshold,
        seed_base: 42,
        game_mode: GameMode::CombatPatrol,
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
    };

    let harness = GatingHarness::new(config);
    let artifact = NnueModelArtifact::load_from_file(Path::new(candidate_weights_path))
        .map_err(|e| PyIOError::new_err(format!("Failed to load candidate: {:?}", e)))?;

    let result = harness
        .evaluate_heuristic_vs_nnue(&artifact)
        .map_err(|e| PyRuntimeError::new_err(format!("Gating error: {:?}", e)))?;

    let (conf_lo, conf_hi) = result.elo_confidence_interval;

    Ok(PyGatingResult {
        candidate_wins: result.candidate_wins,
        baseline_wins: result.baseline_wins,
        draws: result.draws,
        total_games: result.total_games,
        win_rate: result.candidate_win_rate,
        promoted: result.promoted,
        elo_delta: result.elo_delta,
        confidence_lower: conf_lo,
        confidence_upper: conf_hi,
    })
}

/// Result of one Perturabo vs Basic matchup.
#[pyclass(name = "MatchupResult")]
#[derive(Clone)]
struct PyMatchupResult {
    #[pyo3(get)]
    perturabo_level: String,
    #[pyo3(get)]
    basic_level: String,
    #[pyo3(get)]
    perturabo_wins: usize,
    #[pyo3(get)]
    basic_wins: usize,
    #[pyo3(get)]
    draws: usize,
    #[pyo3(get)]
    win_rate: f64,
    #[pyo3(get)]
    total_games: usize,
}

#[pymethods]
impl PyMatchupResult {
    fn __repr__(&self) -> String {
        format!(
            "{} vs {}: {}W-{}L-{}D ({:.0}%)",
            self.perturabo_level, self.basic_level,
            self.perturabo_wins, self.basic_wins, self.draws,
            self.win_rate * 100.0,
        )
    }
}

/// Run Perturabo vs Basic tier gating.
///
/// Tests Perturabo (ID6, 30K node budget) against all 4 Basic levels
/// (Recruit=Greedy, Battle_Ready=OnePly, Veteran=Negamax2, Elite=Negamax3).
///
/// 4 matchups total, `games_per_matchup` games each.
///
/// Args:
///     games_per_matchup: Number of games per matchup (default 20).
///     nnue_path: Optional path to .nnue model. Omit for heuristic baseline.
///
/// Returns:
///     List of 4 MatchupResult objects.
#[pyfunction]
#[pyo3(signature = (games_per_matchup=20, nnue_path=None))]
fn perturabo_vs_basic(
    games_per_matchup: usize,
    nnue_path: Option<&str>,
) -> PyResult<Vec<PyMatchupResult>> {
    // Load NNUE model if provided (post-training evaluation)
    let nnue_artifact = match nnue_path {
        Some(path) => {
            let artifact = NnueModelArtifact::load_from_file(Path::new(path))
                .map_err(|e| PyIOError::new_err(format!("Failed to load NNUE: {:?}", e)))?;
            println!("  Using NNUE model: {}", path);
            Some(artifact)
        }
        None => {
            println!("  Using heuristic evaluator (baseline)");
            None
        }
    };

    // Single Perturabo level — ID6 with 30K node budget
    let perturabo_levels: Vec<(&str, AiType)> = vec![
        ("Perturabo (ID6)", AiType::IterativeDeepening(6)),
    ];

    let basic_levels: Vec<(&str, AiType)> = vec![
        ("Recruit (Greedy)", AiType::Greedy),
        ("Battle_Ready (OnePly)", AiType::OnePly),
        ("Veteran (Negamax2)", AiType::Negamax(2)),
        ("Elite (Negamax3)", AiType::Negamax(3)),
    ];

    let missions: Vec<Option<MissionId>> = (1..=6)
        .map(|i| Some(MissionId::new(i)))
        .collect();

    let mut all_results = Vec::new();
    let total_matchups = perturabo_levels.len() * basic_levels.len();
    let mut matchup_num = 0;

    let eval_label = if nnue_artifact.is_some() { "NNUE" } else { "Heuristic" };
    println!("{}", "=".repeat(60));
    println!("  PERTURABO ({}) vs BASIC — Full Tier Gating", eval_label);
    println!("  {} matchups × {} games = {} total games",
        total_matchups, games_per_matchup, total_matchups * games_per_matchup);
    println!("{}\n", "=".repeat(60));

    for (p_name, p_ai) in &perturabo_levels {
        for (b_name, b_ai) in &basic_levels {
            matchup_num += 1;
            let p_config = PlayerConfig {
                name: p_name.to_string(),
                ai_type: p_ai.clone(),
                weights: None,
                model_artifact: nnue_artifact.clone(),
                search_config: None,
            };
            let b_config = PlayerConfig {
                name: b_name.to_string(),
                ai_type: b_ai.clone(),
                weights: None,
                model_artifact: None,
                search_config: None,
            };

            let match_configs = wh40k_selfplay::GameVariation::generate_configs(
                games_per_matchup,
                42 + matchup_num as u64,
                &missions,
                true,
                GameMode::CombatPatrol,
            );

            let mut p_wins: usize = 0;
            let mut b_wins: usize = 0;
            let mut draws: usize = 0;

            let start = std::time::Instant::now();

            for (game_idx, match_config) in match_configs.iter().enumerate() {
                // Alternate sides
                let (p1, p2) = if game_idx % 2 == 0 {
                    (&p_config, &b_config)
                } else {
                    (&b_config, &p_config)
                };

                match play_single_game(match_config, p1, p2, false, false) {
                    Ok(result) => {
                        let perturabo_is_p1 = game_idx % 2 == 0;
                        match result.outcome {
                            GameOutcome::Victory(winner) => {
                                let winner_is_p1 = winner == PlayerId::new(0);
                                if (perturabo_is_p1 && winner_is_p1)
                                    || (!perturabo_is_p1 && !winner_is_p1)
                                {
                                    p_wins += 1;
                                } else {
                                    b_wins += 1;
                                }
                            }
                            _ => { draws += 1; }
                        }
                    }
                    Err(_) => { draws += 1; }
                }
            }

            let total = p_wins + b_wins + draws;
            let win_rate = if total > 0 {
                (p_wins as f64 + 0.5 * draws as f64) / total as f64
            } else {
                0.5
            };

            let elapsed = start.elapsed();
            println!(
                "[{:2}/{}] {} vs {} : {}W-{}L-{}D  ({:.0}%)  {:.1}s",
                matchup_num, total_matchups,
                p_name, b_name,
                p_wins, b_wins, draws,
                win_rate * 100.0,
                elapsed.as_secs_f64(),
            );

            all_results.push(PyMatchupResult {
                perturabo_level: p_name.to_string(),
                basic_level: b_name.to_string(),
                perturabo_wins: p_wins,
                basic_wins: b_wins,
                draws,
                win_rate,
                total_games: total,
            });
        }
    }

    // Print summary table
    println!("\n{}", "=".repeat(60));
    println!("  RESULTS SUMMARY");
    println!("{}", "=".repeat(60));
    println!("{:<22} {:<22} {:>5} {:>5} {:>5} {:>6}",
        "Perturabo", "Basic", "W", "L", "D", "Win%");
    println!("{:-<68}", "");
    for r in &all_results {
        println!("{:<22} {:<22} {:>5} {:>5} {:>5} {:>5.0}%",
            r.perturabo_level, r.basic_level,
            r.perturabo_wins, r.basic_wins, r.draws,
            r.win_rate * 100.0);
    }
    println!("{}", "=".repeat(60));

    Ok(all_results)
}

/// Compute Elo rating delta from a win rate.
///
/// Args:
///     win_rate: Fraction of games won (0.0 to 1.0).
///     num_games: Number of games played.
///
/// Returns:
///     Estimated Elo delta (positive = stronger than opponent).
#[pyfunction]
fn elo_delta(win_rate: f64, num_games: usize) -> i32 {
    EloRating::estimate_elo_delta(win_rate, num_games)
}

/// Get engine constants.
///
/// Returns a dict with:
///     total_features, action_vocab_size, intent_count, max_unit_slots,
///     shard_format_version, nnue_dimensions.
#[pyfunction]
fn engine_constants(py: Python) -> PyResult<PyObject> {
    let result = PyDict::new(py);
    let dims = NnueDimensions::DEFAULT;

    result.set_item("total_features", TOTAL_FEATURES)?;
    result.set_item("action_vocab_size", ACTION_VOCAB_SIZE)?;
    result.set_item("intent_count", INTENT_COUNT)?;
    result.set_item("max_unit_slots", MAX_UNIT_SLOTS)?;
    result.set_item("shard_format_version", SHARD_FORMAT_VERSION)?;

    let nnue_dims = PyDict::new(py);
    nnue_dims.set_item("input_size", dims.input_size)?;
    nnue_dims.set_item("accumulator_size", dims.accumulator_size)?;
    nnue_dims.set_item("hidden1_size", dims.hidden1_size)?;
    nnue_dims.set_item("hidden2_size", dims.hidden2_size)?;
    nnue_dims.set_item("output_size", dims.output_size)?;
    result.set_item("nnue_dimensions", nnue_dims)?;

    Ok(result.into())
}

// ============================================================================
// Python Module Definition
// ============================================================================

/// WH40K Engine Training Bridge - Python bindings for NNUE training.
///
/// This module exposes the Rust WH40K engine to Python for:
/// - Running self-play games and collecting training data
/// - Loading training shards produced by the Rust self-play runner
/// - Importing/exporting NNUE weights for PyTorch training
/// - Evaluating candidate models against baselines (gating)
/// - Stepping through games interactively for debugging
///
/// Usage:
///     import wh40k_trainer_bridge as wtb
///
///     # Check engine constants
///     consts = wtb.engine_constants()
///     print(f"Features: {consts['total_features']}, Vocab: {consts['action_vocab_size']}")
///
///     # Run self-play
///     result = wtb.play_game()
///     print(result)
///
///     # Interactive game stepping
///     state = wtb.GameState(faction_a=0, faction_b=1)
///     while state.is_in_progress:
///         mask = state.encode_legal_mask()
///         features = state.encode_state_sparse()
///         info = state.step(0)  # Take first legal action
///
///     # Load training data
///     shard = wtb.load_shard("path/to/shard.bin")
///     for sample in shard.samples():
///         print(sample)
///
///     # Weight import/export
///     weights = wtb.NnueWeights.zeros()
///     weights.save("model.nnue", model_id="gen0", generation=0)
///     loaded = wtb.NnueWeights.load("model.nnue")
#[pymodule]
fn wh40k_trainer_bridge(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyGameState>()?;
    m.add_class::<PyTrainingSample>()?;
    m.add_class::<PyTrainingShard>()?;
    m.add_class::<PyNnueWeights>()?;
    m.add_class::<PyMatchResult>()?;
    m.add_class::<PyGatingResult>()?;
    m.add_class::<PyMatchupResult>()?;
    m.add_class::<PyModelLineage>()?;

    // Functions
    m.add_function(wrap_pyfunction!(load_shard, m)?)?;
    m.add_function(wrap_pyfunction!(load_all_shards, m)?)?;
    m.add_function(wrap_pyfunction!(count_samples, m)?)?;
    m.add_function(wrap_pyfunction!(play_game, m)?)?;
    m.add_function(wrap_pyfunction!(run_selfplay_batch, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(perturabo_vs_basic, m)?)?;
    m.add_function(wrap_pyfunction!(elo_delta, m)?)?;
    m.add_function(wrap_pyfunction!(engine_constants, m)?)?;

    // Constants
    m.add("TOTAL_FEATURES", PY_TOTAL_FEATURES)?;
    m.add("ACTION_VOCAB_SIZE", PY_ACTION_VOCAB_SIZE)?;
    m.add("INTENT_COUNT", INTENT_COUNT)?;
    m.add("MAX_UNIT_SLOTS", MAX_UNIT_SLOTS)?;
    m.add("SHARD_FORMAT_VERSION", SHARD_FORMAT_VERSION)?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(PY_TOTAL_FEATURES, 1209);
        assert_eq!(PY_ACTION_VOCAB_SIZE, 640);
    }

    #[test]
    fn test_nnue_weights_zeros() {
        let w = PyNnueWeights::zeros();
        let dims = NnueDimensions::DEFAULT;
        assert_eq!(w.feature_weights.len(), dims.input_size * dims.accumulator_size);
        assert_eq!(w.feature_biases.len(), dims.accumulator_size);
        assert_eq!(w.hidden1_weights.len(), dims.accumulator_size * dims.hidden1_size);
        assert_eq!(w.hidden1_biases.len(), dims.hidden1_size);
        assert_eq!(w.hidden2_weights.len(), dims.hidden1_size * dims.hidden2_size);
        assert_eq!(w.hidden2_biases.len(), dims.hidden2_size);
        assert_eq!(w.output_weights.len(), dims.hidden2_size);
        assert_eq!(w.output_bias, 0);
    }

    #[test]
    fn test_nnue_weights_dimensions() {
        let (inp, acc, h1, h2, out) = PyNnueWeights::dimensions();
        assert_eq!(inp, 1209);
        assert_eq!(acc, 128);
        assert_eq!(h1, 32);
        assert_eq!(h2, 32);
        assert_eq!(out, 1);
    }

    #[test]
    fn test_nnue_weights_save_load_roundtrip() {
        let w = PyNnueWeights::zeros();
        let tmp = std::env::temp_dir().join("test_bridge_model.nnue");
        let path_str = tmp.to_str().unwrap();

        w.save(path_str, "test_model", 0, "Test model").unwrap();
        let loaded = PyNnueWeights::load(path_str).unwrap();

        assert_eq!(loaded.feature_weights.len(), w.feature_weights.len());
        assert_eq!(loaded.feature_biases.len(), w.feature_biases.len());
        assert_eq!(loaded.hidden1_weights.len(), w.hidden1_weights.len());
        assert_eq!(loaded.hidden1_biases.len(), w.hidden1_biases.len());
        assert_eq!(loaded.hidden2_weights.len(), w.hidden2_weights.len());
        assert_eq!(loaded.hidden2_biases.len(), w.hidden2_biases.len());
        assert_eq!(loaded.output_weights.len(), w.output_weights.len());
        assert_eq!(loaded.output_bias, w.output_bias);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_model_lineage_operations() {
        let mut lineage = PyModelLineage::new();
        assert_eq!(lineage.next_generation(), 1);

        lineage.add_entry(
            "gen1".to_string(),
            1,
            Some("bootstrap".to_string()),
            10000,
            0.5,
            20,
            true,
            Some(0.58),
            Some(100),
            Some(35),
            "First trained model",
        );

        assert_eq!(lineage.next_generation(), 2);
        assert_eq!(lineage.latest_promoted_id(), Some("gen1".to_string()));
    }

    #[test]
    fn test_lineage_save_load() {
        let mut lineage = PyModelLineage::new();
        lineage.add_entry(
            "gen1".to_string(),
            1,
            None,
            5000,
            0.4,
            15,
            true,
            Some(0.56),
            Some(50),
            Some(20),
            "Test gen1",
        );

        let tmp = std::env::temp_dir().join("test_bridge_lineage.json");
        let path_str = tmp.to_str().unwrap();

        lineage.save(path_str).unwrap();
        let loaded = PyModelLineage::load(path_str).unwrap();

        assert_eq!(loaded.next_generation(), 2);
        assert_eq!(loaded.latest_promoted_id(), Some("gen1".to_string()));

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_elo_delta_function() {
        let delta_even = elo_delta(0.5, 100);
        assert_eq!(delta_even, 0);

        let delta_win = elo_delta(0.6, 100);
        assert!(delta_win > 0);

        let delta_loss = elo_delta(0.4, 100);
        assert!(delta_loss < 0);
    }

    #[test]
    fn test_make_seed_valid() {
        let seed = make_seed(Some(vec![1u8; 32])).unwrap();
        assert_eq!(seed.len(), 32);
        assert_eq!(seed[0], 1);
    }

    #[test]
    fn test_make_seed_wrong_length() {
        assert!(make_seed(Some(vec![1u8; 16])).is_err());
    }

    #[test]
    fn test_make_seed_none_random() {
        let s1 = make_seed(None).unwrap();
        let s2 = make_seed(None).unwrap();
        // Two random seeds should differ (probabilistic, but near-certain)
        assert_ne!(s1, s2);
    }
}
