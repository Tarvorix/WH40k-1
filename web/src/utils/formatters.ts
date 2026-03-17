export function formatInches(value: number): string {
  if (value === 0) return '0"';
  if (Number.isInteger(value)) return `${value}"`;
  return `${value.toFixed(1)}"`;
}

export function formatVP(vp: number): string {
  return `${vp} VP`;
}

export function formatCP(cp: number): string {
  return `${cp} CP`;
}

export function formatSave(save: string): string {
  return save; // Already formatted as "3+", "2+", etc.
}

export function formatWeaponRange(range: number, weaponType: string): string {
  if (weaponType === 'Melee') return 'Melee';
  return formatInches(range);
}

export function formatScore(score: number): string {
  if (score > 90000) return '+WIN';
  if (score < -90000) return '-WIN';
  return score > 0 ? `+${score}` : `${score}`;
}

export function phaseColor(phase: string): string {
  const colors: Record<string, string> = {
    Command: 'text-phase-command',
    Movement: 'text-phase-movement',
    Shooting: 'text-phase-shooting',
    Charge: 'text-phase-charge',
    Fight: 'text-phase-fight',
    PreBattle: 'text-phase-prebattle',
    GameEnd: 'text-phase-gameend',
  };
  return colors[phase] ?? 'text-gray-400';
}

export function phaseBgColor(phase: string): string {
  const colors: Record<string, string> = {
    Command: 'bg-phase-command',
    Movement: 'bg-phase-movement',
    Shooting: 'bg-phase-shooting',
    Charge: 'bg-phase-charge',
    Fight: 'bg-phase-fight',
    PreBattle: 'bg-phase-prebattle',
    GameEnd: 'bg-phase-gameend',
  };
  return colors[phase] ?? 'bg-gray-600';
}

const FACTION_TEXT_COLORS: Record<number, string> = {
  0: 'text-custodes-gold',
  1: 'text-worldeaters-red',
  2: 'text-blue-400',
  3: 'text-red-600',
  4: 'text-red-700',
  5: 'text-gray-400',
  6: 'text-yellow-700',
  7: 'text-green-500',
};

const FACTION_BG_COLORS: Record<number, string> = {
  0: 'bg-custodes-gold',
  1: 'bg-worldeaters-red',
  2: 'bg-blue-600',
  3: 'bg-red-700',
  4: 'bg-red-800',
  5: 'bg-gray-600',
  6: 'bg-yellow-800',
  7: 'bg-green-700',
};

const FACTION_NAMES: Record<number, string> = {
  0: 'Adeptus Custodes',
  1: 'World Eaters',
  2: 'Space Marines - Terminator Assault',
  3: 'World Eaters - Boarding Butchers',
  4: 'World Eaters - Skullsworn',
  5: 'Chaos Space Marines - Champions of Chaos',
  6: 'Chaos Space Marines - Underdeck Uprising',
  7: 'Astra Militarum - Tempestus Regiment',
};

export function factionColor(factionId: number | null): string {
  if (factionId == null) return 'text-gray-400';
  return FACTION_TEXT_COLORS[factionId] ?? 'text-gray-400';
}

export function factionBgColor(factionId: number | null): string {
  if (factionId == null) return 'bg-gray-600';
  return FACTION_BG_COLORS[factionId] ?? 'bg-gray-600';
}

export function factionName(factionId: number | null): string {
  if (factionId == null) return 'Unknown';
  return FACTION_NAMES[factionId] ?? 'Unknown';
}
