// Worker client - Promise-based API for the main thread to communicate with the engine worker

import type { WorkerRequest, WorkerResponse } from '@/types/worker-messages';
import type {
  GameView,
  DecisionSurfaceView,
  ValidationResultView,
  AiResultView,
  ReplayInfoView,
  AiDifficulty,
} from '@/types/game';

type PendingRequest = {
  resolve: (value: any) => void;
  reject: (error: Error) => void;
  expectedType: string;
};

class EngineWorkerClient {
  private worker: Worker | null = null;
  private pendingRequests: PendingRequest[] = [];
  private ready = false;

  async init(): Promise<void> {
    if (this.worker) return;

    this.worker = new Worker(
      new URL('./engineWorker.ts', import.meta.url),
      { type: 'module' },
    );

    this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      this.handleResponse(event.data);
    };

    this.worker.onerror = (error) => {
      console.error('Engine worker error:', error);
      const pending = this.pendingRequests.shift();
      if (pending) {
        pending.reject(new Error(`Worker error: ${error.message}`));
      }
    };

    return this.sendRequest({ type: 'init' }, 'ready').then(() => {
      this.ready = true;
    });
  }

  isReady(): boolean {
    return this.ready;
  }

  private handleResponse(response: WorkerResponse): void {
    if (response.type === 'error') {
      const pending = this.pendingRequests.shift();
      if (pending) {
        pending.reject(new Error(response.message));
      }
      return;
    }

    const pending = this.pendingRequests.shift();
    if (pending) {
      if ('data' in response) {
        pending.resolve(JSON.parse(response.data));
      } else {
        pending.resolve(undefined);
      }
    }
  }

  private sendRequest<T>(request: WorkerRequest, expectedType: string): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.worker) {
        reject(new Error('Worker not initialized'));
        return;
      }
      this.pendingRequests.push({ resolve, reject, expectedType });
      this.worker.postMessage(request);
    });
  }

  async createMatch(
    factionA: number,
    factionB: number,
    mission: number,
    seedJson: string,
    enhancementA: number = -1,
    enhancementB: number = -1,
    secondaryA: number = -1,
    secondaryB: number = -1,
    patrolSquadA: number = -1,
    patrolSquadB: number = -1,
  ): Promise<GameView> {
    return this.sendRequest<GameView>(
      {
        type: 'create_match',
        faction_a: factionA,
        faction_b: factionB,
        mission,
        seed_json: seedJson,
        enhancement_a: enhancementA,
        enhancement_b: enhancementB,
        secondary_a: secondaryA,
        secondary_b: secondaryB,
        patrol_squad_a: patrolSquadA,
        patrol_squad_b: patrolSquadB,
      },
      'state',
    );
  }

  async loadScenario(stateJson: string): Promise<GameView> {
    return this.sendRequest<GameView>(
      { type: 'load_scenario', state_json: stateJson },
      'state',
    );
  }

  async getState(): Promise<GameView> {
    return this.sendRequest<GameView>({ type: 'get_state' }, 'state');
  }

  async getDecisionSurface(): Promise<DecisionSurfaceView> {
    return this.sendRequest<DecisionSurfaceView>(
      { type: 'get_decision_surface' },
      'decision_surface',
    );
  }

  async validateAction(index: number): Promise<ValidationResultView> {
    return this.sendRequest<ValidationResultView>(
      { type: 'validate_action', index },
      'validation',
    );
  }

  async applyAction(index: number): Promise<GameView> {
    return this.sendRequest<GameView>(
      { type: 'apply_action', index },
      'action_applied',
    );
  }

  async runAi(difficulty: AiDifficulty): Promise<AiResultView> {
    return this.sendRequest<AiResultView>(
      { type: 'run_ai', difficulty },
      'ai_result',
    );
  }

  async applyAiAction(): Promise<GameView> {
    return this.sendRequest<GameView>(
      { type: 'apply_ai_action' },
      'ai_action_applied',
    );
  }

  async exportReplay(): Promise<string> {
    return this.sendRequest<string>(
      { type: 'export_replay' },
      'replay_exported',
    );
  }

  async loadReplay(replayJson: string): Promise<ReplayInfoView> {
    return this.sendRequest<ReplayInfoView>(
      { type: 'load_replay', replay_json: replayJson },
      'replay_loaded',
    );
  }

  async replayStepForward(count: number): Promise<ReplayInfoView> {
    return this.sendRequest<ReplayInfoView>(
      { type: 'replay_step_forward', count },
      'replay_stepped',
    );
  }

  async replayStepBackward(count: number): Promise<ReplayInfoView> {
    return this.sendRequest<ReplayInfoView>(
      { type: 'replay_step_backward', count },
      'replay_stepped',
    );
  }

  async replayGetInfo(): Promise<ReplayInfoView> {
    return this.sendRequest<ReplayInfoView>(
      { type: 'replay_get_info' },
      'replay_info',
    );
  }

  async submitPlaceUnit(player: number, unitId: number, x: number, y: number): Promise<GameView> {
    return this.sendRequest<GameView>(
      { type: 'submit_place_unit', player, unit_id: unitId, x, y },
      'direct_command_applied',
    );
  }

  async submitNormalMove(unitId: number, x: number, y: number): Promise<GameView> {
    return this.sendRequest<GameView>(
      { type: 'submit_normal_move', unit_id: unitId, x, y },
      'direct_command_applied',
    );
  }

  terminate(): void {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
      this.ready = false;
    }
  }
}

// Singleton instance
export const engineClient = new EngineWorkerClient();
