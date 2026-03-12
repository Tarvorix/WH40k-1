import { useSetupStore, FACTIONS } from '@/store/setupStore';
import { clsx } from 'clsx';

const factions = [
  {
    id: FACTIONS.CUSTODES,
    name: 'Adeptus Custodes',
    subtitle: 'The Emperor\'s Golden Legion',
    description: 'Elite warriors of the Imperium. Few in number but incredibly powerful, each Custodian is a masterwork of martial perfection. Features Martial Ka\'tah stances and Vaultswords weapon profiles.',
    color: 'custodes',
    ability: 'Martial Ka\'tah: Choose combat stances each Fight phase for different bonuses.',
  },
  {
    id: FACTIONS.WORLD_EATERS,
    name: 'World Eaters',
    subtitle: 'Frenzied Reavers of Khorne',
    description: 'Blood-crazed berzerkers devoted to Khorne. Overwhelm enemies with sheer aggression and numbers. Features Blessings of Khorne dice allocation for powerful buffs.',
    color: 'worldeaters',
    ability: 'Blessings of Khorne: Roll 5D6 each Command phase and allocate to powerful blessings.',
  },
];

export function FactionSelect() {
  const selectFaction = useSetupStore((s) => s.selectFaction);

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-6">
        Choose Your Faction
      </h2>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {factions.map((faction) => (
          <button
            key={faction.id}
            onClick={() => selectFaction(faction.id)}
            className={clsx(
              'card-hover text-left p-5 group',
              faction.color === 'custodes'
                ? 'hover:border-custodes-gold'
                : 'hover:border-worldeaters-red',
            )}
          >
            <h3
              className={clsx(
                'font-heading text-xl font-bold mb-1',
                faction.color === 'custodes' ? 'text-custodes-gold' : 'text-worldeaters-red',
              )}
            >
              {faction.name}
            </h3>
            <p className="text-gray-400 text-xs mb-3">{faction.subtitle}</p>
            <p className="text-gray-300 text-sm mb-3">{faction.description}</p>
            <div className="text-xs bg-surface rounded px-3 py-2">
              <span className="text-gray-400 font-semibold">Faction Ability: </span>
              <span className="text-gray-300">{faction.ability}</span>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
