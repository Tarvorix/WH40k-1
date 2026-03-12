import { useSetupStore, FACTIONS, SECONDARIES } from '@/store/setupStore';
import { clsx } from 'clsx';

export function SecondarySelect() {
  const playerFaction = useSetupStore((s) => s.playerFaction);
  const selectSecondary = useSetupStore((s) => s.selectSecondary);
  const setStep = useSetupStore((s) => s.setStep);

  const secondaries =
    playerFaction === FACTIONS.CUSTODES
      ? SECONDARIES.CUSTODES
      : SECONDARIES.WORLD_EATERS;

  const factionColor =
    playerFaction === FACTIONS.CUSTODES ? 'custodes-gold' : 'worldeaters-red';

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Secondary Objective
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Choose a secondary objective for additional VP
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {secondaries.map((sec) => (
          <button
            key={sec.id}
            onClick={() => selectSecondary(sec.id)}
            className={clsx('card-hover text-left p-5', `hover:border-${factionColor}`)}
          >
            <h3 className={clsx('font-semibold text-lg mb-2', `text-${factionColor}`)}>
              {sec.name}
            </h3>
            <p className="text-gray-300 text-sm">{sec.description}</p>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setStep('enhancement_select')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Enhancement Select
        </button>
      </div>
    </div>
  );
}
