import { clsx } from 'clsx';

export type MobileTab = 'actions' | 'units' | 'log' | 'score';

interface BottomTabBarProps {
  activeTab: MobileTab;
  onTabChange: (tab: MobileTab) => void;
  panelOpen: boolean;
}

const tabs: { id: MobileTab; label: string; icon: string }[] = [
  { id: 'actions', label: 'Actions', icon: '⚔' },
  { id: 'units', label: 'Units', icon: '🛡' },
  { id: 'log', label: 'Log', icon: '📜' },
  { id: 'score', label: 'Score', icon: '🏆' },
];

export function BottomTabBar({ activeTab, onTabChange, panelOpen }: BottomTabBarProps) {
  return (
    <nav
      className="fixed bottom-0 left-0 right-0 h-14 bg-surface-light border-t border-gray-700 flex items-center justify-around z-40"
      style={{ paddingBottom: 'env(safe-area-inset-bottom)' }}
    >
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={clsx(
            'flex flex-col items-center justify-center h-full px-3 py-1 text-[10px] transition-colors min-w-0 flex-1',
            activeTab === tab.id && panelOpen
              ? 'text-accent border-t-2 border-accent'
              : 'text-gray-400 border-t-2 border-transparent',
          )}
        >
          <span className="text-base leading-none mb-0.5">{tab.icon}</span>
          <span className="truncate">{tab.label}</span>
        </button>
      ))}
    </nav>
  );
}
