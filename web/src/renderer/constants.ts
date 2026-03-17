// Battlefield rendering constants

// Board size in inches (Combat Patrol)
export const BOARD_WIDTH_INCHES = 44;
export const BOARD_HEIGHT_INCHES = 30;

// Pixels per inch
export const PX_PER_INCH = 20;

// Canvas size in pixels
export const CANVAS_WIDTH = BOARD_WIDTH_INCHES * PX_PER_INCH; // 880
export const CANVAS_HEIGHT = BOARD_HEIGHT_INCHES * PX_PER_INCH; // 600

// Grid
export const GRID_LINE_COLOR = 0x333344;
export const GRID_LINE_ALPHA = 0.3;
export const GRID_LINE_WIDTH = 1;

// Background
export const BOARD_BG_COLOR = 0x1a1a2e;
export const BOARD_BORDER_COLOR = 0x555577;
export const BOARD_BORDER_WIDTH = 2;

// Units
export const UNIT_CIRCLE_RADIUS = 12;
export const UNIT_SELECTED_RING_RADIUS = 16;
export const UNIT_TARGET_RING_RADIUS = 16;
export const UNIT_LABEL_FONT_SIZE = 9;
export const UNIT_COUNT_BADGE_SIZE = 8;
export const WOUND_BAR_WIDTH = 20;
export const WOUND_BAR_HEIGHT = 3;

// Objectives
export const OBJECTIVE_RADIUS = 12;
export const OBJECTIVE_LABEL_FONT_SIZE = 10;

// Terrain
export const TERRAIN_ALPHA = 0.5;
export const TERRAIN_BORDER_WIDTH = 1;

// Deployment zones
export const DEPLOYMENT_ZONE_ALPHA = 0.15;
export const DEPLOYMENT_ZONE_BORDER_ALPHA = 0.4;
export const DEPLOYMENT_ZONE_BORDER_WIDTH = 2;

// Movement preview
export const MOVE_GHOST_ALPHA = 0.4;
export const MOVE_LINE_COLOR = 0x50C878;
export const MOVE_LINE_ALPHA = 0.6;
export const MOVE_LINE_WIDTH = 2;
export const RANGE_CIRCLE_ALPHA = 0.2;

// Attack visualization
export const ATTACK_LINE_COLOR = 0xE8A317;
export const ATTACK_LINE_ALPHA = 0.7;
export const ATTACK_LINE_WIDTH = 2;
export const CHARGE_LINE_COLOR = 0xCC5500;
export const CHARGE_LINE_ALPHA = 0.7;

// Camera
export const MIN_ZOOM = 0.5;
export const MAX_ZOOM = 4.0;
export const ZOOM_SPEED = 0.1;

// Board dimensions helper (supports multiple game modes)
export function getBoardDimensions(gameMode?: string): { widthInches: number; heightInches: number } {
  if (gameMode === 'BoardingActions') {
    return { widthInches: 48, heightInches: 28 };
  }
  return { widthInches: BOARD_WIDTH_INCHES, heightInches: BOARD_HEIGHT_INCHES };
}

// Per-model rendering
/** Convert a model's base size in mm to a pixel radius on the board. */
export function baseRadiusPx(baseSizeMm: number): number {
  return (baseSizeMm / 2 / 25.4) * PX_PER_INCH;
}

// Touch
export const LONG_PRESS_MS = 500;
export const TAP_THRESHOLD_PX = 10;
export const TAP_THRESHOLD_MS = 300;
