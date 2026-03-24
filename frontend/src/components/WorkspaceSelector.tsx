import { useState, useRef, useEffect } from 'react';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface WorkspaceSelectorProps {
  currentWorkspace: string | null;
  recentWorkspaces: string[];
  onSelect: (path: string) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Return the last segment of a path (the directory name). */
function dirName(fullPath: string): string {
  const trimmed = fullPath.replace(/\/+$/, '');
  const parts = trimmed.split(/[/\\]/);
  return parts[parts.length - 1] || fullPath;
}

// ---------------------------------------------------------------------------
// Folder icon (inline SVG)
// ---------------------------------------------------------------------------

function FolderIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
      />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Chevron icon
// ---------------------------------------------------------------------------

function ChevronIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      fill="none"
      viewBox="0 0 24 24"
      stroke="currentColor"
      strokeWidth={2}
    >
      <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function WorkspaceSelector({
  currentWorkspace,
  recentWorkspaces,
  onSelect,
}: WorkspaceSelectorProps) {
  const [open, setOpen] = useState(false);
  const [browsing, setBrowsing] = useState(false);
  const [browseValue, setBrowseValue] = useState('');

  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
        setBrowsing(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  // Auto-focus the browse input when it appears
  useEffect(() => {
    if (browsing) {
      inputRef.current?.focus();
    }
  }, [browsing]);

  // ---- handlers ----------------------------------------------------------

  function handleToggle() {
    setOpen((prev) => !prev);
    setBrowsing(false);
    setBrowseValue('');
  }

  function handleSelectWorkspace(path: string) {
    onSelect(path);
    setOpen(false);
    setBrowsing(false);
    setBrowseValue('');
  }

  function handleBrowseSubmit() {
    const trimmed = browseValue.trim();
    if (trimmed) {
      handleSelectWorkspace(trimmed);
    }
  }

  function handleBrowseKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter') {
      handleBrowseSubmit();
    } else if (e.key === 'Escape') {
      setBrowsing(false);
      setBrowseValue('');
    }
  }

  // ---- derived -----------------------------------------------------------

  const displayName = currentWorkspace ? dirName(currentWorkspace) : 'All Workspaces';

  // Deduplicate: recent list may already contain current workspace
  const dropdownItems = recentWorkspaces.filter((w) => w !== currentWorkspace);

  return (
    <div ref={containerRef} className="relative">
      {/* ===== Trigger ===== */}
      <button
        onClick={handleToggle}
        className="flex h-12 w-full items-center justify-between border-b border-gray-800 px-3 transition-colors hover:bg-gray-800/50"
        title={currentWorkspace ?? 'All Workspaces'}
      >
        <span className="truncate font-semibold text-white">{displayName}</span>
        <ChevronIcon
          className={`h-4 w-4 shrink-0 text-gray-500 transition-transform ${
            open ? 'rotate-180' : ''
          }`}
        />
      </button>

      {/* ===== Dropdown ===== */}
      {open && (
        <div className="absolute left-0 right-0 z-50 mt-px overflow-hidden rounded-b-lg border border-t-0 border-gray-700 bg-gray-900 shadow-xl">
          {/* Current workspace (if set) */}
          {currentWorkspace && (
            <div
              className="flex items-center gap-2 bg-gray-800/60 px-3 py-2 text-xs text-white"
              title={currentWorkspace}
            >
              <FolderIcon className="h-3.5 w-3.5 shrink-0 text-indigo-400" />
              <span className="truncate font-medium">{dirName(currentWorkspace)}</span>
              <span className="ml-auto shrink-0 rounded bg-indigo-500/20 px-1.5 py-0.5 text-[10px] font-medium text-indigo-300">
                active
              </span>
            </div>
          )}

          {/* Recent workspaces */}
          {dropdownItems.length > 0 && (
            <ul className="max-h-48 overflow-y-auto">
              {dropdownItems.map((ws) => (
                <li key={ws}>
                  <button
                    onClick={() => handleSelectWorkspace(ws)}
                    className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-gray-300 transition-colors hover:bg-gray-800 hover:text-white"
                    title={ws}
                  >
                    <FolderIcon className="h-3.5 w-3.5 shrink-0 text-gray-500" />
                    <span className="truncate">{dirName(ws)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}

          {/* Divider (only if there are items above) */}
          {(currentWorkspace || dropdownItems.length > 0) && (
            <div className="border-t border-gray-800" />
          )}

          {/* Browse input or Browse button */}
          {browsing ? (
            <div className="flex items-center gap-1 px-2 py-2">
              <input
                ref={inputRef}
                type="text"
                value={browseValue}
                onChange={(e) => setBrowseValue(e.target.value)}
                onKeyDown={handleBrowseKeyDown}
                placeholder="/path/to/project"
                className="min-w-0 flex-1 rounded bg-gray-800 px-2 py-1 text-xs text-white placeholder-gray-500 outline-none ring-1 ring-gray-700 focus:ring-indigo-500"
              />
              <button
                onClick={handleBrowseSubmit}
                disabled={!browseValue.trim()}
                className="shrink-0 rounded bg-indigo-600 px-2 py-1 text-xs font-medium text-white transition-colors hover:bg-indigo-500 disabled:opacity-40 disabled:hover:bg-indigo-600"
              >
                Open
              </button>
            </div>
          ) : (
            <button
              onClick={() => setBrowsing(true)}
              className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs text-gray-400 transition-colors hover:bg-gray-800 hover:text-white"
            >
              <svg
                className="h-3.5 w-3.5 shrink-0"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M21 21l-4.35-4.35M11 19a8 8 0 100-16 8 8 0 000 16z"
                />
              </svg>
              <span>Browse...</span>
            </button>
          )}
        </div>
      )}
    </div>
  );
}
