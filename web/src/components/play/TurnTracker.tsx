import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { factionName, factionColor } from '@/utils/formatters';

export function TurnTracker() {
  const gameState = useGameStore((s) => s.gameState);
  if (!gameState) return null;

  const activePlayer = gameState.players[gameState.active_player];
  const decisionOwner = gameState.players[gameState.decision_owner];

  return (
    <Panel title="Turn Info" compact>
      <div className="space-y-1.5 text-sm">
        <div className="flex justify-between">
          <span className="text-gray-400">Battle Round</span>
          <span className="text-gray-200 font-semibold">{gameState.battle_round} / 5</span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Active Player</span>
          <span className={factionColor(activePlayer.faction_id)}>
            {activePlayer.name || factionName(activePlayer.faction_id)}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-400">Deciding</span>
          <span className={factionColor(decisionOwner.faction_id)}>
            {decisionOwner.name || factionName(decisionOwner.faction_id)}
          </span>
        </div>
      </div>
    </Panel>
  );
}
