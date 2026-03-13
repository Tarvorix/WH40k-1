import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { Button } from '@/components/shared/Button';
import type { ActionView } from '@/types/game';
import { clsx } from 'clsx';

export function ActionPanel() {
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const gameState = useGameStore((s) => s.gameState);
  const selectedActionIndex = useGameStore((s) => s.selectedActionIndex);
  const selectedUnitId = useGameStore((s) => s.selectedUnitId);
  const selectAction = useGameStore((s) => s.selectAction);
  const selectUnit = useGameStore((s) => s.selectUnit);
  const applyAction = useGameStore((s) => s.applyAction);
  const loading = useGameStore((s) => s.loading);

  if (!decisionSurface) return null;

  if (decisionSurface.actions.length === 0) {
    return (
      <Panel title="Actions">
        <p className="text-gray-500 text-sm italic">Waiting for opponent...</p>
      </Panel>
    );
  }

  // Check if we're in deployment phase
  const isDeployment = decisionSurface.actions.some((a) => a.command_type === 'Setup');
  const isMovement = gameState?.phase === 'Movement';

  // Get undeployed units for the deployment unit selector
  const undeployedUnits = isDeployment && gameState
    ? gameState.units.filter(
        (u) => u.owner === gameState.decision_owner && u.status === 'Undeployed',
      )
    : [];

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
        {/* Deployment: Unit selector + click-to-deploy hint */}
        {isDeployment && undeployedUnits.length > 0 && (
          <div className="mb-2">
            <h4 className="text-[10px] uppercase tracking-wider text-accent mb-1">
              Select Unit to Deploy
            </h4>
            <div className="space-y-0.5">
              {undeployedUnits.map((unit) => (
                <button
                  key={unit.id}
                  onClick={() => selectUnit(unit.id)}
                  disabled={loading}
                  className={clsx(
                    'w-full text-left px-2 py-1.5 rounded text-xs transition-colors',
                    selectedUnitId === unit.id
                      ? 'bg-accent/20 text-accent border border-accent/30'
                      : 'bg-surface hover:bg-surface-lighter text-gray-300',
                  )}
                >
                  {unit.name} ({unit.models_alive} models)
                </button>
              ))}
            </div>
            <p className="text-[10px] text-gray-500 mt-1 italic">
              {selectedUnitId != null
                ? 'Click on the map to deploy this unit'
                : 'Select a unit above, then click on the map'}
            </p>
          </div>
        )}

        {/* Movement: Click-to-move hint when a unit is selected */}
        {isMovement && selectedUnitId != null && (
          <div className="mb-2">
            <p className="text-[10px] text-accent italic">
              Click within the movement circle to move, or use actions below
            </p>
          </div>
        )}

        {/* Standard action groups */}
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
