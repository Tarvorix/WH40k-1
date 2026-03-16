import { useEffect } from 'react';
import { useGameStore } from '@/store/gameStore';
import { MainMenu } from '@/components/menu/MainMenu';
import { SetupScreen } from '@/components/setup/SetupScreen';
import { BoardingSetupScreen } from '@/components/boarding-setup/BoardingSetupScreen';
import { GameScreen } from '@/components/play/GameScreen';
import { ReplayScreen } from '@/components/replay/ReplayScreen';
import { GameEndScreen } from '@/components/game-end/GameEndScreen';
import { AppShell } from '@/components/layout/AppShell';

export default function App() {
  const screen = useGameStore((s) => s.screen);
  const initEngine = useGameStore((s) => s.initEngine);
  const engineReady = useGameStore((s) => s.engineReady);
  const loading = useGameStore((s) => s.loading);
  const error = useGameStore((s) => s.error);
  const clearError = useGameStore((s) => s.clearError);

  useEffect(() => {
    initEngine();
  }, [initEngine]);

  if (!engineReady && loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-surface">
        <div className="text-center">
          <div className="animate-pulse text-accent text-2xl font-heading mb-4">
            Initializing Engine...
          </div>
          <div className="text-gray-400 text-sm">Loading WASM module</div>
        </div>
      </div>
    );
  }

  return (
    <AppShell>
      {error && (
        <div className="fixed top-4 right-4 z-50 bg-red-900/90 border border-red-700 text-red-100 px-4 py-3 rounded-lg shadow-lg max-w-md">
          <div className="flex items-start gap-2">
            <span className="text-red-400 font-bold">Error:</span>
            <span className="text-sm">{error}</span>
            <button
              onClick={clearError}
              className="ml-auto text-red-400 hover:text-red-200 font-bold"
            >
              x
            </button>
          </div>
        </div>
      )}
      {screen === 'menu' && <MainMenu />}
      {screen === 'setup' && <SetupScreen />}
      {screen === 'boarding_setup' && <BoardingSetupScreen />}
      {screen === 'play' && <GameScreen />}
      {screen === 'replay' && <ReplayScreen />}
      {screen === 'game_end' && <GameEndScreen />}
    </AppShell>
  );
}
