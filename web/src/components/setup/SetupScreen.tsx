import { useSetupStore } from '@/store/setupStore';
import { useGameStore } from '@/store/gameStore';
import { FactionSelect } from './FactionSelect';
import { EnhancementSelect } from './EnhancementSelect';
import { SecondarySelect } from './SecondarySelect';
import { MissionSelect } from './MissionSelect';
import { Button } from '@/components/shared/Button';

export function SetupScreen() {
  const step = useSetupStore((s) => s.step);
  const setup = useSetupStore();
  const createMatch = useGameStore((s) => s.createMatch);
  const engineReady = useGameStore((s) => s.engineReady);
  const loading = useGameStore((s) => s.loading);

  const handleStartGame = async () => {
    if (!setup.isReady()) return;
    await createMatch(
      setup.playerFaction!,
      setup.opponentFaction!,
      setup.missionId!,
    );
  };

  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8 overflow-y-auto">
      <div className="max-w-3xl w-full">
        <h1 className="font-heading text-4xl text-accent font-bold text-center mb-2">
          WH40K DIGITAL
        </h1>
        <p className="text-gray-400 text-center mb-8 text-sm">
          Combat Patrol - Battle Setup
        </p>

        {/* Progress steps */}
        <div className="flex items-center justify-center gap-2 mb-8">
          {(['faction_select', 'enhancement_select', 'secondary_select', 'mission_select'] as const).map(
            (s, i) => (
              <div
                key={s}
                className={`h-1.5 w-16 rounded-full transition-colors ${
                  stepIndex(step) >= i ? 'bg-accent' : 'bg-surface-lighter'
                }`}
              />
            ),
          )}
        </div>

        {step === 'faction_select' && <FactionSelect />}
        {step === 'enhancement_select' && <EnhancementSelect />}
        {step === 'secondary_select' && <SecondarySelect />}
        {step === 'mission_select' && <MissionSelect />}
        {step === 'ready' && (
          <div className="text-center">
            <h2 className="font-heading text-2xl text-gray-200 mb-6">Ready to Battle</h2>
            <div className="card mb-6 text-left">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-400">Your Faction:</span>
                  <span className="ml-2 text-gray-100">
                    {setup.playerFaction === 0 ? 'Adeptus Custodes' : 'World Eaters'}
                  </span>
                </div>
                <div>
                  <span className="text-gray-400">Opponent:</span>
                  <span className="ml-2 text-gray-100">
                    {setup.opponentFaction === 0 ? 'Adeptus Custodes' : 'World Eaters'}
                  </span>
                </div>
              </div>
            </div>
            <div className="flex gap-4 justify-center">
              <Button variant="secondary" onClick={() => setup.reset()}>
                Back to Setup
              </Button>
              <Button
                variant="primary"
                size="lg"
                onClick={handleStartGame}
                disabled={!engineReady || loading}
              >
                {loading ? 'Creating Match...' : 'Start Battle'}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function stepIndex(step: string): number {
  const steps = ['faction_select', 'enhancement_select', 'secondary_select', 'mission_select', 'ready'];
  return steps.indexOf(step);
}
