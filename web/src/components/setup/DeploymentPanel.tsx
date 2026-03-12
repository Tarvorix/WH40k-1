import { useGameStore } from '@/store/gameStore';
import type { UnitView } from '@/types/game';

export function DeploymentPanel() {
  const gameState = useGameStore((s) => s.gameState);
  const decisionSurface = useGameStore((s) => s.decisionSurface);
  const applyAction = useGameStore((s) => s.applyAction);

  if (!gameState) return null;

  const undeployedUnits = gameState.units.filter(
    (u) => u.status === 'Undeployed' && u.owner === gameState.decision_owner,
  );

  if (undeployedUnits.length === 0) return null;

  return (
    <div className="p-3 border-b border-gray-700">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">
        Deploy Units
      </h3>
      <div className="space-y-2">
        {undeployedUnits.map((unit) => (
          <UnitDeployCard key={unit.id} unit={unit} />
        ))}
      </div>
      {decisionSurface && (
        <div className="mt-3 space-y-1">
          {decisionSurface.actions
            .filter((a) => a.command_type === 'Setup')
            .map((action) => (
              <button
                key={action.index}
                onClick={() => applyAction(action.index)}
                className="w-full text-left text-xs px-2 py-1.5 bg-surface hover:bg-surface-lighter rounded text-gray-300"
              >
                {action.label}
              </button>
            ))}
        </div>
      )}
    </div>
  );
}

function UnitDeployCard({ unit }: { unit: UnitView }) {
  return (
    <div className="flex items-center gap-2 px-2 py-1.5 bg-surface rounded text-sm">
      <span className="text-gray-300">{unit.name}</span>
      <span className="text-gray-500 text-xs ml-auto">
        {unit.models_alive} models
      </span>
    </div>
  );
}
