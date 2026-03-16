import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type {
  GameView,
  DecisionSurfaceView,
  ActionView,
  AiResultView,
  AiDifficulty,
  Screen,
} from '@/types/game';
import { engineClient } from '@/engine/workerClient';

interface GameState {
  // Routing
  screen: Screen;

  // Engine state
  engineReady: boolean;
  gameState: GameView | null;
  decisionSurface: DecisionSurfaceView | null;

  // Selection state
  selectedUnitId: number | null;
  targetUnitId: number | null;
  hoveredUnitId: number | null;
  selectedActionIndex: number | null;

  // AI
  aiDifficulty: AiDifficulty;
  aiThinking: boolean;
  aiResult: AiResultView | null;
  autoPlayAi: boolean;
  // Which players are AI-controlled: [player0, player1]
  playerControlled: [boolean, boolean];

  // Event log
  eventLog: string[];

  // Loading states
  loading: boolean;
  error: string | null;

  // Actions
  setScreen: (screen: Screen) => void;
  initEngine: () => Promise<void>;
  createMatch: (factionA: number, factionB: number, mission: number, seed?: number) => Promise<void>;
  refreshState: () => Promise<void>;
  refreshDecisionSurface: () => Promise<void>;
  selectUnit: (unitId: number | null) => void;
  setTargetUnit: (unitId: number | null) => void;
  setHoveredUnit: (unitId: number | null) => void;
  selectAction: (index: number | null) => void;
  applyAction: (index: number) => Promise<void>;
  submitDeploy: (unitId: number, x: number, y: number) => Promise<void>;
  submitMove: (unitId: number, x: number, y: number) => Promise<void>;
  setAiDifficulty: (difficulty: AiDifficulty) => void;
  setAutoPlayAi: (autoPlay: boolean) => void;
  setPlayerControlled: (playerIndex: number, isHuman: boolean) => void;
  runAi: () => Promise<void>;
  applyAiAction: () => Promise<void>;
  runAiTurn: () => Promise<void>;
  clearError: () => void;
  addEvent: (message: string) => void;
}

export const useGameStore = create<GameState>()(
  immer((set, get) => ({
    // Initial state
    screen: 'menu',
    engineReady: false,
    gameState: null,
    decisionSurface: null,
    selectedUnitId: null,
    targetUnitId: null,
    hoveredUnitId: null,
    selectedActionIndex: null,
    aiDifficulty: 'Basic_Recruit',
    aiThinking: false,
    aiResult: null,
    autoPlayAi: true,
    playerControlled: [true, false], // Player 0 = human, Player 1 = AI
    eventLog: [],
    loading: false,
    error: null,

    setScreen: (screen) => {
      set((state) => {
        state.screen = screen;
      });
    },

    initEngine: async () => {
      try {
        set((state) => { state.loading = true; });
        await engineClient.init();
        set((state) => {
          state.engineReady = true;
          state.loading = false;
        });
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to initialize engine';
          state.loading = false;
        });
      }
    },

    createMatch: async (factionA, factionB, mission, seed) => {
      try {
        set((state) => {
          state.loading = true;
          state.error = null;
          state.eventLog = [];
        });

        const seedValue = seed ?? Math.floor(Math.random() * Number.MAX_SAFE_INTEGER);
        const seedJson = JSON.stringify({ seed_u64: seedValue });

        // Get enhancement, secondary, and patrol squad selections from setup store
        const { useSetupStore } = await import('./setupStore');
        const setupState = useSetupStore.getState();
        const enhA = setupState.playerEnhancement ?? -1;
        const enhB = -1; // AI enhancement selected by engine
        const secA = setupState.playerSecondary ?? -1;
        const secB = -1; // AI secondary selected by engine
        const sqA = setupState.playerPatrolSquad ?? -1;
        const sqB = -1; // AI patrol squad selected by engine (defaults to Wardens)

        console.log('[createMatch] Calling engine with params:', { factionA, factionB, mission, enhA, enhB, secA, secB, sqA, sqB });
        const gameView = await engineClient.createMatch(factionA, factionB, mission, seedJson, enhA, enhB, secA, secB, sqA, sqB);
        console.log('[createMatch] Got gameView:', {
          phase: gameView.phase,
          subphase: gameView.subphase,
          decision_owner: gameView.decision_owner,
          in_progress: gameView.in_progress,
          units: gameView.units.length,
          deployment_zones: gameView.board.deployment_zones.length,
          undeployed: gameView.units.filter((u: any) => u.status === 'Undeployed').length,
        });

        set((state) => {
          state.gameState = gameView;
          state.loading = false;
          state.screen = 'play';
          state.selectedUnitId = null;
          state.targetUnitId = null;
          state.selectedActionIndex = null;
        });

        // Auto-fetch decision surface
        await get().refreshDecisionSurface();
        const ds = get().decisionSurface;
        console.log('[createMatch] Decision surface after refresh:', {
          actions: ds?.actions.length,
          owner: ds?.owner,
          decision_type: ds?.decision_type,
          first_action: ds?.actions[0],
        });

        // If the AI is the initial decision owner (e.g. defender deploys first),
        // auto-run AI turns until it's the human player's turn
        const gs = get().gameState;
        const pc = get().playerControlled;
        console.log('[createMatch] AI check:', {
          decision_owner: gs?.decision_owner,
          playerControlled: pc,
          autoPlayAi: get().autoPlayAi,
          shouldRunAi: gs && !pc[gs.decision_owner] && get().autoPlayAi,
        });
        if (gs && !pc[gs.decision_owner] && get().autoPlayAi) {
          console.log('[createMatch] Starting AI turn loop for deployment...');
          await get().runAiTurn();
          console.log('[createMatch] AI turn loop finished. New state:', {
            decision_owner: get().gameState?.decision_owner,
            phase: get().gameState?.phase,
            undeployed: get().gameState?.units.filter((u: any) => u.status === 'Undeployed').length,
            surface_actions: get().decisionSurface?.actions.length,
          });
        }
      } catch (error) {
        console.error('[createMatch] ERROR:', error);
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to create match';
          state.loading = false;
        });
      }
    },

    refreshState: async () => {
      try {
        const gameView = await engineClient.getState();
        set((state) => {
          state.gameState = gameView;
          // Check for game end
          if (!gameView.in_progress) {
            state.screen = 'game_end';
          }
        });
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to refresh state';
        });
      }
    },

    refreshDecisionSurface: async () => {
      try {
        const surface = await engineClient.getDecisionSurface();
        set((state) => {
          state.decisionSurface = surface;
          state.selectedActionIndex = null;
        });
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to get decision surface';
        });
      }
    },

    selectUnit: (unitId) => {
      set((state) => {
        state.selectedUnitId = unitId;
        state.selectedActionIndex = null;
      });
    },

    setTargetUnit: (unitId) => {
      set((state) => {
        state.targetUnitId = unitId;
      });
    },

    setHoveredUnit: (unitId) => {
      set((state) => {
        state.hoveredUnitId = unitId;
      });
    },

    selectAction: (index) => {
      set((state) => {
        state.selectedActionIndex = index;
      });
    },

    applyAction: async (index) => {
      try {
        set((state) => { state.loading = true; });
        const gameView = await engineClient.applyAction(index);
        set((state) => {
          state.gameState = gameView;
          state.loading = false;
          state.selectedUnitId = null;
          state.targetUnitId = null;
          state.selectedActionIndex = null;
          state.decisionSurface = null;
          // Check for game end
          if (!gameView.in_progress) {
            state.screen = 'game_end';
          }
        });

        // Refresh decision surface for next action
        if (get().gameState?.in_progress) {
          await get().refreshDecisionSurface();

          // Check if it's an AI player's turn
          const gs = get().gameState;
          const pc = get().playerControlled;
          if (gs && !pc[gs.decision_owner] && get().autoPlayAi) {
            await get().runAiTurn();
          }
        }
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to apply action';
          state.loading = false;
        });
      }
    },

    submitDeploy: async (unitId, x, y) => {
      try {
        set((state) => { state.loading = true; });
        const decisionOwner = get().gameState?.decision_owner ?? 0;
        const gameView = await engineClient.submitPlaceUnit(decisionOwner, unitId, x, y);
        set((state) => {
          state.gameState = gameView;
          state.loading = false;
          state.selectedUnitId = null;
          state.targetUnitId = null;
          state.selectedActionIndex = null;
          state.decisionSurface = null;
          if (!gameView.in_progress) {
            state.screen = 'game_end';
          }
        });

        // Refresh decision surface for next action
        if (get().gameState?.in_progress) {
          await get().refreshDecisionSurface();

          // Check if it's an AI player's turn
          const gs = get().gameState;
          const pc = get().playerControlled;
          if (gs && !pc[gs.decision_owner] && get().autoPlayAi) {
            await get().runAiTurn();
          }
        }
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Invalid deployment position';
          state.loading = false;
        });
      }
    },

    submitMove: async (unitId, x, y) => {
      try {
        set((state) => { state.loading = true; });
        const gameView = await engineClient.submitNormalMove(unitId, x, y);
        set((state) => {
          state.gameState = gameView;
          state.loading = false;
          state.selectedUnitId = null;
          state.targetUnitId = null;
          state.selectedActionIndex = null;
          state.decisionSurface = null;
          if (!gameView.in_progress) {
            state.screen = 'game_end';
          }
        });

        // Refresh decision surface for next action
        if (get().gameState?.in_progress) {
          await get().refreshDecisionSurface();

          // Check if it's an AI player's turn
          const gs = get().gameState;
          const pc = get().playerControlled;
          if (gs && !pc[gs.decision_owner] && get().autoPlayAi) {
            await get().runAiTurn();
          }
        }
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Invalid move position';
          state.loading = false;
        });
      }
    },

    setAiDifficulty: (difficulty) => {
      set((state) => {
        state.aiDifficulty = difficulty;
      });
    },

    setAutoPlayAi: (autoPlay) => {
      set((state) => {
        state.autoPlayAi = autoPlay;
      });
    },

    setPlayerControlled: (playerIndex, isHuman) => {
      set((state) => {
        state.playerControlled[playerIndex] = isHuman;
      });
    },

    runAi: async () => {
      try {
        set((state) => {
          state.aiThinking = true;
          state.error = null;
        });
        const result = await engineClient.runAi(get().aiDifficulty);
        set((state) => {
          state.aiResult = result;
          state.aiThinking = false;
        });
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'AI search failed';
          state.aiThinking = false;
          state.aiResult = null;
        });
      }
    },

    applyAiAction: async () => {
      try {
        set((state) => { state.loading = true; });
        const gameView = await engineClient.applyAiAction();
        set((state) => {
          state.gameState = gameView;
          state.loading = false;
          state.aiResult = null;
          state.selectedUnitId = null;
          state.targetUnitId = null;
          state.selectedActionIndex = null;
          state.decisionSurface = null;
          if (!gameView.in_progress) {
            state.screen = 'game_end';
          }
        });

        if (get().gameState?.in_progress) {
          await get().refreshDecisionSurface();
        }
      } catch (error) {
        set((state) => {
          state.error = error instanceof Error ? error.message : 'Failed to apply AI action';
          state.loading = false;
          state.aiResult = null;
        });
      }
    },

    runAiTurn: async () => {
      // Loop to handle consecutive AI decisions (e.g. deploying all units)
      // without recursive calls that could stack overflow
      const MAX_AI_ITERATIONS = 200;
      let iteration = 0;
      let lastActionLabel = '';
      let sameActionCount = 0;
      while (iteration < MAX_AI_ITERATIONS) {
        iteration++;
        const current = get();
        if (!current.gameState?.in_progress) {
          console.log(`[runAiTurn] iter=${iteration}: game not in progress, breaking`);
          break;
        }

        const gs = current.gameState;
        const pc = current.playerControlled;
        // Stop if it's a human player's turn
        if (pc[gs.decision_owner]) {
          console.log(`[runAiTurn] iter=${iteration}: human turn (owner=${gs.decision_owner}), breaking`);
          break;
        }
        if (!current.autoPlayAi) {
          console.log(`[runAiTurn] iter=${iteration}: autoPlayAi=false, breaking`);
          break;
        }

        console.log(`[runAiTurn] iter=${iteration}: running AI for owner=${gs.decision_owner}, phase=${gs.phase}`);
        await current.runAi();
        if (!get().aiResult) {
          console.log(`[runAiTurn] iter=${iteration}: AI returned no result, breaking`);
          break;
        }
        const actionLabel = get().aiResult?.best_action_label ?? '';
        console.log(`[runAiTurn] iter=${iteration}: applying AI action: ${actionLabel}`);

        // Detect repeated identical actions (stuck loop)
        if (actionLabel === lastActionLabel) {
          sameActionCount++;
          if (sameActionCount >= 3) {
            console.error(`[runAiTurn] iter=${iteration}: same action repeated ${sameActionCount} times ("${actionLabel}"), breaking to prevent infinite loop`);
            set((state) => {
              state.error = `AI stuck: repeated "${actionLabel}" ${sameActionCount} times`;
              state.aiResult = null;
              state.loading = false;
            });
            break;
          }
        } else {
          lastActionLabel = actionLabel;
          sameActionCount = 1;
        }

        try {
          await get().applyAiAction();
        } catch (error) {
          console.error(`[runAiTurn] iter=${iteration}: applyAiAction failed:`, error);
          set((state) => {
            state.error = error instanceof Error ? error.message : 'AI action failed';
            state.aiResult = null;
            state.loading = false;
          });
          break;
        }
      }
      if (iteration >= MAX_AI_ITERATIONS) {
        console.error(`[runAiTurn] hit max iterations (${MAX_AI_ITERATIONS}), breaking`);
        set((state) => {
          state.error = `AI exceeded maximum iterations (${MAX_AI_ITERATIONS})`;
          state.aiResult = null;
          state.loading = false;
        });
      }
      console.log(`[runAiTurn] finished after ${iteration} iterations`);
    },

    clearError: () => {
      set((state) => {
        state.error = null;
      });
    },

    addEvent: (message) => {
      set((state) => {
        state.eventLog.push(message);
        // Keep last 200 events
        if (state.eventLog.length > 200) {
          state.eventLog = state.eventLog.slice(-200);
        }
      });
    },
  })),
);
