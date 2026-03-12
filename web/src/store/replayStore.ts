import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type { ReplayInfoView } from '@/types/game';
import { engineClient } from '@/engine/workerClient';

interface ReplayState {
  replayLoaded: boolean;
  replayInfo: ReplayInfoView | null;
  isPlaying: boolean;
  playbackSpeed: number;
  playbackTimer: ReturnType<typeof setInterval> | null;

  // Actions
  loadReplay: (replayJson: string) => Promise<void>;
  stepForward: (count?: number) => Promise<void>;
  stepBackward: (count?: number) => Promise<void>;
  goToStart: () => Promise<void>;
  goToEnd: () => Promise<void>;
  togglePlay: () => void;
  setPlaybackSpeed: (speed: number) => void;
  refreshInfo: () => Promise<void>;
  unload: () => void;
}

export const useReplayStore = create<ReplayState>()(
  immer((set, get) => ({
    replayLoaded: false,
    replayInfo: null,
    isPlaying: false,
    playbackSpeed: 1,
    playbackTimer: null,

    loadReplay: async (replayJson) => {
      const info = await engineClient.loadReplay(replayJson);
      set((state) => {
        state.replayLoaded = true;
        state.replayInfo = info;
        state.isPlaying = false;
      });
    },

    stepForward: async (count = 1) => {
      const info = await engineClient.replayStepForward(count);
      set((state) => {
        state.replayInfo = info;
        if (!info.has_more) {
          // Stop playback when we reach the end
          if (state.playbackTimer) {
            clearInterval(state.playbackTimer);
            state.playbackTimer = null;
          }
          state.isPlaying = false;
        }
      });
    },

    stepBackward: async (count = 1) => {
      const info = await engineClient.replayStepBackward(count);
      set((state) => {
        state.replayInfo = info;
      });
    },

    goToStart: async () => {
      const info = await engineClient.replayStepBackward(999999);
      set((state) => {
        state.replayInfo = info;
      });
    },

    goToEnd: async () => {
      const info = await engineClient.replayStepForward(999999);
      set((state) => {
        state.replayInfo = info;
      });
    },

    togglePlay: () => {
      const state = get();
      if (state.isPlaying) {
        // Stop
        if (state.playbackTimer) {
          clearInterval(state.playbackTimer);
        }
        set((s) => {
          s.isPlaying = false;
          s.playbackTimer = null;
        });
      } else {
        // Start
        const interval = 1000 / state.playbackSpeed;
        const timer = setInterval(() => {
          get().stepForward(1);
        }, interval);
        set((s) => {
          s.isPlaying = true;
          s.playbackTimer = timer;
        });
      }
    },

    setPlaybackSpeed: (speed) => {
      set((state) => {
        state.playbackSpeed = speed;
        // If playing, restart the timer with new speed
        if (state.isPlaying && state.playbackTimer) {
          clearInterval(state.playbackTimer);
          const interval = 1000 / speed;
          state.playbackTimer = setInterval(() => {
            get().stepForward(1);
          }, interval);
        }
      });
    },

    refreshInfo: async () => {
      const info = await engineClient.replayGetInfo();
      set((state) => {
        state.replayInfo = info;
      });
    },

    unload: () => {
      const state = get();
      if (state.playbackTimer) {
        clearInterval(state.playbackTimer);
      }
      set((s) => {
        s.replayLoaded = false;
        s.replayInfo = null;
        s.isPlaying = false;
        s.playbackTimer = null;
      });
    },
  })),
);
