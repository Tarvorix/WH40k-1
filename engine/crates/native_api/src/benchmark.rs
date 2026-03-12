//! CLI benchmark command - run AI benchmarks.
//!
//! Source: implementation_v3.md Phase 10

use std::time::Instant;

use wh40k_core_types::*;
use wh40k_game_core::{ScenarioLoader, CommandValidator, CommandExecutor};
use wh40k_search_core::{SearchRoot, AiLevel};
use wh40k_search_abstraction::ActionGenerator;

/// Configuration for benchmarking.
pub struct BenchmarkConfig {
    pub games: u32,
    pub ai_level_a: AiLevel,
    pub ai_level_b: AiLevel,
    pub verbose: bool,
}

/// Benchmark results.
#[derive(Debug)]
pub struct BenchmarkResult {
    pub games_played: u32,
    pub games_completed: u32,
    pub p0_wins: u32,
    pub p1_wins: u32,
    pub draws: u32,
    pub total_commands: u64,
    pub total_time_ms: u64,
    pub avg_game_time_ms: u64,
    pub avg_commands_per_game: f64,
    pub games_per_hour: f64,
    pub p0_avg_vp: f64,
    pub p1_avg_vp: f64,
}

/// Run benchmark with the given configuration.
pub fn run_benchmark(config: BenchmarkConfig) -> BenchmarkResult {
    println!("=== WH40K AI Benchmark ===");
    println!("Games: {}", config.games);
    println!("AI: P0={:?} vs P1={:?}", config.ai_level_a, config.ai_level_b);
    println!("========================\n");

    let overall_start = Instant::now();

    let mut games_completed: u32 = 0;
    let mut p0_wins: u32 = 0;
    let mut p1_wins: u32 = 0;
    let mut draws: u32 = 0;
    let mut total_commands: u64 = 0;
    let mut total_p0_vp: i64 = 0;
    let mut total_p1_vp: i64 = 0;

    for game_num in 0..config.games {
        // Alternate factions and missions for variety
        let faction_a = if game_num % 2 == 0 { FactionId::new(0) } else { FactionId::new(1) };
        let faction_b = if game_num % 2 == 0 { FactionId::new(1) } else { FactionId::new(0) };
        let mission = MissionId::new((game_num % 6) as u32 + 1);

        // Generate seed from game number for reproducibility
        let mut seed = [0u8; 32];
        let game_bytes = game_num.to_le_bytes();
        seed[..4].copy_from_slice(&game_bytes);
        seed[4] = 0xBE;
        seed[5] = 0xEF;

        let mut state = ScenarioLoader::load_scenario(
            faction_a,
            faction_b,
            Some(mission),
            seed,
        );

        let mut game_commands: u64 = 0;
        let max_commands = 10_000u64;

        while state.is_in_progress() && game_commands < max_commands {
            let perspective = state.decision_owner;
            let ai_level = if perspective == PlayerId::new(0) {
                config.ai_level_a
            } else {
                config.ai_level_b
            };

            let candidates = ActionGenerator::generate(&state, perspective);
            if candidates.candidates.is_empty() {
                break;
            }

            let mut search = SearchRoot::new(ai_level);
            let result = search.search(&state, perspective);

            match result {
                Some(search_result) => {
                    for command in &search_result.best_action.commands {
                        let validation = CommandValidator::validate(&state, command);
                        if !validation.is_legal() {
                            continue;
                        }
                        if CommandExecutor::execute(&mut state, command).is_ok() {
                            game_commands += 1;
                        }
                    }
                }
                None => break,
            }
        }

        // Tally results
        let p0_vp = state.player(PlayerId::new(0)).vp.value();
        let p1_vp = state.player(PlayerId::new(1)).vp.value();

        match state.game_outcome {
            GameOutcome::Victory(pid) if pid == PlayerId::new(0) => p0_wins += 1,
            GameOutcome::Victory(_) => p1_wins += 1,
            GameOutcome::Draw => draws += 1,
            _ => draws += 1,
        }

        games_completed += 1;
        total_commands += game_commands;
        total_p0_vp += p0_vp as i64;
        total_p1_vp += p1_vp as i64;

        if config.verbose || (game_num + 1) % 10 == 0 {
            println!("Game {}/{}: {:?} | P0: {}VP | P1: {}VP | {} cmds",
                game_num + 1, config.games,
                state.game_outcome,
                p0_vp, p1_vp,
                game_commands,
            );
        }
    }

    let total_time = overall_start.elapsed();
    let total_time_ms = total_time.as_millis() as u64;
    let avg_game_time_ms = if games_completed > 0 {
        total_time_ms / games_completed as u64
    } else {
        0
    };

    let games_per_hour = if total_time_ms > 0 {
        (games_completed as f64 / total_time_ms as f64) * 3_600_000.0
    } else {
        0.0
    };

    let avg_commands_per_game = if games_completed > 0 {
        total_commands as f64 / games_completed as f64
    } else {
        0.0
    };

    let result = BenchmarkResult {
        games_played: config.games,
        games_completed,
        p0_wins,
        p1_wins,
        draws,
        total_commands,
        total_time_ms,
        avg_game_time_ms,
        avg_commands_per_game,
        games_per_hour,
        p0_avg_vp: if games_completed > 0 { total_p0_vp as f64 / games_completed as f64 } else { 0.0 },
        p1_avg_vp: if games_completed > 0 { total_p1_vp as f64 / games_completed as f64 } else { 0.0 },
    };

    println!("\n=== Benchmark Results ===");
    println!("Games: {}/{}", result.games_completed, result.games_played);
    println!("P0 wins: {} ({:.1}%)", result.p0_wins, result.p0_wins as f64 / result.games_completed as f64 * 100.0);
    println!("P1 wins: {} ({:.1}%)", result.p1_wins, result.p1_wins as f64 / result.games_completed as f64 * 100.0);
    println!("Draws: {}", result.draws);
    println!("Avg VP: P0={:.1}, P1={:.1}", result.p0_avg_vp, result.p1_avg_vp);
    println!("Avg commands/game: {:.0}", result.avg_commands_per_game);
    println!("Avg game time: {}ms", result.avg_game_time_ms);
    println!("Games/hour: {:.0}", result.games_per_hour);
    println!("Total time: {:.1}s", total_time_ms as f64 / 1000.0);

    result
}
