import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';

// Faction IDs (matching engine FactionId values)
export const FACTIONS = {
  CUSTODES: 0,
  WORLD_EATERS: 1,
} as const;

// Enhancement IDs
export const ENHANCEMENTS = {
  CUSTODES: [
    { id: 0, name: 'Ceaseless Hunter', description: 'Blade Champion re-rolls Advance and Charge rolls.' },
    { id: 1, name: 'Unstoppable Destroyer', description: 'Blade Champion: melee attacks of S7+, AP-3 gain [DEVASTATING WOUNDS].' },
  ],
  WORLD_EATERS: [
    { id: 0, name: 'Berzerker Glaive', description: 'Master of Executions gains +1 to hit on the charge.' },
    { id: 1, name: 'Helm of Brazen Ire', description: 'Master of Executions: 4+ Feel No Pain against mortal wounds.' },
  ],
} as const;

// Secondary Objective IDs
export const SECONDARIES = {
  CUSTODES: [
    { id: 0, name: 'Auric Mortalis', description: 'Score VP for destroying enemy CHARACTER units.' },
    { id: 1, name: 'Warrior of the Imperium', description: 'Score VP for controlling objectives in enemy territory.' },
  ],
  WORLD_EATERS: [
    { id: 0, name: 'Blood Tithe', description: 'Score VP for destroying enemy units in the Fight phase.' },
    { id: 1, name: 'Skull Throne', description: 'Score VP for destroying the most expensive enemy unit.' },
  ],
} as const;

// Mission definitions
export const MISSIONS = [
  { id: 0, name: 'Sites of Power', description: 'Control objectives to score VP. Central objective is worth double.' },
  { id: 1, name: 'Supply Drop', description: 'Objectives appear at the start of rounds 2 and 4.' },
  { id: 2, name: 'Purge the Foe', description: 'Score VP for each enemy unit destroyed. Bonus for tabling.' },
  { id: 3, name: 'The Ritual', description: 'Score VP by performing actions on objectives.' },
  { id: 4, name: 'Scorched Earth', description: 'Score VP for controlling or razing objectives.' },
  { id: 5, name: 'Take and Hold', description: 'Score VP for each objective you control at the end of your turn.' },
] as const;

type SetupStep = 'faction_select' | 'enhancement_select' | 'secondary_select' | 'mission_select' | 'ready';

interface SetupState {
  step: SetupStep;
  playerFaction: number | null;
  opponentFaction: number | null;
  playerEnhancement: number | null;
  playerSecondary: number | null;
  missionId: number | null;

  // Actions
  setStep: (step: SetupStep) => void;
  selectFaction: (faction: number) => void;
  selectEnhancement: (enhancementId: number) => void;
  selectSecondary: (secondaryId: number) => void;
  selectMission: (missionId: number) => void;
  reset: () => void;
  isReady: () => boolean;
}

export const useSetupStore = create<SetupState>()(
  immer((set, get) => ({
    step: 'faction_select',
    playerFaction: null,
    opponentFaction: null,
    playerEnhancement: null,
    playerSecondary: null,
    missionId: null,

    setStep: (step) => {
      set((state) => {
        state.step = step;
      });
    },

    selectFaction: (faction) => {
      set((state) => {
        state.playerFaction = faction;
        state.opponentFaction = faction === FACTIONS.CUSTODES ? FACTIONS.WORLD_EATERS : FACTIONS.CUSTODES;
        state.step = 'enhancement_select';
      });
    },

    selectEnhancement: (enhancementId) => {
      set((state) => {
        state.playerEnhancement = enhancementId;
        state.step = 'secondary_select';
      });
    },

    selectSecondary: (secondaryId) => {
      set((state) => {
        state.playerSecondary = secondaryId;
        state.step = 'mission_select';
      });
    },

    selectMission: (missionId) => {
      set((state) => {
        state.missionId = missionId;
        state.step = 'ready';
      });
    },

    reset: () => {
      set((state) => {
        state.step = 'faction_select';
        state.playerFaction = null;
        state.opponentFaction = null;
        state.playerEnhancement = null;
        state.playerSecondary = null;
        state.missionId = null;
      });
    },

    isReady: () => {
      const s = get();
      return s.playerFaction !== null &&
        s.playerEnhancement !== null &&
        s.playerSecondary !== null &&
        s.missionId !== null;
    },
  })),
);
