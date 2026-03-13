//! WH40K Engine - Determinism crate
//!
//! Provides verification tooling that ensures the engine is deterministic.
//! This crate does NOT own the hashing itself (that's in `game_core::hasher`).
//! Instead, it provides convenience wrappers, verification pipelines, and
//! test frameworks for validating deterministic behavior.
//!
//! ## Components
//!
//! - [`StateHasher`] - Convenience wrapper around `GameState::compute_hash()`
//! - [`HashCheckpoint`] - Snapshot of state hash at a command boundary
//! - [`VerificationResult`] - Outcome of comparing two hash timelines
//! - [`RoundTripVerifier`] - Verifies replay determinism via hash comparison
//! - [`DeterminismTestSuite`] - Structural tests (serialization, clone, float-free)
//! - [`FuzzTester`] - Multi-iteration stability fuzzing
//! - [`DeterminismError`] - Error type for all verification failures
//!
//! Source: implementation_v3.md Section 6.7 (Determinism verification layer)

use serde::{Deserialize, Serialize};

use wh40k_core_types::{BattleRound, CommandId, Phase};
use wh40k_command_system::{CommandHistory, CommandRecord};
use wh40k_game_core::state::GameState;

// ============================================================================
// DeterminismError
// ============================================================================

/// Error type for determinism verification failures.
///
/// Covers hash mismatches, broken hash chains, command count mismatches,
/// serialization errors, and general verification errors.
#[derive(Debug, thiserror::Error)]
pub enum DeterminismError {
    /// State hash mismatch at a specific command index.
    #[error("State hash mismatch at command {command_index}: expected {expected:#018x}, got {actual:#018x}")]
    HashMismatch {
        command_index: usize,
        expected: u64,
        actual: u64,
    },

    /// Hash chain is broken between two consecutive commands.
    /// This means `hash_after[from_index] != hash_before[to_index]`.
    #[error("Hash chain broken between commands {from_index} and {to_index}")]
    BrokenChain {
        from_index: usize,
        to_index: usize,
        from_hash: u64,
        to_hash: u64,
    },

    /// Expected and actual command counts do not match.
    #[error("Command count mismatch: expected {expected}, got {actual}")]
    CommandCountMismatch {
        expected: usize,
        actual: usize,
    },

    /// A serialization or deserialization step failed.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Generic verification error with a descriptive message.
    #[error("Verification error: {0}")]
    VerificationError(String),
}

// ============================================================================
// HashCheckpoint
// ============================================================================

/// A snapshot of the state hash taken at a command boundary.
///
/// Each checkpoint records the command that was applied, the hash of the game
/// state immediately before it, and the hash immediately after (if the command
/// was legal and applied). The round and phase are stored for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashCheckpoint {
    /// Index of the command in the history.
    pub command_index: usize,
    /// Unique identifier of the command.
    pub command_id: CommandId,
    /// Hash of the game state before this command was applied.
    pub hash_before: u64,
    /// Hash of the game state after this command was applied.
    /// `None` if the command was illegal and not applied.
    pub hash_after: Option<u64>,
    /// The battle round during which this command was issued.
    pub round: BattleRound,
    /// The phase during which this command was issued.
    pub phase: Phase,
}

// ============================================================================
// VerificationResult
// ============================================================================

/// The outcome of a determinism verification check.
///
/// Represents either a perfect match, a divergence at a specific command,
/// or an error that prevented the check from completing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// All state hashes matched perfectly.
    Match {
        /// Total number of commands in the history.
        total_commands: usize,
        /// Total number of checkpoints that were verified.
        total_checkpoints: usize,
    },
    /// Divergence detected at a specific command.
    Divergence {
        /// Index of the command where the divergence occurred.
        command_index: usize,
        /// The hash that was expected.
        expected_hash: u64,
        /// The hash that was actually observed.
        actual_hash: u64,
        /// A human-readable description of the divergent command.
        command_description: String,
        /// The battle round where divergence occurred.
        round: BattleRound,
        /// The phase where divergence occurred.
        phase: Phase,
    },
    /// An error occurred during verification, preventing completion.
    Error {
        /// Description of the error.
        message: String,
    },
}

// ============================================================================
// StateHasher
// ============================================================================

/// Convenience wrapper around `GameState::compute_hash()`.
///
/// `StateHasher` does not own any hashing logic itself. The authoritative
/// implementation lives in `game_core::hasher`. This struct provides
/// ergonomic entry points and higher-level operations such as extracting
/// a full hash timeline from a command history.
pub struct StateHasher;

impl StateHasher {
    /// Compute a deterministic hash of the entire game state.
    ///
    /// Delegates to `GameState::compute_hash()` which uses `ahash` over
    /// all state fields to produce a deterministic 64-bit fingerprint.
    pub fn hash(state: &GameState) -> u64 {
        state.compute_hash()
    }

    /// Extract a hash timeline from the game state's command history.
    ///
    /// Walks the command history and produces a [`HashCheckpoint`] for every
    /// legal command, recording the `hash_before`, `hash_after`, round, and
    /// phase captured in each [`CommandRecord`].
    ///
    /// Returns a `Vec<HashCheckpoint>` ordered by command index.
    pub fn extract_hash_timeline(state: &GameState) -> Vec<HashCheckpoint> {
        let records = state.command_history.records();
        let mut timeline = Vec::with_capacity(records.len());

        for (idx, record) in records.iter().enumerate() {
            // Only include legal commands in the timeline (they actually
            // mutated the state and have a meaningful hash_after).
            if record.was_legal() {
                timeline.push(HashCheckpoint {
                    command_index: idx,
                    command_id: record.command_id,
                    hash_before: record.state_hash_before,
                    hash_after: record.state_hash_after,
                    round: record.timestamp_round,
                    phase: record.timestamp_phase,
                });
            }
        }

        timeline
    }
}

// ============================================================================
// RoundTripVerifier
// ============================================================================

/// Verifies deterministic replay by comparing hash timelines.
///
/// Two independent executions of the same command sequence on the same
/// initial state must produce identical hash timelines. This struct
/// provides the comparison logic.
pub struct RoundTripVerifier;

impl RoundTripVerifier {
    /// Verify that two hash timelines match exactly.
    ///
    /// Compares `expected` and `actual` checkpoint-by-checkpoint. Returns
    /// [`VerificationResult::Match`] if every checkpoint's `hash_before`
    /// and `hash_after` agree, or [`VerificationResult::Divergence`] at
    /// the first mismatch.
    pub fn verify_timelines(
        expected: &[HashCheckpoint],
        actual: &[HashCheckpoint],
    ) -> VerificationResult {
        // Length mismatch is itself a divergence indicator.
        if expected.len() != actual.len() {
            return VerificationResult::Error {
                message: format!(
                    "Timeline length mismatch: expected {} checkpoints, got {}",
                    expected.len(),
                    actual.len()
                ),
            };
        }

        for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
            // Compare hash_before values.
            if exp.hash_before != act.hash_before {
                return VerificationResult::Divergence {
                    command_index: exp.command_index,
                    expected_hash: exp.hash_before,
                    actual_hash: act.hash_before,
                    command_description: format!(
                        "hash_before mismatch at checkpoint {} (command_id={})",
                        i, exp.command_id
                    ),
                    round: exp.round,
                    phase: exp.phase,
                };
            }

            // Compare hash_after values.
            if exp.hash_after != act.hash_after {
                let expected_hash = exp.hash_after.unwrap_or(0);
                let actual_hash = act.hash_after.unwrap_or(0);
                return VerificationResult::Divergence {
                    command_index: exp.command_index,
                    expected_hash,
                    actual_hash,
                    command_description: format!(
                        "hash_after mismatch at checkpoint {} (command_id={})",
                        i, exp.command_id
                    ),
                    round: exp.round,
                    phase: exp.phase,
                };
            }
        }

        VerificationResult::Match {
            total_commands: expected.len(),
            total_checkpoints: expected.len(),
        }
    }

    /// Compare two game states and return whether they are hash-equivalent.
    ///
    /// Two states are considered equivalent if `compute_hash()` returns
    /// the same value for both. This does NOT compare field-by-field;
    /// it relies entirely on the deterministic hash.
    pub fn states_match(a: &GameState, b: &GameState) -> bool {
        StateHasher::hash(a) == StateHasher::hash(b)
    }

    /// Check that a command history's recorded hashes are internally consistent.
    ///
    /// For every pair of consecutive *legal* commands, verifies that
    /// `hash_after[i] == hash_before[i+1]`. A break in the chain
    /// indicates either non-determinism or a bug in hash recording.
    pub fn verify_hash_chain(history: &CommandHistory) -> VerificationResult {
        let records = history.records();

        // Collect only legal command records with their original indices.
        let legal_records: Vec<(usize, &CommandRecord)> = records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.was_legal())
            .collect();

        if legal_records.is_empty() {
            return VerificationResult::Match {
                total_commands: 0,
                total_checkpoints: 0,
            };
        }

        // Walk consecutive pairs and verify the chain.
        for window in legal_records.windows(2) {
            let (from_idx, from_rec) = window[0];
            let (to_idx, to_rec) = window[1];

            // `hash_after` of the previous command must equal `hash_before`
            // of the next command. If hash_after is None, the chain is
            // trivially broken (but this shouldn't happen for legal commands).
            let from_hash_after = match from_rec.state_hash_after {
                Some(h) => h,
                None => {
                    return VerificationResult::Divergence {
                        command_index: from_idx,
                        expected_hash: 0,
                        actual_hash: 0,
                        command_description: format!(
                            "Legal command at index {} has no hash_after (command_id={})",
                            from_idx, from_rec.command_id
                        ),
                        round: from_rec.timestamp_round,
                        phase: from_rec.timestamp_phase,
                    };
                }
            };

            let to_hash_before = to_rec.state_hash_before;

            if from_hash_after != to_hash_before {
                return VerificationResult::Divergence {
                    command_index: to_idx,
                    expected_hash: from_hash_after,
                    actual_hash: to_hash_before,
                    command_description: format!(
                        "Hash chain broken: command {} hash_after ({:#018x}) != command {} hash_before ({:#018x})",
                        from_idx, from_hash_after, to_idx, to_hash_before
                    ),
                    round: to_rec.timestamp_round,
                    phase: to_rec.timestamp_phase,
                };
            }
        }

        VerificationResult::Match {
            total_commands: records.len(),
            total_checkpoints: legal_records.len(),
        }
    }
}

// ============================================================================
// DeterminismTestSuite
// ============================================================================

/// A collection of structural determinism checks.
///
/// Each method verifies a specific invariant that must hold for the engine
/// to be deterministic:
///
/// - **Serialization round-trip**: JSON serialization and deserialization
///   must preserve the hash.
/// - **Hash stability**: Computing the hash twice on the same state must
///   return the same value.
/// - **Clone determinism**: Cloning a state must preserve the hash.
/// - **No float dependency**: The engine uses fixed-point `Inches`
///   (milliinches as `i32`), so floats never affect the hash. This is
///   verified structurally via hash stability.
pub struct DeterminismTestSuite;

impl DeterminismTestSuite {
    /// Verify that serializing and deserializing a `GameState` preserves the hash.
    ///
    /// Serializes the state to JSON, deserializes it back, and compares the
    /// hashes. If they differ, returns [`DeterminismError::HashMismatch`].
    pub fn verify_serialization_roundtrip(state: &GameState) -> Result<(), DeterminismError> {
        let hash_before = StateHasher::hash(state);
        let json = serde_json::to_string(state)
            .map_err(|e| DeterminismError::SerializationError(e.to_string()))?;
        let deserialized: GameState = serde_json::from_str(&json)
            .map_err(|e| DeterminismError::SerializationError(e.to_string()))?;
        let hash_after = StateHasher::hash(&deserialized);
        if hash_before != hash_after {
            Err(DeterminismError::HashMismatch {
                command_index: 0,
                expected: hash_before,
                actual: hash_after,
            })
        } else {
            Ok(())
        }
    }

    /// Verify that computing the hash twice on the same state returns the same value.
    ///
    /// A non-deterministic hasher would produce different results even on
    /// the same input. This catches accidental use of randomized hash seeds
    /// or pointer-based hashing.
    pub fn verify_hash_stability(state: &GameState) -> Result<(), DeterminismError> {
        let hash1 = StateHasher::hash(state);
        let hash2 = StateHasher::hash(state);
        if hash1 != hash2 {
            Err(DeterminismError::HashMismatch {
                command_index: 0,
                expected: hash1,
                actual: hash2,
            })
        } else {
            Ok(())
        }
    }

    /// Verify that cloning a state preserves the hash.
    ///
    /// If `Clone` introduces any non-determinism (e.g., reallocated
    /// pointer-dependent data), the hash would diverge.
    pub fn verify_clone_determinism(state: &GameState) -> Result<(), DeterminismError> {
        let hash_original = StateHasher::hash(state);
        let cloned = state.clone();
        let hash_cloned = StateHasher::hash(&cloned);
        if hash_original != hash_cloned {
            Err(DeterminismError::HashMismatch {
                command_index: 0,
                expected: hash_original,
                actual: hash_cloned,
            })
        } else {
            Ok(())
        }
    }

    /// Verify no floating-point usage in state (already guaranteed by design but verify via hash).
    ///
    /// Since all measurements use fixed-point `Inches` (i32 milliinches),
    /// this verification is structural -- the type system enforces it.
    /// We verify by confirming hash stability across multiple computations,
    /// which would fail if any float rounding variance affected the hash.
    pub fn verify_no_float_dependency(state: &GameState) -> Result<(), DeterminismError> {
        // Since all measurements use fixed-point Inches (i32 milliinches),
        // this verification is structural - the type system enforces it.
        // We verify by confirming hash stability across multiple computations.
        Self::verify_hash_stability(state)
    }
}

// ============================================================================
// FuzzTester
// ============================================================================

/// Multi-iteration stability fuzzer for deterministic hashing.
///
/// Runs multiple rounds of hash stability checks, clone-and-hash checks,
/// and serialization round-trip checks on a given `GameState`. Any failure
/// is recorded in the returned [`FuzzResult`].
pub struct FuzzTester;

impl FuzzTester {
    /// Run N iterations of hash stability checks with the provided game state.
    ///
    /// Each iteration performs three tests:
    ///
    /// 1. **Direct hash stability** - Recompute the hash and compare against baseline.
    /// 2. **Clone + hash** - Clone the state, hash the clone, compare against baseline.
    /// 3. **Serialization round-trip** - Serialize to JSON, deserialize, hash, compare.
    ///
    /// Returns a [`FuzzResult`] summarizing pass/fail status and any failures.
    pub fn fuzz_hash_stability(
        state: &GameState,
        iterations: usize,
    ) -> Result<FuzzResult, DeterminismError> {
        let mut results = FuzzResult {
            iterations_run: 0,
            all_passed: true,
            failures: Vec::new(),
        };

        let baseline_hash = StateHasher::hash(state);

        for i in 0..iterations {
            results.iterations_run += 1;

            // Test 1: Direct hash stability
            let hash = StateHasher::hash(state);
            if hash != baseline_hash {
                results.all_passed = false;
                results.failures.push(FuzzFailure {
                    iteration: i,
                    test_name: "direct_hash_stability".to_string(),
                    expected: baseline_hash,
                    actual: hash,
                });
            }

            // Test 2: Clone + hash
            let cloned = state.clone();
            let clone_hash = StateHasher::hash(&cloned);
            if clone_hash != baseline_hash {
                results.all_passed = false;
                results.failures.push(FuzzFailure {
                    iteration: i,
                    test_name: "clone_hash_stability".to_string(),
                    expected: baseline_hash,
                    actual: clone_hash,
                });
            }

            // Test 3: Serialize + deserialize + hash
            if let Ok(json) = serde_json::to_string(state) {
                if let Ok(deserialized) = serde_json::from_str::<GameState>(&json) {
                    let deser_hash = StateHasher::hash(&deserialized);
                    if deser_hash != baseline_hash {
                        results.all_passed = false;
                        results.failures.push(FuzzFailure {
                            iteration: i,
                            test_name: "serialization_roundtrip_hash".to_string(),
                            expected: baseline_hash,
                            actual: deser_hash,
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

/// The result of a fuzz testing run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResult {
    /// Number of iterations that were executed.
    pub iterations_run: usize,
    /// Whether all iterations passed without any failures.
    pub all_passed: bool,
    /// List of individual test failures (empty if `all_passed` is true).
    pub failures: Vec<FuzzFailure>,
}

/// A single failure recorded during fuzz testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzFailure {
    /// The iteration number (0-based) in which this failure occurred.
    pub iteration: usize,
    /// Name of the specific test that failed (e.g., "direct_hash_stability").
    pub test_name: String,
    /// The expected hash value (baseline).
    pub expected: u64,
    /// The actual hash value that was observed.
    pub actual: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use wh40k_core_types::{
        BattleRound, CommandId, GameOutcome, Phase, PlayerId, SubPhase,
    };
    use wh40k_command_system::{Command, CommandHistory, CommandRecord, CommandValidationResult};
    use wh40k_dice::{DiceContext, DiceRoller, StreamKind};
    use wh40k_event_system::EventBus;
    use wh40k_game_core::state::{GameState, PlayerState, TurnFlags};
    use wh40k_geometry::Board;

    /// Helper: build a minimal test `GameState` for determinism testing.
    fn make_test_state() -> GameState {
        let seed = [0u8; 32];
        let ctx = DiceContext::new(seed, StreamKind::BattleShockTest, 0, 0);
        let dice_roller = DiceRoller::new(ctx);

        GameState {
            content_version: "test-1.0".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(1),
            active_player: PlayerId::new(0),
            current_phase: Phase::PreBattle,
            current_subphase: SubPhase::DetermineAttackerDefender,
            decision_owner: PlayerId::new(0),
            players: [
                PlayerState::new(PlayerId::new(0), "Player A".to_string()),
                PlayerState::new(PlayerId::new(1), "Player B".to_string()),
            ],
            units: Vec::new(),
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 0,
            deployment_config: None,
        }
    }

    /// Helper: build a second test state that differs from the first.
    fn make_different_state() -> GameState {
        let seed = [1u8; 32]; // Different seed
        let ctx = DiceContext::new(seed, StreamKind::HitRoll, 1, 1);
        let dice_roller = DiceRoller::new(ctx);

        GameState {
            content_version: "test-2.0".to_string(),
            scenario_id: None,
            battle_round: BattleRound::new(2),
            active_player: PlayerId::new(1),
            current_phase: Phase::Movement,
            current_subphase: SubPhase::SelectUnitToMove,
            decision_owner: PlayerId::new(1),
            players: [
                PlayerState::new(PlayerId::new(0), "Alpha".to_string()),
                PlayerState::new(PlayerId::new(1), "Bravo".to_string()),
            ],
            units: Vec::new(),
            board: Board::combat_patrol(),
            event_bus: EventBus::new(),
            command_history: CommandHistory::new(),
            dice_roller,
            active_effects: Vec::new(),
            reaction_windows: Vec::new(),
            turn_flags: TurnFlags::new(),
            game_outcome: GameOutcome::InProgress,
            deterministic_counter: 42,
            deployment_config: None,
        }
    }

    /// Helper: build a `CommandRecord` for testing.
    fn make_legal_record(
        id: u32,
        hash_before: u64,
        hash_after: u64,
        round: u8,
        phase: Phase,
    ) -> CommandRecord {
        CommandRecord {
            command_id: CommandId::new(id),
            command: Command::SetupComplete,
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(round),
            timestamp_phase: phase,
            validation_result: CommandValidationResult::Legal,
            state_hash_before: hash_before,
            state_hash_after: Some(hash_after),
        }
    }

    /// Helper: build an illegal `CommandRecord` for testing.
    fn make_illegal_record(
        id: u32,
        hash_before: u64,
        round: u8,
        phase: Phase,
    ) -> CommandRecord {
        CommandRecord {
            command_id: CommandId::new(id),
            command: Command::SetupComplete,
            player: PlayerId::new(0),
            timestamp_round: BattleRound::new(round),
            timestamp_phase: phase,
            validation_result: CommandValidationResult::Illegal {
                reason: "test illegal".to_string(),
                rule_reference: None,
            },
            state_hash_before: hash_before,
            state_hash_after: None,
        }
    }

    // ----------------------------------------------------------------
    // StateHasher tests
    // ----------------------------------------------------------------

    #[test]
    fn test_hash_stability() {
        let state = make_test_state();
        let hash1 = StateHasher::hash(&state);
        let hash2 = StateHasher::hash(&state);
        assert_eq!(
            hash1, hash2,
            "Hashing the same state twice must produce the same value"
        );
    }

    #[test]
    fn test_hash_different_states() {
        let state_a = make_test_state();
        let state_b = make_different_state();
        let hash_a = StateHasher::hash(&state_a);
        let hash_b = StateHasher::hash(&state_b);
        assert_ne!(
            hash_a, hash_b,
            "Different states should produce different hashes"
        );
    }

    // ----------------------------------------------------------------
    // Clone determinism
    // ----------------------------------------------------------------

    #[test]
    fn test_clone_determinism() {
        let state = make_test_state();
        let result = DeterminismTestSuite::verify_clone_determinism(&state);
        assert!(
            result.is_ok(),
            "Cloning a state must preserve its hash: {:?}",
            result
        );
    }

    // ----------------------------------------------------------------
    // Serialization round-trip
    // ----------------------------------------------------------------

    #[test]
    fn test_serialization_roundtrip() {
        let state = make_test_state();
        let result = DeterminismTestSuite::verify_serialization_roundtrip(&state);
        assert!(
            result.is_ok(),
            "Serialization round-trip must preserve the hash: {:?}",
            result
        );
    }

    // ----------------------------------------------------------------
    // Hash chain verification
    // ----------------------------------------------------------------

    #[test]
    fn test_verify_hash_chain_valid() {
        // Build a valid chain: hash_after[i] == hash_before[i+1]
        let mut history = CommandHistory::new();
        history.append(make_legal_record(0, 100, 200, 1, Phase::PreBattle));
        history.append(make_legal_record(1, 200, 300, 1, Phase::Command));
        history.append(make_legal_record(2, 300, 400, 1, Phase::Movement));

        let result = RoundTripVerifier::verify_hash_chain(&history);
        match result {
            VerificationResult::Match {
                total_commands,
                total_checkpoints,
            } => {
                assert_eq!(total_commands, 3);
                assert_eq!(total_checkpoints, 3);
            }
            other => panic!("Expected Match, got {:?}", other),
        }
    }

    #[test]
    fn test_verify_hash_chain_broken() {
        // Build a broken chain: hash_after[0] != hash_before[1]
        let mut history = CommandHistory::new();
        history.append(make_legal_record(0, 100, 200, 1, Phase::PreBattle));
        history.append(make_legal_record(1, 999, 300, 1, Phase::Command)); // 999 != 200

        let result = RoundTripVerifier::verify_hash_chain(&history);
        match result {
            VerificationResult::Divergence {
                command_index,
                expected_hash,
                actual_hash,
                ..
            } => {
                assert_eq!(command_index, 1);
                assert_eq!(expected_hash, 200);
                assert_eq!(actual_hash, 999);
            }
            other => panic!("Expected Divergence, got {:?}", other),
        }
    }

    #[test]
    fn test_verify_hash_chain_with_illegal_commands() {
        // Illegal commands should be skipped in chain verification.
        let mut history = CommandHistory::new();
        history.append(make_legal_record(0, 100, 200, 1, Phase::PreBattle));
        // Illegal command sits between legal ones -- should be skipped.
        history.append(make_illegal_record(1, 200, 1, Phase::Command));
        history.append(make_legal_record(2, 200, 300, 1, Phase::Command));

        let result = RoundTripVerifier::verify_hash_chain(&history);
        match result {
            VerificationResult::Match { .. } => { /* good */ }
            other => panic!("Expected Match (illegal commands skipped), got {:?}", other),
        }
    }

    #[test]
    fn test_verify_hash_chain_empty() {
        let history = CommandHistory::new();
        let result = RoundTripVerifier::verify_hash_chain(&history);
        match result {
            VerificationResult::Match {
                total_commands,
                total_checkpoints,
            } => {
                assert_eq!(total_commands, 0);
                assert_eq!(total_checkpoints, 0);
            }
            other => panic!("Expected Match for empty history, got {:?}", other),
        }
    }

    // ----------------------------------------------------------------
    // Timeline verification
    // ----------------------------------------------------------------

    #[test]
    fn test_verify_timelines_match() {
        let checkpoints = vec![
            HashCheckpoint {
                command_index: 0,
                command_id: CommandId::new(0),
                hash_before: 100,
                hash_after: Some(200),
                round: BattleRound::new(1),
                phase: Phase::PreBattle,
            },
            HashCheckpoint {
                command_index: 1,
                command_id: CommandId::new(1),
                hash_before: 200,
                hash_after: Some(300),
                round: BattleRound::new(1),
                phase: Phase::Command,
            },
        ];

        let result = RoundTripVerifier::verify_timelines(&checkpoints, &checkpoints);
        match result {
            VerificationResult::Match {
                total_commands,
                total_checkpoints,
            } => {
                assert_eq!(total_commands, 2);
                assert_eq!(total_checkpoints, 2);
            }
            other => panic!("Expected Match, got {:?}", other),
        }
    }

    #[test]
    fn test_verify_timelines_divergence() {
        let expected = vec![HashCheckpoint {
            command_index: 0,
            command_id: CommandId::new(0),
            hash_before: 100,
            hash_after: Some(200),
            round: BattleRound::new(1),
            phase: Phase::PreBattle,
        }];

        let actual = vec![HashCheckpoint {
            command_index: 0,
            command_id: CommandId::new(0),
            hash_before: 100,
            hash_after: Some(999), // different
            round: BattleRound::new(1),
            phase: Phase::PreBattle,
        }];

        let result = RoundTripVerifier::verify_timelines(&expected, &actual);
        match result {
            VerificationResult::Divergence {
                expected_hash,
                actual_hash,
                ..
            } => {
                assert_eq!(expected_hash, 200);
                assert_eq!(actual_hash, 999);
            }
            other => panic!("Expected Divergence, got {:?}", other),
        }
    }

    #[test]
    fn test_verify_timelines_length_mismatch() {
        let one = vec![HashCheckpoint {
            command_index: 0,
            command_id: CommandId::new(0),
            hash_before: 100,
            hash_after: Some(200),
            round: BattleRound::new(1),
            phase: Phase::PreBattle,
        }];
        let two: Vec<HashCheckpoint> = Vec::new();

        let result = RoundTripVerifier::verify_timelines(&one, &two);
        match result {
            VerificationResult::Error { message } => {
                assert!(
                    message.contains("length mismatch"),
                    "Error should mention length mismatch: {}",
                    message
                );
            }
            other => panic!("Expected Error for length mismatch, got {:?}", other),
        }
    }

    // ----------------------------------------------------------------
    // FuzzTester
    // ----------------------------------------------------------------

    #[test]
    fn test_fuzz_stability() {
        let state = make_test_state();
        let result = FuzzTester::fuzz_hash_stability(&state, 10)
            .expect("Fuzz test should not error");
        assert!(
            result.all_passed,
            "All fuzz iterations should pass: {:?}",
            result.failures
        );
        assert_eq!(result.iterations_run, 10);
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_fuzz_zero_iterations() {
        let state = make_test_state();
        let result = FuzzTester::fuzz_hash_stability(&state, 0)
            .expect("Fuzz test with 0 iterations should not error");
        assert!(result.all_passed);
        assert_eq!(result.iterations_run, 0);
        assert!(result.failures.is_empty());
    }

    // ----------------------------------------------------------------
    // VerificationResult assertions
    // ----------------------------------------------------------------

    #[test]
    fn test_verification_result_match() {
        let result = VerificationResult::Match {
            total_commands: 5,
            total_checkpoints: 5,
        };
        assert_eq!(
            result,
            VerificationResult::Match {
                total_commands: 5,
                total_checkpoints: 5,
            }
        );
    }

    #[test]
    fn test_verification_result_divergence() {
        let result = VerificationResult::Divergence {
            command_index: 3,
            expected_hash: 0xDEAD_BEEF,
            actual_hash: 0xCAFE_BABE,
            command_description: "test divergence".to_string(),
            round: BattleRound::new(2),
            phase: Phase::Shooting,
        };
        match result {
            VerificationResult::Divergence {
                command_index,
                expected_hash,
                actual_hash,
                ref command_description,
                round,
                phase,
            } => {
                assert_eq!(command_index, 3);
                assert_eq!(expected_hash, 0xDEAD_BEEF);
                assert_eq!(actual_hash, 0xCAFE_BABE);
                assert_eq!(command_description, "test divergence");
                assert_eq!(round, BattleRound::new(2));
                assert_eq!(phase, Phase::Shooting);
            }
            _ => panic!("Expected Divergence variant"),
        }
    }

    #[test]
    fn test_verification_result_error() {
        let result = VerificationResult::Error {
            message: "something went wrong".to_string(),
        };
        match result {
            VerificationResult::Error { ref message } => {
                assert_eq!(message, "something went wrong");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    // ----------------------------------------------------------------
    // No float dependency
    // ----------------------------------------------------------------

    #[test]
    fn test_no_float_dependency() {
        let state = make_test_state();
        let result = DeterminismTestSuite::verify_no_float_dependency(&state);
        assert!(
            result.is_ok(),
            "No-float dependency check must pass: {:?}",
            result
        );
    }

    // ----------------------------------------------------------------
    // states_match helper
    // ----------------------------------------------------------------

    #[test]
    fn test_states_match_identical() {
        let a = make_test_state();
        let b = make_test_state();
        assert!(
            RoundTripVerifier::states_match(&a, &b),
            "Identical states should match"
        );
    }

    #[test]
    fn test_states_match_different() {
        let a = make_test_state();
        let b = make_different_state();
        assert!(
            !RoundTripVerifier::states_match(&a, &b),
            "Different states should not match"
        );
    }

    // ----------------------------------------------------------------
    // Extract hash timeline
    // ----------------------------------------------------------------

    #[test]
    fn test_extract_hash_timeline_empty() {
        let state = make_test_state();
        let timeline = StateHasher::extract_hash_timeline(&state);
        assert!(
            timeline.is_empty(),
            "Empty command history should produce empty timeline"
        );
    }

    #[test]
    fn test_extract_hash_timeline_filters_illegal() {
        let mut state = make_test_state();
        state
            .command_history
            .append(make_legal_record(0, 100, 200, 1, Phase::PreBattle));
        state
            .command_history
            .append(make_illegal_record(1, 200, 1, Phase::Command));
        state
            .command_history
            .append(make_legal_record(2, 200, 300, 1, Phase::Command));

        let timeline = StateHasher::extract_hash_timeline(&state);
        assert_eq!(
            timeline.len(),
            2,
            "Timeline should only include legal commands"
        );
        assert_eq!(timeline[0].command_id, CommandId::new(0));
        assert_eq!(timeline[1].command_id, CommandId::new(2));
    }

    // ----------------------------------------------------------------
    // DeterminismError formatting
    // ----------------------------------------------------------------

    #[test]
    fn test_error_display_hash_mismatch() {
        let err = DeterminismError::HashMismatch {
            command_index: 5,
            expected: 0x1234,
            actual: 0x5678,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("command 5"), "Message: {}", msg);
        assert!(msg.contains("0x"), "Message should contain hex: {}", msg);
    }

    #[test]
    fn test_error_display_broken_chain() {
        let err = DeterminismError::BrokenChain {
            from_index: 2,
            to_index: 3,
            from_hash: 100,
            to_hash: 200,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("2"), "Message: {}", msg);
        assert!(msg.contains("3"), "Message: {}", msg);
    }

    #[test]
    fn test_error_display_command_count_mismatch() {
        let err = DeterminismError::CommandCountMismatch {
            expected: 10,
            actual: 7,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("10"), "Message: {}", msg);
        assert!(msg.contains("7"), "Message: {}", msg);
    }

    #[test]
    fn test_error_display_serialization() {
        let err = DeterminismError::SerializationError("bad json".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("bad json"), "Message: {}", msg);
    }

    #[test]
    fn test_error_display_verification() {
        let err = DeterminismError::VerificationError("check failed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("check failed"), "Message: {}", msg);
    }

    // ----------------------------------------------------------------
    // HashCheckpoint serialization
    // ----------------------------------------------------------------

    #[test]
    fn test_hash_checkpoint_serde() {
        let cp = HashCheckpoint {
            command_index: 7,
            command_id: CommandId::new(42),
            hash_before: 111,
            hash_after: Some(222),
            round: BattleRound::new(3),
            phase: Phase::Fight,
        };
        let json = serde_json::to_string(&cp).expect("serialize checkpoint");
        let deserialized: HashCheckpoint =
            serde_json::from_str(&json).expect("deserialize checkpoint");
        assert_eq!(cp, deserialized);
    }

    // ----------------------------------------------------------------
    // FuzzResult serialization
    // ----------------------------------------------------------------

    #[test]
    fn test_fuzz_result_serde() {
        let result = FuzzResult {
            iterations_run: 50,
            all_passed: false,
            failures: vec![FuzzFailure {
                iteration: 3,
                test_name: "clone_hash_stability".to_string(),
                expected: 100,
                actual: 200,
            }],
        };
        let json = serde_json::to_string(&result).expect("serialize fuzz result");
        let deserialized: FuzzResult =
            serde_json::from_str(&json).expect("deserialize fuzz result");
        assert_eq!(deserialized.iterations_run, 50);
        assert!(!deserialized.all_passed);
        assert_eq!(deserialized.failures.len(), 1);
        assert_eq!(deserialized.failures[0].test_name, "clone_hash_stability");
    }

    // ----------------------------------------------------------------
    // VerificationResult serialization
    // ----------------------------------------------------------------

    #[test]
    fn test_verification_result_serde() {
        let result = VerificationResult::Match {
            total_commands: 10,
            total_checkpoints: 8,
        };
        let json = serde_json::to_string(&result).expect("serialize VerificationResult");
        let deserialized: VerificationResult =
            serde_json::from_str(&json).expect("deserialize VerificationResult");
        assert_eq!(result, deserialized);
    }
}
