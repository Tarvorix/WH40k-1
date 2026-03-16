//! WH40K Combat Patrol CLI binary.
//!
//! Provides headless game execution, AI benchmarking, replay verification,
//! and self-play data generation from the command line.
//!
//! Usage:
//!   wh40k play [OPTIONS]
//!   wh40k benchmark [OPTIONS]
//!   wh40k verify [OPTIONS]
//!   wh40k selfplay [OPTIONS]

use clap::{Parser, Subcommand};
use wh40k_core_types::*;
use wh40k_search_core::AiLevel;

use wh40k_native_api::{play, benchmark, verify, selfplay_cmd};

/// WH40K Combat Patrol Engine - Command Line Interface
#[derive(Parser)]
#[command(name = "wh40k")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "WH40K Combat Patrol game engine CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Play a game (AI vs AI or with specific configurations)
    Play {
        /// Faction for player A: custodes or world_eaters
        #[arg(long, default_value = "custodes")]
        faction_a: String,

        /// Faction for player B: custodes or world_eaters
        #[arg(long, default_value = "world_eaters")]
        faction_b: String,

        /// Mission number (1-6)
        #[arg(long, default_value = "1")]
        mission: u32,

        /// AI level for player A: greedy, one_ply, negamax, id
        #[arg(long, default_value = "greedy")]
        ai_a: String,

        /// AI level for player B: greedy, one_ply, negamax, id
        #[arg(long, default_value = "greedy")]
        ai_b: String,

        /// Random seed (hex string). Random if omitted.
        #[arg(long)]
        seed: Option<String>,

        /// Print move-by-move output
        #[arg(long, short)]
        verbose: bool,

        /// Export replay to file path
        #[arg(long)]
        replay: Option<String>,
    },

    /// Run AI benchmarks
    Benchmark {
        /// Number of games to play
        #[arg(long, default_value = "100")]
        games: u32,

        /// AI level for player A: greedy, one_ply, negamax, id
        #[arg(long, default_value = "greedy")]
        ai_a: String,

        /// AI level for player B: greedy, one_ply, negamax, id
        #[arg(long, default_value = "greedy")]
        ai_b: String,

        /// Print per-game results
        #[arg(long, short)]
        verbose: bool,
    },

    /// Verify replay determinism
    Verify {
        /// Path to replay file
        #[arg(long)]
        replay: String,

        /// Replay format: json or binary
        #[arg(long, default_value = "json")]
        format: String,

        /// Print per-frame verification
        #[arg(long, short)]
        verbose: bool,
    },

    /// Run self-play data generation
    Selfplay {
        /// Number of games to play
        #[arg(long, default_value = "100")]
        games: u32,

        /// Output directory for training shards
        #[arg(long, default_value = "./shards")]
        output_dir: String,

        /// AI level: greedy, one_ply, negamax, id
        #[arg(long, default_value = "greedy")]
        ai: String,

        /// Samples per shard
        #[arg(long, default_value = "4096")]
        shard_size: usize,

        /// Print per-game results
        #[arg(long, short)]
        verbose: bool,

        /// Game mode: combat_patrol or boarding_actions
        #[arg(long, default_value = "combat_patrol")]
        mode: String,
    },
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("wh40k=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Play {
            faction_a,
            faction_b,
            mission,
            ai_a,
            ai_b,
            seed,
            verbose,
            replay,
        } => {
            let faction_a_id = parse_faction(&faction_a);
            let faction_b_id = parse_faction(&faction_b);

            if mission < 1 || mission > 6 {
                eprintln!("Invalid mission: {}. Must be 1-6.", mission);
                std::process::exit(1);
            }

            let seed_bytes = match seed {
                Some(ref s) => parse_hex_seed(s),
                None => random_seed(),
            };

            let config = play::PlayConfig {
                faction_a: faction_a_id,
                faction_b: faction_b_id,
                mission: MissionId::new(mission),
                seed: seed_bytes,
                ai_level_a: parse_ai_level(&ai_a),
                ai_level_b: parse_ai_level(&ai_b),
                verbose,
                export_replay: replay,
                enhancement_a: None,
                enhancement_b: None,
                secondary_a: None,
                secondary_b: None,
            };

            play::run_play(config);
        }

        Commands::Benchmark {
            games,
            ai_a,
            ai_b,
            verbose,
        } => {
            let config = benchmark::BenchmarkConfig {
                games,
                ai_level_a: parse_ai_level(&ai_a),
                ai_level_b: parse_ai_level(&ai_b),
                verbose,
            };

            benchmark::run_benchmark(config);
        }

        Commands::Verify {
            replay,
            format,
            verbose,
        } => {
            let fmt = match format.to_lowercase().as_str() {
                "json" => verify::ReplayFormat::Json,
                "binary" | "bin" => verify::ReplayFormat::Binary,
                _ => {
                    eprintln!("Unknown format: {}. Use 'json' or 'binary'.", format);
                    std::process::exit(1);
                }
            };

            let config = verify::VerifyConfig {
                replay_path: replay,
                format: fmt,
                verbose,
            };

            let result = verify::run_verify(config);
            if !result.passed {
                std::process::exit(1);
            }
        }

        Commands::Selfplay {
            games,
            output_dir,
            ai,
            shard_size,
            verbose,
            mode,
        } => {
            let config = selfplay_cmd::SelfPlayCmdConfig {
                games,
                output_dir,
                ai_level: ai,
                shard_size,
                verbose,
                mode,
            };

            selfplay_cmd::run_selfplay(config);
        }
    }
}

fn parse_faction(name: &str) -> FactionId {
    match name.to_lowercase().as_str() {
        "custodes" | "adeptus_custodes" => FactionId::new(0),
        "world_eaters" | "frenzied_reavers" => FactionId::new(1),
        _ => {
            eprintln!("Unknown faction: {}. Use 'custodes' or 'world_eaters'.", name);
            std::process::exit(1);
        }
    }
}

fn parse_ai_level(level: &str) -> AiLevel {
    match level.to_lowercase().as_str() {
        "greedy" => AiLevel::Greedy,
        "one_ply" | "oneply" => AiLevel::OnePly,
        "negamax" => AiLevel::Negamax,
        "id" | "iterative" | "iterative_deepening" => AiLevel::IterativeDeepening,
        _ => {
            eprintln!("Unknown AI level: {}. Use 'greedy', 'one_ply', 'negamax', or 'id'.", level);
            std::process::exit(1);
        }
    }
}

fn parse_hex_seed(hex: &str) -> [u8; 32] {
    let hex = hex.trim_start_matches("0x").trim_start_matches("0X");
    let padded = format!("{:0>64}", hex);
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&padded[i * 2..i * 2 + 2], 16)
            .unwrap_or_else(|_| {
                eprintln!("Invalid hex seed at position {}", i * 2);
                std::process::exit(1);
            });
    }
    bytes
}

fn random_seed() -> [u8; 32] {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut seed = [0u8; 32];
    rng.fill(&mut seed);
    seed
}
