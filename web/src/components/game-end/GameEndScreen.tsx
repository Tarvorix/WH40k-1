import { useGameStore } from '@/store/gameStore';
import { useSetupStore } from '@/store/setupStore';
import { useReplayStore } from '@/store/replayStore';
import { Button } from '@/components/shared/Button';
import { factionName, factionColor, formatVP } from '@/utils/formatters';
import { engineClient } from '@/engine/workerClient';

const ENHANCEMENT_NAMES: Record<number, string> = {
  1: 'Fearsome Presence',
  2: 'Bane of the Craven',
  3: 'Watchman of Terra',
  4: 'Warrior Exemplar',
};

const SECONDARY_NAMES: Record<number, string> = {
  1: 'Champions of Khorne',
  2: 'Skull Takers',
  3: 'Raise the Vexillas',
  4: 'Consecrated Ground',
};

const PATROL_SQUAD_NAMES: Record<number, string> = {
  0: 'Custodian Wardens',
  1: 'Allarus Custodians',
};

export function GameEndScreen() {
  const gameState = useGameStore((s) => s.gameState);
  const setScreen = useGameStore((s) => s.setScreen);
  const resetSetup = useSetupStore((s) => s.reset);
  const loadReplay = useReplayStore((s) => s.loadReplay);

  if (!gameState) return null;

  const outcome = gameState.outcome;
  const isVictory = outcome.status === 'victory';
  const isDraw = outcome.status === 'draw';

  const winner = isVictory && outcome.winner != null
    ? gameState.players[outcome.winner]
    : null;

  const handlePlayAgain = () => {
    resetSetup();
    setScreen('setup');
  };

  const handleExportReplay = async () => {
    try {
      const replayJson = await engineClient.exportReplay();
      // Download as file
      const blob = new Blob([JSON.stringify(replayJson, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `wh40k-replay-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('Failed to export replay:', error);
    }
  };

  const handleViewReplay = async () => {
    try {
      const replayJson = await engineClient.exportReplay();
      await loadReplay(typeof replayJson === 'string' ? replayJson : JSON.stringify(replayJson));
      setScreen('replay');
    } catch (error) {
      console.error('Failed to load replay:', error);
    }
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8">
      <div className="max-w-lg w-full text-center">
        {/* Victory/Draw announcement */}
        {isVictory && winner ? (
          <>
            <h1 className="font-heading text-4xl text-accent font-bold mb-2">VICTORY</h1>
            <p className={`text-xl font-heading ${factionColor(winner.faction_id)} mb-6`}>
              {factionName(winner.faction_id)} Wins!
            </p>
          </>
        ) : isDraw ? (
          <>
            <h1 className="font-heading text-4xl text-gray-400 font-bold mb-2">DRAW</h1>
            <p className="text-gray-400 mb-6">The battle ends in a stalemate</p>
          </>
        ) : (
          <>
            <h1 className="font-heading text-4xl text-gray-400 font-bold mb-2">GAME OVER</h1>
            <p className="text-gray-400 mb-6">The battle has concluded</p>
          </>
        )}

        {/* Final VP breakdown */}
        <div className="card mb-8">
          <h2 className="text-sm font-semibold text-gray-400 uppercase mb-4">Final Score</h2>
          <div className="space-y-3">
            {gameState.players.map((player, idx) => (
              <div
                key={idx}
                className="flex items-center justify-between"
              >
                <div className="flex items-center gap-3">
                  <div
                    className={`w-3 h-3 rounded-full ${
                      idx === 0 ? 'bg-custodes-gold' : 'bg-worldeaters-red'
                    }`}
                  />
                  <span className={factionColor(player.faction_id)}>
                    {factionName(player.faction_id)}
                  </span>
                </div>
                <div className="flex gap-4 text-sm">
                  <div>
                    <span className="text-gray-500">Primary: </span>
                    <span className="text-gray-200">{player.primary_vp}</span>
                  </div>
                  <div>
                    <span className="text-gray-500">Secondary: </span>
                    <span className="text-gray-200">{player.secondary_vp}</span>
                  </div>
                  <div className="font-bold">
                    <span className="text-yellow-400">{formatVP(player.vp)}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Battle Choices */}
        <div className="card mb-8">
          <h2 className="text-sm font-semibold text-gray-400 uppercase mb-4">Battle Choices</h2>
          <div className="space-y-3">
            {gameState.players.map((player, idx) => (
              <div key={idx} className="flex items-center justify-between text-sm">
                <span className={factionColor(player.faction_id)}>
                  {factionName(player.faction_id)}
                </span>
                <div className="flex gap-4">
                  <div>
                    <span className="text-gray-500">Enhancement: </span>
                    <span className="text-gray-200">
                      {player.enhancement_choice != null
                        ? ENHANCEMENT_NAMES[player.enhancement_choice] ?? `Enhancement #${player.enhancement_choice}`
                        : 'None'}
                    </span>
                  </div>
                  <div>
                    <span className="text-gray-500">Secondary: </span>
                    <span className="text-gray-200">
                      {player.secondary_choice != null
                        ? SECONDARY_NAMES[player.secondary_choice] ?? `Secondary #${player.secondary_choice}`
                        : 'None'}
                    </span>
                  </div>
                  {player.patrol_squad_choice != null && (
                  <div>
                    <span className="text-gray-500">Patrol Squad: </span>
                    <span className="text-gray-200">
                      {PATROL_SQUAD_NAMES[player.patrol_squad_choice] ?? `Squad #${player.patrol_squad_choice}`}
                    </span>
                  </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Action buttons */}
        <div className="flex gap-4 justify-center">
          <Button variant="secondary" onClick={handleExportReplay}>
            Export Replay
          </Button>
          <Button variant="secondary" onClick={handleViewReplay}>
            View Replay
          </Button>
          <Button variant="primary" size="lg" onClick={handlePlayAgain}>
            Play Again
          </Button>
        </div>
      </div>
    </div>
  );
}
