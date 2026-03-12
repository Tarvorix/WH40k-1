import type { ReactNode } from 'react';

interface SidebarProps {
  children: ReactNode;
  side?: 'left' | 'right';
  width?: string;
}

export function Sidebar({ children, side = 'right', width = 'w-80' }: SidebarProps) {
  return (
    <aside
      className={`${width} shrink-0 bg-surface-light border-${side === 'left' ? 'r' : 'l'} border-gray-700 overflow-y-auto flex flex-col`}
    >
      {children}
    </aside>
  );
}
