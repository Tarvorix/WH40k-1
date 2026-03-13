import { useRef, useCallback, type ReactNode } from 'react';

interface SlideUpPanelProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  maxHeight?: string;
}

const DISMISS_THRESHOLD = 100; // px dragged downward to dismiss

export function SlideUpPanel({
  isOpen,
  onClose,
  title,
  children,
  maxHeight = '60vh',
}: SlideUpPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const dragStartY = useRef<number | null>(null);
  const dragDelta = useRef(0);

  const onTouchStart = useCallback((e: React.TouchEvent) => {
    dragStartY.current = e.touches[0].clientY;
    dragDelta.current = 0;
  }, []);

  const onTouchMove = useCallback((e: React.TouchEvent) => {
    if (dragStartY.current === null) return;
    const delta = e.touches[0].clientY - dragStartY.current;
    dragDelta.current = delta;

    // Apply visual drag feedback (only allow downward drag)
    if (panelRef.current && delta > 0) {
      panelRef.current.style.transform = `translateY(${delta}px)`;
    }
  }, []);

  const onTouchEnd = useCallback(() => {
    if (dragDelta.current > DISMISS_THRESHOLD) {
      onClose();
    }
    // Reset position
    if (panelRef.current) {
      panelRef.current.style.transform = '';
    }
    dragStartY.current = null;
    dragDelta.current = 0;
  }, [onClose]);

  return (
    <>
      {/* Backdrop */}
      {isOpen && (
        <div
          className="fixed inset-0 bg-black/40 z-30"
          style={{ bottom: '3.5rem' }}
          onClick={onClose}
        />
      )}

      {/* Panel */}
      <div
        ref={panelRef}
        className="fixed left-0 right-0 bg-surface-light border-t border-gray-700 rounded-t-xl z-30 overflow-hidden flex flex-col"
        style={{
          bottom: '3.5rem', // above BottomTabBar (h-14 = 3.5rem)
          maxHeight,
          transform: isOpen ? 'translateY(0)' : 'translateY(100%)',
          transition: dragStartY.current !== null ? 'none' : 'transform 300ms ease-out',
          paddingBottom: 'env(safe-area-inset-bottom)',
        }}
        onTouchStart={onTouchStart}
        onTouchMove={onTouchMove}
        onTouchEnd={onTouchEnd}
      >
        {/* Handle bar */}
        <div className="flex items-center justify-center py-2 cursor-grab">
          <div className="w-10 h-1 rounded-full bg-gray-600" />
        </div>

        {/* Title */}
        <div className="px-3 pb-2 flex items-center justify-between">
          <h3 className="text-xs font-semibold text-gray-200 uppercase tracking-wider">
            {title}
          </h3>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-200 text-sm px-2"
          >
            Close
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-3 pb-3">
          {children}
        </div>
      </div>
    </>
  );
}
