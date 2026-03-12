import { clsx } from 'clsx';
import type { ReactNode } from 'react';

interface PanelProps {
  title?: string;
  children: ReactNode;
  className?: string;
  compact?: boolean;
}

export function Panel({ title, children, className, compact }: PanelProps) {
  return (
    <div className={clsx('border-b border-gray-700', compact ? 'p-2' : 'p-3', className)}>
      {title && (
        <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-400 mb-2">
          {title}
        </h3>
      )}
      {children}
    </div>
  );
}
