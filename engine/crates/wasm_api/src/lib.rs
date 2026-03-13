//! WH40K Engine - WASM API
//!
//! Browser-facing API layer. All functions exchange JSON strings across the WASM
//! boundary. The engine state is held in thread-local storage (WASM is
//! single-threaded) and projected into flat, string-friendly view model structs
//! before serialisation.

mod view_models;
mod conversions;
mod error;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use wh40k_core_types::{FactionId, Inches, MissionId, PlayerId, Position, UnitId};
use wh40k_command_system::Command;
use wh40k_dice::SeedBundle;
use wh40k_game_core::{CommandExecutor, CommandValidator, GameState, ScenarioLoader};
use wh40k_command_system::CommandValidationResult;
use wh40k_replay::{ReplayFile, ReplayPlayer, ReplayRecorder};
use wh40k_search_abstraction::{ActionGenerator, MacroAction};
use wh40k_search_core::SearchResult;

use crate::conversions::*;
use crate::error::WasmError;
use crate::view_models::*;

// ===========================================================================
// Thread-local state
// ===========================================================================

thread_local! {
    /// The active game state. `None` when no game is loaded.
    static GAME_STATE: RefCell<Option<GameState>> = const { RefCell::new(None) };

    /// Replay recorder for the current game. Created alongside the game state.
    static REPLAY_RECORDER: RefCell<Option<ReplayRecorder>> = const { RefCell::new(None) };

    /// Replay player for replay playback mode.
    static REPLAY_PLAYER: RefCell<Option<ReplayPlayer>> = const { RefCell::new(None) };

    /// Cached macro-actions from the last call to `get_decision_surface`.
    /// Indexed by the `ActionView::index` returned to JavaScript.
    static DECISION_CACHE: RefCell<Vec<MacroAction>> = const { RefCell::new(Vec::new()) };

    /// The last AI search result, stored so `apply_ai_action` can execute it.
    static AI_RESULT: RefCell<Option<SearchResult>> = const { RefCell::new(None) };

    /// Cached reference to the ReplayFile when a replay is loaded.
    /// Kept separately because ReplayPlayer's replay field is private.
    static REPLAY_FILE_CACHE: RefCell<Option<ReplayFile>> = const { RefCell::new(None) };
}

// ===========================================================================
// Helper macros / functions
// ===========================================================================

/// Read the game state or return an error.
fn with_state<F, T>(f: F) -> Result<T, WasmError>
where
    F: FnOnce(&GameState) -> T,
{
    GAME_STATE.with(|cell| {
        let borrow = cell.borrow();
        match borrow.as_ref() {
            Some(state) => Ok(f(state)),
            None => Err(WasmError::new("No game state loaded")),
        }
    })
}

/// Serialise a value to a JSON string.
fn to_json<T: serde::Serialize>(val: &T) -> Result<String, WasmError> {
    serde_json::to_string(val).map_err(WasmError::from)
}

// ===========================================================================
// Seed parsing helper
// ===========================================================================

/// A JSON payload for specifying the random seed.
#[derive(serde::Deserialize)]
struct SeedInput {
    /// Raw 32-byte seed (takes priority if present).
    seed: Option<Vec<u8>>,
    /// Convenience: a u64 that gets expanded to 32 bytes.
    seed_u64: Option<u64>,
}

fn parse_seed(json: &str) -> Result<[u8; 32], WasmError> {
    let input: SeedInput =
        serde_json::from_str(json).map_err(|e| WasmError::new(format!("Invalid seed JSON: {}", e)))?;

    if let Some(raw) = input.seed {
        if raw.len() != 32 {
            return Err(WasmError::new(format!(
                "seed array must be exactly 32 bytes, got {}",
                raw.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&raw);
        Ok(arr)
    } else if let Some(u) = input.seed_u64 {
        let bundle = SeedBundle::from_u64(u, "wasm_match".to_string());
        Ok(bundle.root_seed)
    } else {
        Err(WasmError::new(
            "seed JSON must contain either \"seed\" (array of 32 bytes) or \"seed_u64\" (number)",
        ))
    }
}

// ===========================================================================
// 1. init_panic_hook
// ===========================================================================

/// Install a panic hook that logs to `console.error` in the browser.
/// Call once at application startup.
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

// ===========================================================================
// 2. create_match
// ===========================================================================

/// Create a new game from faction IDs, a mission ID, and a seed.
///
/// # Arguments
/// * `faction_a` - Faction ID for Player A (Attacker)
/// * `faction_b` - Faction ID for Player B (Defender)
/// * `mission`   - Mission / scenario ID
/// * `seed_json` - JSON with `{"seed":[...32 bytes...]}` or `{"seed_u64":12345}`
/// * `enhancement_a/b` - Enhancement choice (-1 = none)
/// * `secondary_a/b` - Secondary objective choice (-1 = none)
/// * `patrol_squad_a/b` - Patrol squad choice (-1 = default/Wardens, 0 = Wardens, 1 = Allarus)
///
/// # Returns
/// JSON-encoded `GameView`.
#[wasm_bindgen]
pub fn create_match(
    faction_a: u32,
    faction_b: u32,
    mission: u32,
    seed_json: &str,
    enhancement_a: i32,
    enhancement_b: i32,
    secondary_a: i32,
    secondary_b: i32,
    patrol_squad_a: i32,
    patrol_squad_b: i32,
) -> Result<String, JsValue> {
    let seed = parse_seed(seed_json).map_err(JsValue::from)?;

    // Convert patrol squad choices: -1 means default (None), >= 0 means specific choice
    let sq_a = if patrol_squad_a >= 0 { Some(patrol_squad_a as usize) } else { None };
    let sq_b = if patrol_squad_b >= 0 { Some(patrol_squad_b as usize) } else { None };

    let mut state = ScenarioLoader::load_scenario_with_squads(
        FactionId(faction_a),
        FactionId(faction_b),
        Some(MissionId(mission)),
        seed,
        sq_a,
        sq_b,
    );

    // Apply enhancement and secondary selections
    // Source: CP_Rules.md - Pre-battle setup selections
    if enhancement_a >= 0 {
        let enh_id = wh40k_core_types::EnhancementId::new(enhancement_a as u32);
        state.players[0].enhancement_choice = Some(enh_id);
        // Find warlord (first CHARACTER unit)
        let warlord = state.units.iter()
            .find(|u| u.owner == wh40k_core_types::PlayerId::new(0) && u.is_character())
            .map(|u| u.id);
        if let Some(warlord_id) = warlord {
            wh40k_game_core::scoring::apply_enhancement(&mut state, wh40k_core_types::PlayerId::new(0), enh_id, warlord_id);
        }
    }
    if enhancement_b >= 0 {
        let enh_id = wh40k_core_types::EnhancementId::new(enhancement_b as u32);
        state.players[1].enhancement_choice = Some(enh_id);
        let warlord = state.units.iter()
            .find(|u| u.owner == wh40k_core_types::PlayerId::new(1) && u.is_character())
            .map(|u| u.id);
        if let Some(warlord_id) = warlord {
            wh40k_game_core::scoring::apply_enhancement(&mut state, wh40k_core_types::PlayerId::new(1), enh_id, warlord_id);
        }
    }
    if secondary_a >= 0 {
        state.players[0].secondary_choice = Some(wh40k_core_types::SecondaryObjectiveId::new(secondary_a as u32));
    }
    if secondary_b >= 0 {
        state.players[1].secondary_choice = Some(wh40k_core_types::SecondaryObjectiveId::new(secondary_b as u32));
    }

    // Create a replay recorder from the initial state.
    let recorder = ReplayRecorder::new(&state);

    // Generate the initial decision surface and cache it.
    let candidates = ActionGenerator::generate(&state, state.decision_owner);
    let cached_actions: Vec<MacroAction> = candidates.candidates.into_iter().collect();

    // Build the view before storing state.
    let view = game_state_to_view(&state);

    // Store everything.
    GAME_STATE.with(|cell| cell.replace(Some(state)));
    REPLAY_RECORDER.with(|cell| cell.replace(Some(recorder)));
    DECISION_CACHE.with(|cell| cell.replace(cached_actions));
    AI_RESULT.with(|cell| cell.replace(None));

    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 3. load_scenario
// ===========================================================================

/// Load a full game state from its JSON serialisation.
///
/// # Returns
/// JSON-encoded `GameView`.
#[wasm_bindgen]
pub fn load_scenario(state_json: &str) -> Result<String, JsValue> {
    let state: GameState =
        serde_json::from_str(state_json).map_err(|e| WasmError::new(format!("Invalid state JSON: {}", e)))?;

    let view = game_state_to_view(&state);

    GAME_STATE.with(|cell| cell.replace(Some(state)));
    DECISION_CACHE.with(|cell| cell.replace(Vec::new()));
    AI_RESULT.with(|cell| cell.replace(None));

    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 4. get_state_snapshot
// ===========================================================================

/// Return the current game state as a `GameView` JSON string.
#[wasm_bindgen]
pub fn get_state_snapshot() -> Result<String, JsValue> {
    let view = with_state(game_state_to_view).map_err(JsValue::from)?;
    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 5. get_decision_surface
// ===========================================================================

/// Generate the legal actions for the current decision point and cache them.
///
/// # Returns
/// JSON-encoded `DecisionSurfaceView`.
#[wasm_bindgen]
pub fn get_decision_surface() -> Result<String, JsValue> {
    let (view, actions) = with_state(|state| {
        let candidates = ActionGenerator::generate(state, state.decision_owner);
        let owner = candidates.owner;
        let phase = state.current_phase;
        let subphase = state.current_subphase;
        let actions: Vec<MacroAction> = candidates.candidates.into_iter().collect();
        let view = macro_actions_to_decision_surface_view(&actions, owner, &phase, &subphase);
        (view, actions)
    })
    .map_err(JsValue::from)?;

    DECISION_CACHE.with(|cell| cell.replace(actions));

    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 6. validate_action
// ===========================================================================

/// Validate the action at the given index against the current state.
///
/// # Returns
/// JSON-encoded `ValidationResultView`.
#[wasm_bindgen]
pub fn validate_action(index: u32) -> Result<String, JsValue> {
    let idx = index as usize;

    // Read the first command from the cached MacroAction.
    let first_cmd: Command = DECISION_CACHE.with(|cell| {
        let cache = cell.borrow();
        cache
            .get(idx)
            .and_then(|ma| ma.commands.first().cloned())
            .ok_or_else(|| WasmError::new(format!("Action index {} out of range", idx)))
    }).map_err(JsValue::from)?;

    let result_view = with_state(|state| {
        let result = CommandValidator::validate(state, &first_cmd);
        match result {
            CommandValidationResult::Legal => ValidationResultView {
                valid: true,
                reason: None,
            },
            CommandValidationResult::Illegal { reason, .. } => ValidationResultView {
                valid: false,
                reason: Some(reason),
            },
        }
    })
    .map_err(JsValue::from)?;

    to_json(&result_view).map_err(JsValue::from)
}

// ===========================================================================
// 7. apply_action
// ===========================================================================

/// Execute the MacroAction at the given index, updating the game state.
///
/// Records each command to the replay recorder and clears the decision cache.
///
/// # Returns
/// JSON-encoded `GameView` reflecting the new state.
#[wasm_bindgen]
pub fn apply_action(index: u32) -> Result<String, JsValue> {
    let idx = index as usize;

    // Clone the commands out of the cached MacroAction.
    let commands: Vec<Command> = DECISION_CACHE.with(|cell| {
        let cache = cell.borrow();
        cache
            .get(idx)
            .map(|ma| ma.commands.clone())
            .ok_or_else(|| WasmError::new(format!("Action index {} out of range", idx)))
    }).map_err(JsValue::from)?;

    // Execute each command sequentially against the game state.
    GAME_STATE.with(|state_cell| -> Result<(), WasmError> {
        let mut state_borrow = state_cell.borrow_mut();
        let state = state_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No game state loaded"))?;

        for cmd in &commands {
            let events = CommandExecutor::execute(state, cmd)
                .map_err(WasmError::from)?;

            // Record to replay recorder if present.
            REPLAY_RECORDER.with(|rec_cell| {
                let mut rec_borrow = rec_cell.borrow_mut();
                if let Some(recorder) = rec_borrow.as_mut() {
                    recorder.record_command(state, cmd, &events, true);
                }
            });
        }

        Ok(())
    }).map_err(JsValue::from)?;

    // Clear the decision cache.
    DECISION_CACHE.with(|cell| cell.replace(Vec::new()));

    // Return the updated state view.
    let view = with_state(game_state_to_view).map_err(JsValue::from)?;
    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 8. run_ai_decision
// ===========================================================================

/// Run the AI search at the specified difficulty level and return the result.
///
/// # Arguments
/// * `difficulty` - AI difficulty tier:
///   - Basic (heuristic evaluator): "Basic_Recruit", "Basic_Battle_Ready", "Basic_Veteran", "Basic_Elite"
///   - Perturabo (iterative deepening + NNUE when trained): "Perturabo_Shallow", "Perturabo_Regular", "Perturabo_Deep"
///   - Alpharius (MCTS + policy/value net when trained): "Alpharius_Operative", "Alpharius_Headhunter", "Alpharius_Primarch"
///
/// # Returns
/// JSON-encoded `AiResultView`.
#[wasm_bindgen]
pub fn run_ai_decision(difficulty: &str) -> Result<String, JsValue> {
    let result = with_state(|state| {
        let perspective = state.decision_owner;
        match difficulty {
            // Basic tier: hand-tuned heuristic evaluator
            "Basic_Recruit" => wh40k_search_core::greedy_choose(state, perspective),
            "Basic_Battle_Ready" => wh40k_search_core::one_ply_choose(state, perspective),
            "Basic_Veteran" => wh40k_search_core::negamax_choose(state, perspective, 2),
            "Basic_Elite" => wh40k_search_core::negamax_choose(state, perspective, 3),
            // Perturabo tier: iterative deepening search (NNUE eval when trained, heuristic for now)
            "Perturabo_Shallow" => wh40k_search_core::id_search_choose(state, perspective, 4),
            "Perturabo_Regular" => wh40k_search_core::id_search_choose(state, perspective, 6),
            "Perturabo_Deep" => wh40k_search_core::id_search_choose(state, perspective, 8),
            // Alpharius tier: MCTS search (policy/value net when trained, heuristic prior for now)
            "Alpharius_Shallow" => wh40k_search_core::mcts_choose_fast(state, perspective),
            "Alpharius_Regular" => wh40k_search_core::mcts_choose(state, perspective),
            "Alpharius_Deep" => wh40k_search_core::mcts_choose_competition(state, perspective),
            _ => None,
        }
    })
    .map_err(JsValue::from)?;

    match result {
        Some(sr) => {
            let view = search_result_to_view(&sr);
            AI_RESULT.with(|cell| cell.replace(Some(sr)));
            to_json(&view).map_err(JsValue::from)
        }
        None => Err(WasmError::new(format!(
            "AI search returned no result for difficulty '{}'",
            difficulty
        ))
        .into()),
    }
}

// ===========================================================================
// 9. apply_ai_action
// ===========================================================================

/// Apply the best action from the last AI search result.
///
/// The caller should have previously called `run_ai_decision` which stores
/// the `SearchResult` internally.
///
/// # Returns
/// JSON-encoded `GameView` reflecting the new state.
#[wasm_bindgen]
pub fn apply_ai_action() -> Result<String, JsValue> {
    // Take the stored SearchResult.
    let commands: Vec<Command> = AI_RESULT.with(|cell| {
        let mut borrow = cell.borrow_mut();
        match borrow.take() {
            Some(sr) => Ok(sr.best_action.commands),
            None => Err(WasmError::new(
                "No AI result stored. Call run_ai_decision first.",
            )),
        }
    }).map_err(JsValue::from)?;

    // Execute each command.
    GAME_STATE.with(|state_cell| -> Result<(), WasmError> {
        let mut state_borrow = state_cell.borrow_mut();
        let state = state_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No game state loaded"))?;

        for cmd in &commands {
            let events = CommandExecutor::execute(state, cmd)
                .map_err(WasmError::from)?;

            REPLAY_RECORDER.with(|rec_cell| {
                let mut rec_borrow = rec_cell.borrow_mut();
                if let Some(recorder) = rec_borrow.as_mut() {
                    recorder.record_command(state, cmd, &events, true);
                }
            });
        }

        Ok(())
    }).map_err(JsValue::from)?;

    // Clear the decision cache since the state changed.
    DECISION_CACHE.with(|cell| cell.replace(Vec::new()));

    let view = with_state(game_state_to_view).map_err(JsValue::from)?;
    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 10. submit_place_unit (direct command - bypasses decision cache)
// ===========================================================================

/// Deploy a unit at an arbitrary position within the player's deployment zone.
///
/// This bypasses the ActionGenerator's pre-sampled positions, allowing the
/// human player to click anywhere on the map. The engine validator enforces
/// all placement rules (zone containment, board bounds, unit ownership, etc.).
///
/// # Arguments
/// * `player` - Player ID (0 or 1)
/// * `unit_id` - Unit to deploy
/// * `x` - X coordinate in inches (f64)
/// * `y` - Y coordinate in inches (f64)
///
/// # Returns
/// JSON-encoded `GameView` reflecting the new state, or an error if invalid.
#[wasm_bindgen]
pub fn submit_place_unit(player: u32, unit_id: u32, x: f64, y: f64) -> Result<String, JsValue> {
    let position = Position {
        x: Inches::from_f64(x),
        y: Inches::from_f64(y),
    };

    let cmd = Command::PlaceUnit {
        player: PlayerId::new(player),
        unit_id: UnitId::new(unit_id),
        position,
    };

    // Execute the command (internally validates via CommandValidator).
    GAME_STATE.with(|state_cell| -> Result<(), WasmError> {
        let mut state_borrow = state_cell.borrow_mut();
        let state = state_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No game state loaded"))?;

        let events = CommandExecutor::execute(state, &cmd)
            .map_err(WasmError::from)?;

        // Record to replay recorder if present.
        REPLAY_RECORDER.with(|rec_cell| {
            let mut rec_borrow = rec_cell.borrow_mut();
            if let Some(recorder) = rec_borrow.as_mut() {
                recorder.record_command(state, &cmd, &events, true);
            }
        });

        Ok(())
    }).map_err(JsValue::from)?;

    // Clear the stale decision cache.
    DECISION_CACHE.with(|cell| cell.replace(Vec::new()));

    let view = with_state(game_state_to_view).map_err(JsValue::from)?;
    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 11. submit_normal_move (direct command - bypasses decision cache)
// ===========================================================================

/// Move a unit to an arbitrary destination within its movement range.
///
/// This bypasses the ActionGenerator's pre-sampled tactical destinations,
/// allowing the human player to click anywhere on the map. The engine
/// validator enforces all movement rules (range, engagement, board bounds).
///
/// # Arguments
/// * `unit_id` - Unit to move
/// * `x` - Destination X in inches (f64)
/// * `y` - Destination Y in inches (f64)
///
/// # Returns
/// JSON-encoded `GameView` reflecting the new state, or an error if invalid.
#[wasm_bindgen]
pub fn submit_normal_move(unit_id: u32, x: f64, y: f64) -> Result<String, JsValue> {
    let destination = Position {
        x: Inches::from_f64(x),
        y: Inches::from_f64(y),
    };

    let cmd = Command::NormalMove {
        unit_id: UnitId::new(unit_id),
        destination,
    };

    // Execute the command (internally validates via CommandValidator).
    GAME_STATE.with(|state_cell| -> Result<(), WasmError> {
        let mut state_borrow = state_cell.borrow_mut();
        let state = state_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No game state loaded"))?;

        let events = CommandExecutor::execute(state, &cmd)
            .map_err(WasmError::from)?;

        // Record to replay recorder if present.
        REPLAY_RECORDER.with(|rec_cell| {
            let mut rec_borrow = rec_cell.borrow_mut();
            if let Some(recorder) = rec_borrow.as_mut() {
                recorder.record_command(state, &cmd, &events, true);
            }
        });

        Ok(())
    }).map_err(JsValue::from)?;

    // Clear the stale decision cache.
    DECISION_CACHE.with(|cell| cell.replace(Vec::new()));

    let view = with_state(game_state_to_view).map_err(JsValue::from)?;
    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 12. export_replay
// ===========================================================================

/// Finalise the replay recorder and export it as a JSON string.
///
/// This consumes the replay recorder. To continue recording, create a new
/// match.
///
/// # Returns
/// JSON-encoded `ReplayFile`.
#[wasm_bindgen]
pub fn export_replay() -> Result<String, JsValue> {
    // We need to take both the recorder and borrow the state to finalize.
    let recorder: ReplayRecorder = REPLAY_RECORDER.with(|cell| {
        cell.borrow_mut()
            .take()
            .ok_or_else(|| WasmError::new("No replay recorder active"))
    }).map_err(JsValue::from)?;

    let replay_file = with_state(|state| recorder.finalize(state)).map_err(JsValue::from)?;

    let json = wh40k_replay::export_json(&replay_file)
        .map_err(|e| WasmError::new(format!("Replay export error: {}", e)))?;

    Ok(json)
}

// ===========================================================================
// 11. load_replay
// ===========================================================================

/// Load a replay from JSON and initialise the replay player.
///
/// # Returns
/// JSON-encoded `ReplayInfoView`.
#[wasm_bindgen]
pub fn load_replay(replay_json: &str) -> Result<String, JsValue> {
    let replay_file = wh40k_replay::import_json(replay_json)
        .map_err(|e| WasmError::new(format!("Replay import error: {}", e)))?;

    let player = ReplayPlayer::new(replay_file.clone());
    let view = replay_info_to_view(&player, &replay_file);

    REPLAY_PLAYER.with(|cell| cell.replace(Some(player)));
    REPLAY_FILE_CACHE.with(|cell| cell.replace(Some(replay_file)));

    to_json(&view).map_err(JsValue::from)
}

// ===========================================================================
// 12. replay_step_forward
// ===========================================================================

/// Advance the replay player by `count` frames.
///
/// # Returns
/// JSON-encoded `ReplayInfoView` after stepping.
#[wasm_bindgen]
pub fn replay_step_forward(count: u32) -> Result<String, JsValue> {
    REPLAY_PLAYER.with(|player_cell| {
        let mut player_borrow = player_cell.borrow_mut();
        let player = player_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No replay loaded"))?;

        for _ in 0..count {
            if player.has_more() {
                let _ = player.next_frame();
            } else {
                break;
            }
        }

        REPLAY_FILE_CACHE.with(|file_cell| {
            let file_borrow = file_cell.borrow();
            let replay_file = file_borrow
                .as_ref()
                .ok_or_else(|| WasmError::new("No replay file cached"))?;
            let view = replay_info_to_view(player, replay_file);
            to_json(&view).map_err(JsValue::from)
        })
    }).map_err(|e: JsValue| e)
}

// ===========================================================================
// 13. replay_step_backward
// ===========================================================================

/// Move the replay player backward by `count` frames.
///
/// Since `ReplayPlayer` only supports forward iteration, this resets to
/// the start and re-advances to `max(0, current - count)`.
///
/// # Returns
/// JSON-encoded `ReplayInfoView` after stepping.
#[wasm_bindgen]
pub fn replay_step_backward(count: u32) -> Result<String, JsValue> {
    REPLAY_PLAYER.with(|player_cell| {
        let mut player_borrow = player_cell.borrow_mut();
        let player = player_borrow
            .as_mut()
            .ok_or_else(|| WasmError::new("No replay loaded"))?;

        let current = player.current_frame_index();
        let target = current.saturating_sub(count as usize);

        // Reset to the beginning and re-advance to the target frame.
        player.reset();
        for _ in 0..target {
            if player.has_more() {
                let _ = player.next_frame();
            } else {
                break;
            }
        }

        REPLAY_FILE_CACHE.with(|file_cell| {
            let file_borrow = file_cell.borrow();
            let replay_file = file_borrow
                .as_ref()
                .ok_or_else(|| WasmError::new("No replay file cached"))?;
            let view = replay_info_to_view(player, replay_file);
            to_json(&view).map_err(JsValue::from)
        })
    }).map_err(|e: JsValue| e)
}

// ===========================================================================
// 14. replay_get_info
// ===========================================================================

/// Return the current replay info without advancing.
///
/// # Returns
/// JSON-encoded `ReplayInfoView`.
#[wasm_bindgen]
pub fn replay_get_info() -> Result<String, JsValue> {
    REPLAY_PLAYER.with(|player_cell| {
        let player_borrow = player_cell.borrow();
        let player = player_borrow
            .as_ref()
            .ok_or_else(|| WasmError::new("No replay loaded"))?;

        REPLAY_FILE_CACHE.with(|file_cell| {
            let file_borrow = file_cell.borrow();
            let replay_file = file_borrow
                .as_ref()
                .ok_or_else(|| WasmError::new("No replay file cached"))?;
            let view = replay_info_to_view(player, replay_file);
            to_json(&view).map_err(JsValue::from)
        })
    }).map_err(|e: JsValue| e)
}
