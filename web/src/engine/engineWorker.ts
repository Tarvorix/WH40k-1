// Engine Web Worker - owns the WASM module and handles all engine calls
// This runs in a separate thread to keep the UI responsive

import type { WorkerRequest, WorkerResponse } from '@/types/worker-messages';
import * as bridge from './wasmBridge';

function sendResponse(response: WorkerResponse): void {
  self.postMessage(response);
}

function sendError(requestType: string, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  sendResponse({ type: 'error', message, request_type: requestType });
}

async function handleMessage(request: WorkerRequest): Promise<void> {
  try {
    switch (request.type) {
      case 'init': {
        await bridge.initWasm();
        sendResponse({ type: 'ready' });
        break;
      }
      case 'create_match': {
        const data = bridge.createMatch(
          request.faction_a,
          request.faction_b,
          request.mission,
          request.seed_json,
          request.enhancement_a ?? -1,
          request.enhancement_b ?? -1,
          request.secondary_a ?? -1,
          request.secondary_b ?? -1,
          request.patrol_squad_a ?? -1,
          request.patrol_squad_b ?? -1,
        );
        sendResponse({ type: 'state', data });
        break;
      }
      case 'load_scenario': {
        const data = bridge.loadScenario(request.state_json);
        sendResponse({ type: 'state', data });
        break;
      }
      case 'get_state': {
        const data = bridge.getStateSnapshot();
        sendResponse({ type: 'state', data });
        break;
      }
      case 'get_decision_surface': {
        const data = bridge.getDecisionSurface();
        sendResponse({ type: 'decision_surface', data });
        break;
      }
      case 'validate_action': {
        const data = bridge.validateAction(request.index);
        sendResponse({ type: 'validation', data });
        break;
      }
      case 'apply_action': {
        const data = bridge.applyAction(request.index);
        sendResponse({ type: 'action_applied', data });
        break;
      }
      case 'run_ai': {
        const data = bridge.runAiDecision(request.difficulty);
        sendResponse({ type: 'ai_result', data });
        break;
      }
      case 'apply_ai_action': {
        const data = bridge.applyAiAction();
        sendResponse({ type: 'ai_action_applied', data });
        break;
      }
      case 'export_replay': {
        const data = bridge.exportReplay();
        sendResponse({ type: 'replay_exported', data });
        break;
      }
      case 'load_replay': {
        const data = bridge.loadReplay(request.replay_json);
        sendResponse({ type: 'replay_loaded', data });
        break;
      }
      case 'replay_step_forward': {
        const data = bridge.replayStepForward(request.count);
        sendResponse({ type: 'replay_stepped', data });
        break;
      }
      case 'replay_step_backward': {
        const data = bridge.replayStepBackward(request.count);
        sendResponse({ type: 'replay_stepped', data });
        break;
      }
      case 'replay_get_info': {
        const data = bridge.replayGetInfo();
        sendResponse({ type: 'replay_info', data });
        break;
      }
      case 'submit_place_unit': {
        const data = bridge.submitPlaceUnit(request.player, request.unit_id, request.x, request.y);
        sendResponse({ type: 'direct_command_applied', data });
        break;
      }
      case 'submit_normal_move': {
        const data = bridge.submitNormalMove(request.unit_id, request.x, request.y);
        sendResponse({ type: 'direct_command_applied', data });
        break;
      }
    }
  } catch (error) {
    sendError(request.type, error);
  }
}

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  handleMessage(event.data);
};
