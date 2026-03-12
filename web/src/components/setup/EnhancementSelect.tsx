import { useSetupStore, FACTIONS, ENHANCEMENTS } from '@/store/setupStore';
import { clsx } from 'clsx';

export function EnhancementSelect() {
  const playerFaction = useSetupStore((s) => s.playerFaction);
  const selectEnhancement = useSetupStore((s) => s.selectEnhancement);
  const setStep = useSetupStore((s) => s.setStep);

  const enhancements =
    playerFaction === FACTIONS.CUSTODES
      ? ENHANCEMENTS.CUSTODES
      : ENHANCEMENTS.WORLD_EATERS;

  const factionColor =
    playerFaction === FACTIONS.CUSTODES ? 'custodes-gold' : 'worldeaters-red';

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Enhancement
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Choose an enhancement for your leader
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {enhancements.map((enh) => (
          <button
            key={enh.id}
            onClick={() => selectEnhancement(enh.id)}
            className={clsx('card-hover text-left p-5', `hover:border-${factionColor}`)}
          >
            <h3 className={clsx('font-semibold text-lg mb-2', `text-${factionColor}`)}>
              {enh.name}
            </h3>
            <p className="text-gray-300 text-sm">{enh.description}</p>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setStep('faction_select')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Faction Select
        </button>
      </div>
    </div>
  );
}
