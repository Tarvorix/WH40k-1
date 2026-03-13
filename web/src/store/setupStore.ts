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
    { id: 0, name: 'Watchman of Terra', description: 'While the bearer is within Engagement Range of one or more enemy units, its Objective Control characteristic is 4.' },
    { id: 1, name: 'Warrior Exemplar', description: 'Each time the bearer destroys an enemy unit, roll one D6. On a 3+, you gain 1CP.' },
  ],
  WORLD_EATERS: [
    { id: 0, name: 'Fearsome Presence', description: 'While the bearer is not Battle-shocked, it has an Objective Control characteristic of 5.' },
    { id: 1, name: 'Bane of the Craven', description: 'Each time an enemy unit (excluding MONSTERS and VEHICLES) within Engagement Range of the bearer Falls Back, all models in that unit must take a Desperate Escape test. If Battle-shocked, subtract 1 from each test.' },
  ],
} as const;

// Patrol Squad options (Choose One)
// Source: Custodes.md §2 - Force Composition
export const PATROL_SQUADS = {
  CUSTODES: [
    { id: 0, name: 'Custodian Wardens', models: 3, description: '3 models (T6, W3, 2+/4++, OC2). Mixed loadout: 1 Castellan axe (S9 D3) + 2 Guardian spears (S7 D2). Higher model count, OC 6 total.' },
    { id: 1, name: 'Allarus Custodians', models: 2, description: '2 models (T7, W4, 2+/4++, OC2, TERMINATOR). Balistus grenade launcher (18", D6 Blast) + Guardian spears. Maximum durability, OC 4 total.' },
  ],
  WORLD_EATERS: [] as { id: number; name: string; models: number; description: string }[],
} as const;

// Secondary Objective IDs
export const SECONDARIES = {
  CUSTODES: [
    { id: 0, name: 'Raise the Vexillas', description: 'From battle round 3 onwards, at the end of your turn, score 3VP if you control both the objective marker closest to your battlefield edge and the one closest to your opponent\'s edge.' },
    { id: 1, name: 'Consecrated Ground', description: 'Each time an enemy unit is destroyed, score 3VP. Each time an Adeptus Custodes model is destroyed, lose 1VP. Minimum 0VP total.' },
  ],
  WORLD_EATERS: [
    { id: 0, name: 'Champions of Khorne', description: 'From round 2 onwards, at end of your Command phase: score 2VP if one objective in No Man\'s Land is controlled with a non-Battle-shocked CHARACTER nearby; 3VP if two such objectives.' },
    { id: 1, name: 'Skull Takers', description: 'At the end of the Fight phase, for each CHARACTER model that destroyed one or more enemy units that phase, score 3VP. Maximum 12VP.' },
  ],
} as const;

// Mission definitions (from CP_Rules.md Section 2.2)
export const MISSIONS = [
  { id: 0, name: 'Clash of Patrols', description: 'Take and Hold — 5VP per objective controlled from round 2. Retrieve Intelligence: select a controlled objective in Command phase for 1CP (each objective once only).' },
  { id: 1, name: 'Archeotech Recovery', description: 'Recover Archeotech — 5VP per objective from round 2. Irradiated Power Cells remove No Man\'s Land objectives in rounds 3-5. Control the last one for +10VP.' },
  { id: 2, name: 'Forward Outpost', description: 'Vital Ground — 5VP per No Man\'s Land objective, 10VP for enemy deployment zone objective. Sabotage Enemy Comms: control opponent\'s objective to block their Command Re-roll.' },
  { id: 3, name: 'Scorched Earth', description: 'Raze and Ruin — 5VP per objective from round 2. You may raze a controlled objective (no enemies within 3") to remove it and score 5VP.' },
  { id: 4, name: 'Sweeping Raid', description: 'Priority Targets — 5VP per objective from round 2. End of battle bonus for specific objectives. Supply Lines: 4+ on D6 for 1CP if you control your deployment zone objective.' },
  { id: 5, name: 'Display of Might', description: 'Symbolic Sites — 5VP each for controlling objectives, claiming symbolic sites with CHARACTERs, and maintaining claims across turns. Break Their Spirit limits Insane Bravery to within 6" of WARLORD.' },
] as const;

type SetupStep = 'faction_select' | 'enhancement_select' | 'secondary_select' | 'patrol_squad_select' | 'mission_select' | 'ready';

interface SetupState {
  step: SetupStep;
  playerFaction: number | null;
  opponentFaction: number | null;
  playerEnhancement: number | null;
  playerSecondary: number | null;
  playerPatrolSquad: number | null;
  missionId: number | null;

  // Actions
  setStep: (step: SetupStep) => void;
  selectFaction: (faction: number) => void;
  selectEnhancement: (enhancementId: number) => void;
  selectSecondary: (secondaryId: number) => void;
  selectPatrolSquad: (squadId: number) => void;
  selectMission: (missionId: number) => void;
  reset: () => void;
  isReady: () => boolean;
  hasPatrolSquadChoice: () => boolean;
}

export const useSetupStore = create<SetupState>()(
  immer((set, get) => ({
    step: 'faction_select',
    playerFaction: null,
    opponentFaction: null,
    playerEnhancement: null,
    playerSecondary: null,
    playerPatrolSquad: null,
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
        // If the faction has patrol squad choices, go to that step; otherwise skip to mission
        const faction = state.playerFaction;
        const hasSquads = faction === FACTIONS.CUSTODES && PATROL_SQUADS.CUSTODES.length > 0;
        state.step = hasSquads ? 'patrol_squad_select' : 'mission_select';
      });
    },

    selectPatrolSquad: (squadId) => {
      set((state) => {
        state.playerPatrolSquad = squadId;
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
        state.playerPatrolSquad = null;
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

    hasPatrolSquadChoice: () => {
      const s = get();
      return s.playerFaction === FACTIONS.CUSTODES && PATROL_SQUADS.CUSTODES.length > 0;
    },
  })),
);
