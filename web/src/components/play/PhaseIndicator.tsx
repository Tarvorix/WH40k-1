import { useGameStore } from '@/store/gameStore';
import { clsx } from 'clsx';
import { PHASES } from '@/types/game';
import { phaseBgColor } from '@/utils/formatters';

export function PhaseIndicator() {
  const gameState = useGameStore((s) => s.gameState);
  if (!gameState) return null;

  const currentPhase = gameState.phase;

  return (
    <div className="h-8 bg-surface-light border-b border-gray-700 flex items-center px-4 gap-1 shrink-0">
      {PHASES.map((phase) => {
        const isCurrent = currentPhase === phase;
        return (
          <div
            key={phase}
            className={clsx(
              'px-3 py-1 rounded text-xs font-semibold uppercase tracking-wider transition-all',
              isCurrent
                ? `${phaseBgColor(phase)} text-white`
                : 'bg-surface text-gray-500',
            )}
          >
            {phase}
          </div>
        );
      })}
      {currentPhase === 'PreBattle' && (
        <div className="px-3 py-1 rounded text-xs font-semibold uppercase tracking-wider bg-phase-prebattle text-white">
          Pre-Battle
        </div>
      )}
    </div>
  );
}
