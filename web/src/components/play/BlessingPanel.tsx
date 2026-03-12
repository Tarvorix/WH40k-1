import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';

export function BlessingPanel() {
  const gameState = useGameStore((s) => s.gameState);
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const applyAction = useGameStore((s) => s.applyAction);

  if (!gameState) return null;

  // Check if any player has blessing dice
  const playerWithBlessings = gameState.players.find(
    (p) => p.blessing_dice.length > 0,
  );

  if (!playerWithBlessings) return null;

  const blessingActions = decisionSurface?.actions.filter(
    (a) => a.command_type === 'Scoring' || a.label.toLowerCase().includes('blessing'),
  ) ?? [];

  return (
    <Panel title="Blessings of Khorne">
      {/* Dice pool display */}
      <div className="flex gap-1 mb-3">
        {playerWithBlessings.blessing_dice.map((die, i) => (
          <div
            key={i}
            className="w-8 h-8 bg-worldeaters-dark border border-worldeaters-red rounded flex items-center justify-center text-sm font-bold text-worldeaters-light"
          >
            {die}
          </div>
        ))}
      </div>

      {/* Active blessings */}
      {playerWithBlessings.active_blessings.length > 0 && (
        <div className="mb-3">
          <h4 className="text-xs text-gray-400 mb-1">Active Blessings</h4>
          <div className="flex flex-wrap gap-1">
            {playerWithBlessings.active_blessings.map((b) => (
              <span key={b} className="px-2 py-0.5 bg-worldeaters-dark/50 text-worldeaters-light text-xs rounded">
                {b}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Allocation actions */}
      {blessingActions.length > 0 && (
        <div className="space-y-1">
          {blessingActions.map((action) => (
            <button
              key={action.index}
              onClick={() => applyAction(action.index)}
              className="w-full text-left px-2 py-1.5 bg-surface hover:bg-worldeaters-dark/30 rounded text-xs text-gray-300 transition-colors"
            >
              {action.label}
            </button>
          ))}
        </div>
      )}
    </Panel>
  );
}
