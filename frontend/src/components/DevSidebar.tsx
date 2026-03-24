import { useState, useCallback } from 'react';
import { X, FolderTree, GitBranch, Search } from 'lucide-react';
import { FileTree } from './FileTree';
import { CodeViewer } from './CodeViewer';
import { GitGraph } from './git/GitGraph';
import { searchFiles, type SearchResult } from '../lib/api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type Tab = 'files' | 'git' | 'search';

interface DevSidebarProps {
  workspacePath: string;
  onClose: () => void;
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
      <div className="flex gap-1 p-2 border-b border-gray-800">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleSearch();
          }}
          placeholder="Search files..."
          className="flex-1 rounded bg-gray-800 px-2 py-1.5 text-sm text-gray-200 placeholder-gray-500 border border-gray-700 focus:border-indigo-500 focus:outline-none"
        />
        <button
          onClick={handleSearch}
          disabled={searching || !query.trim()}
          className="rounded bg-indigo-600 px-3 py-1.5 text-sm text-white hover:bg-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {searching ? '...' : 'Go'}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        {error && (
          <div className="p-3 text-xs text-red-400">{error}</div>
        )}

        {!error && searched && results.length === 0 && !searching && (
          <div className="p-3 text-sm text-gray-500">No results found.</div>
        )}

        {results.map((r, idx) => (
          <button
            key={`${r.file}:${r.line}:${idx}`}
            onClick={() => onResultClick(r.file, r.line)}
            className="w-full text-left px-3 py-2 text-sm border-b border-gray-800/50 hover:bg-gray-800 transition-colors"
          >
            <div className="text-indigo-400 text-xs font-mono truncate">
              {r.file}:{r.line}
            </div>
            <div className="text-gray-300 text-xs font-mono truncate mt-0.5">
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

export function DevSidebar({ workspacePath, onClose }: DevSidebarProps) {
  const [activeTab, setActiveTab] = useState<Tab>('files');
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const handleFileSelect = useCallback((path: string) => {
    setSelectedFile(path);
  }, []);

  const handleSearchResultClick = useCallback((file: string, _line: number) => {
    setSelectedFile(file);
    setActiveTab('files');
  }, []);

  if (!workspacePath) {
    return (
      <div className="flex w-96 flex-col border-l border-gray-800 bg-gray-900">
        <div className="flex h-12 shrink-0 items-center justify-between border-b border-gray-800 px-4">
          <span className="text-sm font-semibold text-white">Developer</span>
          <button
            onClick={onClose}
            className="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
          >
            <X size={16} />
          </button>
        </div>
        <div className="flex flex-1 items-center justify-center text-sm text-gray-500">
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
    <div className="flex w-96 flex-col border-l border-gray-800 bg-gray-900">
      {/* Header */}
      <div className="flex h-12 shrink-0 items-center justify-between border-b border-gray-800 px-4">
        <span className="text-sm font-semibold text-white">Developer</span>
        <button
          onClick={onClose}
          className="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
        >
          <X size={16} />
        </button>
      </div>

      {/* Tab bar */}
      <div className="flex shrink-0 border-b border-gray-800">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex flex-1 items-center justify-center gap-1.5 py-2 text-xs font-medium transition-colors ${
              activeTab === tab.id
                ? 'border-b-2 border-indigo-500 text-indigo-400'
                : 'text-gray-500 hover:text-gray-300'
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
          <div className="flex flex-1 flex-col overflow-hidden">
            {/* File tree pane */}
            <div
              className={`overflow-y-auto ${selectedFile ? 'h-1/3 shrink-0 border-b border-gray-800' : 'flex-1'}`}
            >
              <FileTree basePath={workspacePath} onFileSelect={handleFileSelect} />
            </div>
            {/* Code viewer pane */}
            {selectedFile && (
              <div className="flex flex-1 flex-col overflow-hidden">
                <div className="flex shrink-0 items-center justify-between border-b border-gray-800 px-3 py-1.5">
                  <span className="truncate text-xs font-mono text-gray-400">
                    {selectedFile}
                  </span>
                  <button
                    onClick={() => setSelectedFile(null)}
                    className="ml-2 rounded p-0.5 text-gray-500 hover:bg-gray-800 hover:text-white"
                  >
                    <X size={12} />
                  </button>
                </div>
                <div className="flex-1 overflow-auto">
                  <CodeViewer filePath={selectedFile} basePath={workspacePath} />
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'git' && (
          <div className="flex-1 overflow-y-auto">
            <GitGraph repoPath={workspacePath} fontSize={12} />
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
