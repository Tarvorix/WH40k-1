import { useGameStore } from '@/store/gameStore';

interface HeaderProps {
  compact?: boolean;
}

export function Header({ compact = false }: HeaderProps) {
  const gameState = useGameStore((s) => s.gameState);
  const screen = useGameStore((s) => s.screen);

  if (!gameState || screen === 'setup') return null;

  if (compact) {
    return (
      <header className="h-8 bg-surface-light border-b border-gray-700 flex items-center px-3 gap-2 shrink-0">
        <span className="text-[10px] text-gray-400">
          R{gameState.battle_round}/5
        </span>
        <span className="text-[10px] text-gray-400">|</span>
        <span className="text-[10px] text-gray-200 font-medium">
          {gameState.phase}
        </span>
        <div className="flex-1" />
        <span className="text-[10px] text-gray-400">
          P{gameState.active_player + 1}
        </span>
      </header>
    );
  }

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
