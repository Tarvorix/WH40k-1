import { useGameStore } from '@/store/gameStore';

export function Header() {
  const gameState = useGameStore((s) => s.gameState);
  const screen = useGameStore((s) => s.screen);

  if (!gameState || screen === 'setup') return null;

  return (
    <header className="h-10 bg-surface-light border-b border-gray-700 flex items-center px-4 gap-4 shrink-0">
      <span className="font-heading text-accent text-sm font-bold tracking-wide">
        WH40K DIGITAL
      </span>
      <div className="flex-1" />
      <span className="text-xs text-gray-400">
        Round {gameState.battle_round}/5
      </span>
      <span className="text-xs text-gray-400">|</span>
      <span className="text-xs text-gray-300">
        {gameState.phase}
      </span>
    </header>
  );
}
