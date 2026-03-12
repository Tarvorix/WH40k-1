import { useReplayStore } from '@/store/replayStore';
import { useGameStore } from '@/store/gameStore';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { BattlefieldCanvas } from '@/renderer/BattlefieldCanvas';
import { ReplayControls } from './ReplayControls';
import { ReplayTimeline } from './ReplayTimeline';
import { UnitInfoPanel } from '@/components/play/UnitInfoPanel';
import { CombatLog } from '@/components/play/CombatLog';
import { ScoreBoard } from '@/components/play/ScoreBoard';
import { Panel } from '@/components/shared/Panel';
import { Button } from '@/components/shared/Button';

export function ReplayScreen() {
  const replayInfo = useReplayStore((s) => s.replayInfo);
  const replayLoaded = useReplayStore((s) => s.replayLoaded);
  const unload = useReplayStore((s) => s.unload);
  const setScreen = useGameStore((s) => s.setScreen);

  const handleBack = () => {
    unload();
    setScreen('setup');
  };

  if (!replayLoaded || !replayInfo) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center">
        <h2 className="font-heading text-2xl text-gray-200 mb-4">Load Replay</h2>
        <ReplayFileLoader />
        <Button variant="secondary" className="mt-4" onClick={handleBack}>
          Back to Setup
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen">
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar side="left" width="w-72">
          <Panel title="Replay Info" compact>
            <div className="space-y-1 text-xs">
              <div className="flex justify-between text-gray-400">
                <span>Frame</span>
                <span className="text-gray-200">
                  {replayInfo.current_frame} / {replayInfo.total_frames}
                </span>
              </div>
              <div className="flex justify-between text-gray-400">
                <span>Outcome</span>
                <span className="text-gray-200">{replayInfo.outcome.status}</span>
              </div>
            </div>
          </Panel>
          <ScoreBoard />
          <CombatLog />
        </Sidebar>

        <div className="flex-1 flex flex-col overflow-hidden">
          <BattlefieldCanvas />
          <ReplayTimeline />
          <ReplayControls />
        </div>

        <Sidebar side="right" width="w-80">
          <UnitInfoPanel />
        </Sidebar>
      </div>
    </div>
  );
}

function ReplayFileLoader() {
  const loadReplay = useReplayStore((s) => s.loadReplay);

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const text = await file.text();
    await loadReplay(text);
  };

  return (
    <div>
      <label className="btn-secondary cursor-pointer">
        Choose Replay File
        <input
          type="file"
          accept=".json"
          onChange={handleFileChange}
          className="hidden"
        />
      </label>
    </div>
  );
}
