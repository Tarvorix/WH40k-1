import { useGameStore } from '@/store/gameStore';
import type {
  HatchwayView,
  UnitView,
  ObjectiveView,
  TerrainView,
  DeploymentZoneView,
} from '@/types/game';
import { clsx } from 'clsx';

// Hatchway state display colors
const HATCHWAY_COLORS: Record<HatchwayView['state'], string> = {
  Open: 'text-phase-movement',
  Closed: 'text-phase-shooting',
  Locked: 'text-phase-fight',
  OneWayOpened: 'text-phase-charge',
};

const HATCHWAY_LABELS: Record<HatchwayView['state'], string> = {
  Open: 'OPEN',
  Closed: 'CLOSED',
  Locked: 'LOCKED',
  OneWayOpened: 'ONE-WAY',
};

// Player color schemes (CSS equivalents of the PixiJS FACTION_COLORS)
const PLAYER_COLORS: Record<number, { primary: string; dark: string; ring: string }> = {
  0: { primary: '#C8A951', dark: '#8B6914', ring: 'rgba(200,169,81,0.4)' },  // Custodes gold
  1: { primary: '#CC3333', dark: '#8B0000', ring: 'rgba(204,51,51,0.4)' },   // World Eaters red
};

const OBJECTIVE_CONTROL_COLORS: Record<string, string> = {
  Uncontrolled: '#888888',
  Contested: '#FFFF00',
};

// Terrain type CSS colors
const TERRAIN_CSS_COLORS: Record<string, { fill: string; border: string }> = {
  cover: { fill: 'rgba(45,80,22,0.45)', border: 'rgba(45,80,22,0.8)' },
  blocking: { fill: 'rgba(74,55,40,0.45)', border: 'rgba(74,55,40,0.8)' },
  impassable: { fill: 'rgba(61,61,61,0.55)', border: 'rgba(61,61,61,0.9)' },
  default: { fill: 'rgba(74,103,65,0.35)', border: 'rgba(74,103,65,0.7)' },
};

/** Convert board-inches position to a CSS percentage for overlay positioning */
function toPercent(value: number, extent: number): number {
  return (value / extent) * 100;
}

/** Health bar color based on wounds ratio */
function getHealthColor(remaining: number, max: number): string {
  const ratio = remaining / Math.max(max, 1);
  if (ratio > 0.5) return '#00FF00';
  if (ratio > 0.25) return '#FFAA00';
  return '#FF0000';
}

/** Get color for a player-owned element */
function getPlayerColor(owner: number): { primary: string; dark: string; ring: string } {
  return PLAYER_COLORS[owner] ?? { primary: '#888888', dark: '#555555', ring: 'rgba(136,136,136,0.4)' };
}

/** Get objective marker color based on control status */
function getObjectiveColor(obj: ObjectiveView): string {
  if (obj.control_status === 'Contested') return OBJECTIVE_CONTROL_COLORS.Contested;
  if (obj.controlling_player === 0) return PLAYER_COLORS[0].primary;
  if (obj.controlling_player === 1) return PLAYER_COLORS[1].primary;
  return OBJECTIVE_CONTROL_COLORS.Uncontrolled;
}

/** Determine terrain CSS colors from terrain properties */
function getTerrainColors(terrain: TerrainView): { fill: string; border: string } {
  if (terrain.impassable) return TERRAIN_CSS_COLORS.impassable;
  if (terrain.blocks_los) return TERRAIN_CSS_COLORS.blocking;
  if (terrain.provides_cover) return TERRAIN_CSS_COLORS.cover;
  return TERRAIN_CSS_COLORS.default;
}

interface BoardingBoardViewProps {
  hatchways?: HatchwayView[];
}

export function BoardingBoardView({ hatchways }: BoardingBoardViewProps) {
  const gameState = useGameStore((s) => s.gameState);

  const boardWidth = gameState?.board?.width ?? 48;
  const boardHeight = gameState?.board?.height ?? 28;

  const units = gameState?.units ?? [];
  const objectives = gameState?.board?.objectives ?? [];
  const terrain = gameState?.board?.terrain ?? [];
  const deploymentZones = gameState?.board?.deployment_zones ?? [];

  // Filter to only units that have a position and are on the battlefield
  const deployedUnits = units.filter(
    (u) => u.position && u.status !== 'Destroyed' && u.status !== 'Undeployed',
  );

  return (
    <div className="w-full h-full bg-surface border border-gray-700 rounded-lg p-4 flex flex-col">
      {/* Header */}
      <div className="text-center mb-4">
        <div className="text-accent font-heading text-xl mb-1">
          BOARDING ACTIONS
        </div>
        <div className="text-gray-400 text-sm">
          Ship interior combat map &mdash; {boardWidth}&quot; x {boardHeight}&quot;
        </div>
      </div>

      {/* Board area — SVG/CSS tactical map */}
      <div
        className="flex-1 bg-surface-light rounded border border-gray-700 min-h-[256px] relative overflow-hidden"
        style={{ aspectRatio: `${boardWidth} / ${boardHeight}` }}
      >
        {/* Grid overlay */}
        <div className="absolute inset-0 opacity-20 pointer-events-none">
          <svg width="100%" height="100%">
            <defs>
              <pattern id="ba-grid" width="20" height="20" patternUnits="userSpaceOnUse">
                <path d="M 20 0 L 0 0 0 20" fill="none" stroke="white" strokeWidth="0.5" />
              </pattern>
            </defs>
            <rect width="100%" height="100%" fill="url(#ba-grid)" />
          </svg>
        </div>

        {/* Deployment zones */}
        {deploymentZones.map((dz, idx) => (
          <DeploymentZoneOverlay
            key={`dz-${idx}`}
            zone={dz}
            boardWidth={boardWidth}
            boardHeight={boardHeight}
          />
        ))}

        {/* Terrain / compartment walls */}
        {terrain.map((t) => (
          <TerrainBlock
            key={`terrain-${t.id}`}
            terrain={t}
            boardWidth={boardWidth}
            boardHeight={boardHeight}
          />
        ))}

        {/* Objective markers */}
        {objectives.map((obj) => (
          <ObjectiveMarker
            key={`obj-${obj.id}`}
            objective={obj}
            boardWidth={boardWidth}
            boardHeight={boardHeight}
          />
        ))}

        {/* Unit markers */}
        {deployedUnits.map((unit) => (
          <UnitMarker
            key={`unit-${unit.id}`}
            unit={unit}
            boardWidth={boardWidth}
            boardHeight={boardHeight}
          />
        ))}

        {/* Board edge labels */}
        <div className="absolute top-1 left-1 text-[10px] text-gray-600 font-mono pointer-events-none">
          0,0
        </div>
        <div className="absolute bottom-1 right-1 text-[10px] text-gray-600 font-mono pointer-events-none">
          {boardWidth}&quot;,{boardHeight}&quot;
        </div>
      </div>

      {/* Hatchway status panel */}
      {hatchways && hatchways.length > 0 && (
        <div className="mt-4">
          <div className="text-gray-400 text-xs font-semibold uppercase tracking-wider mb-2">
            Hatchway Status
          </div>
          <div className="flex flex-wrap gap-2">
            {hatchways.map((hw) => (
              <div
                key={hw.id}
                className="bg-surface rounded px-2 py-1 flex items-center gap-1.5"
              >
                <span className="text-gray-500 text-xs font-mono">H{hw.id}</span>
                <span
                  className={clsx(
                    'text-xs font-semibold',
                    HATCHWAY_COLORS[hw.state],
                  )}
                >
                  {HATCHWAY_LABELS[hw.state]}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Game info from current state */}
      {gameState && (
        <div className="mt-4 flex gap-4 text-xs text-gray-500">
          <span>Round: {gameState.battle_round}</span>
          <span>Phase: {gameState.phase}</span>
          <span>Active: Player {gameState.active_player + 1}</span>
          <span>
            Score: {gameState.players[0].vp} - {gameState.players[1].vp}
          </span>
        </div>
      )}
    </div>
  );
}

/* ========== Sub-components for board elements ========== */

/** Renders a deployment zone as a semi-transparent polygon overlay */
function DeploymentZoneOverlay({
  zone,
  boardWidth,
  boardHeight,
}: {
  zone: DeploymentZoneView;
  boardWidth: number;
  boardHeight: number;
}) {
  if (!zone.vertices || zone.vertices.length === 0) return null;

  const colors = getPlayerColor(zone.player);

  // Compute polygon points within a 1000x1000 viewBox for precise SVG rendering
  const viewBoxPoints = zone.vertices
    .map((v) => `${(v.x / boardWidth) * 1000},${(v.y / boardHeight) * 1000}`)
    .join(' ');

  return (
    <div className="absolute inset-0 pointer-events-none">
      <svg width="100%" height="100%" viewBox="0 0 1000 1000" preserveAspectRatio="none">
        <polygon
          points={viewBoxPoints}
          fill={colors.ring}
          stroke={colors.primary}
          strokeWidth="2"
          strokeDasharray="8 4"
          opacity="0.5"
        />
        <text
          x={
            (zone.vertices.reduce((sum, v) => sum + v.x, 0) /
              zone.vertices.length /
              boardWidth) *
            1000
          }
          y={
            (zone.vertices.reduce((sum, v) => sum + v.y, 0) /
              zone.vertices.length /
              boardHeight) *
            1000
          }
          textAnchor="middle"
          dominantBaseline="middle"
          fill={colors.primary}
          fontSize="28"
          fontWeight="bold"
          fontFamily="Inter, sans-serif"
          opacity="0.7"
        >
          P{zone.player + 1} Deploy
        </text>
      </svg>
    </div>
  );
}

/** Renders a terrain feature / compartment wall as a positioned rectangle */
function TerrainBlock({
  terrain,
  boardWidth,
  boardHeight,
}: {
  terrain: TerrainView;
  boardWidth: number;
  boardHeight: number;
}) {
  const { fill, border } = getTerrainColors(terrain);

  const left = toPercent(terrain.rect.min_x, boardWidth);
  const top = toPercent(terrain.rect.min_y, boardHeight);
  const width = toPercent(terrain.rect.max_x - terrain.rect.min_x, boardWidth);
  const height = toPercent(terrain.rect.max_y - terrain.rect.min_y, boardHeight);

  return (
    <div
      className="absolute pointer-events-none"
      style={{
        left: `${left}%`,
        top: `${top}%`,
        width: `${width}%`,
        height: `${height}%`,
        backgroundColor: fill,
        border: `1px solid ${border}`,
        borderRadius: '2px',
      }}
    >
      <span
        className="absolute inset-0 flex items-center justify-center text-[8px] text-gray-400 font-mono truncate px-0.5"
        title={terrain.name}
      >
        {terrain.name}
      </span>
    </div>
  );
}

/** Renders an objective marker as a positioned circle with label */
function ObjectiveMarker({
  objective,
  boardWidth,
  boardHeight,
}: {
  objective: ObjectiveView;
  boardWidth: number;
  boardHeight: number;
}) {
  const color = getObjectiveColor(objective);
  const left = toPercent(objective.position.x, boardWidth);
  const top = toPercent(objective.position.y, boardHeight);

  return (
    <div
      className="absolute z-20 pointer-events-none"
      style={{
        left: `${left}%`,
        top: `${top}%`,
        transform: 'translate(-50%, -50%)',
      }}
    >
      {/* Outer ring */}
      <div
        className="flex items-center justify-center rounded-full"
        style={{
          width: '28px',
          height: '28px',
          border: `2px solid ${color}`,
          backgroundColor: `${color}33`,
        }}
      >
        <span
          className="text-[10px] font-bold text-white leading-none"
          style={{ textShadow: '0 0 3px rgba(0,0,0,0.8)' }}
        >
          {objective.label}
        </span>
      </div>
      {/* Control status label */}
      <div
        className="text-[8px] text-center mt-0.5 font-mono"
        style={{ color }}
      >
        {objective.control_status === 'Contested'
          ? 'CONTESTED'
          : objective.controlling_player != null
            ? `P${objective.controlling_player + 1}`
            : ''}
      </div>
    </div>
  );
}

/** Renders a unit as a positioned marker with name, model count, and wound bar */
function UnitMarker({
  unit,
  boardWidth,
  boardHeight,
}: {
  unit: UnitView;
  boardWidth: number;
  boardHeight: number;
}) {
  if (!unit.position) return null;

  const colors = getPlayerColor(unit.owner);
  const left = toPercent(unit.position.x, boardWidth);
  const top = toPercent(unit.position.y, boardHeight);

  // Calculate total wounds for the health bar
  const totalMaxWounds = unit.models.reduce((sum, m) => sum + m.wounds_max, 0);
  const healthColor = getHealthColor(unit.total_wounds_remaining, totalMaxWounds);
  const healthPct = (unit.total_wounds_remaining / Math.max(totalMaxWounds, 1)) * 100;

  // Short display name: first word or abbreviation
  const shortName =
    unit.name.length > 12 ? unit.name.slice(0, 11) + '\u2026' : unit.name;

  return (
    <div
      className="absolute z-30"
      style={{
        left: `${left}%`,
        top: `${top}%`,
        transform: 'translate(-50%, -50%)',
      }}
    >
      {/* Unit circle */}
      <div
        className="flex items-center justify-center rounded-full relative"
        style={{
          width: '26px',
          height: '26px',
          backgroundColor: colors.dark,
          border: `2px solid ${colors.primary}`,
          boxShadow: `0 0 6px ${colors.ring}`,
        }}
      >
        {/* Model count inside circle */}
        <span className="text-[10px] font-bold text-white leading-none">
          {unit.models_alive}
        </span>

        {/* Battle-shocked indicator */}
        {unit.battle_shocked && (
          <div
            className="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full"
            style={{ backgroundColor: '#FF00FF' }}
            title="Battle-shocked"
          />
        )}
      </div>

      {/* Unit name label below the circle */}
      <div
        className="text-[9px] font-semibold text-center whitespace-nowrap mt-0.5 leading-tight"
        style={{
          color: colors.primary,
          textShadow: '0 0 4px rgba(0,0,0,0.9), 0 0 2px rgba(0,0,0,0.9)',
          maxWidth: '80px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
        title={`${unit.name} (P${unit.owner + 1})`}
      >
        {shortName}
      </div>

      {/* Health bar */}
      <div
        className="mt-0.5 rounded-full overflow-hidden"
        style={{
          width: '30px',
          height: '3px',
          backgroundColor: 'rgba(255,255,255,0.15)',
          margin: '0 auto',
        }}
      >
        <div
          className="h-full rounded-full transition-all duration-200"
          style={{
            width: `${healthPct}%`,
            backgroundColor: healthColor,
          }}
        />
      </div>

      {/* Wounds text */}
      <div className="text-[8px] text-gray-400 text-center font-mono leading-tight">
        {unit.total_wounds_remaining}W
      </div>
    </div>
  );
}
