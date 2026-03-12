import { useEffect, useRef } from 'react';
import { useGameStore } from '@/store/gameStore';
import { Panel } from '@/components/shared/Panel';
import { phaseColor } from '@/utils/formatters';
import type { EventView } from '@/types/game';

export function CombatLog() {
  const gameState = useGameStore((s) => s.gameState);
  const scrollRef = useRef<HTMLDivElement>(null);

  const events = gameState?.events ?? [];

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [events.length]);

  return (
    <Panel title="Combat Log" className="flex-1 flex flex-col min-h-0">
      <div ref={scrollRef} className="flex-1 overflow-y-auto space-y-0.5 max-h-64">
        {events.length === 0 ? (
          <p className="text-gray-500 text-xs">No events yet</p>
        ) : (
          events.map((event, i) => (
            <EventEntry key={i} event={event} />
          ))
        )}
      </div>
    </Panel>
  );
}

function EventEntry({ event }: { event: EventView }) {
  return (
    <div className="text-xs py-0.5 flex gap-1.5 items-start">
      <span className={`shrink-0 font-mono ${phaseColor(event.phase)}`}>
        R{event.round}
      </span>
      <span className="text-gray-300">{event.description}</span>
    </div>
  );
}
