import { useState, useEffect } from 'react';
import { useGameStore } from '@/store/gameStore';
import { Header } from '@/components/layout/Header';
import { BattlefieldCanvas } from '@/renderer/BattlefieldCanvas';
import { BottomTabBar, type MobileTab } from './BottomTabBar';
import { SlideUpPanel } from './SlideUpPanel';
import { ActionPanel } from './ActionPanel';
import { UnitInfoPanel } from './UnitInfoPanel';
import { CombatLog } from './CombatLog';
import { ScoreBoard } from './ScoreBoard';
import { TurnTracker } from './TurnTracker';
import { BlessingPanel } from './BlessingPanel';
import { StratagemPanel } from './StratagemPanel';
import { DeploymentPanel } from '@/components/setup/DeploymentPanel';
import { AiControls } from '@/components/ai/AiControls';

const TAB_TITLES: Record<MobileTab, string> = {
  actions: 'Actions',
  units: 'Unit Info',
  log: 'Combat Log',
  score: 'Score',
};

export function MobileGameLayout() {
  const [activeTab, setActiveTab] = useState<MobileTab>('actions');
  const [panelOpen, setPanelOpen] = useState(false);
  const decisionSurface = useGameStore((s) => s.decisionSurface);

  // Auto-open the actions panel when a new decision surface arrives
  useEffect(() => {
    if (decisionSurface && decisionSurface.actions.length > 0) {
      setActiveTab('actions');
      setPanelOpen(true);
    }
  }, [decisionSurface]);

  const handleTabChange = (tab: MobileTab) => {
    if (tab === activeTab && panelOpen) {
      // Toggle panel closed if tapping the same tab
      setPanelOpen(false);
    } else {
      setActiveTab(tab);
      setPanelOpen(true);
    }
  };

  const handleClosePanel = () => {
    setPanelOpen(false);
  };

  return (
    <div className="flex flex-col h-screen">
      <Header compact />

      {/* Canvas fills remaining space above the tab bar */}
      <div
        className="flex-1 overflow-hidden"
        style={{ marginBottom: '3.5rem' }} // Space for BottomTabBar
      >
        <BattlefieldCanvas />
      </div>

      {/* Slide-up panel for tab content */}
      <SlideUpPanel
        isOpen={panelOpen}
        onClose={handleClosePanel}
        title={TAB_TITLES[activeTab]}
      >
        {activeTab === 'actions' && (
          <div className="space-y-2">
            <ActionPanel />
            <DeploymentPanel />
            <BlessingPanel />
            <StratagemPanel />
            <AiControls />
          </div>
        )}
        {activeTab === 'units' && (
          <UnitInfoPanel />
        )}
        {activeTab === 'log' && (
          <CombatLog />
        )}
        {activeTab === 'score' && (
          <div className="space-y-2">
            <TurnTracker />
            <ScoreBoard />
          </div>
        )}
      </SlideUpPanel>

      {/* Bottom tab bar */}
      <BottomTabBar
        activeTab={activeTab}
        onTabChange={handleTabChange}
        panelOpen={panelOpen}
      />
    </div>
  );
}
