import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { Button } from '@/components/shared/Button';
import type { ActionView } from '@/types/game';
import { clsx } from 'clsx';

export function ActionPanel() {
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const selectedActionIndex = useGameStore((s) => s.selectedActionIndex);
  const selectAction = useGameStore((s) => s.selectAction);
  const applyAction = useGameStore((s) => s.applyAction);
  const loading = useGameStore((s) => s.loading);

  if (!decisionSurface) return null;

  // Group actions by command type
  const grouped = new Map<string, ActionView[]>();
  for (const action of decisionSurface.actions) {
    const key = action.command_type;
    if (!grouped.has(key)) grouped.set(key, []);
    grouped.get(key)!.push(action);
  }

  return (
    <Panel title={`Actions (${decisionSurface.decision_type})`}>
      <div className="space-y-2">
        {Array.from(grouped.entries()).map(([category, actions]) => (
          <div key={category}>
            <h4 className="text-[10px] uppercase tracking-wider text-gray-500 mb-1">
              {category}
            </h4>
            <div className="space-y-0.5">
              {actions.map((action) => (
                <button
                  key={action.index}
                  onClick={() => {
                    selectAction(action.index);
                    applyAction(action.index);
                  }}
                  disabled={loading}
                  className={clsx(
                    'w-full text-left px-2 py-1.5 rounded text-xs transition-colors',
                    selectedActionIndex === action.index
                      ? 'bg-accent/20 text-accent border border-accent/30'
                      : 'bg-surface hover:bg-surface-lighter text-gray-300',
                  )}
                >
                  {action.label}
                </button>
              ))}
            </div>
          </div>
        ))}

        {decisionSurface.is_forced && decisionSurface.actions.length === 1 && (
          <div className="text-xs text-gray-500 italic">
            Only one action available (auto-executing...)
          </div>
        )}
      </div>
    </Panel>
  );
}
