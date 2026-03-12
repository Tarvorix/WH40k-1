import { useSetupStore, MISSIONS } from '@/store/setupStore';

export function MissionSelect() {
  const selectMission = useSetupStore((s) => s.selectMission);
  const setStep = useSetupStore((s) => s.setStep);

  return (
    <div>
      <h2 className="font-heading text-2xl text-gray-200 text-center mb-2">
        Select Mission
      </h2>
      <p className="text-gray-400 text-center text-sm mb-6">
        Choose the mission for this battle
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
        {MISSIONS.map((mission) => (
          <button
            key={mission.id}
            onClick={() => selectMission(mission.id)}
            className="card-hover text-left p-4"
          >
            <h3 className="font-semibold text-accent mb-2">{mission.name}</h3>
            <p className="text-gray-300 text-xs">{mission.description}</p>
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
