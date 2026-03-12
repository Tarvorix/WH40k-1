import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';

export function StratagemPanel() {
  const gameState = useGameStore((s) => s.gameState);
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const applyAction = useGameStore((s) => s.applyAction);

  if (!gameState || !decisionSurface) return null;

  // Filter stratagem-related actions
  const stratagemActions = decisionSurface.actions.filter(
    (a) => a.command_type === 'Stratagem',
  );

  if (stratagemActions.length === 0) return null;

  return (
    <Panel title="Stratagems">
      <div className="space-y-1.5">
        {stratagemActions.map((action) => (
          <button
            key={action.index}
            onClick={() => applyAction(action.index)}
            className="w-full text-left px-2 py-2 bg-surface hover:bg-surface-lighter rounded transition-colors"
          >
            <div className="flex items-center gap-2">
              <span className="shrink-0 w-6 h-6 flex items-center justify-center bg-phase-command/20 rounded text-phase-command text-xs font-bold">
                CP
              </span>
              <span className="text-sm text-gray-200">{action.label}</span>
            </div>
          </button>
        ))}
      </div>
    </Panel>
  );
}
