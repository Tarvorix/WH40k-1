import { useReplayStore } from '@/store/replayStore';

export function ReplayTimeline() {
  const replayInfo = useReplayStore((s) => s.replayInfo);
  const stepForward = useReplayStore((s) => s.stepForward);
  const stepBackward = useReplayStore((s) => s.stepBackward);

  if (!replayInfo || replayInfo.total_frames === 0) return null;

  const progress = (replayInfo.current_frame / replayInfo.total_frames) * 100;

  const handleClick = async (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const percentage = clickX / rect.width;
    const targetFrame = Math.round(percentage * replayInfo.total_frames);
    const delta = targetFrame - replayInfo.current_frame;

    if (delta > 0) {
      await stepForward(delta);
    } else if (delta < 0) {
      await stepBackward(Math.abs(delta));
    }
  };

  return (
    <div
      className="h-3 bg-surface cursor-pointer border-t border-gray-800 relative"
      onClick={handleClick}
    >
      {/* Progress bar */}
      <div
        className="absolute top-0 bottom-0 left-0 bg-accent/40 transition-all duration-100"
        style={{ width: `${progress}%` }}
      />
      {/* Playhead */}
      <div
        className="absolute top-0 bottom-0 w-0.5 bg-accent transition-all duration-100"
        style={{ left: `${progress}%` }}
      />
    </div>
  );
}
