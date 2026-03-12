import { useReplayStore } from '@/store/replayStore';
import { Button } from '@/components/shared/Button';

const SPEED_OPTIONS = [0.5, 1, 2, 5];

export function ReplayControls() {
  const isPlaying = useReplayStore((s) => s.isPlaying);
  const playbackSpeed = useReplayStore((s) => s.playbackSpeed);
  const replayInfo = useReplayStore((s) => s.replayInfo);
  const stepForward = useReplayStore((s) => s.stepForward);
  const stepBackward = useReplayStore((s) => s.stepBackward);
  const goToStart = useReplayStore((s) => s.goToStart);
  const goToEnd = useReplayStore((s) => s.goToEnd);
  const togglePlay = useReplayStore((s) => s.togglePlay);
  const setPlaybackSpeed = useReplayStore((s) => s.setPlaybackSpeed);

  if (!replayInfo) return null;

  return (
    <div className="h-10 bg-surface-light border-t border-gray-700 flex items-center px-4 gap-3 shrink-0">
      {/* Transport buttons */}
      <div className="flex gap-1">
        <Button variant="secondary" size="sm" onClick={goToStart} title="Go to start">
          |&lt;&lt;
        </Button>
        <Button variant="secondary" size="sm" onClick={() => stepBackward(1)} title="Step back">
          &lt;&lt;
        </Button>
        <Button
          variant={isPlaying ? 'danger' : 'primary'}
          size="sm"
          onClick={togglePlay}
          title={isPlaying ? 'Pause' : 'Play'}
        >
          {isPlaying ? '||' : '|>'}
        </Button>
        <Button variant="secondary" size="sm" onClick={() => stepForward(1)} title="Step forward">
          &gt;&gt;
        </Button>
        <Button variant="secondary" size="sm" onClick={goToEnd} title="Go to end">
          &gt;&gt;|
        </Button>
      </div>

      {/* Frame counter */}
      <span className="text-xs text-gray-400 font-mono">
        {replayInfo.current_frame}/{replayInfo.total_frames}
      </span>

      <div className="flex-1" />

      {/* Speed selector */}
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-500">Speed:</span>
        {SPEED_OPTIONS.map((speed) => (
          <button
            key={speed}
            onClick={() => setPlaybackSpeed(speed)}
            className={`px-2 py-0.5 rounded text-xs transition-colors ${
              playbackSpeed === speed
                ? 'bg-accent text-black font-semibold'
                : 'bg-surface text-gray-400 hover:text-gray-200'
            }`}
          >
            {speed}x
          </button>
        ))}
      </div>
    </div>
  );
}
