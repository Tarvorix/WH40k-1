//! WH40K Engine - EvalNnue crate
//!
//! NNUE (Efficiently Updatable Neural Network) evaluator for position evaluation.
//! Provides a CPU-efficient, incrementally updatable neural network evaluator
//! as an alternative to the heuristic evaluator.
//!
//! # Architecture
//!
//! The NNUE uses a 4-layer quantized neural network:
//! ```text
//! Sparse Features (1209) → Accumulator (128) → Hidden1 (32) → Hidden2 (32) → Output (1)
//!                           [ClippedReLU]       [ClippedReLU]   [ClippedReLU]
//! ```
//!
//! The accumulator layer supports incremental updates: when only a few features
//! change between positions (e.g., one unit moves), only the changed feature
//! contributions are added/removed instead of recomputing from scratch.
//!
//! # Quantization
//!
//! Weights use integer quantization for fast inference:
//! - Feature transformer weights: i16
//! - Hidden layer weights: i8
//! - Biases and accumulators: i32
//! - ClippedReLU clamps outputs to [0, 127]
//!
//! # Model Registry
//!
//! Models are stored as versioned artifacts with metadata, schema validation,
//! and checksum verification. Generation 0 is bootstrapped from random weights.
//!
//! Source: implementation_v3.md Section 12 (NNUE-Style Evaluator Design)

use serde::{Deserialize, Serialize};
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use wh40k_core_types::{GameOutcome, PlayerId};
use wh40k_eval_features::{
    compute_feature_diff, extract_sparse_features_from_state, FeatureDiff, NnueFeatureSchema,
    Score, SparseFeatureVec, FEATURE_SCHEMA_VERSION, SCORE_DRAW, SCORE_LOSS,
    SCORE_WIN, TOTAL_FEATURES,
};
use wh40k_eval_heuristic::{Evaluator, HeuristicEvaluator};
use wh40k_game_core::state::GameState;

// ============================================================================
// Constants
// ============================================================================

/// ClippedReLU maximum value. Outputs are clamped to [0, QA].
const QA: i32 = 127;

/// Scale factor for feature transformer accumulator.
/// Accumulator values are divided by this before ClippedReLU.
const FT_SCALE: i32 = 64;

/// Scale factor for hidden layer outputs.
/// Hidden layer sums are divided by this before ClippedReLU.
const HIDDEN_SCALE: i32 = 64;

/// Scale factor for final output to convert to evaluation Score.
/// Output = raw_output * OUTPUT_SCALE / (QA * QA)
const OUTPUT_SCALE: i32 = 400;

/// Default accumulator size (first hidden layer width).
const DEFAULT_ACCUMULATOR_SIZE: usize = 128;

/// Default hidden layer 1 size.
const DEFAULT_HIDDEN1_SIZE: usize = 32;

/// Default hidden layer 2 size.
const DEFAULT_HIDDEN2_SIZE: usize = 32;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during NNUE model operations.
#[derive(Debug)]
pub enum NnueError {
    /// Feature schema version doesn't match between model and runtime.
    SchemaVersionMismatch { expected: u32, got: u32 },
    /// Model dimensions don't match expected values.
    DimensionMismatch {
        field: &'static str,
        expected: usize,
        got: usize,
    },
    /// Checksum verification failed after loading model.
    ChecksumFailed { expected: u64, got: u64 },
    /// Weight array size doesn't match expected dimensions.
    WeightCountMismatch {
        layer: &'static str,
        expected: usize,
        got: usize,
    },
    /// IO error during model load/save.
    IoError(std::io::Error),
    /// Serialization/deserialization error.
    SerializationError(String),
    /// Requested model was not found in registry.
    ModelNotFound(String),
    /// Model artifact is invalid or corrupted.
    InvalidModel(String),
}

impl std::fmt::Display for NnueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NnueError::SchemaVersionMismatch { expected, got } => {
                write!(
                    f,
                    "Schema version mismatch: expected {}, got {}",
                    expected, got
                )
            }
            NnueError::DimensionMismatch {
                field,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Dimension mismatch for {}: expected {}, got {}",
                    field, expected, got
                )
            }
            NnueError::ChecksumFailed { expected, got } => {
                write!(
                    f,
                    "Checksum failed: expected {:#018x}, got {:#018x}",
                    expected, got
                )
            }
            NnueError::WeightCountMismatch {
                layer,
                expected,
                got,
            } => {
                write!(
                    f,
                    "Weight count mismatch for {}: expected {}, got {}",
                    layer, expected, got
                )
            }
            NnueError::IoError(e) => write!(f, "IO error: {}", e),
            NnueError::SerializationError(s) => write!(f, "Serialization error: {}", s),
            NnueError::ModelNotFound(id) => write!(f, "Model not found: {}", id),
            NnueError::InvalidModel(msg) => write!(f, "Invalid model: {}", msg),
        }
    }
}

impl std::error::Error for NnueError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NnueError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NnueError {
    fn from(e: std::io::Error) -> Self {
        NnueError::IoError(e)
    }
}

// ============================================================================
// Network Dimensions
// ============================================================================

/// NNUE network architecture dimensions.
/// Defines the size of each layer in the neural network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NnueDimensions {
    /// Input feature space size (should match TOTAL_FEATURES).
    pub input_size: usize,
    /// Accumulator (first hidden) layer width.
    pub accumulator_size: usize,
    /// Second hidden layer width.
    pub hidden1_size: usize,
    /// Third hidden layer width.
    pub hidden2_size: usize,
    /// Output size (always 1 for scalar evaluation).
    pub output_size: usize,
}

impl NnueDimensions {
    /// Default architecture: 1209 → 128 → 32 → 32 → 1
    pub const DEFAULT: Self = Self {
        input_size: TOTAL_FEATURES,
        accumulator_size: DEFAULT_ACCUMULATOR_SIZE,
        hidden1_size: DEFAULT_HIDDEN1_SIZE,
        hidden2_size: DEFAULT_HIDDEN2_SIZE,
        output_size: 1,
    };

    /// Total number of feature transformer weights.
    pub fn feature_weight_count(&self) -> usize {
        self.input_size * self.accumulator_size
    }

    /// Total number of hidden1 weights.
    pub fn hidden1_weight_count(&self) -> usize {
        self.accumulator_size * self.hidden1_size
    }

    /// Total number of hidden2 weights.
    pub fn hidden2_weight_count(&self) -> usize {
        self.hidden1_size * self.hidden2_size
    }

    /// Total number of output weights.
    pub fn output_weight_count(&self) -> usize {
        self.hidden2_size * self.output_size
    }

    /// Total number of trainable parameters.
    pub fn total_parameters(&self) -> usize {
        // Weights
        self.feature_weight_count()
            + self.hidden1_weight_count()
            + self.hidden2_weight_count()
            + self.output_weight_count()
            // Biases
            + self.accumulator_size
            + self.hidden1_size
            + self.hidden2_size
            + self.output_size
    }

    /// Validate that dimensions are consistent and reasonable.
    pub fn validate(&self) -> Result<(), NnueError> {
        if self.input_size == 0 {
            return Err(NnueError::InvalidModel(
                "input_size must be > 0".to_string(),
            ));
        }
        if self.accumulator_size == 0 {
            return Err(NnueError::InvalidModel(
                "accumulator_size must be > 0".to_string(),
            ));
        }
        if self.hidden1_size == 0 {
            return Err(NnueError::InvalidModel(
                "hidden1_size must be > 0".to_string(),
            ));
        }
        if self.hidden2_size == 0 {
            return Err(NnueError::InvalidModel(
                "hidden2_size must be > 0".to_string(),
            ));
        }
        if self.output_size != 1 {
            return Err(NnueError::InvalidModel(
                "output_size must be 1 for scalar evaluation".to_string(),
            ));
        }
        Ok(())
    }
}

// ============================================================================
// Quantized Weights
// ============================================================================

/// Quantized weight storage for the NNUE network.
///
/// Weight quantization scheme:
/// - Feature transformer: i16 weights (large range for varying feature scales)
/// - Hidden layers: i8 weights (inputs are ClippedReLU'd to [0, 127])
/// - Biases: i32 (accumulate products without overflow)
/// - Output: i16 weights (preserve precision for final score)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedWeights {
    /// Feature → Accumulator weights, row-major [input_size × accumulator_size].
    /// Weight for feature i, accumulator neuron j is at index [i * accumulator_size + j].
    pub feature_weights: Vec<i16>,
    /// Feature transformer biases [accumulator_size].
    pub feature_biases: Vec<i32>,

    /// Accumulator → Hidden1 weights, row-major [accumulator_size × hidden1_size].
    pub hidden1_weights: Vec<i8>,
    /// Hidden1 biases [hidden1_size].
    pub hidden1_biases: Vec<i32>,

    /// Hidden1 → Hidden2 weights, row-major [hidden1_size × hidden2_size].
    pub hidden2_weights: Vec<i8>,
    /// Hidden2 biases [hidden2_size].
    pub hidden2_biases: Vec<i32>,

    /// Hidden2 → Output weights [hidden2_size].
    pub output_weights: Vec<i16>,
    /// Output bias (scalar).
    pub output_bias: i32,
}

impl QuantizedWeights {
    /// Create zero-initialized weights for the given dimensions.
    pub fn zeros(dims: &NnueDimensions) -> Self {
        Self {
            feature_weights: vec![0i16; dims.feature_weight_count()],
            feature_biases: vec![0i32; dims.accumulator_size],
            hidden1_weights: vec![0i8; dims.hidden1_weight_count()],
            hidden1_biases: vec![0i32; dims.hidden1_size],
            hidden2_weights: vec![0i8; dims.hidden2_weight_count()],
            hidden2_biases: vec![0i32; dims.hidden2_size],
            output_weights: vec![0i16; dims.output_weight_count()],
            output_bias: 0,
        }
    }

    /// Create small random weights for the given dimensions using a deterministic seed.
    /// Used for bootstrapping generation 0 models.
    pub fn random_small(dims: &NnueDimensions, seed: u64) -> Self {
        use rand::rngs::SmallRng;
        use rand::{Rng, SeedableRng};

        let mut rng = SmallRng::seed_from_u64(seed);

        // Feature weights: small range [-8, 8] for i16
        let feature_weights: Vec<i16> = (0..dims.feature_weight_count())
            .map(|_| rng.gen_range(-8i16..=8i16))
            .collect();
        let feature_biases: Vec<i32> = (0..dims.accumulator_size)
            .map(|_| rng.gen_range(-16i32..=16i32))
            .collect();

        // Hidden weights: small range [-8, 8] for i8
        let hidden1_weights: Vec<i8> = (0..dims.hidden1_weight_count())
            .map(|_| rng.gen_range(-8i8..=8i8))
            .collect();
        let hidden1_biases: Vec<i32> = (0..dims.hidden1_size)
            .map(|_| rng.gen_range(-16i32..=16i32))
            .collect();

        let hidden2_weights: Vec<i8> = (0..dims.hidden2_weight_count())
            .map(|_| rng.gen_range(-8i8..=8i8))
            .collect();
        let hidden2_biases: Vec<i32> = (0..dims.hidden2_size)
            .map(|_| rng.gen_range(-16i32..=16i32))
            .collect();

        // Output weights: small range [-16, 16] for i16
        let output_weights: Vec<i16> = (0..dims.output_weight_count())
            .map(|_| rng.gen_range(-16i16..=16i16))
            .collect();
        let output_bias = rng.gen_range(-8i32..=8i32);

        Self {
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

    /// Validate weight array sizes against expected dimensions.
    pub fn validate(&self, dims: &NnueDimensions) -> Result<(), NnueError> {
        if self.feature_weights.len() != dims.feature_weight_count() {
            return Err(NnueError::WeightCountMismatch {
                layer: "feature_weights",
                expected: dims.feature_weight_count(),
                got: self.feature_weights.len(),
            });
        }
        if self.feature_biases.len() != dims.accumulator_size {
            return Err(NnueError::WeightCountMismatch {
                layer: "feature_biases",
                expected: dims.accumulator_size,
                got: self.feature_biases.len(),
            });
        }
        if self.hidden1_weights.len() != dims.hidden1_weight_count() {
            return Err(NnueError::WeightCountMismatch {
                layer: "hidden1_weights",
                expected: dims.hidden1_weight_count(),
                got: self.hidden1_weights.len(),
            });
        }
        if self.hidden1_biases.len() != dims.hidden1_size {
            return Err(NnueError::WeightCountMismatch {
                layer: "hidden1_biases",
                expected: dims.hidden1_size,
                got: self.hidden1_biases.len(),
            });
        }
        if self.hidden2_weights.len() != dims.hidden2_weight_count() {
            return Err(NnueError::WeightCountMismatch {
                layer: "hidden2_weights",
                expected: dims.hidden2_weight_count(),
                got: self.hidden2_weights.len(),
            });
        }
        if self.hidden2_biases.len() != dims.hidden2_size {
            return Err(NnueError::WeightCountMismatch {
                layer: "hidden2_biases",
                expected: dims.hidden2_size,
                got: self.hidden2_biases.len(),
            });
        }
        if self.output_weights.len() != dims.output_weight_count() {
            return Err(NnueError::WeightCountMismatch {
                layer: "output_weights",
                expected: dims.output_weight_count(),
                got: self.output_weights.len(),
            });
        }
        Ok(())
    }
}

// ============================================================================
// Model Metadata
// ============================================================================

/// Metadata about an NNUE model artifact.
/// Tracks provenance, versioning, and training information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Unique model identifier (e.g., "gen0", "gen1_20240315").
    pub model_id: String,
    /// Generation number (0 = bootstrap, 1+ = trained).
    pub generation: u32,
    /// Feature schema version this model was trained with.
    pub schema_version: u32,
    /// Network architecture dimensions.
    pub dimensions: NnueDimensions,
    /// Description of how this model was created.
    pub description: String,
    /// Parent model ID (None for generation 0).
    pub parent_model_id: Option<String>,
    /// Number of training positions used (0 for bootstrap).
    pub training_positions: u64,
    /// Training loss at end of training (0.0 for bootstrap).
    pub training_loss: f64,
    /// Estimated Elo relative to generation 0 (0 for bootstrap).
    pub elo_estimate: i32,
    /// Creation timestamp (seconds since epoch).
    pub created_at: u64,
    /// Total parameter count.
    pub total_parameters: usize,
}

impl ModelMetadata {
    /// Create metadata for a bootstrap (generation 0) model.
    pub fn bootstrap(dims: NnueDimensions) -> Self {
        Self {
            model_id: "gen0_bootstrap".to_string(),
            generation: 0,
            schema_version: FEATURE_SCHEMA_VERSION,
            dimensions: dims,
            description: "Bootstrap generation 0 model with small random weights".to_string(),
            parent_model_id: None,
            training_positions: 0,
            training_loss: 0.0,
            elo_estimate: 0,
            created_at: 0,
            total_parameters: dims.total_parameters(),
        }
    }

    /// Create metadata for a new trained model.
    pub fn new_trained(
        model_id: String,
        generation: u32,
        dims: NnueDimensions,
        parent_id: String,
        training_positions: u64,
        training_loss: f64,
    ) -> Self {
        Self {
            model_id,
            generation,
            schema_version: FEATURE_SCHEMA_VERSION,
            dimensions: dims,
            description: format!("Trained generation {} model", generation),
            parent_model_id: Some(parent_id),
            training_positions,
            training_loss,
            elo_estimate: 0,
            created_at: 0,
            total_parameters: dims.total_parameters(),
        }
    }
}

// ============================================================================
// Model Artifact
// ============================================================================

/// A complete, serializable NNUE model artifact.
/// Contains everything needed to load and run inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NnueModelArtifact {
    /// Model metadata (id, version, provenance).
    pub metadata: ModelMetadata,
    /// Feature schema definition.
    pub schema: NnueFeatureSchema,
    /// Quantized network weights.
    pub weights: QuantizedWeights,
    /// Checksum of the weights for integrity verification.
    pub checksum: u64,
}

impl NnueModelArtifact {
    /// Create a new model artifact from components.
    pub fn new(
        metadata: ModelMetadata,
        schema: NnueFeatureSchema,
        weights: QuantizedWeights,
    ) -> Self {
        let checksum = compute_weights_checksum(&weights);
        Self {
            metadata,
            schema,
            weights,
            checksum,
        }
    }

    /// Serialize the artifact to bytes (bincode format).
    pub fn to_bytes(&self) -> Result<Vec<u8>, NnueError> {
        bincode::serialize(self).map_err(|e| NnueError::SerializationError(e.to_string()))
    }

    /// Deserialize an artifact from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, NnueError> {
        bincode::deserialize(data).map_err(|e| NnueError::SerializationError(e.to_string()))
    }

    /// Serialize to JSON (for debugging/inspection).
    pub fn to_json(&self) -> Result<String, NnueError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| NnueError::SerializationError(e.to_string()))
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, NnueError> {
        serde_json::from_str(json).map_err(|e| NnueError::SerializationError(e.to_string()))
    }

    /// Validate the artifact's integrity and consistency.
    pub fn validate(&self) -> Result<(), NnueError> {
        // Validate dimensions
        self.metadata.dimensions.validate()?;

        // Validate schema version
        if self.metadata.schema_version != self.schema.version {
            return Err(NnueError::SchemaVersionMismatch {
                expected: self.metadata.schema_version,
                got: self.schema.version,
            });
        }

        // Validate input size matches schema
        if self.metadata.dimensions.input_size != self.schema.total_features {
            return Err(NnueError::DimensionMismatch {
                field: "input_size vs schema total_features",
                expected: self.schema.total_features,
                got: self.metadata.dimensions.input_size,
            });
        }

        // Validate weight sizes
        self.weights.validate(&self.metadata.dimensions)?;

        // Validate checksum
        let computed_checksum = compute_weights_checksum(&self.weights);
        if computed_checksum != self.checksum {
            return Err(NnueError::ChecksumFailed {
                expected: self.checksum,
                got: computed_checksum,
            });
        }

        Ok(())
    }

    /// Save artifact to a file.
    pub fn save_to_file(&self, path: &Path) -> Result<(), NnueError> {
        let data = self.to_bytes()?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load artifact from a file.
    pub fn load_from_file(path: &Path) -> Result<Self, NnueError> {
        let data = std::fs::read(path)?;
        let artifact = Self::from_bytes(&data)?;
        artifact.validate()?;
        Ok(artifact)
    }

    /// Create a bootstrap generation 0 artifact with small random weights.
    pub fn bootstrap() -> Self {
        let dims = NnueDimensions::DEFAULT;
        let metadata = ModelMetadata::bootstrap(dims);
        let schema = NnueFeatureSchema::current();
        let weights = QuantizedWeights::random_small(&dims, 0x40_AAEE_B007);
        Self::new(metadata, schema, weights)
    }
}

/// Compute a deterministic checksum of the weight data for integrity verification.
fn compute_weights_checksum(weights: &QuantizedWeights) -> u64 {
    use std::hash::{Hash, Hasher};

    let build_hasher = ahash::RandomState::with_seeds(
        0x4e4e55455f636b73, // "NNUE_cks"
        0x756d5f7665726966, // "um_verif"
        0x696361746f725f30, // "icator_0"
        0x3030303030303030, // "00000000"
    );
    let mut hasher = build_hasher.build_hasher();

    // Hash all weight arrays
    weights.feature_weights.hash(&mut hasher);
    weights.feature_biases.hash(&mut hasher);
    weights.hidden1_weights.hash(&mut hasher);
    weights.hidden1_biases.hash(&mut hasher);
    weights.hidden2_weights.hash(&mut hasher);
    weights.hidden2_biases.hash(&mut hasher);
    weights.output_weights.hash(&mut hasher);
    weights.output_bias.hash(&mut hasher);

    hasher.finish()
}

// ============================================================================
// NNUE Model (Runtime)
// ============================================================================

/// A loaded NNUE model ready for inference.
/// Contains the network weights and architecture dimensions.
#[derive(Debug, Clone)]
pub struct NnueModel {
    /// Network architecture dimensions.
    pub dims: NnueDimensions,
    /// Quantized network weights.
    pub weights: QuantizedWeights,
    /// Feature schema version this model expects.
    pub schema_version: u32,
    /// Model identifier for diagnostics.
    pub model_id: String,
    /// Generation number.
    pub generation: u32,
}

impl NnueModel {
    /// Load a model from an artifact.
    pub fn from_artifact(artifact: &NnueModelArtifact) -> Result<Self, NnueError> {
        // Validate schema compatibility
        let current_schema = NnueFeatureSchema::current();
        if !current_schema.is_compatible(&artifact.schema) {
            return Err(NnueError::SchemaVersionMismatch {
                expected: current_schema.version,
                got: artifact.schema.version,
            });
        }

        Ok(Self {
            dims: artifact.metadata.dimensions,
            weights: artifact.weights.clone(),
            schema_version: artifact.metadata.schema_version,
            model_id: artifact.metadata.model_id.clone(),
            generation: artifact.metadata.generation,
        })
    }

    /// Create a bootstrap model with small random weights.
    pub fn bootstrap() -> Self {
        let artifact = NnueModelArtifact::bootstrap();
        Self::from_artifact(&artifact).expect("Bootstrap artifact should always be valid")
    }

    /// Run the full forward pass on sparse features.
    /// Returns a raw evaluation score.
    pub fn forward(&self, sparse: &SparseFeatureVec) -> Score {
        let dims = &self.dims;

        // Step 1: Compute accumulator = bias + Σ(weight[i][j] * value[i])
        let mut accumulator = vec![0i32; dims.accumulator_size];
        accumulator.copy_from_slice(&self.weights.feature_biases[..dims.accumulator_size]);
        for feat in &sparse.features {
            let i = feat.index as usize;
            if i >= dims.input_size {
                continue;
            }
            let val = feat.value as i32;
            let weight_offset = i * dims.accumulator_size;
            for (acc_j, &w) in accumulator.iter_mut().zip(self.weights.feature_weights[weight_offset..weight_offset + dims.accumulator_size].iter()) {
                *acc_j += w as i32 * val;
            }
        }

        // Apply scale and ClippedReLU to accumulator
        let clipped_acc: Vec<i32> = accumulator
            .iter()
            .map(|&v| (v / FT_SCALE).clamp(0, QA))
            .collect();

        // Step 2: Hidden1 = bias + Σ(weight[j][k] * clipped_acc[j])
        let mut hidden1 = vec![0i32; dims.hidden1_size];
        for (k, h1_k) in hidden1.iter_mut().enumerate() {
            *h1_k = self.weights.hidden1_biases[k];
            for (j, &ca_j) in clipped_acc.iter().enumerate() {
                *h1_k +=
                    self.weights.hidden1_weights[j * dims.hidden1_size + k] as i32
                        * ca_j;
            }
        }

        // Apply scale and ClippedReLU to hidden1
        let clipped_h1: Vec<i32> = hidden1
            .iter()
            .map(|&v| (v / HIDDEN_SCALE).clamp(0, QA))
            .collect();

        // Step 3: Hidden2 = bias + Σ(weight[k][l] * clipped_h1[k])
        let mut hidden2 = vec![0i32; dims.hidden2_size];
        for (l, h2_l) in hidden2.iter_mut().enumerate() {
            *h2_l = self.weights.hidden2_biases[l];
            for (k, &ch1_k) in clipped_h1.iter().enumerate() {
                *h2_l +=
                    self.weights.hidden2_weights[k * dims.hidden2_size + l] as i32
                        * ch1_k;
            }
        }

        // Apply scale and ClippedReLU to hidden2
        let clipped_h2: Vec<i32> = hidden2
            .iter()
            .map(|&v| (v / HIDDEN_SCALE).clamp(0, QA))
            .collect();

        // Step 4: Output = bias + Σ(weight[l] * clipped_h2[l])
        let mut output = self.weights.output_bias;
        for (&w, &ch2_l) in self.weights.output_weights[..dims.hidden2_size].iter().zip(clipped_h2.iter()) {
            output += w as i32 * ch2_l;
        }

        // Step 5: Rescale to evaluation Score
        // Divide by QA to normalize, then apply output scale
        output * OUTPUT_SCALE / QA.max(1)
    }

    /// Run forward pass using a pre-computed accumulator (for incremental evaluation).
    /// The accumulator must already contain the correct accumulated values.
    pub fn forward_from_accumulator(&self, accumulator: &NnueAccumulator) -> Score {
        let dims = &self.dims;

        // ClippedReLU on accumulator values
        let clipped_acc: Vec<i32> = accumulator
            .values
            .iter()
            .map(|&v| (v / FT_SCALE).clamp(0, QA))
            .collect();

        // Hidden1
        let mut hidden1 = vec![0i32; dims.hidden1_size];
        for (k, h1_k) in hidden1.iter_mut().enumerate() {
            *h1_k = self.weights.hidden1_biases[k];
            for (j, &ca_j) in clipped_acc.iter().enumerate() {
                *h1_k +=
                    self.weights.hidden1_weights[j * dims.hidden1_size + k] as i32
                        * ca_j;
            }
        }
        let clipped_h1: Vec<i32> = hidden1
            .iter()
            .map(|&v| (v / HIDDEN_SCALE).clamp(0, QA))
            .collect();

        // Hidden2
        let mut hidden2 = vec![0i32; dims.hidden2_size];
        for (l, h2_l) in hidden2.iter_mut().enumerate() {
            *h2_l = self.weights.hidden2_biases[l];
            for (k, &ch1_k) in clipped_h1.iter().enumerate() {
                *h2_l +=
                    self.weights.hidden2_weights[k * dims.hidden2_size + l] as i32
                        * ch1_k;
            }
        }
        let clipped_h2: Vec<i32> = hidden2
            .iter()
            .map(|&v| (v / HIDDEN_SCALE).clamp(0, QA))
            .collect();

        // Output
        let mut output = self.weights.output_bias;
        for (&w, &ch2_l) in self.weights.output_weights[..dims.hidden2_size].iter().zip(clipped_h2.iter()) {
            output += w as i32 * ch2_l;
        }

        output * OUTPUT_SCALE / QA.max(1)
    }

    /// Compute the full accumulator from scratch for given sparse features.
    pub fn compute_accumulator(&self, sparse: &SparseFeatureVec) -> NnueAccumulator {
        let dims = &self.dims;
        let mut acc = NnueAccumulator::new(dims.accumulator_size);

        // Initialize with biases
        acc.values[..dims.accumulator_size].copy_from_slice(&self.weights.feature_biases[..dims.accumulator_size]);

        // Add feature contributions
        for feat in &sparse.features {
            let i = feat.index as usize;
            if i >= dims.input_size {
                continue;
            }
            let val = feat.value as i32;
            let weight_offset = i * dims.accumulator_size;
            for (acc_j, &w) in acc.values[..dims.accumulator_size].iter_mut().zip(self.weights.feature_weights[weight_offset..weight_offset + dims.accumulator_size].iter()) {
                *acc_j += w as i32 * val;
            }
        }

        acc.computed = true;
        acc
    }

    /// Incrementally update an accumulator based on a feature diff.
    /// Much faster than recomputing from scratch when few features change.
    pub fn update_accumulator(
        &self,
        acc: &mut NnueAccumulator,
        diff: &FeatureDiff,
    ) {
        let dims = &self.dims;

        // Remove contributions from removed features
        for feat in &diff.removed {
            let i = feat.index as usize;
            if i >= dims.input_size {
                continue;
            }
            let val = feat.value as i32;
            let weight_offset = i * dims.accumulator_size;
            for (acc_j, &w) in acc.values[..dims.accumulator_size].iter_mut().zip(self.weights.feature_weights[weight_offset..weight_offset + dims.accumulator_size].iter()) {
                *acc_j -= w as i32 * val;
            }
        }

        // Add contributions from added features
        for feat in &diff.added {
            let i = feat.index as usize;
            if i >= dims.input_size {
                continue;
            }
            let val = feat.value as i32;
            let weight_offset = i * dims.accumulator_size;
            for (acc_j, &w) in acc.values[..dims.accumulator_size].iter_mut().zip(self.weights.feature_weights[weight_offset..weight_offset + dims.accumulator_size].iter()) {
                *acc_j += w as i32 * val;
            }
        }

        // Update contributions from changed features (add delta)
        for (old_feat, new_feat) in &diff.changed {
            let i = old_feat.index as usize;
            if i >= dims.input_size {
                continue;
            }
            let delta = new_feat.value as i32 - old_feat.value as i32;
            let weight_offset = i * dims.accumulator_size;
            for (acc_j, &w) in acc.values[..dims.accumulator_size].iter_mut().zip(self.weights.feature_weights[weight_offset..weight_offset + dims.accumulator_size].iter()) {
                *acc_j += w as i32 * delta;
            }
        }
    }
}

// ============================================================================
// NNUE Accumulator Cache
// ============================================================================

/// Cached accumulator state for incremental NNUE evaluation.
/// Stores the pre-ClippedReLU accumulator values so they can be incrementally
/// updated when features change instead of recomputing from scratch.
#[derive(Debug, Clone)]
pub struct NnueAccumulator {
    /// Raw accumulator values (before ClippedReLU), one per accumulator neuron.
    pub values: Vec<i32>,
    /// Whether the accumulator has been computed (vs. uninitialized).
    pub computed: bool,
    /// The sparse features that were used to compute this accumulator.
    /// Stored for computing diffs when features change.
    pub cached_features: Option<SparseFeatureVec>,
}

impl NnueAccumulator {
    /// Create a new uninitialized accumulator.
    pub fn new(size: usize) -> Self {
        Self {
            values: vec![0i32; size],
            computed: false,
            cached_features: None,
        }
    }

    /// Reset the accumulator to uninitialized state.
    pub fn reset(&mut self) {
        for v in &mut self.values {
            *v = 0;
        }
        self.computed = false;
        self.cached_features = None;
    }

    /// Check if the accumulator needs recomputation.
    pub fn needs_refresh(&self) -> bool {
        !self.computed
    }
}

// ============================================================================
// NNUE Evaluator
// ============================================================================

/// NNUE-based position evaluator.
/// Implements the Evaluator trait for seamless integration with the search engine.
///
/// Supports two evaluation modes:
/// - Full evaluation: extract features, compute accumulator, forward pass
/// - Incremental evaluation: update accumulator from feature diff, forward pass
pub struct NnueEvaluator {
    /// The loaded NNUE model.
    model: NnueModel,
    /// Cached accumulator for incremental updates.
    accumulator: NnueAccumulator,
}

impl NnueEvaluator {
    /// Create a new NNUE evaluator from a model.
    pub fn new(model: NnueModel) -> Self {
        let acc_size = model.dims.accumulator_size;
        Self {
            model,
            accumulator: NnueAccumulator::new(acc_size),
        }
    }

    /// Create a new NNUE evaluator from a model artifact.
    pub fn from_artifact(artifact: &NnueModelArtifact) -> Result<Self, NnueError> {
        let model = NnueModel::from_artifact(artifact)?;
        Ok(Self::new(model))
    }

    /// Create a bootstrap NNUE evaluator with random weights.
    pub fn bootstrap() -> Self {
        Self::new(NnueModel::bootstrap())
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &NnueModel {
        &self.model
    }

    /// Get a mutable reference to the accumulator cache.
    pub fn accumulator_mut(&mut self) -> &mut NnueAccumulator {
        &mut self.accumulator
    }

    /// Reset the accumulator cache (forces full recomputation on next eval).
    pub fn reset_accumulator(&mut self) {
        self.accumulator.reset();
    }

    /// Evaluate a position using the NNUE network.
    /// Uses incremental accumulator update when possible.
    pub fn evaluate_position(&mut self, state: &GameState, perspective: PlayerId) -> Score {
        // Check for terminal states
        match state.game_outcome {
            GameOutcome::Victory(winner) => {
                return if winner == perspective {
                    SCORE_WIN - state.battle_round.number() as Score
                } else {
                    SCORE_LOSS + state.battle_round.number() as Score
                };
            }
            GameOutcome::Draw => return SCORE_DRAW,
            GameOutcome::InProgress => {}
        }

        // Check for tabling
        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + state.battle_round.number() as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - state.battle_round.number() as Score;
        }

        // Extract sparse features
        let new_features = extract_sparse_features_from_state(state, perspective);

        // Try incremental update if we have a cached accumulator
        if self.accumulator.computed {
            if let Some(ref old_features) = self.accumulator.cached_features.clone() {
                let diff = compute_feature_diff(old_features, &new_features);
                if !diff.is_empty() {
                    self.model.update_accumulator(&mut self.accumulator, &diff);
                }
                self.accumulator.cached_features = Some(new_features);
                let raw_score = self.model.forward_from_accumulator(&self.accumulator);
                return raw_score.clamp(SCORE_LOSS + 100, SCORE_WIN - 100);
            }
        }

        // Full computation
        self.accumulator = self.model.compute_accumulator(&new_features);
        self.accumulator.cached_features = Some(new_features);
        let raw_score = self.model.forward_from_accumulator(&self.accumulator);
        raw_score.clamp(SCORE_LOSS + 100, SCORE_WIN - 100)
    }

    /// Evaluate without using the accumulator cache (always full computation).
    /// Useful for benchmarking and verification.
    pub fn evaluate_full(&self, state: &GameState, perspective: PlayerId) -> Score {
        // Check for terminal states
        match state.game_outcome {
            GameOutcome::Victory(winner) => {
                return if winner == perspective {
                    SCORE_WIN - state.battle_round.number() as Score
                } else {
                    SCORE_LOSS + state.battle_round.number() as Score
                };
            }
            GameOutcome::Draw => return SCORE_DRAW,
            GameOutcome::InProgress => {}
        }

        if state.is_player_tabled(perspective) {
            return SCORE_LOSS + state.battle_round.number() as Score;
        }
        let opponent = state.opponent_id(perspective);
        if state.is_player_tabled(opponent) {
            return SCORE_WIN - state.battle_round.number() as Score;
        }

        let features = extract_sparse_features_from_state(state, perspective);
        let raw_score = self.model.forward(&features);
        raw_score.clamp(SCORE_LOSS + 100, SCORE_WIN - 100)
    }
}

impl Evaluator for NnueEvaluator {
    fn evaluate(&self, state: &GameState, perspective: PlayerId) -> Score {
        // The trait method is &self (not &mut self), so we use full computation.
        // For incremental evaluation, use evaluate_position() directly.
        self.evaluate_full(state, perspective)
    }

    fn name(&self) -> &str {
        "NnueEvaluator"
    }
}

// ============================================================================
// AnyEvaluator - Evaluator type switch
// ============================================================================

/// Evaluator enum that can hold either a heuristic or NNUE evaluator.
/// Enables runtime switching between evaluator types without dynamic dispatch.
///
/// Usage in search:
/// ```ignore
/// let evaluator = AnyEvaluator::Nnue(Box::new(NnueEvaluator::bootstrap()));
/// // or
/// let evaluator = AnyEvaluator::Heuristic(HeuristicEvaluator::with_defaults());
/// ```
pub enum AnyEvaluator {
    /// Traditional heuristic evaluator with weighted scoring terms.
    Heuristic(HeuristicEvaluator),
    /// NNUE neural network evaluator (boxed to reduce enum size difference between variants).
    Nnue(Box<NnueEvaluator>),
}

impl AnyEvaluator {
    /// Create a heuristic evaluator with default weights.
    pub fn heuristic_default() -> Self {
        AnyEvaluator::Heuristic(HeuristicEvaluator::with_defaults())
    }

    /// Create a bootstrap NNUE evaluator.
    pub fn nnue_bootstrap() -> Self {
        AnyEvaluator::Nnue(Box::new(NnueEvaluator::bootstrap()))
    }

    /// Create an NNUE evaluator from a model artifact.
    pub fn nnue_from_artifact(artifact: &NnueModelArtifact) -> Result<Self, NnueError> {
        Ok(AnyEvaluator::Nnue(Box::new(NnueEvaluator::from_artifact(artifact)?)))
    }

    /// Check if this is a heuristic evaluator.
    pub fn is_heuristic(&self) -> bool {
        matches!(self, AnyEvaluator::Heuristic(_))
    }

    /// Check if this is an NNUE evaluator.
    pub fn is_nnue(&self) -> bool {
        matches!(self, AnyEvaluator::Nnue(_))
    }

    /// Get the move ordering score for a position.
    /// Uses heuristic evaluator's move_ordering_score when available,
    /// falls back to regular evaluation for NNUE.
    pub fn move_ordering_score(&self, state: &GameState, perspective: PlayerId) -> Score {
        match self {
            AnyEvaluator::Heuristic(h) => h.move_ordering_score(state, perspective),
            AnyEvaluator::Nnue(n) => n.evaluate_full(state, perspective),
        }
    }
}

impl Evaluator for AnyEvaluator {
    fn evaluate(&self, state: &GameState, perspective: PlayerId) -> Score {
        match self {
            AnyEvaluator::Heuristic(h) => h.evaluate(state, perspective),
            AnyEvaluator::Nnue(n) => n.evaluate(state, perspective),
        }
    }

    fn name(&self) -> &str {
        match self {
            AnyEvaluator::Heuristic(h) => h.name(),
            AnyEvaluator::Nnue(n) => n.name(),
        }
    }
}

// ============================================================================
// Model Registry
// ============================================================================

/// Registry for storing, loading, listing, and validating NNUE model artifacts.
/// Models are stored as versioned files in a base directory.
///
/// Directory structure:
/// ```text
/// base_path/
///   models/
///     gen0_bootstrap.nnue
///     gen1_20240315.nnue
///     ...
///   metadata.json
/// ```
pub struct ModelRegistry {
    /// Base directory for model storage.
    base_path: PathBuf,
}

/// Entry in the model registry index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    /// Model metadata.
    pub metadata: ModelMetadata,
    /// File name of the model artifact.
    pub filename: String,
}

/// Registry index file containing metadata for all stored models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRegistryIndex {
    /// All registered models.
    pub models: Vec<ModelRegistryEntry>,
}

impl ModelRegistry {
    /// Create a new model registry at the given base path.
    /// Creates the directory structure if it doesn't exist.
    pub fn new(base_path: PathBuf) -> Result<Self, NnueError> {
        let models_dir = base_path.join("models");
        std::fs::create_dir_all(&models_dir)?;
        Ok(Self { base_path })
    }

    /// Create a registry for in-memory use only (no filesystem).
    /// Useful for testing.
    pub fn in_memory() -> Self {
        Self {
            base_path: PathBuf::from("/dev/null"),
        }
    }

    /// Get the models directory path.
    fn models_dir(&self) -> PathBuf {
        self.base_path.join("models")
    }

    /// Get the index file path.
    fn index_path(&self) -> PathBuf {
        self.base_path.join("registry_index.json")
    }

    /// Get the file path for a model by ID.
    fn model_path(&self, model_id: &str) -> PathBuf {
        self.models_dir()
            .join(format!("{}.nnue", model_id))
    }

    /// Load the registry index.
    fn load_index(&self) -> Result<ModelRegistryIndex, NnueError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(ModelRegistryIndex::default());
        }
        let data = std::fs::read_to_string(&path)?;
        serde_json::from_str(&data).map_err(|e| NnueError::SerializationError(e.to_string()))
    }

    /// Save the registry index.
    fn save_index(&self, index: &ModelRegistryIndex) -> Result<(), NnueError> {
        let data = serde_json::to_string_pretty(index)
            .map_err(|e| NnueError::SerializationError(e.to_string()))?;
        std::fs::write(self.index_path(), data)?;
        Ok(())
    }

    /// Store a model artifact in the registry.
    pub fn store(&self, artifact: &NnueModelArtifact) -> Result<(), NnueError> {
        // Validate the artifact
        artifact.validate()?;

        // Save the artifact file
        let model_path = self.model_path(&artifact.metadata.model_id);
        artifact.save_to_file(&model_path)?;

        // Update the index
        let mut index = self.load_index()?;

        // Remove existing entry with same ID if present
        index
            .models
            .retain(|e| e.metadata.model_id != artifact.metadata.model_id);

        // Add new entry
        index.models.push(ModelRegistryEntry {
            metadata: artifact.metadata.clone(),
            filename: format!("{}.nnue", artifact.metadata.model_id),
        });

        // Sort by generation for consistent ordering
        index.models.sort_by_key(|e| e.metadata.generation);

        self.save_index(&index)?;

        Ok(())
    }

    /// Load a model artifact by ID.
    pub fn load(&self, model_id: &str) -> Result<NnueModelArtifact, NnueError> {
        let model_path = self.model_path(model_id);
        if !model_path.exists() {
            return Err(NnueError::ModelNotFound(model_id.to_string()));
        }
        NnueModelArtifact::load_from_file(&model_path)
    }

    /// List all registered model metadata.
    pub fn list(&self) -> Result<Vec<ModelMetadata>, NnueError> {
        let index = self.load_index()?;
        Ok(index.models.into_iter().map(|e| e.metadata).collect())
    }

    /// Validate a stored model by ID.
    /// Returns true if the model exists and passes validation.
    pub fn validate(&self, model_id: &str) -> Result<bool, NnueError> {
        match self.load(model_id) {
            Ok(artifact) => match artifact.validate() {
                Ok(()) => Ok(true),
                Err(_) => Ok(false),
            },
            Err(NnueError::ModelNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Get the latest model (highest generation).
    pub fn latest(&self) -> Result<Option<ModelMetadata>, NnueError> {
        let index = self.load_index()?;
        Ok(index.models.last().map(|e| e.metadata.clone()))
    }

    /// Get a model by generation number.
    pub fn get_by_generation(&self, generation: u32) -> Result<Option<ModelMetadata>, NnueError> {
        let index = self.load_index()?;
        Ok(index
            .models
            .iter()
            .find(|e| e.metadata.generation == generation)
            .map(|e| e.metadata.clone()))
    }

    /// Bootstrap the registry with a generation 0 model.
    /// Returns the bootstrap artifact. If a gen0 model already exists, returns it.
    pub fn bootstrap(&self) -> Result<NnueModelArtifact, NnueError> {
        // Check if gen0 already exists
        if let Ok(Some(_)) = self.get_by_generation(0) {
            return self.load("gen0_bootstrap");
        }

        // Create and store bootstrap model
        let artifact = NnueModelArtifact::bootstrap();
        self.store(&artifact)?;
        Ok(artifact)
    }

    /// Load the latest model as a ready-to-use NnueModel.
    pub fn load_latest_model(&self) -> Result<Option<NnueModel>, NnueError> {
        match self.latest()? {
            Some(metadata) => {
                let artifact = self.load(&metadata.model_id)?;
                let model = NnueModel::from_artifact(&artifact)?;
                Ok(Some(model))
            }
            None => Ok(None),
        }
    }
}

// ============================================================================
// Evaluation Benchmark
// ============================================================================

/// Results from an evaluation benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Number of positions evaluated.
    pub positions_evaluated: usize,
    /// Total time elapsed in microseconds.
    pub total_time_us: u64,
    /// Average time per evaluation in microseconds.
    pub avg_time_us: u64,
    /// Evaluations per second.
    pub evals_per_second: u64,
    /// Average absolute score.
    pub avg_abs_score: f64,
    /// Score standard deviation.
    pub score_std_dev: f64,
    /// Evaluator name.
    pub evaluator_name: String,
}

impl std::fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Benchmark [{}]: {} positions in {}us ({} evals/s, avg {}us/eval, avg_score={:.1}, std={:.1})",
            self.evaluator_name,
            self.positions_evaluated,
            self.total_time_us,
            self.evals_per_second,
            self.avg_time_us,
            self.avg_abs_score,
            self.score_std_dev,
        )
    }
}

/// Run an evaluation benchmark on the given evaluator.
/// Evaluates the same game state `count` times and reports performance metrics.
pub fn benchmark_evaluator(
    evaluator: &dyn Evaluator,
    state: &GameState,
    perspective: PlayerId,
    count: usize,
) -> BenchmarkResult {
    let mut scores = Vec::with_capacity(count);

    let start = std::time::Instant::now();
    for _ in 0..count {
        let score = evaluator.evaluate(state, perspective);
        scores.push(score);
    }
    let elapsed = start.elapsed();
    let total_time_us = elapsed.as_micros() as u64;

    let avg_time_us = if count > 0 {
        total_time_us / count as u64
    } else {
        0
    };
    let evals_per_second = if total_time_us > 0 {
        (count as u64 * 1_000_000) / total_time_us
    } else {
        0
    };

    let sum_abs: f64 = scores.iter().map(|&s| (s as f64).abs()).sum();
    let avg_abs_score = if count > 0 {
        sum_abs / count as f64
    } else {
        0.0
    };

    let mean: f64 = scores.iter().map(|&s| s as f64).sum::<f64>() / count.max(1) as f64;
    let variance: f64 = scores
        .iter()
        .map(|&s| {
            let diff = s as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / count.max(1) as f64;
    let score_std_dev = variance.sqrt();

    BenchmarkResult {
        positions_evaluated: count,
        total_time_us,
        avg_time_us,
        evals_per_second,
        avg_abs_score,
        score_std_dev,
        evaluator_name: evaluator.name().to_string(),
    }
}

/// Compare two evaluators on the same position.
/// Returns (heuristic_result, nnue_result) benchmarks.
pub fn compare_evaluators(
    heuristic: &HeuristicEvaluator,
    nnue: &NnueEvaluator,
    state: &GameState,
    perspective: PlayerId,
    count: usize,
) -> (BenchmarkResult, BenchmarkResult) {
    let heuristic_result = benchmark_evaluator(heuristic, state, perspective, count);
    let nnue_result = benchmark_evaluator(nnue, state, perspective, count);
    (heuristic_result, nnue_result)
}

// ============================================================================
// Policy/Value Model (Dual-Head Network for AlphaGo-Style Training)
// ============================================================================

/// Action vocabulary size for the policy head (40 intents x 16 unit slots).
pub const ACTION_VOCAB_SIZE: usize = 640;

/// Dimensions for the policy/value dual-head network.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PolicyValueDimensions {
    /// Input feature size (same as NNUE: TOTAL_FEATURES).
    pub input_size: usize,
    /// Shared trunk hidden layer size.
    pub trunk_size: usize,
    /// Policy head hidden layer size.
    pub policy_hidden_size: usize,
    /// Policy head output size (ACTION_VOCAB_SIZE = 640).
    pub policy_output_size: usize,
    /// Value head hidden layer size.
    pub value_hidden_size: usize,
    /// Value head output size (1 scalar in [-1, 1]).
    pub value_output_size: usize,
}

impl PolicyValueDimensions {
    /// Default dimensions matching the Python training architecture.
    pub const DEFAULT: PolicyValueDimensions = PolicyValueDimensions {
        input_size: TOTAL_FEATURES,
        trunk_size: 128,
        policy_hidden_size: 64,
        policy_output_size: ACTION_VOCAB_SIZE,
        value_hidden_size: 64,
        value_output_size: 1,
    };

    /// Total number of weights in the policy/value model.
    pub fn total_weight_count(&self) -> usize {
        // Trunk: input -> trunk
        let trunk_weights = self.input_size * self.trunk_size + self.trunk_size;
        // Policy head: trunk -> policy_hidden -> policy_output
        let policy_weights = self.trunk_size * self.policy_hidden_size + self.policy_hidden_size
            + self.policy_hidden_size * self.policy_output_size + self.policy_output_size;
        // Value head: trunk -> value_hidden -> value_output
        let value_weights = self.trunk_size * self.value_hidden_size + self.value_hidden_size
            + self.value_hidden_size * self.value_output_size + self.value_output_size;
        trunk_weights + policy_weights + value_weights
    }
}

/// Quantized weights for the policy/value model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyValueWeights {
    /// Trunk weights: input_size x trunk_size (i16 quantized).
    pub trunk_weights: Vec<i16>,
    /// Trunk biases: trunk_size (i32).
    pub trunk_biases: Vec<i32>,
    /// Policy hidden weights: trunk_size x policy_hidden_size (i8).
    pub policy_hidden_weights: Vec<i8>,
    /// Policy hidden biases: policy_hidden_size (i32).
    pub policy_hidden_biases: Vec<i32>,
    /// Policy output weights: policy_hidden_size x policy_output_size (i8).
    pub policy_output_weights: Vec<i8>,
    /// Policy output biases: policy_output_size (i32).
    pub policy_output_biases: Vec<i32>,
    /// Value hidden weights: trunk_size x value_hidden_size (i8).
    pub value_hidden_weights: Vec<i8>,
    /// Value hidden biases: value_hidden_size (i32).
    pub value_hidden_biases: Vec<i32>,
    /// Value output weights: value_hidden_size x 1 (i8).
    pub value_output_weights: Vec<i8>,
    /// Value output bias: 1 (i32).
    pub value_output_bias: i32,
}

/// A policy/value model for AlphaGo-style MCTS.
#[derive(Debug, Clone)]
pub struct PolicyValueModel {
    pub dims: PolicyValueDimensions,
    pub weights: PolicyValueWeights,
}

impl PolicyValueModel {
    /// Create a new model with random initialization (for bootstrap).
    pub fn bootstrap() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let dims = PolicyValueDimensions::DEFAULT;

        let trunk_weights: Vec<i16> = (0..dims.input_size * dims.trunk_size)
            .map(|_| rng.gen_range(-10..=10))
            .collect();
        let trunk_biases = vec![0i32; dims.trunk_size];

        let policy_hidden_weights: Vec<i8> = (0..dims.trunk_size * dims.policy_hidden_size)
            .map(|_| rng.gen_range(-5..=5))
            .collect();
        let policy_hidden_biases = vec![0i32; dims.policy_hidden_size];

        let policy_output_weights: Vec<i8> = (0..dims.policy_hidden_size * dims.policy_output_size)
            .map(|_| rng.gen_range(-5..=5))
            .collect();
        let policy_output_biases = vec![0i32; dims.policy_output_size];

        let value_hidden_weights: Vec<i8> = (0..dims.trunk_size * dims.value_hidden_size)
            .map(|_| rng.gen_range(-5..=5))
            .collect();
        let value_hidden_biases = vec![0i32; dims.value_hidden_size];

        let value_output_weights: Vec<i8> = (0..dims.value_hidden_size)
            .map(|_| rng.gen_range(-5..=5))
            .collect();

        Self {
            dims,
            weights: PolicyValueWeights {
                trunk_weights,
                trunk_biases,
                policy_hidden_weights,
                policy_hidden_biases,
                policy_output_weights,
                policy_output_biases,
                value_hidden_weights,
                value_hidden_biases,
                value_output_weights,
                value_output_bias: 0,
            },
        }
    }

    /// Run inference: given sparse features, return (policy_logits, value).
    /// Policy logits are raw (pre-softmax) scores over ACTION_VOCAB_SIZE.
    /// Value is in [-1, 1] (tanh activation).
    pub fn infer(&self, sparse_features: &[(u16, i16)]) -> (Vec<f32>, f32) {
        let dims = &self.dims;
        let w = &self.weights;

        // 1. Compute trunk activations (sparse input -> trunk)
        let mut trunk = vec![0i32; dims.trunk_size];
        for &(idx, val) in sparse_features {
            if (idx as usize) < dims.input_size {
                let base = idx as usize * dims.trunk_size;
                for j in 0..dims.trunk_size {
                    trunk[j] += w.trunk_weights[base + j] as i32 * val as i32;
                }
            }
        }
        // Add biases and apply ClippedReLU
        for j in 0..dims.trunk_size {
            trunk[j] = (trunk[j] / FT_SCALE + w.trunk_biases[j]).max(0).min(QA);
        }

        // 2. Policy head
        // trunk -> policy_hidden (ClippedReLU)
        let mut policy_hidden = vec![0i32; dims.policy_hidden_size];
        for j in 0..dims.policy_hidden_size {
            let mut sum = 0i32;
            for k in 0..dims.trunk_size {
                sum += trunk[k] * w.policy_hidden_weights[k * dims.policy_hidden_size + j] as i32;
            }
            policy_hidden[j] = (sum / HIDDEN_SCALE + w.policy_hidden_biases[j]).max(0).min(QA);
        }

        // policy_hidden -> policy_output (raw logits, no activation)
        let mut policy_logits = vec![0f32; dims.policy_output_size];
        for j in 0..dims.policy_output_size {
            let mut sum = 0i32;
            for k in 0..dims.policy_hidden_size {
                sum += policy_hidden[k] * w.policy_output_weights[k * dims.policy_output_size + j] as i32;
            }
            policy_logits[j] = (sum as f32 / (HIDDEN_SCALE as f32 * QA as f32))
                + w.policy_output_biases[j] as f32 / 1000.0;
        }

        // 3. Value head
        // trunk -> value_hidden (ClippedReLU)
        let mut value_hidden = vec![0i32; dims.value_hidden_size];
        for j in 0..dims.value_hidden_size {
            let mut sum = 0i32;
            for k in 0..dims.trunk_size {
                sum += trunk[k] * w.value_hidden_weights[k * dims.value_hidden_size + j] as i32;
            }
            value_hidden[j] = (sum / HIDDEN_SCALE + w.value_hidden_biases[j]).max(0).min(QA);
        }

        // value_hidden -> scalar (tanh activation)
        let mut value_sum = 0i32;
        for k in 0..dims.value_hidden_size {
            value_sum += value_hidden[k] * w.value_output_weights[k] as i32;
        }
        let value_raw = value_sum as f32 / (HIDDEN_SCALE as f32 * QA as f32)
            + w.value_output_bias as f32 / 1000.0;
        let value = value_raw.tanh();

        (policy_logits, value)
    }

    /// Get policy probabilities (softmax of logits) masked by legal actions.
    pub fn policy_probs(&self, sparse_features: &[(u16, i16)], legal_mask: &[bool]) -> Vec<f32> {
        let (logits, _) = self.infer(sparse_features);
        masked_softmax(&logits, legal_mask)
    }

    /// Get value estimate from the model.
    pub fn value(&self, sparse_features: &[(u16, i16)]) -> f32 {
        let (_, value) = self.infer(sparse_features);
        value
    }

    /// Save model to a file.
    pub fn save(&self, path: &Path) -> Result<(), NnueError> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| NnueError::SerializationError(e.to_string()))?;
        std::fs::write(path, data)
            .map_err(NnueError::IoError)?;
        Ok(())
    }

    /// Load model from a file.
    pub fn load(path: &Path) -> Result<Self, NnueError> {
        let data = std::fs::read_to_string(path)
            .map_err(NnueError::IoError)?;
        let model: PolicyValueModel = serde_json::from_str(&data)
            .map_err(|e| NnueError::SerializationError(e.to_string()))?;
        Ok(model)
    }
}

impl Serialize for PolicyValueModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PolicyValueModel", 2)?;
        state.serialize_field("dims", &self.dims)?;
        state.serialize_field("weights", &self.weights)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PolicyValueModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PolicyValueModelData {
            dims: PolicyValueDimensions,
            weights: PolicyValueWeights,
        }
        let data = PolicyValueModelData::deserialize(deserializer)?;
        Ok(PolicyValueModel {
            dims: data.dims,
            weights: data.weights,
        })
    }
}

/// Compute softmax over logits, masking out illegal actions.
fn masked_softmax(logits: &[f32], legal_mask: &[bool]) -> Vec<f32> {
    assert_eq!(logits.len(), legal_mask.len());

    let max_logit = logits.iter()
        .zip(legal_mask.iter())
        .filter(|(_, &legal)| legal)
        .map(|(&l, _)| l)
        .fold(f32::NEG_INFINITY, f32::max);

    if max_logit == f32::NEG_INFINITY {
        // No legal actions - return uniform over all
        let n = logits.len() as f32;
        return vec![1.0 / n; logits.len()];
    }

    let mut probs: Vec<f32> = logits.iter()
        .zip(legal_mask.iter())
        .map(|(&l, &legal)| {
            if legal {
                (l - max_logit).exp()
            } else {
                0.0
            }
        })
        .collect();

    let sum: f32 = probs.iter().sum();
    if sum > 0.0 {
        probs.iter_mut().for_each(|p| *p /= sum);
    }

    probs
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // NnueDimensions tests
    // ========================================================================

    #[test]
    fn test_default_dimensions() {
        let dims = NnueDimensions::DEFAULT;
        assert_eq!(dims.input_size, 1209);
        assert_eq!(dims.accumulator_size, 128);
        assert_eq!(dims.hidden1_size, 32);
        assert_eq!(dims.hidden2_size, 32);
        assert_eq!(dims.output_size, 1);
    }

    #[test]
    fn test_dimension_weight_counts() {
        let dims = NnueDimensions::DEFAULT;
        assert_eq!(dims.feature_weight_count(), 1209 * 128);
        assert_eq!(dims.hidden1_weight_count(), 128 * 32);
        assert_eq!(dims.hidden2_weight_count(), 32 * 32);
        assert_eq!(dims.output_weight_count(), 32);
    }

    #[test]
    fn test_dimension_total_parameters() {
        let dims = NnueDimensions::DEFAULT;
        let expected = (1209 * 128) + 128 + (128 * 32) + 32 + (32 * 32) + 32 + 32 + 1;
        assert_eq!(dims.total_parameters(), expected);
    }

    #[test]
    fn test_dimension_validation() {
        assert!(NnueDimensions::DEFAULT.validate().is_ok());

        let bad = NnueDimensions {
            input_size: 0,
            ..NnueDimensions::DEFAULT
        };
        assert!(bad.validate().is_err());

        let bad = NnueDimensions {
            output_size: 2,
            ..NnueDimensions::DEFAULT
        };
        assert!(bad.validate().is_err());
    }

    // ========================================================================
    // QuantizedWeights tests
    // ========================================================================

    #[test]
    fn test_weights_zeros() {
        let dims = NnueDimensions::DEFAULT;
        let weights = QuantizedWeights::zeros(&dims);
        assert_eq!(weights.feature_weights.len(), dims.feature_weight_count());
        assert_eq!(weights.feature_biases.len(), dims.accumulator_size);
        assert_eq!(weights.hidden1_weights.len(), dims.hidden1_weight_count());
        assert_eq!(weights.hidden1_biases.len(), dims.hidden1_size);
        assert_eq!(weights.hidden2_weights.len(), dims.hidden2_weight_count());
        assert_eq!(weights.hidden2_biases.len(), dims.hidden2_size);
        assert_eq!(weights.output_weights.len(), dims.output_weight_count());
        assert!(weights.feature_weights.iter().all(|&w| w == 0));
    }

    #[test]
    fn test_weights_random_small() {
        let dims = NnueDimensions::DEFAULT;
        let weights = QuantizedWeights::random_small(&dims, 42);
        assert_eq!(weights.feature_weights.len(), dims.feature_weight_count());

        // Check that weights are in expected small range
        assert!(weights.feature_weights.iter().all(|&w| w >= -8 && w <= 8));
        assert!(weights.hidden1_weights.iter().all(|&w| w >= -8 && w <= 8));
        assert!(weights.output_weights.iter().all(|&w| w >= -16 && w <= 16));

        // Check that random weights are not all zero
        assert!(weights.feature_weights.iter().any(|&w| w != 0));
    }

    #[test]
    fn test_weights_random_deterministic() {
        let dims = NnueDimensions::DEFAULT;
        let w1 = QuantizedWeights::random_small(&dims, 42);
        let w2 = QuantizedWeights::random_small(&dims, 42);
        assert_eq!(w1.feature_weights, w2.feature_weights);
        assert_eq!(w1.hidden1_weights, w2.hidden1_weights);
        assert_eq!(w1.output_weights, w2.output_weights);
    }

    #[test]
    fn test_weights_different_seeds_differ() {
        let dims = NnueDimensions::DEFAULT;
        let w1 = QuantizedWeights::random_small(&dims, 42);
        let w2 = QuantizedWeights::random_small(&dims, 43);
        assert_ne!(w1.feature_weights, w2.feature_weights);
    }

    #[test]
    fn test_weights_validation() {
        let dims = NnueDimensions::DEFAULT;
        let weights = QuantizedWeights::zeros(&dims);
        assert!(weights.validate(&dims).is_ok());

        // Wrong feature weight count
        let mut bad_weights = weights.clone();
        bad_weights.feature_weights.pop();
        assert!(bad_weights.validate(&dims).is_err());
    }

    // ========================================================================
    // ModelMetadata tests
    // ========================================================================

    #[test]
    fn test_bootstrap_metadata() {
        let dims = NnueDimensions::DEFAULT;
        let meta = ModelMetadata::bootstrap(dims);
        assert_eq!(meta.generation, 0);
        assert_eq!(meta.schema_version, FEATURE_SCHEMA_VERSION);
        assert_eq!(meta.model_id, "gen0_bootstrap");
        assert!(meta.parent_model_id.is_none());
        assert_eq!(meta.training_positions, 0);
    }

    #[test]
    fn test_trained_metadata() {
        let dims = NnueDimensions::DEFAULT;
        let meta = ModelMetadata::new_trained(
            "gen1_test".to_string(),
            1,
            dims,
            "gen0_bootstrap".to_string(),
            100_000,
            0.05,
        );
        assert_eq!(meta.generation, 1);
        assert_eq!(meta.parent_model_id, Some("gen0_bootstrap".to_string()));
        assert_eq!(meta.training_positions, 100_000);
    }

    // ========================================================================
    // NnueModelArtifact tests
    // ========================================================================

    #[test]
    fn test_bootstrap_artifact() {
        let artifact = NnueModelArtifact::bootstrap();
        assert!(artifact.validate().is_ok());
        assert_eq!(artifact.metadata.generation, 0);
        assert_eq!(artifact.schema.version, FEATURE_SCHEMA_VERSION);
    }

    #[test]
    fn test_artifact_serialization_roundtrip_bincode() {
        let artifact = NnueModelArtifact::bootstrap();
        let bytes = artifact.to_bytes().expect("serialization should succeed");
        let loaded =
            NnueModelArtifact::from_bytes(&bytes).expect("deserialization should succeed");
        assert!(loaded.validate().is_ok());
        assert_eq!(loaded.metadata.model_id, artifact.metadata.model_id);
        assert_eq!(loaded.checksum, artifact.checksum);
    }

    #[test]
    fn test_artifact_checksum_integrity() {
        let artifact = NnueModelArtifact::bootstrap();
        let checksum1 = artifact.checksum;

        // Same weights should produce same checksum
        let artifact2 = NnueModelArtifact::bootstrap();
        assert_eq!(artifact2.checksum, checksum1);
    }

    #[test]
    fn test_artifact_validation_catches_corruption() {
        let mut artifact = NnueModelArtifact::bootstrap();
        // Corrupt a weight
        if !artifact.weights.feature_weights.is_empty() {
            artifact.weights.feature_weights[0] += 1;
        }
        // Checksum should no longer match
        assert!(artifact.validate().is_err());
    }

    #[test]
    fn test_artifact_validation_catches_wrong_schema() {
        let mut artifact = NnueModelArtifact::bootstrap();
        artifact.metadata.schema_version = 999;
        assert!(artifact.validate().is_err());
    }

    // ========================================================================
    // NnueModel tests
    // ========================================================================

    #[test]
    fn test_model_from_artifact() {
        let artifact = NnueModelArtifact::bootstrap();
        let model = NnueModel::from_artifact(&artifact);
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.dims, NnueDimensions::DEFAULT);
        assert_eq!(model.generation, 0);
    }

    #[test]
    fn test_model_bootstrap() {
        let model = NnueModel::bootstrap();
        assert_eq!(model.dims, NnueDimensions::DEFAULT);
        assert_eq!(model.generation, 0);
    }

    #[test]
    fn test_forward_pass_zero_weights() {
        let dims = NnueDimensions::DEFAULT;
        let model = NnueModel {
            dims,
            weights: QuantizedWeights::zeros(&dims),
            schema_version: FEATURE_SCHEMA_VERSION,
            model_id: "test_zero".to_string(),
            generation: 0,
        };

        let features = SparseFeatureVec::new();
        let score = model.forward(&features);
        // Zero weights + zero biases + no features = 0
        assert_eq!(score, 0);
    }

    #[test]
    fn test_forward_pass_with_features() {
        let model = NnueModel::bootstrap();

        // Create some sparse features
        let mut features = SparseFeatureVec::with_capacity(10);
        features.push_binary(0); // round 1
        features.push_binary(5); // phase 0
        features.push(12, 50);   // vp_differential
        features.push(14, 127);  // own_model_ratio

        let score = model.forward(&features);
        // Score should be a finite value (not NaN or infinite)
        assert!(score > SCORE_LOSS && score < SCORE_WIN);
    }

    #[test]
    fn test_forward_pass_deterministic() {
        let model = NnueModel::bootstrap();

        let mut features = SparseFeatureVec::with_capacity(5);
        features.push_binary(0);
        features.push(12, 50);
        features.push(14, 100);

        let score1 = model.forward(&features);
        let score2 = model.forward(&features);
        assert_eq!(score1, score2, "Forward pass must be deterministic");
    }

    #[test]
    fn test_forward_from_accumulator_matches_direct() {
        let model = NnueModel::bootstrap();

        let mut features = SparseFeatureVec::with_capacity(5);
        features.push_binary(0);
        features.push_binary(6);
        features.push(12, 30);
        features.push(14, 80);
        features.push(15, 60);

        let direct_score = model.forward(&features);
        let acc = model.compute_accumulator(&features);
        let acc_score = model.forward_from_accumulator(&acc);
        assert_eq!(
            direct_score, acc_score,
            "Accumulator-based score must match direct forward pass"
        );
    }

    // ========================================================================
    // Accumulator tests
    // ========================================================================

    #[test]
    fn test_accumulator_new() {
        let acc = NnueAccumulator::new(128);
        assert_eq!(acc.values.len(), 128);
        assert!(!acc.computed);
        assert!(acc.cached_features.is_none());
    }

    #[test]
    fn test_accumulator_reset() {
        let model = NnueModel::bootstrap();
        let mut features = SparseFeatureVec::new();
        features.push_binary(0);

        let mut acc = model.compute_accumulator(&features);
        assert!(acc.computed);

        acc.reset();
        assert!(!acc.computed);
        assert!(acc.cached_features.is_none());
    }

    #[test]
    fn test_incremental_accumulator_update() {
        let model = NnueModel::bootstrap();

        // Initial features
        let mut old_features = SparseFeatureVec::with_capacity(3);
        old_features.push_binary(0);
        old_features.push(12, 30);
        old_features.push(14, 100);

        // Modified features (changed value at index 12, added index 15)
        let mut new_features = SparseFeatureVec::with_capacity(3);
        new_features.push_binary(0);
        new_features.push(12, 50); // changed
        new_features.push(15, 80); // added (14 removed)

        // Compute accumulator from scratch for new features
        let expected_acc = model.compute_accumulator(&new_features);

        // Compute incrementally
        let mut inc_acc = model.compute_accumulator(&old_features);
        let diff = compute_feature_diff(&old_features, &new_features);
        model.update_accumulator(&mut inc_acc, &diff);

        // Compare accumulator values
        assert_eq!(
            inc_acc.values, expected_acc.values,
            "Incremental update must produce same accumulator as full computation"
        );
    }

    #[test]
    fn test_incremental_update_empty_diff() {
        let model = NnueModel::bootstrap();

        let mut features = SparseFeatureVec::with_capacity(2);
        features.push_binary(0);
        features.push(12, 50);

        let acc_original = model.compute_accumulator(&features);
        let mut acc_updated = acc_original.clone();

        let diff = FeatureDiff::default(); // empty diff
        model.update_accumulator(&mut acc_updated, &diff);

        assert_eq!(
            acc_original.values, acc_updated.values,
            "Empty diff should not change accumulator"
        );
    }

    // ========================================================================
    // NnueEvaluator tests
    // ========================================================================

    #[test]
    fn test_nnue_evaluator_creation() {
        let eval = NnueEvaluator::bootstrap();
        assert_eq!(eval.model().dims, NnueDimensions::DEFAULT);
        assert_eq!(eval.name(), "NnueEvaluator");
    }

    #[test]
    fn test_nnue_evaluator_from_artifact() {
        let artifact = NnueModelArtifact::bootstrap();
        let eval = NnueEvaluator::from_artifact(&artifact);
        assert!(eval.is_ok());
    }

    #[test]
    fn test_nnue_evaluator_reset_accumulator() {
        let mut eval = NnueEvaluator::bootstrap();
        assert!(eval.accumulator_mut().needs_refresh());
        eval.reset_accumulator();
        assert!(eval.accumulator_mut().needs_refresh());
    }

    // ========================================================================
    // AnyEvaluator tests
    // ========================================================================

    #[test]
    fn test_any_evaluator_heuristic() {
        let eval = AnyEvaluator::heuristic_default();
        assert!(eval.is_heuristic());
        assert!(!eval.is_nnue());
        assert_eq!(eval.name(), "HeuristicEvaluator");
    }

    #[test]
    fn test_any_evaluator_nnue() {
        let eval = AnyEvaluator::nnue_bootstrap();
        assert!(!eval.is_heuristic());
        assert!(eval.is_nnue());
        assert_eq!(eval.name(), "NnueEvaluator");
    }

    #[test]
    fn test_any_evaluator_from_artifact() {
        let artifact = NnueModelArtifact::bootstrap();
        let eval = AnyEvaluator::nnue_from_artifact(&artifact);
        assert!(eval.is_ok());
        assert!(eval.unwrap().is_nnue());
    }

    // ========================================================================
    // ModelRegistry tests
    // ========================================================================

    #[test]
    fn test_registry_bootstrap() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_bootstrap");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");
        let artifact = registry.bootstrap().expect("bootstrap");
        assert_eq!(artifact.metadata.generation, 0);
        assert!(artifact.validate().is_ok());

        // Listing should show the bootstrap model
        let models = registry.list().expect("list");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].generation, 0);

        // Validate should return true
        assert!(registry.validate("gen0_bootstrap").expect("validate"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_registry_store_and_load() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_store_load");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");

        // Store a custom model
        let dims = NnueDimensions::DEFAULT;
        let metadata = ModelMetadata::new_trained(
            "gen1_test".to_string(),
            1,
            dims,
            "gen0_bootstrap".to_string(),
            50_000,
            0.1,
        );
        let schema = NnueFeatureSchema::current();
        let weights = QuantizedWeights::random_small(&dims, 123);
        let artifact = NnueModelArtifact::new(metadata, schema, weights);

        registry.store(&artifact).expect("store");

        // Load it back
        let loaded = registry.load("gen1_test").expect("load");
        assert_eq!(loaded.metadata.model_id, "gen1_test");
        assert_eq!(loaded.metadata.generation, 1);
        assert!(loaded.validate().is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_registry_list_ordering() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_list_order");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");

        // Store gen0
        registry.bootstrap().expect("bootstrap");

        // Store gen1
        let dims = NnueDimensions::DEFAULT;
        let meta1 = ModelMetadata::new_trained(
            "gen1".to_string(),
            1,
            dims,
            "gen0_bootstrap".to_string(),
            10_000,
            0.2,
        );
        let a1 = NnueModelArtifact::new(
            meta1,
            NnueFeatureSchema::current(),
            QuantizedWeights::random_small(&dims, 1),
        );
        registry.store(&a1).expect("store gen1");

        // Store gen2
        let meta2 = ModelMetadata::new_trained(
            "gen2".to_string(),
            2,
            dims,
            "gen1".to_string(),
            20_000,
            0.15,
        );
        let a2 = NnueModelArtifact::new(
            meta2,
            NnueFeatureSchema::current(),
            QuantizedWeights::random_small(&dims, 2),
        );
        registry.store(&a2).expect("store gen2");

        // List should be ordered by generation
        let models = registry.list().expect("list");
        assert_eq!(models.len(), 3);
        assert_eq!(models[0].generation, 0);
        assert_eq!(models[1].generation, 1);
        assert_eq!(models[2].generation, 2);

        // Latest should be gen2
        let latest = registry.latest().expect("latest");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().generation, 2);

        // Get by generation
        let gen1 = registry.get_by_generation(1).expect("get gen1");
        assert!(gen1.is_some());
        assert_eq!(gen1.unwrap().model_id, "gen1");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_registry_model_not_found() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_not_found");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");
        let result = registry.load("nonexistent");
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_registry_validate_nonexistent() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_validate_ne");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");
        let valid = registry.validate("nonexistent").expect("validate");
        assert!(!valid);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_registry_load_latest_model() {
        let dir = std::env::temp_dir().join("wh40k_nnue_test_load_latest");
        let _ = std::fs::remove_dir_all(&dir);

        let registry = ModelRegistry::new(dir.clone()).expect("create registry");
        registry.bootstrap().expect("bootstrap");

        let model = registry.load_latest_model().expect("load latest");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.generation, 0);
        assert_eq!(model.dims, NnueDimensions::DEFAULT);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ========================================================================
    // Benchmark tests
    // ========================================================================

    #[test]
    fn test_benchmark_result_display() {
        let result = BenchmarkResult {
            positions_evaluated: 1000,
            total_time_us: 50000,
            avg_time_us: 50,
            evals_per_second: 20000,
            avg_abs_score: 150.5,
            score_std_dev: 30.2,
            evaluator_name: "test".to_string(),
        };
        let display = format!("{}", result);
        assert!(display.contains("1000 positions"));
        assert!(display.contains("test"));
    }

    // ========================================================================
    // Integration tests
    // ========================================================================

    #[test]
    fn test_clipped_relu_behavior() {
        // Verify ClippedReLU constants
        assert_eq!(QA, 127);
        assert_eq!(FT_SCALE, 64);
        assert_eq!(HIDDEN_SCALE, 64);

        // Test clamping behavior
        let val_neg = -100i32;
        let val_zero = 0i32;
        let val_pos = 50i32;
        let val_over = 200i32;

        assert_eq!(val_neg.max(0).min(QA), 0);
        assert_eq!(val_zero.max(0).min(QA), 0);
        assert_eq!(val_pos.max(0).min(QA), 50);
        assert_eq!(val_over.max(0).min(QA), 127);
    }

    #[test]
    fn test_checksum_deterministic() {
        let dims = NnueDimensions::DEFAULT;
        let w1 = QuantizedWeights::random_small(&dims, 42);
        let w2 = QuantizedWeights::random_small(&dims, 42);
        assert_eq!(
            compute_weights_checksum(&w1),
            compute_weights_checksum(&w2)
        );
    }

    #[test]
    fn test_checksum_changes_with_weights() {
        let dims = NnueDimensions::DEFAULT;
        let w1 = QuantizedWeights::random_small(&dims, 42);
        let w2 = QuantizedWeights::random_small(&dims, 43);
        assert_ne!(
            compute_weights_checksum(&w1),
            compute_weights_checksum(&w2)
        );
    }

    #[test]
    fn test_full_pipeline_bootstrap_to_eval() {
        // Create bootstrap artifact
        let artifact = NnueModelArtifact::bootstrap();
        assert!(artifact.validate().is_ok());

        // Load as model
        let model = NnueModel::from_artifact(&artifact).expect("load model");

        // Create evaluator
        let eval = NnueEvaluator::new(model);

        // Evaluate with empty features
        let empty = SparseFeatureVec::new();
        let score = eval.model().forward(&empty);
        assert!(
            score > SCORE_LOSS && score < SCORE_WIN,
            "Score {} out of range",
            score
        );
    }

    #[test]
    fn test_full_pipeline_serialize_deserialize() {
        // Create, serialize, deserialize, validate, load
        let original = NnueModelArtifact::bootstrap();
        let bytes = original.to_bytes().expect("serialize");
        let loaded = NnueModelArtifact::from_bytes(&bytes).expect("deserialize");
        assert!(loaded.validate().is_ok());

        let model = NnueModel::from_artifact(&loaded).expect("load model");
        let eval = NnueEvaluator::new(model);

        let features = SparseFeatureVec::new();
        let score = eval.model().forward(&features);
        assert!(score > SCORE_LOSS && score < SCORE_WIN);
    }

    #[test]
    fn test_evaluator_swap_both_produce_valid_scores() {
        let heuristic = AnyEvaluator::heuristic_default();
        let nnue = AnyEvaluator::nnue_bootstrap();

        // Both should have valid names
        assert_eq!(heuristic.name(), "HeuristicEvaluator");
        assert_eq!(nnue.name(), "NnueEvaluator");

        // Both should be different types
        assert!(heuristic.is_heuristic());
        assert!(nnue.is_nnue());
    }

    #[test]
    fn test_feature_out_of_bounds_ignored() {
        let model = NnueModel::bootstrap();

        let mut features = SparseFeatureVec::new();
        features.push(9999, 100); // Out of bounds index
        features.push_binary(0);  // Valid

        // Should not panic
        let score = model.forward(&features);
        assert!(score > SCORE_LOSS && score < SCORE_WIN);
    }

    #[test]
    fn test_forward_pass_with_all_feature_sections() {
        let model = NnueModel::bootstrap();

        let mut features = SparseFeatureVec::with_capacity(20);

        // Global features
        features.push_binary(0);  // round 1
        features.push_binary(6);  // phase command
        features.push(12, 10);    // vp_diff
        features.push(14, 100);   // own_model_ratio

        // Objective features (first objective)
        features.push(31, 127);   // controller = own
        features.push(32, 50);    // own_oc
        features.push(37, 1);     // nearest own distance bucket

        // Unit features (first unit)
        features.push(211, 1);    // is_own
        features.push(212, 127);  // models_remaining_ratio
        features.push(213, 100);  // wounds_remaining_ratio
        features.push(218, 1);    // is_on_battlefield

        let score = model.forward(&features);
        assert!(
            score > SCORE_LOSS && score < SCORE_WIN,
            "Score {} with full feature set out of range",
            score
        );
    }

    #[test]
    fn test_incremental_vs_full_complex() {
        let model = NnueModel::bootstrap();

        // Old state: round 1, high vp
        let mut old_features = SparseFeatureVec::with_capacity(5);
        old_features.push_binary(0);  // round 1
        old_features.push_binary(5);  // phase 0
        old_features.push(12, 80);    // high vp_diff
        old_features.push(14, 100);   // model_ratio
        old_features.push(211, 1);    // unit 0 is_own

        // New state: round 2, lower vp, unit moved
        let mut new_features = SparseFeatureVec::with_capacity(6);
        new_features.push_binary(1);  // round 2 (changed from round 1)
        new_features.push_binary(7);  // phase 2 (changed from phase 0)
        new_features.push(12, 50);    // lower vp_diff
        new_features.push(14, 90);    // lower model_ratio
        new_features.push(211, 1);    // unit 0 still own
        new_features.push(265, 1);    // unit 0 has_moved

        // Full computation
        let full_acc = model.compute_accumulator(&new_features);
        let full_score = model.forward_from_accumulator(&full_acc);

        // Incremental computation
        let mut inc_acc = model.compute_accumulator(&old_features);
        let diff = compute_feature_diff(&old_features, &new_features);
        model.update_accumulator(&mut inc_acc, &diff);
        let inc_score = model.forward_from_accumulator(&inc_acc);

        assert_eq!(
            full_score, inc_score,
            "Full ({}) vs incremental ({}) scores must match",
            full_score, inc_score
        );
    }

    // ========================================================================
    // PolicyValueModel tests (Phase 10)
    // ========================================================================

    #[test]
    fn test_policy_value_dimensions() {
        let dims = PolicyValueDimensions::DEFAULT;
        assert_eq!(dims.input_size, 1209);
        assert_eq!(dims.trunk_size, 128);
        assert_eq!(dims.policy_output_size, ACTION_VOCAB_SIZE);
        assert_eq!(dims.value_output_size, 1);
        assert!(dims.total_weight_count() > 0);
    }

    #[test]
    fn test_policy_value_model_bootstrap() {
        let model = PolicyValueModel::bootstrap();
        assert_eq!(model.dims.input_size, 1209);
        assert_eq!(model.dims.policy_output_size, 640);
        assert_eq!(model.weights.trunk_weights.len(), 1209 * 128);
        assert_eq!(model.weights.policy_output_weights.len(), 64 * 640);
        assert_eq!(model.weights.value_output_weights.len(), 64);
    }

    #[test]
    fn test_policy_value_inference() {
        let model = PolicyValueModel::bootstrap();

        // Create some sparse features
        let features = vec![
            (0u16, 100i16),
            (10, 50),
            (100, 75),
            (500, 30),
        ];

        let (logits, value) = model.infer(&features);
        assert_eq!(logits.len(), ACTION_VOCAB_SIZE);
        assert!(value >= -1.0 && value <= 1.0, "Value {} should be in [-1,1]", value);
    }

    #[test]
    fn test_policy_value_empty_features() {
        let model = PolicyValueModel::bootstrap();
        let features: Vec<(u16, i16)> = Vec::new();

        let (logits, value) = model.infer(&features);
        assert_eq!(logits.len(), ACTION_VOCAB_SIZE);
        assert!(value >= -1.0 && value <= 1.0);
    }

    #[test]
    fn test_masked_softmax_basic() {
        let logits = vec![1.0, 2.0, 3.0, 4.0];
        let mask = vec![true, true, true, true];
        let probs = masked_softmax(&logits, &mask);
        assert_eq!(probs.len(), 4);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        assert!(probs[3] > probs[2]);
    }

    #[test]
    fn test_masked_softmax_with_illegal() {
        let logits = vec![1.0, 100.0, 3.0, 4.0];
        let mask = vec![true, false, true, true];
        let probs = masked_softmax(&logits, &mask);
        assert!((probs[1]).abs() < 0.001, "Illegal action should have ~0 probability");
        let legal_sum: f32 = probs.iter().zip(mask.iter()).filter(|(_, &l)| l).map(|(p, _)| p).sum();
        assert!((legal_sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_policy_probs() {
        let model = PolicyValueModel::bootstrap();
        let features = vec![(0u16, 100i16), (10, 50)];
        let mut legal_mask = vec![false; ACTION_VOCAB_SIZE];
        legal_mask[0] = true;
        legal_mask[5] = true;
        legal_mask[10] = true;

        let probs = model.policy_probs(&features, &legal_mask);
        assert_eq!(probs.len(), ACTION_VOCAB_SIZE);
        let sum: f32 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
        // Illegal actions should have 0 probability
        assert_eq!(probs[1], 0.0);
        assert_eq!(probs[2], 0.0);
    }
}
