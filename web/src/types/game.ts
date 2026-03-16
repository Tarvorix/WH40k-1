// Game view model types - mirrors Rust view_models.rs

export interface GameView {
  battle_round: number;
  phase: string;
  subphase: string;
  active_player: number;
  decision_owner: number;
  players: [PlayerView, PlayerView];
  units: UnitView[];
  board: BoardView;
  events: EventView[];
  outcome: GameOutcomeView;
  in_progress: boolean;
  content_version: string;
  scenario_id: number | null;
}

export interface PlayerView {
  id: number;
  name: string;
  faction_id: number | null;
  cp: number;
  vp: number;
  primary_vp: number;
  secondary_vp: number;
  enhancement_choice: number | null;
  secondary_choice: number | null;
  patrol_squad_choice: number | null;
  first_turn: boolean;
  active_blessings: string[];
  blessing_dice: number[];
}

export interface UnitView {
  id: number;
  owner: number;
  name: string;
  datasheet_id: number;
  keywords: string[];
  models: ModelView[];
  movement: number;
  toughness: number;
  armor_save: string;
  invulnerable_save: string | null;
  leadership: number;
  oc: number;
  status: string;
  battle_shocked: boolean;
  below_half_strength: boolean;
  engagement_status: string;
  attached_leader: number | null;
  bodyguard_for: number | null;
  reserve_type: string | null;
  models_alive: number;
  starting_model_count: number;
  total_wounds_remaining: number;
  wargear_abilities: WargearAbilityView[];
  is_character: boolean;
  is_infantry: boolean;
  position: PositionView | null;
  turn_flags: UnitTurnFlagsView;
}

export interface UnitTurnFlagsView {
  has_moved: boolean;
  has_shot: boolean;
  has_charged: boolean;
  has_fought: boolean;
  has_advanced: boolean;
  has_fell_back: boolean;
  is_stationary: boolean;
  charged_this_turn: boolean;
}

export interface WargearAbilityView {
  name: string;
  description: string;
  active: boolean;
}

export interface ModelView {
  id: number;
  unit_id: number;
  alive: boolean;
  wounds_max: number;
  wounds_remaining: number;
  position: PositionView;
  base_size_mm: number;
  ranged_weapons: WeaponView[];
  melee_weapons: WeaponView[];
  is_leader: boolean;
  feel_no_pain: string | null;
  allocation_status: string;
}

export interface WeaponView {
  id: number;
  name: string;
  weapon_type: string;
  range: number;
  attacks: string;
  skill: string;
  strength: number;
  ap: string;
  damage: string;
  abilities: string[];
}

export interface BoardView {
  width: number;
  height: number;
  terrain: TerrainView[];
  objectives: ObjectiveView[];
  deployment_zones: DeploymentZoneView[];
}

export interface TerrainView {
  id: number;
  name: string;
  rect: RectView;
  provides_cover: boolean;
  blocks_los: boolean;
  impassable: boolean;
  height: number;
}

export interface RectView {
  min_x: number;
  min_y: number;
  max_x: number;
  max_y: number;
}

export interface ObjectiveView {
  id: number;
  position: PositionView;
  label: string;
  control_status: string;
  controlling_player: number | null;
}

export interface DeploymentZoneView {
  player: number;
  vertices: PositionView[];
  map_type: string;
}

export interface PositionView {
  x: number;
  y: number;
}

export interface EventView {
  event_type: string;
  description: string;
  player: number | null;
  unit_ids: number[];
  round: number;
  phase: string;
}

export interface DecisionSurfaceView {
  decision_type: string;
  owner: number;
  actions: ActionView[];
  is_mandatory: boolean;
  num_options: number;
  is_forced: boolean;
}

export interface ActionView {
  index: number;
  label: string;
  command_type: string;
  unit_id: number | null;
  target_id: number | null;
  position: PositionView | null;
  player: number | null;
}

export interface StratagemView {
  id: number;
  name: string;
  cp_cost: number;
  phase: string;
  description: string;
  can_use: boolean;
  reason: string | null;
}

export interface AiResultView {
  best_action_label: string;
  best_action_commands: string[];
  score: number;
  nodes_evaluated: number;
  max_depth: number;
  time_ms: number;
  pv: string[];
  candidates: AiCandidateView[];
  intent: string;
}

export interface AiCandidateView {
  label: string;
  score: number;
  intent: string;
}

export interface GameOutcomeView {
  status: string;
  winner: number | null;
  player_0_vp: number;
  player_1_vp: number;
}

export interface ReplayInfoView {
  format_version: number;
  total_frames: number;
  current_frame: number;
  outcome: GameOutcomeView;
  players: ReplayPlayerInfoView[];
  has_more: boolean;
}

export interface ReplayPlayerInfoView {
  player_id: number;
  name: string;
  faction_id: number | null;
  enhancement: number | null;
  secondary: number | null;
}

export interface ValidationResultView {
  valid: boolean;
  reason: string | null;
}

// Difficulty levels for AI
export type AiDifficulty =
  | 'Basic_Recruit' | 'Basic_Battle_Ready' | 'Basic_Veteran' | 'Basic_Elite'
  | 'Perturabo';

// Screen routing
export type Screen = 'menu' | 'setup' | 'boarding_setup' | 'play' | 'replay' | 'game_end';

// Phase names
export const PHASES = ['Command', 'Movement', 'Shooting', 'Charge', 'Fight'] as const;
export type PhaseName = typeof PHASES[number];

// ===== Boarding Actions Types =====

export interface BoardingFaction {
  faction_name: string;
  faction_keyword: string;
  army_rule_name: string;
  army_rule_description: string;
  detachments: BoardingDetachment[];
  datasheets: BoardingUnitDatasheet[];
}

export interface BoardingDetachment {
  detachment_name: string;
  detachment_rule_name: string;
  detachment_rule_description: string;
  enhancements: BoardingEnhancement[];
  stratagems: BoardingStratagem[];
  allowed_units: BoardingUnitRef[];
  mustering_rules: MusteringRule[];
}

export interface BoardingEnhancement {
  name: string;
  description: string;
}

export interface BoardingStratagem {
  name: string;
  cp_cost: number;
  timing: string;
  target: string;
  effect: string;
}

export interface BoardingUnitRef {
  datasheet_name: string;
  max_count: number | null;
  allowed_sizes: number[];
  conditional_limit: string | null;
}

export interface MusteringRule {
  rule_type: string;
  description: string;
}

export interface BoardingUnitDatasheet {
  name: string;
  points: { model_count: number; points: number }[];
  is_epic_hero: boolean;
  is_character: boolean;
  is_battleline: boolean;
  profile: {
    movement: string;
    toughness: number;
    save: string;
    wounds: number;
    leadership: string;
    oc: number;
  };
  ranged_weapons: WeaponProfileView[];
  melee_weapons: WeaponProfileView[];
  abilities: { name: string; description: string; ability_type: string }[];
  leader_info: { can_lead: string[]; leader_abilities: string[] } | null;
  keywords: string[];
  faction_keywords: string[];
  wargear_options: { description: string }[];
  unit_composition: { model_name: string; count: string }[];
}

export interface WeaponProfileView {
  name: string;
  range: string;
  attacks: string;
  skill: string;
  strength: number;
  ap: string;
  damage: string;
  abilities: string[];
}

export interface BoardingMissionSummary {
  mission_id: string;
  name: string;
  mission_type: string;
  tags: string[];
}

export interface BoardingRosterValidation {
  valid: boolean;
  errors: { code: string; message: string }[];
  warnings: { code: string; message: string }[];
}

export interface SelectedUnit {
  datasheet_name: string;
  model_count: number;
  points: number;
  wargear_selections: number[];
}

export interface EnhancementAssignment {
  enhancement_name: string;
  unit_index: number;
}

// Hatchway view for board rendering
export interface HatchwayView {
  id: number;
  state: 'Open' | 'Closed' | 'Locked' | 'OneWayOpened';
}

// Secured objective view
export interface SecuredObjectiveView {
  objective_id: number;
  player: number;
}
