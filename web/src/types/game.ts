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
  | 'Perturabo_Shallow' | 'Perturabo_Regular' | 'Perturabo_Deep'
  | 'Alpharius_Shallow' | 'Alpharius_Regular' | 'Alpharius_Deep';

// Screen routing
export type Screen = 'setup' | 'play' | 'replay' | 'game_end';

// Phase names
export const PHASES = ['Command', 'Movement', 'Shooting', 'Charge', 'Fight'] as const;
export type PhaseName = typeof PHASES[number];
