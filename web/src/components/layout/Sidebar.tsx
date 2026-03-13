import type { ReactNode } from 'react';

interface SidebarProps {
  children: ReactNode;
  side?: 'left' | 'right';
  width?: string;
  className?: string;
}

export function Sidebar({ children, side = 'right', width = 'w-80', className = '' }: SidebarProps) {
  const borderClass = side === 'left' ? 'border-r' : 'border-l';

  return (
    <aside
      className={`${width} shrink-0 bg-surface-light ${borderClass} border-gray-700 overflow-y-auto hidden lg:flex flex-col ${className}`}
    >
      {children}
    </aside>
  );
}
