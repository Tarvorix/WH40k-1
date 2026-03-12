//! CLI play command - run interactive or AI vs AI games headlessly.
//!
//! Source: implementation_v3.md Phase 10

use wh40k_core_types::*;
use wh40k_game_core::{ScenarioLoader, CommandValidator, CommandExecutor};
use wh40k_search_core::{SearchRoot, AiLevel};
use wh40k_search_abstraction::ActionGenerator;
use wh40k_replay::ReplayRecorder;

/// Configuration for a play session.
pub struct PlayConfig {
    pub faction_a: FactionId,
    pub faction_b: FactionId,
    pub mission: MissionId,
    pub seed: [u8; 32],
    pub ai_level_a: AiLevel,
    pub ai_level_b: AiLevel,
    pub verbose: bool,
    pub export_replay: Option<String>,
    pub enhancement_a: Option<EnhancementId>,
    pub enhancement_b: Option<EnhancementId>,
    pub secondary_a: Option<SecondaryObjectiveId>,
    pub secondary_b: Option<SecondaryObjectiveId>,
}

/// Result of a completed play session.
pub struct PlayResult {
    pub outcome: GameOutcome,
    pub p0_vp: i16,
    pub p1_vp: i16,
    pub battle_rounds_played: u8,
    pub total_commands: usize,
    pub replay_path: Option<String>,
}

/// Run a complete game with the given configuration.
pub fn run_play(config: PlayConfig) -> PlayResult {
    // Load scenario (4 args: faction_a, faction_b, Option<MissionId>, seed)
    let mut state = ScenarioLoader::load_scenario(
        config.faction_a,
        config.faction_b,
        Some(config.mission),
        config.seed,
    );

    // Set enhancements and secondaries on the state after loading
    if let Some(enh) = config.enhancement_a {
        state.players[0].enhancement_choice = Some(enh);
    }
    if let Some(enh) = config.enhancement_b {
        state.players[1].enhancement_choice = Some(enh);
    }
    if let Some(sec) = config.secondary_a {
        state.players[0].secondary_choice = Some(sec);
    }
    if let Some(sec) = config.secondary_b {
        state.players[1].secondary_choice = Some(sec);
    }

    let mut recorder = ReplayRecorder::new(&state);

    if config.verbose {
        println!("=== WH40K Combat Patrol ===");
        println!("Player 0: {}", faction_name(config.faction_a));
        println!("Player 1: {}", faction_name(config.faction_b));
        println!("Mission: {}", config.mission.raw());
        println!("Seed: {}", hex_seed(&config.seed));
        println!("AI Levels: P0={:?}, P1={:?}", config.ai_level_a, config.ai_level_b);
        println!("==========================\n");
    }

    let mut total_commands: usize = 0;
    let max_commands = 10_000usize;

    // Main game loop
    while state.is_in_progress() && total_commands < max_commands {
        let perspective = state.decision_owner;
        let ai_level = if perspective == PlayerId::new(0) {
            config.ai_level_a
        } else {
            config.ai_level_b
        };

        // Generate candidates
        let candidates = ActionGenerator::generate(&state, perspective);
        if candidates.candidates.is_empty() {
            if config.verbose {
                println!("[WARN] No candidates available at BR{} {:?} for P{}",
                    state.battle_round.number(),
                    state.current_phase,
                    perspective.raw(),
                );
            }
            break;
        }

        // Search for best action using AiLevel
        let mut search = SearchRoot::new(ai_level);
        let result = search.search(&state, perspective);

        match result {
            Some(search_result) => {
                let action = &search_result.best_action;

                if config.verbose {
                    println!("BR{} {:?} P{}: {} (intent: {:?}, score: {})",
                        state.battle_round.number(),
                        state.current_phase,
                        perspective.raw(),
                        action.label,
                        action.intent,
                        search_result.score,
                    );
                }

                // Execute all commands in the macro action
                for command in &action.commands {
                    let validation = CommandValidator::validate(&state, command);
                    if !validation.is_legal() {
                        if config.verbose {
                            println!("  [SKIP] Invalid command: {:?}", command);
                        }
                        continue;
                    }

                    match CommandExecutor::execute(&mut state, command) {
                        Ok(events) => {
                            recorder.record_command(
                                &state,
                                command,
                                &events,
                                true,
                            );
                            total_commands += 1;
                        }
                        Err(e) => {
                            if config.verbose {
                                println!("  [ERR] Execution failed: {}", e);
                            }
                        }
                    }
                }
            }
            None => {
                if config.verbose {
                    println!("[WARN] Search returned no result");
                }
                break;
            }
        }
    }

    // Print final result
    let p0_vp = state.player(PlayerId::new(0)).vp.value();
    let p1_vp = state.player(PlayerId::new(1)).vp.value();

    if config.verbose {
        println!("\n=== Game Over ===");
        println!("Outcome: {:?}", state.game_outcome);
        println!("Player 0 ({}): {} VP", faction_name(config.faction_a), p0_vp);
        println!("Player 1 ({}): {} VP", faction_name(config.faction_b), p1_vp);
        println!("Battle Rounds: {}", state.battle_round.number());
        println!("Total Commands: {}", total_commands);
    }

    // Export replay if requested
    let replay_path = if let Some(ref path) = config.export_replay {
        let replay = recorder.finalize(&state);
        match wh40k_replay::export_json(&replay) {
            Ok(json) => {
                match std::fs::write(path, json) {
                    Ok(_) => {
                        if config.verbose {
                            println!("Replay exported to: {}", path);
                        }
                        Some(path.clone())
                    }
                    Err(e) => {
                        eprintln!("Failed to write replay: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to export replay: {}", e);
                None
            }
        }
    } else {
        None
    };

    PlayResult {
        outcome: state.game_outcome,
        p0_vp,
        p1_vp,
        battle_rounds_played: state.battle_round.number(),
        total_commands,
        replay_path,
    }
}

fn faction_name(id: FactionId) -> &'static str {
    match id.raw() {
        0 => "Custodes",
        1 => "World Eaters",
        _ => "Unknown",
    }
}

fn hex_seed(seed: &[u8; 32]) -> String {
    seed.iter().map(|b| format!("{:02x}", b)).collect()
}
