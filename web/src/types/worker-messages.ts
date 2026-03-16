import type { AiDifficulty } from './game';

// Messages from main thread to worker
export type WorkerRequest =
  | { type: 'init' }
  | { type: 'create_match'; faction_a: number; faction_b: number; mission: number; seed_json: string; enhancement_a?: number; enhancement_b?: number; secondary_a?: number; secondary_b?: number; patrol_squad_a?: number; patrol_squad_b?: number }
  | { type: 'load_scenario'; state_json: string }
  | { type: 'get_state' }
  | { type: 'get_decision_surface' }
  | { type: 'validate_action'; index: number }
  | { type: 'apply_action'; index: number }
  | { type: 'run_ai'; difficulty: AiDifficulty }
  | { type: 'apply_ai_action' }
  | { type: 'export_replay' }
  | { type: 'load_replay'; replay_json: string }
  | { type: 'replay_step_forward'; count: number }
  | { type: 'replay_step_backward'; count: number }
  | { type: 'replay_get_info' }
  | { type: 'submit_place_unit'; player: number; unit_id: number; x: number; y: number }
  | { type: 'submit_normal_move'; unit_id: number; x: number; y: number }
  | { type: 'get_boarding_factions' }
  | { type: 'get_boarding_missions' }
  | { type: 'validate_boarding_roster'; roster_json: string; faction_json: string }
  | { type: 'create_boarding_match'; config_json: string }
  | { type: 'get_game_mode' };

// Messages from worker to main thread
export type WorkerResponse =
  | { type: 'ready' }
  | { type: 'state'; data: string }
  | { type: 'decision_surface'; data: string }
  | { type: 'validation'; data: string }
  | { type: 'action_applied'; data: string }
  | { type: 'ai_result'; data: string }
  | { type: 'ai_action_applied'; data: string }
  | { type: 'replay_exported'; data: string }
  | { type: 'replay_loaded'; data: string }
  | { type: 'replay_stepped'; data: string }
  | { type: 'replay_info'; data: string }
  | { type: 'direct_command_applied'; data: string }
  | { type: 'boarding_factions'; data: string }
  | { type: 'boarding_missions'; data: string }
  | { type: 'boarding_roster_validation'; data: string }
  | { type: 'game_mode'; data: string }
  | { type: 'error'; message: string; request_type: string };
