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
    screen: 'setup',
    engineReady: false,
    gameState: null,
    decisionSurface: null,
    selectedUnitId: null,
    targetUnitId: null,
    hoveredUnitId: null,
    selectedActionIndex: null,
    aiDifficulty: 'greedy',
    aiThinking: false,
    aiResult: null,
    autoPlayAi: false,
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
        const gameView = await engineClient.createMatch(factionA, factionB, mission, seedJson);

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
      } catch (error) {
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
        });
      }
    },

    runAiTurn: async () => {
      const state = get();
      if (!state.gameState?.in_progress) return;

      await state.runAi();
      if (get().aiResult) {
        await get().applyAiAction();
      }
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
