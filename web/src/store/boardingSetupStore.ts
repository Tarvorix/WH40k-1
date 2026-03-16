import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import type {
  BoardingFaction,
  BoardingDetachment,
  SelectedUnit,
  EnhancementAssignment,
  BoardingMissionSummary,
  BoardingRosterValidation,
} from '@/types/game';

export type BoardingSetupStep =
  | 'select_faction'
  | 'select_detachment'
  | 'build_army'
  | 'select_enhancements'
  | 'designate_warlord'
  | 'opponent_setup'
  | 'select_mission'
  | 'ready';

interface BoardingSetupState {
  step: BoardingSetupStep;

  // Available data
  factions: BoardingFaction[];
  missions: BoardingMissionSummary[];

  // Player selections
  playerFaction: BoardingFaction | null;
  playerDetachment: BoardingDetachment | null;
  selectedUnits: SelectedUnit[];
  enhancements: EnhancementAssignment[];
  warlordIndex: number | null;
  totalPoints: number;

  // Opponent (AI for now)
  opponentFaction: BoardingFaction | null;
  opponentDetachment: BoardingDetachment | null;

  // Mission
  selectedMission: BoardingMissionSummary | null;

  // Validation
  validation: BoardingRosterValidation | null;

  // Actions
  setStep: (step: BoardingSetupStep) => void;
  setFactions: (factions: BoardingFaction[]) => void;
  setMissions: (missions: BoardingMissionSummary[]) => void;
  selectFaction: (faction: BoardingFaction) => void;
  selectDetachment: (detachment: BoardingDetachment) => void;
  addUnit: (unit: SelectedUnit) => void;
  removeUnit: (index: number) => void;
  updateUnitSize: (index: number, modelCount: number, points: number) => void;
  setWarlord: (index: number) => void;
  addEnhancement: (assignment: EnhancementAssignment) => void;
  removeEnhancement: (index: number) => void;
  selectMission: (mission: BoardingMissionSummary) => void;
  setOpponent: (faction: BoardingFaction, detachment: BoardingDetachment) => void;
  setValidation: (result: BoardingRosterValidation) => void;
  reset: () => void;
}

const initialState = {
  step: 'select_faction' as BoardingSetupStep,
  factions: [] as BoardingFaction[],
  missions: [] as BoardingMissionSummary[],
  playerFaction: null as BoardingFaction | null,
  playerDetachment: null as BoardingDetachment | null,
  selectedUnits: [] as SelectedUnit[],
  enhancements: [] as EnhancementAssignment[],
  warlordIndex: null as number | null,
  totalPoints: 0,
  opponentFaction: null as BoardingFaction | null,
  opponentDetachment: null as BoardingDetachment | null,
  selectedMission: null as BoardingMissionSummary | null,
  validation: null as BoardingRosterValidation | null,
};

export const useBoardingSetupStore = create<BoardingSetupState>()(
  immer((set) => ({
    ...initialState,

    setStep: (step) => {
      set((state) => {
        state.step = step;
      });
    },

    setFactions: (factions) => {
      set((state) => {
        state.factions = factions;
      });
    },

    setMissions: (missions) => {
      set((state) => {
        state.missions = missions;
      });
    },

    selectFaction: (faction) => {
      set((state) => {
        state.playerFaction = faction;
        state.playerDetachment = null;
        state.selectedUnits = [];
        state.enhancements = [];
        state.warlordIndex = null;
        state.totalPoints = 0;
        state.step = 'select_detachment';
      });
    },

    selectDetachment: (detachment) => {
      set((state) => {
        state.playerDetachment = detachment;
        state.selectedUnits = [];
        state.enhancements = [];
        state.warlordIndex = null;
        state.totalPoints = 0;
        state.step = 'build_army';
      });
    },

    addUnit: (unit) => {
      set((state) => {
        state.selectedUnits.push(unit);
        state.totalPoints = state.selectedUnits.reduce((sum, u) => sum + u.points, 0);
      });
    },

    removeUnit: (index) => {
      set((state) => {
        state.selectedUnits.splice(index, 1);
        // Adjust warlord index
        if (state.warlordIndex !== null) {
          if (state.warlordIndex === index) {
            state.warlordIndex = null;
          } else if (state.warlordIndex > index) {
            state.warlordIndex -= 1;
          }
        }
        // Adjust enhancement assignments
        state.enhancements = state.enhancements
          .filter((e) => e.unit_index !== index)
          .map((e) => ({
            ...e,
            unit_index: e.unit_index > index ? e.unit_index - 1 : e.unit_index,
          }));
        state.totalPoints = state.selectedUnits.reduce((sum, u) => sum + u.points, 0);
      });
    },

    updateUnitSize: (index, modelCount, points) => {
      set((state) => {
        if (index >= 0 && index < state.selectedUnits.length) {
          state.selectedUnits[index].model_count = modelCount;
          state.selectedUnits[index].points = points;
          state.totalPoints = state.selectedUnits.reduce((sum, u) => sum + u.points, 0);
        }
      });
    },

    setWarlord: (index) => {
      set((state) => {
        state.warlordIndex = index;
      });
    },

    addEnhancement: (assignment) => {
      set((state) => {
        state.enhancements.push(assignment);
      });
    },

    removeEnhancement: (index) => {
      set((state) => {
        state.enhancements.splice(index, 1);
      });
    },

    selectMission: (mission) => {
      set((state) => {
        state.selectedMission = mission;
      });
    },

    setOpponent: (faction, detachment) => {
      set((state) => {
        state.opponentFaction = faction;
        state.opponentDetachment = detachment;
      });
    },

    setValidation: (result) => {
      set((state) => {
        state.validation = result;
      });
    },

    reset: () => {
      set(() => ({ ...initialState }));
    },
  })),
);
