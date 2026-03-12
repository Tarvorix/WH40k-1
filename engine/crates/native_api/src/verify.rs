//! CLI verify command - verify replay determinism.
//!
//! Loads a replay file and re-executes it through the engine,
//! verifying that every step produces identical state hashes.
//!
//! Source: implementation_v3.md Phase 10

use wh40k_game_core::{CommandValidator, CommandExecutor, ScenarioLoader};
use wh40k_replay::ReplayFile;

/// Configuration for replay verification.
pub struct VerifyConfig {
    pub replay_path: String,
    pub format: ReplayFormat,
    pub verbose: bool,
}

/// Supported replay file formats.
pub enum ReplayFormat {
    Json,
    Binary,
}

/// Result of verification.
#[derive(Debug)]
pub struct VerifyResult {
    pub frames_total: usize,
    pub frames_verified: usize,
    pub frames_failed: usize,
    pub hash_mismatches: Vec<HashMismatch>,
    pub passed: bool,
}

/// Details of a hash mismatch.
#[derive(Debug)]
pub struct HashMismatch {
    pub frame_index: usize,
    pub expected_hash: u64,
    pub actual_hash: u64,
    pub command: String,
}

/// Run replay verification.
pub fn run_verify(config: VerifyConfig) -> VerifyResult {
    println!("=== WH40K Replay Verification ===");
    println!("Replay: {}", config.replay_path);

    // Load replay file
    let replay_data = match std::fs::read_to_string(&config.replay_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to read replay file: {}", e);
            return VerifyResult {
                frames_total: 0,
                frames_verified: 0,
                frames_failed: 0,
                hash_mismatches: Vec::new(),
                passed: false,
            };
        }
    };

    let replay: ReplayFile = match config.format {
        ReplayFormat::Json => {
            match serde_json::from_str(&replay_data) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to parse replay JSON: {}", e);
                    return VerifyResult {
                        frames_total: 0,
                        frames_verified: 0,
                        frames_failed: 0,
                        hash_mismatches: Vec::new(),
                        passed: false,
                    };
                }
            }
        }
        ReplayFormat::Binary => {
            let binary_data = match std::fs::read(&config.replay_path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to read binary replay: {}", e);
                    return VerifyResult {
                        frames_total: 0,
                        frames_verified: 0,
                        frames_failed: 0,
                        hash_mismatches: Vec::new(),
                        passed: false,
                    };
                }
            };
            match bincode::deserialize(&binary_data) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to deserialize binary replay: {}", e);
                    return VerifyResult {
                        frames_total: 0,
                        frames_verified: 0,
                        frames_failed: 0,
                        hash_mismatches: Vec::new(),
                        passed: false,
                    };
                }
            }
        }
    };

    println!("Replay loaded: {} frames", replay.frames.len());
    println!("Players: {} vs {}",
        replay.header.players[0].name,
        replay.header.players[1].name,
    );

    // Reconstruct game state from replay header
    // ScenarioLoader::load_scenario takes (faction_a, faction_b, Option<MissionId>, seed: [u8; 32])
    let faction_a = replay.header.players[0].faction_id.unwrap_or(wh40k_core_types::FactionId::new(0));
    let faction_b = replay.header.players[1].faction_id.unwrap_or(wh40k_core_types::FactionId::new(1));

    let mut state = ScenarioLoader::load_scenario(
        faction_a,
        faction_b,
        replay.header.mission_id,
        replay.header.seed_bundle.root_seed,
    );

    // Apply enhancement and secondary choices from replay header
    if let Some(enh) = replay.header.players[0].enhancement {
        state.players[0].enhancement_choice = Some(enh);
    }
    if let Some(enh) = replay.header.players[1].enhancement {
        state.players[1].enhancement_choice = Some(enh);
    }
    if let Some(sec) = replay.header.players[0].secondary {
        state.players[0].secondary_choice = Some(sec);
    }
    if let Some(sec) = replay.header.players[1].secondary {
        state.players[1].secondary_choice = Some(sec);
    }

    let mut frames_verified: usize = 0;
    let mut frames_failed: usize = 0;
    let mut hash_mismatches = Vec::new();

    // Re-execute each frame
    for (i, frame) in replay.frames.iter().enumerate() {
        // Execute the command
        let validation = CommandValidator::validate(&state, &frame.command);
        if !validation.is_legal() {
            if config.verbose {
                println!("  Frame {}: INVALID command {:?}", i, frame.command);
            }
            frames_failed += 1;
            continue;
        }

        match CommandExecutor::execute(&mut state, &frame.command) {
            Ok(_events) => {
                // Verify hash if available
                if let Some(expected_hash) = frame.state_hash_after {
                    let actual_hash = state.compute_hash();
                    if actual_hash != expected_hash {
                        hash_mismatches.push(HashMismatch {
                            frame_index: i,
                            expected_hash,
                            actual_hash,
                            command: format!("{:?}", frame.command),
                        });
                        frames_failed += 1;

                        if config.verbose {
                            println!("  Frame {}: HASH MISMATCH (expected {:016x}, got {:016x})",
                                i, expected_hash, actual_hash);
                        }
                    } else {
                        frames_verified += 1;
                        if config.verbose {
                            println!("  Frame {}: OK ({:016x})", i, actual_hash);
                        }
                    }
                } else {
                    frames_verified += 1;
                    if config.verbose {
                        println!("  Frame {}: OK (no hash to verify)", i);
                    }
                }
            }
            Err(e) => {
                if config.verbose {
                    println!("  Frame {}: EXECUTION FAILED: {}", i, e);
                }
                frames_failed += 1;
            }
        }
    }

    let passed = frames_failed == 0;

    println!("\n=== Verification Results ===");
    println!("Total frames: {}", replay.frames.len());
    println!("Verified: {}", frames_verified);
    println!("Failed: {}", frames_failed);
    println!("Hash mismatches: {}", hash_mismatches.len());
    println!("Result: {}", if passed { "PASS" } else { "FAIL" });

    VerifyResult {
        frames_total: replay.frames.len(),
        frames_verified,
        frames_failed,
        hash_mismatches,
        passed,
    }
}
