import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { factionName, factionColor, formatVP, formatCP } from '@/utils/formatters';

export function ScoreBoard() {
  const gameState = useGameStore((s) => s.gameState);
  if (!gameState) return null;

  return (
    <Panel title="Score" compact>
      <div className="space-y-2">
        {gameState.players.map((player, idx) => (
          <div
            key={idx}
            className={clsx(
              'flex items-center justify-between text-sm',
              gameState.active_player === idx ? 'opacity-100' : 'opacity-70',
            )}
          >
            <div className="flex items-center gap-2">
              <div
                className={clsx(
                  'w-2 h-2 rounded-full',
                  idx === 0 ? 'bg-custodes-gold' : 'bg-worldeaters-red',
                )}
              />
              <span className={factionColor(player.faction_id)}>
                {factionName(player.faction_id)}
              </span>
            </div>
            <div className="flex gap-3 text-xs">
              <span className="text-yellow-400">{formatVP(player.vp)}</span>
              <span className="text-blue-400">{formatCP(player.cp)}</span>
            </div>
          </div>
        ))}
      </div>
    </Panel>
  );
}

function clsx(...classes: (string | boolean | undefined | null)[]): string {
  return classes.filter(Boolean).join(' ');
}
