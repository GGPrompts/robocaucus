import { X, MessageSquare, FileCode, GitCommit } from 'lucide-react';
import type { EditorTab } from '../types';

interface TabBarProps {
  tabs: EditorTab[];
  activeTabId: string | null;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string) => void;
}

export function TabBar({ tabs, activeTabId, onSelectTab, onCloseTab }: TabBarProps) {
  if (tabs.length === 0) return null;

  return (
    <div className="flex shrink-0 items-center overflow-x-auto border-b border-[var(--border-primary)] bg-[var(--bg-secondary)]">
      {tabs.map((tab) => {
        const isActive = tab.id === activeTabId;
        return (
          <div
            key={tab.id}
            className={`group flex items-center gap-1.5 border-r border-[var(--border-primary)] px-3 py-1.5 text-xs font-medium cursor-pointer select-none transition-colors ${
              isActive
                ? 'bg-[var(--bg-primary)] text-[var(--text-primary)] border-b-2 border-b-[var(--accent)]'
                : 'text-[var(--text-muted)] hover:bg-[var(--bg-primary)]/50 hover:text-[var(--text-secondary)]'
            }`}
            onClick={() => onSelectTab(tab.id)}
          >
            {tab.type === 'chat' ? (
              <MessageSquare size={12} className="shrink-0" />
            ) : tab.type === 'commit' ? (
              <GitCommit size={12} className="shrink-0" />
            ) : (
              <FileCode size={12} className="shrink-0" />
            )}
            <span className="max-w-[140px] truncate">{tab.title}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onCloseTab(tab.id);
              }}
              className="ml-1 shrink-0 rounded p-0.5 opacity-0 group-hover:opacity-100 hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)] transition-opacity"
            >
              <X size={12} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
