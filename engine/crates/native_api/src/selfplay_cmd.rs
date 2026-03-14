//! CLI selfplay command - run self-play data generation batches.
//!
//! Source: implementation_v3.md Phase 10

use std::path::PathBuf;
use std::time::Instant;

use wh40k_core_types::MissionId;
use wh40k_selfplay::{
    PlayerConfig, AiType, GameVariation,
    ShardWriter,
};

/// Configuration for self-play data generation.
pub struct SelfPlayCmdConfig {
    pub games: u32,
    pub output_dir: String,
    pub ai_level: String,
    pub shard_size: usize,
    pub verbose: bool,
}

/// Result of self-play generation.
#[derive(Debug)]
pub struct SelfPlayCmdResult {
    pub games_played: u32,
    pub games_completed: u32,
    pub samples_generated: u64,
    pub shards_written: u64,
    pub total_time_ms: u64,
    pub games_per_hour: f64,
    pub output_dir: String,
}

/// Create a PlayerConfig from an AiType and a name.
fn make_player_config(ai_type: &AiType, name: &str) -> PlayerConfig {
    match ai_type {
        AiType::Greedy => PlayerConfig::greedy(name),
        AiType::OnePly => PlayerConfig::one_ply(name),
        AiType::Negamax(depth) => PlayerConfig::negamax(name, *depth),
        AiType::IterativeDeepening(max_depth) => PlayerConfig::iterative_deepening(name, *max_depth),
        AiType::IterativeDeepeningTimed(time_ms) => PlayerConfig::iterative_deepening_timed(name, *time_ms),
        AiType::GreedyNnue => PlayerConfig::nnue_bootstrap(name),
    }
}

/// Run self-play data generation.
pub fn run_selfplay(config: SelfPlayCmdConfig) -> SelfPlayCmdResult {
    println!("=== WH40K Self-Play Data Generation ===");
    println!("Games: {}", config.games);
    println!("AI Level: {}", config.ai_level);
    println!("Output: {}", config.output_dir);
    println!("Shard size: {}", config.shard_size);
    println!("======================================\n");

    // Create output directory
    let output_path = PathBuf::from(&config.output_dir);
    if let Err(e) = std::fs::create_dir_all(&output_path) {
        eprintln!("Failed to create output directory: {}", e);
        return SelfPlayCmdResult {
            games_played: 0,
            games_completed: 0,
            samples_generated: 0,
            shards_written: 0,
            total_time_ms: 0,
            games_per_hour: 0.0,
            output_dir: config.output_dir,
        };
    }

    let ai_type = match config.ai_level.to_lowercase().as_str() {
        "greedy" => AiType::Greedy,
        "one_ply" | "oneply" => AiType::OnePly,
        "negamax" => AiType::Negamax(3),
        "negamax_deep" => AiType::Negamax(5),
        "id" | "iterative" => AiType::IterativeDeepening(6),
        _ => {
            eprintln!("Unknown AI level '{}', defaulting to greedy", config.ai_level);
            AiType::Greedy
        }
    };

    let overall_start = Instant::now();

    // Set up shard writer (3 args: output_dir, max_samples, model_generation)
    let mut shard_writer = match ShardWriter::new(
        output_path.clone(),
        config.shard_size,
        0, // model_generation: 0 for initial generation
    ) {
        Ok(writer) => writer,
        Err(e) => {
            eprintln!("Failed to create shard writer: {}", e);
            return SelfPlayCmdResult {
                games_played: 0,
                games_completed: 0,
                samples_generated: 0,
                shards_written: 0,
                total_time_ms: 0,
                games_per_hour: 0.0,
                output_dir: config.output_dir,
            };
        }
    };

    let mut total_games_completed: u32 = 0;
    let mut total_samples: u64 = 0;

    // Generate game variations (4 args: num_games, seed_base, missions, alternate_factions)
    let missions: Vec<Option<MissionId>> = (0..6)
        .map(|i| Some(MissionId::new(i + 1)))
        .collect();
    let variations = GameVariation::generate_configs(
        config.games as usize,
        42, // seed_base for determinism
        &missions,
        true, // alternate factions
    );

    // Progress tracking
    let progress_interval = if config.games <= 20 { 1 } else if config.games <= 100 { 5 } else { 10 };
    let mut last_progress_time = overall_start;

    for (i, match_config) in variations.iter().enumerate() {
        let game_start = Instant::now();

        // Create player configs from ai_type
        let player_a = make_player_config(&ai_type, "Player A");
        let player_b = make_player_config(&ai_type, "Player B");

        // Run the game (5 args: match_config, player1, player2, collect_training_data, collect_replay)
        match wh40k_selfplay::play_single_game(
            match_config,
            &player_a,
            &player_b,
            true,  // collect_training_data
            false, // collect_replay
        ) {
            Ok(result) => {
                // Write training samples to shards
                for sample in &result.training_samples {
                    if let Err(e) = shard_writer.add_sample(sample.clone()) {
                        eprintln!("Failed to write sample: {}", e);
                    }
                    total_samples += 1;
                }

                total_games_completed += 1;

                if config.verbose || (i + 1) % progress_interval == 0 || i + 1 == config.games as usize {
                    let elapsed = game_start.elapsed();
                    let total_elapsed = overall_start.elapsed();
                    let games_done = i + 1;
                    let games_remaining = config.games as usize - games_done;
                    let avg_ms = total_elapsed.as_millis() as f64 / games_done as f64;
                    let eta_secs = (avg_ms * games_remaining as f64) / 1000.0;
                    let pct = (games_done as f64 / config.games as f64) * 100.0;

                    let error_info = if let Some(ref msg) = result.error_message {
                        format!(" | ERR: {}", msg)
                    } else {
                        String::new()
                    };
                    println!("[{:5.1}%] Game {}/{}: {:?} | VP: {}v{} | {} cmds | {} samples | {:.0}ms | ETA: {:.0}s{}",
                        pct,
                        games_done, config.games,
                        result.outcome,
                        result.player1_vp, result.player2_vp,
                        result.total_commands,
                        result.training_samples.len(),
                        elapsed.as_millis(),
                        eta_secs,
                        error_info,
                    );
                    last_progress_time = Instant::now();
                }
            }
            Err(e) => {
                println!("[{:5.1}%] Game {}/{}: FAILED - {}",
                    ((i + 1) as f64 / config.games as f64) * 100.0,
                    i + 1, config.games, e);
            }
        }
    }
    let _ = last_progress_time; // suppress unused warning

    // Flush remaining samples and finalize to get stats
    let shard_stats = match shard_writer.finalize() {
        Ok(stats) => stats,
        Err(e) => {
            eprintln!("Failed to finalize shard writer: {}", e);
            // Return what we have so far
            let total_time = overall_start.elapsed();
            let total_time_ms = total_time.as_millis() as u64;
            return SelfPlayCmdResult {
                games_played: config.games,
                games_completed: total_games_completed,
                samples_generated: total_samples,
                shards_written: 0,
                total_time_ms,
                games_per_hour: 0.0,
                output_dir: config.output_dir,
            };
        }
    };

    let total_time = overall_start.elapsed();
    let total_time_ms = total_time.as_millis() as u64;
    let games_per_hour = if total_time_ms > 0 {
        (total_games_completed as f64 / total_time_ms as f64) * 3_600_000.0
    } else {
        0.0
    };

    let result = SelfPlayCmdResult {
        games_played: config.games,
        games_completed: total_games_completed,
        samples_generated: total_samples,
        shards_written: shard_stats.total_shards as u64,
        total_time_ms,
        games_per_hour,
        output_dir: config.output_dir.clone(),
    };

    println!("\n=== Self-Play Results ===");
    println!("Games: {}/{}", result.games_completed, result.games_played);
    println!("Samples generated: {}", result.samples_generated);
    println!("Shards written: {}", result.shards_written);
    println!("Games/hour: {:.0}", result.games_per_hour);
    println!("Total time: {:.1}s", total_time_ms as f64 / 1000.0);
    println!("Output: {}", result.output_dir);

    result
}
