import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { Button } from '@/components/shared/Button';
import type { AiDifficulty } from '@/types/game';

const DIFFICULTY_GROUPS: { group: string; options: { value: AiDifficulty; label: string; description: string }[] }[] = [
  {
    group: 'Basic',
    options: [
      { value: 'Basic_Recruit', label: 'Recruit', description: 'Picks the best immediate move' },
      { value: 'Basic_Battle_Ready', label: 'Battle Ready', description: 'Looks one move ahead' },
      { value: 'Basic_Veteran', label: 'Veteran', description: 'Tactical search (depth 2)' },
      { value: 'Basic_Elite', label: 'Elite', description: 'Deep tactical search (depth 3)' },
    ],
  },
  {
    group: 'Perturabo',
    options: [
      { value: 'Perturabo_Shallow', label: 'Shallow', description: 'Iterative deepening (depth 4)' },
      { value: 'Perturabo_Regular', label: 'Regular', description: 'Iterative deepening with quiescence (depth 6)' },
      { value: 'Perturabo_Deep', label: 'Deep', description: 'Full search with extensions (depth 8)' },
    ],
  },
  {
    group: 'Alpharius',
    options: [
      { value: 'Alpharius_Shallow', label: 'Shallow', description: 'MCTS search (100 simulations)' },
      { value: 'Alpharius_Regular', label: 'Regular', description: 'MCTS search (800 simulations)' },
      { value: 'Alpharius_Deep', label: 'Deep', description: 'Competition MCTS (1600 simulations)' },
    ],
  },
];

const ALL_OPTIONS = DIFFICULTY_GROUPS.flatMap((g) => g.options);

export function AiControls() {
  const aiDifficulty = useGameStore((s) => s.aiDifficulty);
  const aiThinking = useGameStore((s) => s.aiThinking);
  const aiResult = useGameStore((s) => s.aiResult);
  const autoPlayAi = useGameStore((s) => s.autoPlayAi);
  const playerControlled = useGameStore((s) => s.playerControlled);
  const gameState = useGameStore((s) => s.gameState);
  const setAiDifficulty = useGameStore((s) => s.setAiDifficulty);
  const setAutoPlayAi = useGameStore((s) => s.setAutoPlayAi);
  const setPlayerControlled = useGameStore((s) => s.setPlayerControlled);
  const runAi = useGameStore((s) => s.runAi);
  const applyAiAction = useGameStore((s) => s.applyAiAction);

  return (
    <Panel title="AI Controls">
      {/* Player assignment */}
      <div className="mb-3">
        <h4 className="text-[10px] uppercase tracking-wider text-gray-500 mb-1">Player Control</h4>
        <div className="flex gap-2">
          {[0, 1].map((idx) => (
            <button
              key={idx}
              onClick={() => setPlayerControlled(idx, !playerControlled[idx])}
              className={`flex-1 px-2 py-1 rounded text-xs transition-colors ${
                playerControlled[idx]
                  ? 'bg-green-900/30 text-green-400 border border-green-700'
                  : 'bg-red-900/30 text-red-400 border border-red-700'
              }`}
            >
              P{idx}: {playerControlled[idx] ? 'Human' : 'AI'}
            </button>
          ))}
        </div>
      </div>

      {/* Difficulty selector */}
      <div className="mb-3">
        <h4 className="text-[10px] uppercase tracking-wider text-gray-500 mb-1">AI Difficulty</h4>
        <select
          value={aiDifficulty}
          onChange={(e) => setAiDifficulty(e.target.value as AiDifficulty)}
          className="w-full bg-surface border border-gray-600 rounded px-2 py-1 text-sm text-gray-200"
        >
          {DIFFICULTY_GROUPS.map((group) => (
            <optgroup key={group.group} label={group.group}>
              {group.options.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
        <p className="text-[10px] text-gray-500 mt-1">
          {ALL_OPTIONS.find((o) => o.value === aiDifficulty)?.description}
        </p>
      </div>

      {/* Auto-play toggle */}
      <div className="mb-3">
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={autoPlayAi}
            onChange={(e) => setAutoPlayAi(e.target.checked)}
            className="rounded bg-surface border-gray-600"
          />
          <span className="text-xs text-gray-300">Auto-play AI turns</span>
        </label>
      </div>

      {/* Manual AI buttons */}
      <div className="flex gap-2">
        <Button
          variant="secondary"
          size="sm"
          onClick={runAi}
          disabled={aiThinking || !gameState?.in_progress}
        >
          {aiThinking ? 'Thinking...' : 'Run AI'}
        </Button>
        {aiResult && (
          <Button
            variant="primary"
            size="sm"
            onClick={applyAiAction}
          >
            Apply
          </Button>
        )}
      </div>

      {/* AI result display */}
      {aiResult && (
        <div className="mt-3 text-xs bg-surface rounded p-2">
          <div className="text-gray-400 mb-1">
            Best: <span className="text-gray-200">{aiResult.best_action_label}</span>
          </div>
          <div className="flex gap-3 text-gray-500">
            <span>Score: <span className="text-gray-300">{aiResult.score}</span></span>
            <span>Nodes: <span className="text-gray-300">{aiResult.nodes_evaluated}</span></span>
            <span>Depth: <span className="text-gray-300">{aiResult.max_depth}</span></span>
          </div>
          {aiResult.candidates.length > 1 && (
            <div className="mt-2 space-y-0.5">
              <span className="text-gray-500">Top candidates:</span>
              {aiResult.candidates.slice(0, 5).map((c, i) => (
                <div key={i} className="flex justify-between text-gray-400">
                  <span className="truncate">{c.label}</span>
                  <span className="ml-2 shrink-0">{c.score}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Panel>
  );
}
