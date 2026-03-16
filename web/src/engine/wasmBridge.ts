// WASM bridge - typed wrappers around raw wasm-bindgen exports
// This module is meant to run INSIDE the Web Worker

let wasmModule: any = null;
let initialized = false;

export async function initWasm(): Promise<void> {
  if (initialized) return;

  // Dynamic import of the wasm-pack generated module
  const wasm = await import('@wasm/wh40k_wasm_api');
  await wasm.default();
  wasmModule = wasm;
  wasmModule.init_panic_hook();
  initialized = true;
}

export function isInitialized(): boolean {
  return initialized;
}

export function createMatch(
  factionA: number, factionB: number, mission: number, seedJson: string,
  enhancementA: number, enhancementB: number, secondaryA: number, secondaryB: number,
  patrolSquadA: number, patrolSquadB: number
): string {
  return wasmModule.create_match(factionA, factionB, mission, seedJson,
    enhancementA, enhancementB, secondaryA, secondaryB,
    patrolSquadA, patrolSquadB);
}

export function loadScenario(stateJson: string): string {
  return wasmModule.load_scenario(stateJson);
}

export function getStateSnapshot(): string {
  return wasmModule.get_state_snapshot();
}

export function getDecisionSurface(): string {
  return wasmModule.get_decision_surface();
}

export function validateAction(index: number): string {
  return wasmModule.validate_action(index);
}

export function applyAction(index: number): string {
  return wasmModule.apply_action(index);
}

export function runAiDecision(difficulty: string): string {
  return wasmModule.run_ai_decision(difficulty);
}

export function applyAiAction(): string {
  return wasmModule.apply_ai_action();
}

export function exportReplay(): string {
  return wasmModule.export_replay();
}

export function loadReplay(replayJson: string): string {
  return wasmModule.load_replay(replayJson);
}

export function replayStepForward(count: number): string {
  return wasmModule.replay_step_forward(count);
}

export function replayStepBackward(count: number): string {
  return wasmModule.replay_step_backward(count);
}

export function replayGetInfo(): string {
  return wasmModule.replay_get_info();
}

// Direct command submission (bypasses decision cache for free click-to-move/deploy)

export function submitPlaceUnit(player: number, unitId: number, x: number, y: number): string {
  return wasmModule.submit_place_unit(player, unitId, x, y);
}

export function submitNormalMove(unitId: number, x: number, y: number): string {
  return wasmModule.submit_normal_move(unitId, x, y);
}

// Boarding Actions WASM functions

export function getBoardingFactions(): string {
  return wasmModule.get_boarding_factions();
}

export function getBoardingMissions(): string {
  return wasmModule.get_boarding_missions();
}

export function validateBoardingRoster(rosterJson: string, factionJson: string): string {
  return wasmModule.validate_boarding_roster(rosterJson, factionJson);
}

export function createBoardingMatch(configJson: string): string {
  return wasmModule.create_boarding_match(configJson);
}

export function getGameMode(): string {
  return wasmModule.get_game_mode();
}
