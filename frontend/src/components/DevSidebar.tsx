import { useState, useCallback } from 'react';
import { X, FolderTree, GitBranch, Search } from 'lucide-react';
import { FileTree } from './FileTree';
import { GitGraph } from './git/GitGraph';
import { searchFiles, type SearchResult } from '../lib/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Tab = 'files' | 'git' | 'search';

interface DevSidebarProps {
  workspacePath: string;
  onClose: () => void;
  onFileSelect?: (filePath: string) => void;
}

// ---------------------------------------------------------------------------
// Search Tab
// ---------------------------------------------------------------------------

function SearchTab({
  workspacePath,
  onResultClick,
}: {
  workspacePath: string;
  onResultClick: (file: string, line: number) => void;
}) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const handleSearch = useCallback(async () => {
    const q = query.trim();
    if (!q) return;

    setSearching(true);
    setError(null);
    setSearched(true);

    try {
      const res = await searchFiles(workspacePath, q);
      setResults(res.results);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
      setResults([]);
    } finally {
      setSearching(false);
    }
  }, [workspacePath, query]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex gap-1 p-2 border-b border-[var(--border-primary)]">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleSearch();
          }}
          placeholder="Search files..."
          className="flex-1 rounded bg-[var(--bg-secondary)] px-2 py-1.5 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-secondary)] focus:border-[var(--accent-hover)] focus:outline-none"
        />
        <button
          onClick={handleSearch}
          disabled={searching || !query.trim()}
          className="rounded bg-[var(--accent)] px-3 py-1.5 text-sm text-[var(--text-primary)] hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {searching ? '...' : 'Go'}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {error && (
          <div className="p-3 text-xs text-red-400">{error}</div>
        )}

        {!error && searched && results.length === 0 && !searching && (
          <div className="p-3 text-sm text-[var(--text-muted)]">No results found.</div>
        )}

        {results.map((r, idx) => (
          <button
            key={`${r.file}:${r.line}:${idx}`}
            onClick={() => onResultClick(r.file, r.line)}
            className="w-full text-left px-3 py-2 text-sm border-b border-[var(--border-primary)]/50 hover:bg-[var(--bg-secondary)] transition-colors"
          >
            <div className="text-[var(--accent)] text-xs font-mono truncate">
              {r.file}:{r.line}
            </div>
            <div className="text-[var(--text-secondary)] text-xs font-mono truncate mt-0.5">
              {r.text}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// DevSidebar
// ---------------------------------------------------------------------------

export function DevSidebar({ workspacePath, onClose, onFileSelect }: DevSidebarProps) {
  const [activeTab, setActiveTab] = useState<Tab>('files');

  const handleFileSelect = useCallback((path: string) => {
    onFileSelect?.(path);
  }, [onFileSelect]);

  const handleSearchResultClick = useCallback((file: string, _line: number) => {
    onFileSelect?.(file);
  }, [onFileSelect]);

  if (!workspacePath) {
    return (
      <div className="flex w-96 flex-col border-l border-[var(--border-primary)] bg-[var(--bg-primary)]">
        <div className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--border-primary)] px-4">
          <span className="text-sm font-semibold text-[var(--text-primary)]">Developer</span>
          <button
            onClick={onClose}
            className="rounded p-1 text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
          >
            <X size={16} />
          </button>
        </div>
        <div className="flex flex-1 items-center justify-center text-sm text-[var(--text-muted)]">
          No workspace path set for this conversation.
        </div>
      </div>
    );
  }

  const tabs: { id: Tab; label: string; icon: React.ReactNode }[] = [
    { id: 'files', label: 'Files', icon: <FolderTree size={14} /> },
    { id: 'git', label: 'Git', icon: <GitBranch size={14} /> },
    { id: 'search', label: 'Search', icon: <Search size={14} /> },
  ];

  return (
    <div className="flex w-96 flex-col border-l border-[var(--border-primary)] bg-[var(--bg-primary)]">
      {/* Header */}
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-[var(--border-primary)] px-4">
        <span className="text-sm font-semibold text-[var(--text-primary)]">Developer</span>
        <button
          onClick={onClose}
          className="rounded p-1 text-[var(--text-secondary)] hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
        >
          <X size={16} />
        </button>
      </div>

      {/* Tab bar */}
      <div className="flex shrink-0 border-b border-[var(--border-primary)]">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex flex-1 items-center justify-center gap-1.5 py-2 text-xs font-medium transition-colors ${
              activeTab === tab.id
                ? 'border-b-2 border-[var(--accent-hover)] text-[var(--accent)]'
                : 'text-[var(--text-muted)] hover:text-[var(--text-secondary)]'
            }`}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {activeTab === 'files' && (
          <div className="flex-1 overflow-y-auto">
            <FileTree basePath={workspacePath} onFileSelect={handleFileSelect} />
          </div>
        )}

        {activeTab === 'git' && (
          <div className="flex-1 overflow-y-auto">
            <GitGraph repoPath={workspacePath} fontSize={100} />
          </div>
        )}

        {activeTab === 'search' && (
          <SearchTab
            workspacePath={workspacePath}
            onResultClick={handleSearchResultClick}
          />
        )}
      </div>
    </div>
  );
}
