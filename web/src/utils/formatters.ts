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

export function factionColor(factionId: number | null): string {
  if (factionId === 0) return 'text-custodes-gold';
  if (factionId === 1) return 'text-worldeaters-red';
  return 'text-gray-400';
}

export function factionBgColor(factionId: number | null): string {
  if (factionId === 0) return 'bg-custodes-gold';
  if (factionId === 1) return 'bg-worldeaters-red';
  return 'bg-gray-600';
}

export function factionName(factionId: number | null): string {
  if (factionId === 0) return 'Adeptus Custodes';
  if (factionId === 1) return 'World Eaters';
  return 'Unknown';
}
