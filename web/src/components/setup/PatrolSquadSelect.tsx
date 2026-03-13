import { useSetupStore, FACTIONS, PATROL_SQUADS } from '@/store/setupStore';
import { clsx } from 'clsx';

export function PatrolSquadSelect() {
  const playerFaction = useSetupStore((s) => s.playerFaction);
  const selectPatrolSquad = useSetupStore((s) => s.selectPatrolSquad);
  const setStep = useSetupStore((s) => s.setStep);

  const squads =
    playerFaction === FACTIONS.CUSTODES
      ? PATROL_SQUADS.CUSTODES
      : PATROL_SQUADS.WORLD_EATERS;

  const factionColor =
    playerFaction === FACTIONS.CUSTODES ? 'custodes-gold' : 'worldeaters-red';

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Patrol Squad
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Choose one unit to join your Combat Patrol
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {squads.map((squad) => (
          <button
            key={squad.id}
            onClick={() => selectPatrolSquad(squad.id)}
            className={clsx('card-hover text-left p-5', `hover:border-${factionColor}`)}
          >
            <h3 className={clsx('font-semibold text-lg mb-1', `text-${factionColor}`)}>
              {squad.name}
            </h3>
            <p className="text-gray-400 text-xs mb-2">{squad.models} models</p>
            <p className="text-gray-300 text-sm">{squad.description}</p>
          </button>
        ))}
      </div>
      <div className="text-center">
        <button
          onClick={() => setStep('secondary_select')}
          className="text-gray-400 hover:text-gray-200 text-sm"
        >
          Back to Secondary Select
        </button>
      </div>
    </div>
  );
}
