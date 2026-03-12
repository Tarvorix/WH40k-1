// Faction color hex values for PixiJS rendering
export const FACTION_COLORS = {
  0: { primary: 0xC8A951, dark: 0x8B6914, light: 0xE5D48B }, // Custodes
  1: { primary: 0x8B0000, dark: 0x5C0000, light: 0xCC3333 }, // World Eaters
} as const;

// Phase colors for PixiJS
export const PHASE_COLORS = {
  Command: 0x4A90D9,
  Movement: 0x50C878,
  Shooting: 0xE8A317,
  Charge: 0xCC5500,
  Fight: 0xDC143C,
  PreBattle: 0x9B59B6,
  GameEnd: 0x7F8C8D,
} as const;

// Objective control status colors
export const OBJECTIVE_COLORS = {
  Uncontrolled: 0x888888,
  Contested: 0xFFFF00,
  player_0: 0xC8A951,
  player_1: 0x8B0000,
} as const;

// Terrain colors
export const TERRAIN_COLORS = {
  cover: 0x2D5016,
  blocking: 0x4A3728,
  impassable: 0x3D3D3D,
  default: 0x4A6741,
} as const;

// Unit status colors
export const STATUS_COLORS = {
  healthy: 0x00FF00,
  wounded: 0xFFAA00,
  critical: 0xFF0000,
  destroyed: 0x555555,
  battleShocked: 0xFF00FF,
} as const;

export function getFactionPrimaryColor(factionId: number): number {
  return FACTION_COLORS[factionId as keyof typeof FACTION_COLORS]?.primary ?? 0x888888;
}

export function getFactionDarkColor(factionId: number): number {
  return FACTION_COLORS[factionId as keyof typeof FACTION_COLORS]?.dark ?? 0x555555;
}

export function getPhaseColor(phase: string): number {
  return PHASE_COLORS[phase as keyof typeof PHASE_COLORS] ?? 0x888888;
}

export function getUnitHealthColor(woundsRemaining: number, woundsMax: number): number {
  const ratio = woundsRemaining / Math.max(woundsMax, 1);
  if (ratio > 0.5) return STATUS_COLORS.healthy;
  if (ratio > 0.25) return STATUS_COLORS.wounded;
  return STATUS_COLORS.critical;
}
