import { useGameStore } from '@/store/gameStore';
import { formatScore } from '@/utils/formatters';

export function AiEvalBar() {
  const aiResult = useGameStore((s) => s.aiResult);
  const aiThinking = useGameStore((s) => s.aiThinking);

  if (!aiResult && !aiThinking) return null;

  const score = aiResult?.score ?? 0;
  // Map score to 0-100 percentage. Score range is roughly -100000 to +100000
  // but typical game scores are -5000 to +5000
  const clampedScore = Math.max(-5000, Math.min(5000, score));
  const percentage = ((clampedScore + 5000) / 10000) * 100;

  return (
    <div className="h-5 bg-surface-light border-b border-gray-700 flex items-center px-2 shrink-0">
      {aiThinking ? (
        <div className="flex items-center gap-2 w-full">
          <span className="text-[10px] text-gray-400">AI thinking...</span>
          <div className="flex-1 h-2 bg-surface rounded overflow-hidden">
            <div className="h-full bg-accent/50 animate-pulse rounded" style={{ width: '100%' }} />
          </div>
        </div>
      ) : aiResult ? (
        <div className="flex items-center gap-2 w-full">
          <span className="text-[10px] text-gray-400 w-12">{formatScore(score)}</span>
          <div className="flex-1 h-2 bg-surface rounded overflow-hidden relative">
            {/* Center line */}
            <div className="absolute left-1/2 top-0 bottom-0 w-px bg-gray-600" />
            {/* Score bar */}
            <div
              className="absolute top-0 bottom-0 transition-all duration-300"
              style={{
                left: percentage < 50 ? `${percentage}%` : '50%',
                width: `${Math.abs(percentage - 50)}%`,
                backgroundColor: score >= 0 ? '#50C878' : '#DC143C',
                opacity: 0.7,
              }}
            />
          </div>
          <span className="text-[10px] text-gray-500 w-12 text-right">
            {aiResult.time_ms}ms
          </span>
        </div>
      ) : null}
    </div>
  );
}
