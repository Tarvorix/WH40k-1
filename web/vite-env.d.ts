/// <reference types="vite/client" />

declare module '*.wasm' {
  const content: WebAssembly.Module;
  export default content;
}

declare module '@wasm/wh40k_wasm_api' {
  export default function init(): Promise<void>;
  export function init_panic_hook(): void;
  export function create_match(faction_a: number, faction_b: number, mission: number, seed_json: string): string;
  export function load_scenario(state_json: string): string;
  export function get_state_snapshot(): string;
  export function get_decision_surface(): string;
  export function validate_action(index: number): string;
  export function apply_action(index: number): string;
  export function run_ai_decision(difficulty: string): string;
  export function apply_ai_action(): string;
  export function export_replay(): string;
  export function load_replay(replay_json: string): string;
  export function replay_step_forward(count: number): string;
  export function replay_step_backward(count: number): string;
  export function replay_get_info(): string;
}
