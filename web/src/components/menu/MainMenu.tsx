import { useGameStore } from '@/store/gameStore';

export function MainMenu() {
  const setScreen = useGameStore((s) => s.setScreen);

  return (
    <div className="h-full flex flex-col items-center justify-center gap-8">
      <h1 className="text-4xl font-heading text-accent tracking-wider">
        WARHAMMER 40,000
      </h1>
      <p className="text-gray-400 text-lg">Choose Your Battle</p>

      <div className="flex flex-col gap-4 w-80">
        <button
          onClick={() => setScreen('setup')}
          className="card-hover text-left py-4 px-6"
        >
          <span className="block font-heading text-xl text-gray-100 tracking-wide">
            COMBAT PATROL
          </span>
          <span className="block text-sm text-gray-400 font-body mt-1">
            Pre-built 25 PL forces on a 44&quot; x 30&quot; battlefield
          </span>
        </button>

        <button
          onClick={() => setScreen('boarding_setup')}
          className="card-hover text-left py-4 px-6"
        >
          <span className="block font-heading text-xl text-gray-100 tracking-wide">
            BOARDING ACTIONS
          </span>
          <span className="block text-sm text-gray-400 font-body mt-1">
            500-point ship interior combat with custom army building
          </span>
        </button>
      </div>
    </div>
  );
}
