import { useGameStore } from '@/store/gameStore';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { BattlefieldCanvas } from '@/renderer/BattlefieldCanvas';
import { PhaseIndicator } from './PhaseIndicator';
import { TurnTracker } from './TurnTracker';
import { UnitInfoPanel } from './UnitInfoPanel';
import { CombatLog } from './CombatLog';
import { StratagemPanel } from './StratagemPanel';
import { ActionPanel } from './ActionPanel';
import { ScoreBoard } from './ScoreBoard';
import { BlessingPanel } from './BlessingPanel';
import { AiControls } from '@/components/ai/AiControls';
import { AiEvalBar } from '@/components/ai/AiEvalBar';
import { DeploymentPanel } from '@/components/setup/DeploymentPanel';

export function GameScreen() {
  const gameState = useGameStore((s) => s.gameState);

  if (!gameState) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-gray-400">Loading game state...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen">
      <Header />
      <PhaseIndicator />
      <div className="flex flex-1 overflow-hidden">
        {/* Left sidebar: unit info + actions */}
        <Sidebar side="left" width="w-72">
          <TurnTracker />
          <ScoreBoard />
          <ActionPanel />
          <DeploymentPanel />
          <BlessingPanel />
          <StratagemPanel />
          <AiControls />
        </Sidebar>

        {/* Center: battlefield canvas */}
        <div className="flex-1 flex flex-col overflow-hidden">
          <AiEvalBar />
          <BattlefieldCanvas />
        </div>

        {/* Right sidebar: selected unit info + combat log */}
        <Sidebar side="right" width="w-80">
          <UnitInfoPanel />
          <CombatLog />
        </Sidebar>
      </div>
    </div>
  );
}
