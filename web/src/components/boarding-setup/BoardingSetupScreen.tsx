import { useEffect, useState, useCallback } from 'react';
import { clsx } from 'clsx';
import { useGameStore } from '@/store/gameStore';
import { useBoardingSetupStore } from '@/store/boardingSetupStore';
import type { BoardingSetupStep } from '@/store/boardingSetupStore';
import type {
  BoardingFaction,
  BoardingDetachment,
  BoardingUnitDatasheet,
  BoardingEnhancement,
  BoardingMissionSummary,
  SelectedUnit,
} from '@/types/game';
import { Button } from '@/components/shared/Button';

// ===== Placeholder faction data =====
// In a full integration, this would come from the WASM engine via engineClient.
// For now we provide static data matching the boarding actions content so the
// UI renders and is fully navigable end-to-end.

const BOARDING_FACTIONS: BoardingFaction[] = [
  {
    faction_name: 'Adeptus Custodes',
    faction_keyword: 'ADEPTUS CUSTODES',
    army_rule_name: 'Martial Ka\'tah',
    army_rule_description: 'At the start of the Fight phase, select one Ka\'tah stance for all ADEPTUS CUSTODES units: Ka\'tah Assaulting (re-roll Wound rolls of 1 on the charge) or Ka\'tah Shielding (+1 to saving throws in melee).',
    detachments: [
      {
        detachment_name: 'Shield Host',
        detachment_rule_name: 'Aegis of the Emperor',
        detachment_rule_description: 'Each time a ranged attack targets an ADEPTUS CUSTODES unit from this detachment, subtract 1 from the Hit roll.',
        enhancements: [
          { name: 'Watchman of Terra', description: 'While the bearer is within Engagement Range of one or more enemy units, its Objective Control characteristic is 4.' },
          { name: 'Warrior Exemplar', description: 'Each time the bearer destroys an enemy unit, roll one D6. On a 3+, you gain 1CP.' },
        ],
        stratagems: [
          { name: 'Arcane Genetic Alchemy', cp_cost: 1, timing: 'When an ADEPTUS CUSTODES unit is selected as the target of an attack', target: 'One ADEPTUS CUSTODES unit from your army', effect: 'Until the end of the phase, each time an attack is allocated to a model in that unit, subtract 1 from the Damage characteristic of that attack (minimum 1).' },
          { name: 'Slayers of Nightmares', cp_cost: 1, timing: 'Fight phase, when an ADEPTUS CUSTODES unit is selected to fight', target: 'One ADEPTUS CUSTODES unit from your army', effect: 'Until the end of the phase, each time a model in that unit makes a melee attack, add 1 to the Wound roll.' },
        ],
        allowed_units: [
          { datasheet_name: 'Custodian Guard', max_count: 2, allowed_sizes: [3, 5], conditional_limit: null },
          { datasheet_name: 'Custodian Wardens', max_count: 2, allowed_sizes: [3], conditional_limit: null },
          { datasheet_name: 'Allarus Custodians', max_count: 1, allowed_sizes: [2, 3], conditional_limit: null },
          { datasheet_name: 'Shield-Captain', max_count: 1, allowed_sizes: [1], conditional_limit: null },
          { datasheet_name: 'Blade Champion', max_count: 1, allowed_sizes: [1], conditional_limit: null },
        ],
        mustering_rules: [
          { rule_type: 'points_limit', description: 'Maximum 500 points' },
          { rule_type: 'unit_limit', description: 'Maximum 5 units per roster' },
        ],
      },
    ],
    datasheets: [
      {
        name: 'Shield-Captain',
        points: [{ model_count: 1, points: 100 }],
        is_epic_hero: false,
        is_character: true,
        is_battleline: false,
        profile: { movement: '6"', toughness: 6, save: '2+', wounds: 6, leadership: '6+', oc: 2 },
        ranged_weapons: [{ name: 'Guardian Spear (Shooting)', range: '24"', attacks: '2', skill: '2+', strength: 4, ap: '-1', damage: '2', abilities: [] }],
        melee_weapons: [{ name: 'Guardian Spear (Melee)', range: 'Melee', attacks: '6', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: [] }],
        abilities: [
          { name: 'Strategic Mastermind', description: 'Once per battle, when you use a Stratagem, this model can reduce the CP cost by 1 (to a minimum of 0CP).', ability_type: 'core' },
        ],
        leader_info: { can_lead: ['Custodian Guard', 'Custodian Wardens'], leader_abilities: ['Strategic Mastermind'] },
        keywords: ['INFANTRY', 'CHARACTER', 'IMPERIUM'],
        faction_keywords: ['ADEPTUS CUSTODES'],
        wargear_options: [{ description: 'Can replace Guardian Spear with Sentinel Blade and Praesidium Shield' }],
        unit_composition: [{ model_name: 'Shield-Captain', count: '1' }],
      },
      {
        name: 'Blade Champion',
        points: [{ model_count: 1, points: 90 }],
        is_epic_hero: false,
        is_character: true,
        is_battleline: false,
        profile: { movement: '6"', toughness: 6, save: '2+', wounds: 5, leadership: '6+', oc: 2 },
        ranged_weapons: [],
        melee_weapons: [
          { name: 'Vaultsword (Behemor)', range: 'Melee', attacks: '5', skill: '2+', strength: 8, ap: '-3', damage: '3', abilities: ['SUSTAINED HITS 1'] },
          { name: 'Vaultsword (Hurricanis)', range: 'Melee', attacks: '9', skill: '2+', strength: 6, ap: '-2', damage: '1', abilities: [] },
        ],
        abilities: [
          { name: 'Martial Mastery', description: 'At the start of the Fight phase, select either Behemor or Hurricanis stance for this model.', ability_type: 'core' },
        ],
        leader_info: { can_lead: ['Custodian Guard'], leader_abilities: ['Martial Mastery'] },
        keywords: ['INFANTRY', 'CHARACTER', 'IMPERIUM'],
        faction_keywords: ['ADEPTUS CUSTODES'],
        wargear_options: [],
        unit_composition: [{ model_name: 'Blade Champion', count: '1' }],
      },
      {
        name: 'Custodian Guard',
        points: [{ model_count: 3, points: 135 }, { model_count: 5, points: 225 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: true,
        profile: { movement: '6"', toughness: 6, save: '2+', wounds: 3, leadership: '6+', oc: 2 },
        ranged_weapons: [{ name: 'Guardian Spear (Shooting)', range: '24"', attacks: '2', skill: '2+', strength: 4, ap: '-1', damage: '2', abilities: [] }],
        melee_weapons: [{ name: 'Guardian Spear (Melee)', range: 'Melee', attacks: '4', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: [] }],
        abilities: [
          { name: 'Sentinel Storm', description: 'Each time a model in this unit makes a ranged attack that targets the closest eligible target, re-roll a Hit roll of 1.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'BATTLELINE', 'IMPERIUM'],
        faction_keywords: ['ADEPTUS CUSTODES'],
        wargear_options: [{ description: 'Any number of models can each have their guardian spear replaced with a sentinel blade and praesidium shield.' }],
        unit_composition: [{ model_name: 'Custodian Guard', count: '3-5' }],
      },
      {
        name: 'Custodian Wardens',
        points: [{ model_count: 3, points: 150 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: false,
        profile: { movement: '6"', toughness: 6, save: '2+', wounds: 3, leadership: '6+', oc: 2 },
        ranged_weapons: [{ name: 'Guardian Spear (Shooting)', range: '24"', attacks: '2', skill: '2+', strength: 4, ap: '-1', damage: '2', abilities: [] }],
        melee_weapons: [{ name: 'Guardian Spear (Melee)', range: 'Melee', attacks: '4', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: [] }],
        abilities: [
          { name: 'Resolute Will', description: 'Each time a model in this unit would lose a wound, on a 6+, that wound is not lost.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'IMPERIUM'],
        faction_keywords: ['ADEPTUS CUSTODES'],
        wargear_options: [{ description: 'For every 3 models in this unit, 1 model that is not equipped with a Vexilla can have its guardian spear replaced with a castellan axe.' }],
        unit_composition: [{ model_name: 'Custodian Warden', count: '3' }],
      },
      {
        name: 'Allarus Custodians',
        points: [{ model_count: 2, points: 130 }, { model_count: 3, points: 195 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: false,
        profile: { movement: '5"', toughness: 7, save: '2+', wounds: 4, leadership: '6+', oc: 2 },
        ranged_weapons: [
          { name: 'Balistus Grenade Launcher', range: '18"', attacks: 'D6', skill: '2+', strength: 4, ap: '-1', damage: '1', abilities: ['BLAST'] },
          { name: 'Guardian Spear (Shooting)', range: '24"', attacks: '2', skill: '2+', strength: 4, ap: '-1', damage: '2', abilities: [] },
        ],
        melee_weapons: [{ name: 'Guardian Spear (Melee)', range: 'Melee', attacks: '4', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: [] }],
        abilities: [
          { name: 'From Golden Light', description: 'This unit can be set up in reserve. At the end of your opponent\'s Movement phase, it can arrive from reserves within 6" of any battlefield edge and more than 9" from all enemy units.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'TERMINATOR', 'IMPERIUM'],
        faction_keywords: ['ADEPTUS CUSTODES'],
        wargear_options: [{ description: 'Any number of models can each have their guardian spear replaced with a castellan axe.' }],
        unit_composition: [{ model_name: 'Allarus Custodian', count: '2-3' }],
      },
    ],
  },
  {
    faction_name: 'World Eaters',
    faction_keyword: 'WORLD EATERS',
    army_rule_name: 'Blessings of Khorne',
    army_rule_description: 'At the start of your Command phase, roll 8 dice. Each 4+ generates a Blood Tithe point. Spend Blood Tithe on Blessings of Khorne abilities.',
    detachments: [
      {
        detachment_name: 'Berzerker Warband',
        detachment_rule_name: 'Blood Surge',
        detachment_rule_description: 'Each time a WORLD EATERS unit from this detachment makes a Charge move, until the end of the turn, add 1 to the Strength of melee weapons equipped by models in that unit.',
        enhancements: [
          { name: 'Fearsome Presence', description: 'While the bearer is not Battle-shocked, it has an Objective Control characteristic of 5.' },
          { name: 'Bane of the Craven', description: 'Each time an enemy unit within Engagement Range of the bearer Falls Back, all models in that unit must take a Desperate Escape test.' },
        ],
        stratagems: [
          { name: 'Blood for the Blood God', cp_cost: 1, timing: 'Fight phase, when a WORLD EATERS unit is selected to fight', target: 'One WORLD EATERS unit from your army', effect: 'Until the end of the phase, each time a model in that unit makes a melee attack, an unmodified Hit roll of 5+ scores a Critical Hit.' },
          { name: 'Skulls for the Skull Throne', cp_cost: 1, timing: 'Fight phase, after a WORLD EATERS unit destroys an enemy unit', target: 'The WORLD EATERS unit that destroyed the enemy unit', effect: 'That unit regains D3 lost wounds. If the destroyed enemy unit was a CHARACTER, regain D3+3 wounds instead.' },
        ],
        allowed_units: [
          { datasheet_name: 'Khorne Berzerkers', max_count: 3, allowed_sizes: [5, 10], conditional_limit: null },
          { datasheet_name: 'Jakhals', max_count: 2, allowed_sizes: [10], conditional_limit: null },
          { datasheet_name: 'Exalted Eightbound', max_count: 1, allowed_sizes: [3], conditional_limit: null },
          { datasheet_name: 'World Eaters Master of Executions', max_count: 1, allowed_sizes: [1], conditional_limit: null },
          { datasheet_name: 'World Eaters Lord on Juggernaut', max_count: 1, allowed_sizes: [1], conditional_limit: null },
        ],
        mustering_rules: [
          { rule_type: 'points_limit', description: 'Maximum 500 points' },
          { rule_type: 'unit_limit', description: 'Maximum 5 units per roster' },
        ],
      },
    ],
    datasheets: [
      {
        name: 'World Eaters Master of Executions',
        points: [{ model_count: 1, points: 80 }],
        is_epic_hero: false,
        is_character: true,
        is_battleline: false,
        profile: { movement: '6"', toughness: 4, save: '3+', wounds: 5, leadership: '6+', oc: 1 },
        ranged_weapons: [],
        melee_weapons: [{ name: 'Axe of Dismemberment', range: 'Melee', attacks: '5', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: ['DEVASTATING WOUNDS'] }],
        abilities: [
          { name: 'Trophy Taker', description: 'Each time this model destroys an enemy CHARACTER, you gain 1CP.', ability_type: 'core' },
        ],
        leader_info: { can_lead: ['Khorne Berzerkers'], leader_abilities: ['Trophy Taker'] },
        keywords: ['INFANTRY', 'CHARACTER', 'CHAOS', 'KHORNE'],
        faction_keywords: ['WORLD EATERS'],
        wargear_options: [],
        unit_composition: [{ model_name: 'Master of Executions', count: '1' }],
      },
      {
        name: 'World Eaters Lord on Juggernaut',
        points: [{ model_count: 1, points: 110 }],
        is_epic_hero: false,
        is_character: true,
        is_battleline: false,
        profile: { movement: '10"', toughness: 6, save: '3+', wounds: 7, leadership: '6+', oc: 2 },
        ranged_weapons: [],
        melee_weapons: [
          { name: 'Exalted Chainblade', range: 'Melee', attacks: '7', skill: '2+', strength: 6, ap: '-2', damage: '2', abilities: ['LANCE'] },
          { name: 'Juggernaut\'s Bladed Horn', range: 'Melee', attacks: '4', skill: '3+', strength: 6, ap: '-1', damage: '2', abilities: ['EXTRA ATTACKS', 'LANCE'] },
        ],
        abilities: [
          { name: 'Devastating Charge', description: 'Each time this model makes a Charge move, until the end of the turn, melee attacks made by this model have the [DEVASTATING WOUNDS] ability.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['MOUNTED', 'CHARACTER', 'CHAOS', 'KHORNE'],
        faction_keywords: ['WORLD EATERS'],
        wargear_options: [],
        unit_composition: [{ model_name: 'Lord on Juggernaut', count: '1' }],
      },
      {
        name: 'Khorne Berzerkers',
        points: [{ model_count: 5, points: 90 }, { model_count: 10, points: 180 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: true,
        profile: { movement: '6"', toughness: 4, save: '3+', wounds: 2, leadership: '6+', oc: 2 },
        ranged_weapons: [{ name: 'Bolt Pistol', range: '12"', attacks: '1', skill: '3+', strength: 4, ap: '0', damage: '1', abilities: ['PISTOL'] }],
        melee_weapons: [{ name: 'Berzerker Chainblade', range: 'Melee', attacks: '4', skill: '3+', strength: 5, ap: '-1', damage: '1', abilities: [] }],
        abilities: [
          { name: 'Blood Surge', description: 'Each time this unit makes a Charge move, until the end of the turn, melee attacks made by models in this unit have the [SUSTAINED HITS 1] ability.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'BATTLELINE', 'CHAOS', 'KHORNE'],
        faction_keywords: ['WORLD EATERS'],
        wargear_options: [{ description: 'The Berzerker Champion can be equipped with a plasma pistol instead of a bolt pistol.' }],
        unit_composition: [{ model_name: 'Khorne Berzerker', count: '5-10' }],
      },
      {
        name: 'Jakhals',
        points: [{ model_count: 10, points: 65 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: false,
        profile: { movement: '6"', toughness: 3, save: '6+', wounds: 1, leadership: '7+', oc: 1 },
        ranged_weapons: [{ name: 'Autopistol', range: '12"', attacks: '1', skill: '4+', strength: 3, ap: '0', damage: '1', abilities: ['PISTOL'] }],
        melee_weapons: [{ name: 'Jakhal Chainblades', range: 'Melee', attacks: '2', skill: '4+', strength: 4, ap: '0', damage: '1', abilities: [] }],
        abilities: [
          { name: 'Devoted of Khorne', description: 'Each time a model in this unit is destroyed by a melee attack, if that model has not fought this phase, roll one D6. On a 4+, do not remove it from play; that destroyed model can fight after the attacking model\'s unit has finished making its attacks.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'CHAOS', 'KHORNE'],
        faction_keywords: ['WORLD EATERS'],
        wargear_options: [],
        unit_composition: [{ model_name: 'Jakhal', count: '10' }],
      },
      {
        name: 'Exalted Eightbound',
        points: [{ model_count: 3, points: 155 }],
        is_epic_hero: false,
        is_character: false,
        is_battleline: false,
        profile: { movement: '6"', toughness: 6, save: '3+', wounds: 4, leadership: '6+', oc: 1 },
        ranged_weapons: [],
        melee_weapons: [{ name: 'Eightbound Eviscerators', range: 'Melee', attacks: '6', skill: '2+', strength: 7, ap: '-2', damage: '2', abilities: [] }],
        abilities: [
          { name: 'Daemon-touched', description: 'This unit has a 4+ invulnerable save.', ability_type: 'core' },
        ],
        leader_info: null,
        keywords: ['INFANTRY', 'CHAOS', 'DAEMON', 'KHORNE'],
        faction_keywords: ['WORLD EATERS'],
        wargear_options: [],
        unit_composition: [{ model_name: 'Exalted Eightbound', count: '3' }],
      },
    ],
  },
];

// Universal Boarding Actions enhancements (available to all factions)
const UNIVERSAL_ENHANCEMENTS: BoardingEnhancement[] = [
  { name: 'Master of Voidcraft', description: 'Once per battle round, when a friendly unit within 6" of the bearer is targeted by a Stratagem, reduce the CP cost by 1 (minimum 0).' },
  { name: 'Bulkhead Breacher', description: 'The bearer can operate hatchways at 6" range instead of 1". In addition, when the bearer operates a hatchway, it can force it open on a 2+ even if it is locked.' },
  { name: 'Void Hardened', description: 'The bearer has a 4+ Feel No Pain against mortal wounds caused by Environmental Hazards.' },
  { name: 'Ship-Born Veteran', description: 'The bearer and their unit can move through hatchways even when they are Closed (but not Locked).' },
  { name: 'Ruthless Taskmaster', description: 'Once per battle, at the start of your Command phase, select one friendly unit within 6" of the bearer. That unit can act as if it were not Battle-shocked until the end of the turn.' },
  { name: 'Salvage Expert', description: 'Each time the bearer\'s unit controls an objective marker, add 1 to the VP scored from that objective.' },
];

const BOARDING_MISSIONS: BoardingMissionSummary[] = [
  { mission_id: 'BA01', name: 'Search and Destroy', mission_type: 'symmetric', tags: ['action', 'progressive'] },
  { mission_id: 'BA02', name: 'Breach and Control', mission_type: 'symmetric', tags: ['objective', 'progressive'] },
  { mission_id: 'BA03', name: 'Purge the Enemy', mission_type: 'symmetric', tags: ['kill', 'progressive'] },
  { mission_id: 'BA04', name: 'Running Battle', mission_type: 'symmetric', tags: ['action', 'kill'] },
  { mission_id: 'BA05', name: 'Decapitation Strike', mission_type: 'asymmetric', tags: ['character', 'kill'] },
  { mission_id: 'BA06', name: 'Hold the Line', mission_type: 'asymmetric', tags: ['objective', 'endgame'] },
  { mission_id: 'BA07', name: 'Sabotage', mission_type: 'asymmetric', tags: ['action', 'objective'] },
  { mission_id: 'BA08', name: 'Extraction', mission_type: 'asymmetric', tags: ['action', 'progressive'] },
  { mission_id: 'BA09', name: 'Vital Intelligence', mission_type: 'symmetric', tags: ['objective', 'action'] },
  { mission_id: 'BA10', name: 'Power Struggle', mission_type: 'symmetric', tags: ['objective', 'progressive'] },
  { mission_id: 'BA11', name: 'Sweep and Clear', mission_type: 'symmetric', tags: ['action', 'progressive'] },
  { mission_id: 'BA12', name: 'Last Stand', mission_type: 'asymmetric', tags: ['kill', 'endgame'] },
  { mission_id: 'BA13', name: 'Secure the Armoury', mission_type: 'symmetric', tags: ['objective', 'action'] },
  { mission_id: 'BA14', name: 'Void Ambush', mission_type: 'asymmetric', tags: ['kill', 'progressive'] },
  { mission_id: 'BA15', name: 'Boarding Assault', mission_type: 'asymmetric', tags: ['objective', 'endgame'] },
];

// ===== Step indicator component =====

const STEP_LABELS: { step: BoardingSetupStep; label: string }[] = [
  { step: 'select_faction', label: 'Faction' },
  { step: 'select_detachment', label: 'Detachment' },
  { step: 'build_army', label: 'Army' },
  { step: 'select_enhancements', label: 'Enhancements' },
  { step: 'designate_warlord', label: 'Warlord' },
  { step: 'opponent_setup', label: 'Opponent' },
  { step: 'select_mission', label: 'Mission' },
  { step: 'ready', label: 'Ready' },
];

function stepIndex(step: BoardingSetupStep): number {
  return STEP_LABELS.findIndex((s) => s.step === step);
}

function StepIndicator({ currentStep }: { currentStep: BoardingSetupStep }) {
  const currentIdx = stepIndex(currentStep);
  return (
    <div className="flex items-center justify-center gap-1 mb-8">
      {STEP_LABELS.map((s, i) => (
        <div key={s.step} className="flex items-center gap-1">
          <div
            className={clsx(
              'h-1.5 rounded-full transition-colors',
              i <= currentIdx ? 'bg-accent' : 'bg-surface-lighter',
              i === currentIdx ? 'w-20' : 'w-10',
            )}
          />
        </div>
      ))}
    </div>
  );
}

// ===== Points bar component =====

function PointsBar({ current, max }: { current: number; max: number }) {
  const pct = Math.min((current / max) * 100, 100);
  const isOver = current > max;

  return (
    <div className="w-full">
      <div className="flex justify-between text-sm mb-1">
        <span className="text-gray-400">Points</span>
        <span className={clsx(isOver ? 'text-red-400 font-bold' : 'text-gray-200')}>
          {current} / {max}
        </span>
      </div>
      <div className="w-full bg-surface rounded-full h-2.5">
        <div
          className={clsx(
            'h-2.5 rounded-full transition-all duration-300',
            isOver ? 'bg-red-500' : 'bg-accent',
          )}
          style={{ width: `${Math.min(pct, 100)}%` }}
        />
      </div>
    </div>
  );
}

// ===== Select Faction Step =====

function SelectFactionStep() {
  const selectFaction = useBoardingSetupStore((s) => s.selectFaction);
  const setScreen = useGameStore((s) => s.setScreen);

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Choose Your Faction
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Select a faction for Boarding Actions
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {BOARDING_FACTIONS.map((faction) => (
          <button
            key={faction.faction_keyword}
            onClick={() => selectFaction(faction)}
            className={clsx(
              'card-hover text-left p-5 group',
              faction.faction_keyword === 'ADEPTUS CUSTODES'
                ? 'hover:border-custodes-gold'
                : 'hover:border-worldeaters-red',
            )}
          >
            <h3
              className={clsx(
                'font-heading text-xl font-bold mb-1',
                faction.faction_keyword === 'ADEPTUS CUSTODES'
                  ? 'text-custodes-gold'
                  : 'text-worldeaters-red',
              )}
            >
              {faction.faction_name}
            </h3>
            <p className="text-gray-400 text-xs mb-3">{faction.faction_keyword}</p>
            <p className="text-gray-300 text-sm mb-3">{faction.army_rule_description}</p>
            <div className="text-xs bg-surface rounded px-3 py-2">
              <span className="text-gray-400 font-semibold">Army Rule: </span>
              <span className="text-gray-300">{faction.army_rule_name}</span>
            </div>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setScreen('menu')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Main Menu
        </button>
      </div>
    </div>
  );
}

// ===== Select Detachment Step =====

function SelectDetachmentStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const selectDetachment = useBoardingSetupStore((s) => s.selectDetachment);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction) return null;

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Choose Detachment
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Select a detachment for {playerFaction.faction_name}
      </p>
      <div className="grid grid-cols-1 gap-4 mb-6">
        {playerFaction.detachments.map((det) => (
          <button
            key={det.detachment_name}
            onClick={() => selectDetachment(det)}
            className="card-hover text-left p-5"
          >
            <h3 className="font-heading text-lg text-accent font-bold mb-1">
              {det.detachment_name}
            </h3>
            <div className="text-xs text-gray-400 mb-2">{det.detachment_rule_name}</div>
            <p className="text-gray-300 text-sm mb-3">{det.detachment_rule_description}</p>
            <div className="flex flex-wrap gap-2 mb-2">
              {det.stratagems.map((strat) => (
                <span
                  key={strat.name}
                  className="text-xs bg-surface rounded px-2 py-1 text-gray-400"
                >
                  {strat.name} ({strat.cp_cost}CP)
                </span>
              ))}
            </div>
            <div className="text-xs text-gray-500">
              {det.allowed_units.length} unit types available | {det.enhancements.length} enhancements
            </div>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setStep('select_faction')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Faction Select
        </button>
      </div>
    </div>
  );
}

// ===== Build Army Step =====

function BuildArmyStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const totalPoints = useBoardingSetupStore((s) => s.totalPoints);
  const addUnit = useBoardingSetupStore((s) => s.addUnit);
  const removeUnit = useBoardingSetupStore((s) => s.removeUnit);
  const updateUnitSize = useBoardingSetupStore((s) => s.updateUnitSize);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  const [expandedUnit, setExpandedUnit] = useState<string | null>(null);

  if (!playerFaction || !playerDetachment) return null;

  const MAX_POINTS = 500;

  // Get datasheets that are allowed in this detachment
  const allowedNames = new Set(playerDetachment.allowed_units.map((u) => u.datasheet_name));
  const availableDatasheets = playerFaction.datasheets.filter((ds) =>
    allowedNames.has(ds.name),
  );

  // Count how many of each datasheet are currently selected
  const unitCounts: Record<string, number> = {};
  for (const u of selectedUnits) {
    unitCounts[u.datasheet_name] = (unitCounts[u.datasheet_name] ?? 0) + 1;
  }

  // Check if a datasheet can be added
  const canAddDatasheet = (ds: BoardingUnitDatasheet): boolean => {
    const ref = playerDetachment.allowed_units.find(
      (u) => u.datasheet_name === ds.name,
    );
    if (!ref) return false;
    const currentCount = unitCounts[ds.name] ?? 0;
    if (ref.max_count !== null && currentCount >= ref.max_count) return false;
    // Check if adding the cheapest option would exceed points
    const cheapest = Math.min(...ds.points.map((p) => p.points));
    if (totalPoints + cheapest > MAX_POINTS) return false;
    return true;
  };

  const handleAddUnit = (ds: BoardingUnitDatasheet, sizeIndex: number) => {
    const pointsEntry = ds.points[sizeIndex];
    addUnit({
      datasheet_name: ds.name,
      model_count: pointsEntry.model_count,
      points: pointsEntry.points,
      wargear_selections: [],
    });
    setExpandedUnit(null);
  };

  const hasCharacters = selectedUnits.some((u) => {
    const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
    return ds?.is_character;
  });

  const canContinue = selectedUnits.length >= 1 && totalPoints <= MAX_POINTS;

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Build Your Army
      </h2>
      <p className="text-gray-400 text-center text-sm mb-4">
        {playerFaction.faction_name} - {playerDetachment.detachment_name}
      </p>

      <PointsBar current={totalPoints} max={MAX_POINTS} />

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mt-6">
        {/* Left: Available units */}
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">Available Units</h3>
          <div className="space-y-2">
            {availableDatasheets.map((ds) => {
              const ref = playerDetachment.allowed_units.find(
                (u) => u.datasheet_name === ds.name,
              );
              const currentCount = unitCounts[ds.name] ?? 0;
              const atMax =
                ref?.max_count !== null &&
                ref?.max_count !== undefined &&
                currentCount >= ref.max_count;
              const isExpanded = expandedUnit === ds.name;

              return (
                <div
                  key={ds.name}
                  className="card"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="font-semibold text-gray-100 text-sm">
                          {ds.name}
                        </span>
                        {ds.is_character && (
                          <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded">
                            CHARACTER
                          </span>
                        )}
                        {ds.is_battleline && (
                          <span className="text-xs bg-phase-movement/20 text-phase-movement px-1.5 py-0.5 rounded">
                            BATTLELINE
                          </span>
                        )}
                        {ds.is_epic_hero && (
                          <span className="text-xs bg-phase-fight/20 text-phase-fight px-1.5 py-0.5 rounded">
                            EPIC HERO
                          </span>
                        )}
                      </div>
                      <div className="text-xs text-gray-400 mb-1">
                        {ds.points.map((p) => `${p.model_count} model${p.model_count !== 1 ? 's' : ''}: ${p.points}pts`).join(' | ')}
                      </div>
                      <div className="flex gap-2 text-xs text-gray-500">
                        <span>M{ds.profile.movement}</span>
                        <span>T{ds.profile.toughness}</span>
                        <span>Sv{ds.profile.save}</span>
                        <span>W{ds.profile.wounds}</span>
                        <span>OC{ds.profile.oc}</span>
                      </div>
                      {ref?.max_count !== null && (
                        <div className="text-xs text-gray-500 mt-1">
                          {currentCount}/{ref?.max_count} selected
                        </div>
                      )}
                    </div>
                    <div className="flex flex-col gap-1">
                      {ds.points.length === 1 ? (
                        <Button
                          size="sm"
                          variant="primary"
                          disabled={!canAddDatasheet(ds) || atMax}
                          onClick={() => handleAddUnit(ds, 0)}
                        >
                          Add
                        </Button>
                      ) : (
                        <Button
                          size="sm"
                          variant="secondary"
                          disabled={!canAddDatasheet(ds) || atMax}
                          onClick={() =>
                            setExpandedUnit(isExpanded ? null : ds.name)
                          }
                        >
                          {isExpanded ? 'Cancel' : 'Add...'}
                        </Button>
                      )}
                    </div>
                  </div>
                  {/* Size selector when expanded */}
                  {isExpanded && ds.points.length > 1 && (
                    <div className="mt-2 pt-2 border-t border-gray-700">
                      <div className="text-xs text-gray-400 mb-2">Select unit size:</div>
                      <div className="flex gap-2">
                        {ds.points.map((p, idx) => (
                          <Button
                            key={p.model_count}
                            size="sm"
                            variant="primary"
                            disabled={totalPoints + p.points > MAX_POINTS}
                            onClick={() => handleAddUnit(ds, idx)}
                          >
                            {p.model_count} models ({p.points}pts)
                          </Button>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Selected roster */}
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">
            Your Roster ({selectedUnits.length} unit{selectedUnits.length !== 1 ? 's' : ''})
          </h3>
          {selectedUnits.length === 0 ? (
            <div className="card text-center text-gray-500 py-8">
              No units selected yet. Add units from the left panel.
            </div>
          ) : (
            <div className="space-y-2">
              {selectedUnits.map((unit, idx) => {
                const ds = playerFaction.datasheets.find(
                  (d) => d.name === unit.datasheet_name,
                );
                return (
                  <div key={`${unit.datasheet_name}-${idx}`} className="card">
                    <div className="flex items-center justify-between">
                      <div>
                        <span className="font-semibold text-gray-100 text-sm">
                          {unit.datasheet_name}
                        </span>
                        <span className="text-xs text-gray-400 ml-2">
                          {unit.model_count} model{unit.model_count !== 1 ? 's' : ''}
                        </span>
                        {ds?.is_character && (
                          <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded ml-2">
                            CHARACTER
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-3">
                        <span className="text-accent font-mono text-sm">
                          {unit.points}pts
                        </span>
                        {/* Size changer if multiple sizes available */}
                        {ds && ds.points.length > 1 && (
                          <select
                            value={unit.model_count}
                            onChange={(e) => {
                              const newCount = Number(e.target.value);
                              const newEntry = ds.points.find(
                                (p) => p.model_count === newCount,
                              );
                              if (newEntry) {
                                updateUnitSize(idx, newEntry.model_count, newEntry.points);
                              }
                            }}
                            className="bg-surface border border-gray-600 text-gray-200 text-xs rounded px-1 py-0.5"
                          >
                            {ds.points.map((p) => (
                              <option key={p.model_count} value={p.model_count}>
                                {p.model_count} ({p.points}pts)
                              </option>
                            ))}
                          </select>
                        )}
                        <Button
                          size="sm"
                          variant="danger"
                          onClick={() => removeUnit(idx)}
                        >
                          Remove
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => setStep('select_detachment')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Detachment Select
        </button>
        <Button
          variant="primary"
          disabled={!canContinue}
          onClick={() => {
            // If there are characters, go to enhancements. Otherwise skip to warlord or opponent.
            if (hasCharacters) {
              setStep('select_enhancements');
            } else {
              setStep('opponent_setup');
            }
          }}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Select Enhancements Step =====

function SelectEnhancementsStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const enhancements = useBoardingSetupStore((s) => s.enhancements);
  const addEnhancement = useBoardingSetupStore((s) => s.addEnhancement);
  const removeEnhancement = useBoardingSetupStore((s) => s.removeEnhancement);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction || !playerDetachment) return null;

  const MAX_ENHANCEMENTS = 2;

  // All available enhancements: universal + detachment-specific
  const allEnhancements: BoardingEnhancement[] = [
    ...UNIVERSAL_ENHANCEMENTS,
    ...playerDetachment.enhancements,
  ];

  // Character units that can receive enhancements
  const characterUnits = selectedUnits
    .map((u, idx) => ({ unit: u, index: idx }))
    .filter(({ unit }) => {
      const ds = playerFaction.datasheets.find((d) => d.name === unit.datasheet_name);
      return ds?.is_character;
    });

  // Enhancement names already assigned
  const assignedEnhancementNames = new Set(enhancements.map((e) => e.enhancement_name));
  // Unit indices that already have an enhancement
  const unitsWithEnhancements = new Set(enhancements.map((e) => e.unit_index));

  const canAssign = enhancements.length < MAX_ENHANCEMENTS;

  const handleAssign = (enhancementName: string, unitIndex: number) => {
    addEnhancement({ enhancement_name: enhancementName, unit_index: unitIndex });
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Enhancements
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Assign up to {MAX_ENHANCEMENTS} enhancements to CHARACTER units ({enhancements.length}/{MAX_ENHANCEMENTS} assigned)
      </p>

      {/* Current assignments */}
      {enhancements.length > 0 && (
        <div className="mb-6">
          <h3 className="font-heading text-lg text-gray-300 mb-3">Current Assignments</h3>
          <div className="space-y-2">
            {enhancements.map((enh, idx) => {
              const unit = selectedUnits[enh.unit_index];
              return (
                <div key={idx} className="card flex items-center justify-between">
                  <div>
                    <span className="text-accent font-semibold text-sm">{enh.enhancement_name}</span>
                    <span className="text-gray-400 text-xs mx-2">assigned to</span>
                    <span className="text-gray-200 text-sm">{unit?.datasheet_name ?? 'Unknown'}</span>
                  </div>
                  <Button size="sm" variant="danger" onClick={() => removeEnhancement(idx)}>
                    Remove
                  </Button>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Enhancement list */}
      {canAssign && (
        <div>
          <h3 className="font-heading text-lg text-gray-300 mb-3">Available Enhancements</h3>
          <div className="space-y-3">
            {allEnhancements.map((enh) => {
              const isAssigned = assignedEnhancementNames.has(enh.name);
              return (
                <div
                  key={enh.name}
                  className={clsx('card', isAssigned && 'opacity-50')}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1">
                      <h4 className="font-semibold text-accent text-sm mb-1">{enh.name}</h4>
                      <p className="text-gray-300 text-xs">{enh.description}</p>
                    </div>
                    {!isAssigned && (
                      <div className="flex flex-col gap-1 min-w-[120px]">
                        <div className="text-xs text-gray-400 mb-1">Assign to:</div>
                        {characterUnits.map(({ unit, index: unitIdx }) => {
                          const alreadyHas = unitsWithEnhancements.has(unitIdx);
                          return (
                            <Button
                              key={unitIdx}
                              size="sm"
                              variant="secondary"
                              disabled={alreadyHas}
                              onClick={() => handleAssign(enh.name, unitIdx)}
                            >
                              {unit.datasheet_name}
                            </Button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => setStep('build_army')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Army Builder
        </button>
        <Button
          variant="primary"
          onClick={() => setStep('designate_warlord')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Designate Warlord Step =====

function DesignateWarlordStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const warlordIndex = useBoardingSetupStore((s) => s.warlordIndex);
  const setWarlord = useBoardingSetupStore((s) => s.setWarlord);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  if (!playerFaction) return null;

  // Character units eligible to be warlord
  const characterUnits = selectedUnits
    .map((u, idx) => ({ unit: u, index: idx }))
    .filter(({ unit }) => {
      const ds = playerFaction.datasheets.find((d) => d.name === unit.datasheet_name);
      return ds?.is_character;
    });

  // If no characters, auto-select first unit
  const eligibleUnits = characterUnits.length > 0
    ? characterUnits
    : selectedUnits.map((u, idx) => ({ unit: u, index: idx }));

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Designate Warlord
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        {characterUnits.length > 0
          ? 'Select a CHARACTER to be your Warlord'
          : 'Select a unit leader to be your Warlord'}
      </p>

      <div className="space-y-2 max-w-lg mx-auto mb-6">
        {eligibleUnits.map(({ unit, index: unitIdx }) => {
          const ds = playerFaction.datasheets.find(
            (d) => d.name === unit.datasheet_name,
          );
          const isSelected = warlordIndex === unitIdx;
          return (
            <button
              key={unitIdx}
              onClick={() => setWarlord(unitIdx)}
              className={clsx(
                isSelected ? 'card-selected' : 'card-hover',
                'w-full text-left',
              )}
            >
              <div className="flex items-center justify-between">
                <div>
                  <span className="font-semibold text-gray-100 text-sm">
                    {unit.datasheet_name}
                  </span>
                  <span className="text-xs text-gray-400 ml-2">
                    {unit.model_count} model{unit.model_count !== 1 ? 's' : ''}
                  </span>
                  {ds?.is_character && (
                    <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded ml-2">
                      CHARACTER
                    </span>
                  )}
                </div>
                {isSelected && (
                  <span className="text-accent font-heading text-xs tracking-wide">
                    WARLORD
                  </span>
                )}
              </div>
            </button>
          );
        })}
      </div>

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => {
            // Go back based on whether we had characters for enhancements
            const hasChars = selectedUnits.some((u) => {
              const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
              return ds?.is_character;
            });
            setStep(hasChars ? 'select_enhancements' : 'build_army');
          }}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back
        </button>
        <Button
          variant="primary"
          disabled={warlordIndex === null}
          onClick={() => setStep('opponent_setup')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Opponent Setup Step =====

function OpponentSetupStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const opponentFaction = useBoardingSetupStore((s) => s.opponentFaction);
  const opponentDetachment = useBoardingSetupStore((s) => s.opponentDetachment);
  const setOpponent = useBoardingSetupStore((s) => s.setOpponent);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  // Auto-select opponent on mount if not already set
  useEffect(() => {
    if (!opponentFaction && playerFaction) {
      // Pick the other faction by default
      const other = BOARDING_FACTIONS.find(
        (f) => f.faction_keyword !== playerFaction.faction_keyword,
      );
      if (other && other.detachments.length > 0) {
        setOpponent(other, other.detachments[0]);
      }
    }
  }, [opponentFaction, playerFaction, setOpponent]);

  const handleSelectOpponent = (faction: BoardingFaction) => {
    if (faction.detachments.length > 0) {
      setOpponent(faction, faction.detachments[0]);
    }
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Opponent Setup
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        The AI will command the opposing force. Select the opponent faction.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto mb-6">
        {BOARDING_FACTIONS.map((faction) => {
          const isSelected =
            opponentFaction?.faction_keyword === faction.faction_keyword;
          return (
            <button
              key={faction.faction_keyword}
              onClick={() => handleSelectOpponent(faction)}
              className={clsx(
                isSelected ? 'card-selected' : 'card-hover',
                'text-left p-5',
              )}
            >
              <h3
                className={clsx(
                  'font-heading text-lg font-bold mb-1',
                  faction.faction_keyword === 'ADEPTUS CUSTODES'
                    ? 'text-custodes-gold'
                    : 'text-worldeaters-red',
                )}
              >
                {faction.faction_name}
              </h3>
              <p className="text-gray-400 text-xs mb-2">{faction.army_rule_name}</p>
              {isSelected && opponentDetachment && (
                <div className="text-xs text-gray-300 bg-surface rounded px-2 py-1 mt-2">
                  Detachment: {opponentDetachment.detachment_name}
                </div>
              )}
            </button>
          );
        })}
      </div>

      {/* Detachment override if the opponent faction has multiple */}
      {opponentFaction && opponentFaction.detachments.length > 1 && (
        <div className="max-w-lg mx-auto mb-6">
          <h3 className="text-sm text-gray-400 mb-2">Opponent Detachment:</h3>
          <div className="flex gap-2 flex-wrap">
            {opponentFaction.detachments.map((det) => (
              <button
                key={det.detachment_name}
                onClick={() => setOpponent(opponentFaction, det)}
                className={clsx(
                  'text-sm px-3 py-1.5 rounded',
                  opponentDetachment?.detachment_name === det.detachment_name
                    ? 'bg-accent text-black font-semibold'
                    : 'bg-surface-lighter text-gray-300 hover:bg-surface-light',
                )}
              >
                {det.detachment_name}
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="flex justify-between items-center mt-6">
        <button
          onClick={() => {
            const hasChars = playerFaction
              ? (useBoardingSetupStore.getState().selectedUnits).some((u) => {
                  const ds = playerFaction.datasheets.find((d) => d.name === u.datasheet_name);
                  return ds?.is_character;
                })
              : false;
            setStep(hasChars ? 'designate_warlord' : 'build_army');
          }}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back
        </button>
        <Button
          variant="primary"
          disabled={!opponentFaction || !opponentDetachment}
          onClick={() => setStep('select_mission')}
        >
          Continue
        </Button>
      </div>
    </div>
  );
}

// ===== Select Mission Step =====

function SelectMissionStep() {
  const selectMission = useBoardingSetupStore((s) => s.selectMission);
  const selectedMission = useBoardingSetupStore((s) => s.selectedMission);
  const setStep = useBoardingSetupStore((s) => s.setStep);

  const [filterType, setFilterType] = useState<string | null>(null);
  const [filterTag, setFilterTag] = useState<string | null>(null);

  // Collect all unique tags
  const allTags = Array.from(
    new Set(BOARDING_MISSIONS.flatMap((m) => m.tags)),
  ).sort();

  // Filter missions
  const filteredMissions = BOARDING_MISSIONS.filter((m) => {
    if (filterType && m.mission_type !== filterType) return false;
    if (filterTag && !m.tags.includes(filterTag)) return false;
    return true;
  });

  const handleSelect = (mission: BoardingMissionSummary) => {
    selectMission(mission);
    setStep('ready');
  };

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Mission
      </h2>
      <p className="text-gray-400 text-center text-sm mb-4">
        Choose one of the 15 Boarding Actions missions
      </p>

      {/* Filters */}
      <div className="flex flex-wrap gap-2 justify-center mb-6">
        <button
          onClick={() => setFilterType(null)}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === null
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          All Types
        </button>
        <button
          onClick={() => setFilterType('symmetric')}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === 'symmetric'
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          Symmetric
        </button>
        <button
          onClick={() => setFilterType('asymmetric')}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterType === 'asymmetric'
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          Asymmetric
        </button>
        <span className="text-gray-600 mx-1">|</span>
        <button
          onClick={() => setFilterTag(null)}
          className={clsx(
            'text-xs px-3 py-1 rounded-full border',
            filterTag === null
              ? 'bg-accent text-black border-accent font-semibold'
              : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
          )}
        >
          All Tags
        </button>
        {allTags.map((tag) => (
          <button
            key={tag}
            onClick={() => setFilterTag(filterTag === tag ? null : tag)}
            className={clsx(
              'text-xs px-3 py-1 rounded-full border capitalize',
              filterTag === tag
                ? 'bg-accent text-black border-accent font-semibold'
                : 'bg-surface-lighter text-gray-300 border-gray-600 hover:border-accent/50',
            )}
          >
            {tag}
          </button>
        ))}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 mb-6">
        {filteredMissions.map((mission) => (
          <button
            key={mission.mission_id}
            onClick={() => handleSelect(mission)}
            className={clsx(
              selectedMission?.mission_id === mission.mission_id
                ? 'card-selected'
                : 'card-hover',
              'text-left p-4',
            )}
          >
            <div className="flex items-start justify-between mb-2">
              <h3 className="font-semibold text-accent text-sm">{mission.name}</h3>
              <span className="text-xs text-gray-500 font-mono">{mission.mission_id}</span>
            </div>
            <div className="flex gap-2 flex-wrap">
              <span
                className={clsx(
                  'text-xs px-2 py-0.5 rounded',
                  mission.mission_type === 'symmetric'
                    ? 'bg-phase-command/20 text-phase-command'
                    : 'bg-phase-charge/20 text-phase-charge',
                )}
              >
                {mission.mission_type}
              </span>
              {mission.tags.map((tag) => (
                <span
                  key={tag}
                  className="text-xs bg-surface text-gray-400 px-2 py-0.5 rounded capitalize"
                >
                  {tag}
                </span>
              ))}
            </div>
          </button>
        ))}
      </div>

      <div className="text-center">
        <button
          onClick={() => setStep('opponent_setup')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Opponent Setup
        </button>
      </div>
    </div>
  );
}

// ===== Ready Step =====

function ReadyStep() {
  const playerFaction = useBoardingSetupStore((s) => s.playerFaction);
  const playerDetachment = useBoardingSetupStore((s) => s.playerDetachment);
  const selectedUnits = useBoardingSetupStore((s) => s.selectedUnits);
  const totalPoints = useBoardingSetupStore((s) => s.totalPoints);
  const enhancements = useBoardingSetupStore((s) => s.enhancements);
  const warlordIndex = useBoardingSetupStore((s) => s.warlordIndex);
  const opponentFaction = useBoardingSetupStore((s) => s.opponentFaction);
  const opponentDetachment = useBoardingSetupStore((s) => s.opponentDetachment);
  const selectedMission = useBoardingSetupStore((s) => s.selectedMission);
  const setStep = useBoardingSetupStore((s) => s.setStep);
  const reset = useBoardingSetupStore((s) => s.reset);

  const setScreen = useGameStore((s) => s.setScreen);
  const engineReady = useGameStore((s) => s.engineReady);
  const loading = useGameStore((s) => s.loading);

  if (!playerFaction || !playerDetachment || !selectedMission) return null;

  const handleStartBattle = () => {
    // In a future integration, this would call createBoardingMatch on the game store
    // For now, transition to the play screen
    console.log('[BoardingSetup] Starting battle with config:', {
      playerFaction: playerFaction.faction_name,
      playerDetachment: playerDetachment.detachment_name,
      units: selectedUnits,
      enhancements,
      warlordIndex,
      opponentFaction: opponentFaction?.faction_name,
      opponentDetachment: opponentDetachment?.detachment_name,
      mission: selectedMission,
      totalPoints,
    });
    // TODO: Hook into actual game engine boarding match creation
    // For now this logs the config; engine integration is Phase 16.7
    setScreen('play');
  };

  return (
    <div className="max-w-2xl mx-auto">
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-6">
        Ready to Battle
      </h2>

      <div className="card mb-6">
        <h3 className="font-heading text-lg text-accent mb-4">Battle Summary</h3>
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-gray-400">Your Faction:</span>
            <span className="ml-2 text-gray-100">{playerFaction.faction_name}</span>
          </div>
          <div>
            <span className="text-gray-400">Opponent:</span>
            <span className="ml-2 text-gray-100">{opponentFaction?.faction_name ?? 'N/A'}</span>
          </div>
          <div>
            <span className="text-gray-400">Detachment:</span>
            <span className="ml-2 text-gray-100">{playerDetachment.detachment_name}</span>
          </div>
          <div>
            <span className="text-gray-400">Opp. Detachment:</span>
            <span className="ml-2 text-gray-100">{opponentDetachment?.detachment_name ?? 'N/A'}</span>
          </div>
          <div>
            <span className="text-gray-400">Mission:</span>
            <span className="ml-2 text-gray-100">{selectedMission.name}</span>
          </div>
          <div>
            <span className="text-gray-400">Points:</span>
            <span className="ml-2 text-gray-100">{totalPoints} / 500</span>
          </div>
        </div>
      </div>

      {/* Roster */}
      <div className="card mb-6">
        <h3 className="font-heading text-lg text-accent mb-3">Your Roster</h3>
        <div className="space-y-2">
          {selectedUnits.map((unit, idx) => {
            const isWarlord = warlordIndex === idx;
            const unitEnhancements = enhancements.filter((e) => e.unit_index === idx);
            return (
              <div
                key={`${unit.datasheet_name}-${idx}`}
                className="flex items-center justify-between text-sm py-1 border-b border-gray-700 last:border-0"
              >
                <div className="flex items-center gap-2">
                  <span className="text-gray-100">{unit.datasheet_name}</span>
                  <span className="text-xs text-gray-500">
                    x{unit.model_count}
                  </span>
                  {isWarlord && (
                    <span className="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded">
                      WARLORD
                    </span>
                  )}
                  {unitEnhancements.map((e) => (
                    <span
                      key={e.enhancement_name}
                      className="text-xs bg-phase-prebattle/20 text-phase-prebattle px-1.5 py-0.5 rounded"
                    >
                      {e.enhancement_name}
                    </span>
                  ))}
                </div>
                <span className="text-accent font-mono text-xs">{unit.points}pts</span>
              </div>
            );
          })}
        </div>
      </div>

      <div className="flex gap-4 justify-center">
        <Button variant="secondary" onClick={() => reset()}>
          Start Over
        </Button>
        <Button variant="secondary" onClick={() => setStep('select_mission')}>
          Change Mission
        </Button>
        <Button
          variant="primary"
          size="lg"
          onClick={handleStartBattle}
          disabled={!engineReady || loading}
        >
          {loading ? 'Creating Match...' : 'Start Battle'}
        </Button>
      </div>
    </div>
  );
}

// ===== Main Boarding Setup Screen =====

export function BoardingSetupScreen() {
  const step = useBoardingSetupStore((s) => s.step);
  const setFactions = useBoardingSetupStore((s) => s.setFactions);
  const setMissions = useBoardingSetupStore((s) => s.setMissions);

  // Load faction and mission data on mount
  useEffect(() => {
    setFactions(BOARDING_FACTIONS);
    setMissions(BOARDING_MISSIONS);
  }, [setFactions, setMissions]);

  return (
    <div className="flex-1 flex flex-col items-center justify-start p-8 overflow-y-auto">
      <div className="max-w-4xl w-full">
        <h1 className="font-heading text-4xl text-accent font-bold text-center mb-2">
          BOARDING ACTIONS
        </h1>
        <p className="text-gray-400 text-center mb-4 text-sm">
          Ship Interior Combat - Army Setup
        </p>

        <StepIndicator currentStep={step} />

        {step === 'select_faction' && <SelectFactionStep />}
        {step === 'select_detachment' && <SelectDetachmentStep />}
        {step === 'build_army' && <BuildArmyStep />}
        {step === 'select_enhancements' && <SelectEnhancementsStep />}
        {step === 'designate_warlord' && <DesignateWarlordStep />}
        {step === 'opponent_setup' && <OpponentSetupStep />}
        {step === 'select_mission' && <SelectMissionStep />}
        {step === 'ready' && <ReadyStep />}
      </div>
    </div>
  );
}
