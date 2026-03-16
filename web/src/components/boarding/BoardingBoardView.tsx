import { useGameStore } from '@/store/gameStore';
import type { HatchwayView } from '@/types/game';
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

interface BoardingBoardViewProps {
  hatchways?: HatchwayView[];
}

export function BoardingBoardView({ hatchways }: BoardingBoardViewProps) {
  const gameState = useGameStore((s) => s.gameState);

  const boardWidth = gameState?.board?.width ?? 42;
  const boardHeight = gameState?.board?.height ?? 22;

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

      {/* Board area - placeholder for PixiJS/Canvas rendering */}
      <div className="flex-1 bg-surface-light rounded border border-gray-700 flex items-center justify-center min-h-[256px] relative">
        {/* Grid overlay hint */}
        <div className="absolute inset-0 opacity-5">
          <svg width="100%" height="100%">
            <defs>
              <pattern id="ba-grid" width="20" height="20" patternUnits="userSpaceOnUse">
                <path d="M 20 0 L 0 0 0 20" fill="none" stroke="white" strokeWidth="0.5" />
              </pattern>
            </defs>
            <rect width="100%" height="100%" fill="url(#ba-grid)" />
          </svg>
        </div>

        <div className="text-center z-10">
          <div className="text-gray-500 text-lg font-heading mb-2">
            Tactical Map Display
          </div>
          <div className="text-gray-600 text-xs">
            Full board rendering with walls, hatchways, and compartments will be
            implemented with PixiJS/Canvas in Phase 16.8
          </div>
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
