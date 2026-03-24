import { useState, useCallback, useMemo } from 'react';
import { fetchFileList, type FileEntry } from '../lib/api';

// ── Types ─────────────────────────────────────────────────────────────────

interface FileTreeNode {
  name: string;
  path: string;        // relative path from basePath
  isDirectory: boolean;
  size: number;
  modified: number;
  children?: FileTreeNode[];
}

interface FileTreeProps {
  basePath: string;
  onFileSelect: (path: string) => void;
}

interface TreeItemProps {
  node: FileTreeNode;
  depth: number;
  selectedPath: string | null;
  expandedPaths: Set<string>;
  loadedPaths: Set<string>;
  loadingPaths: Set<string>;
  onToggle: (path: string) => void;
  onSelect: (path: string) => void;
  onLoadChildren: (dirPath: string) => Promise<void>;
}

// ── Icons ─────────────────────────────────────────────────────────────────

function ChevronIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" fill="currentColor">
      <path d="M3 1.5L7 5L3 8.5" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function SpinnerIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" className="animate-spin" fill="none">
      <circle cx="5" cy="5" r="4" stroke="currentColor" strokeWidth="1.5" opacity="0.3" />
      <path d="M5 1a4 4 0 0 1 4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function FolderIcon({ open }: { open: boolean }) {
  if (open) {
    return (
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
        <path d="M1.5 3.5h5l1.5 1.5H14.5v8h-13z" fill="var(--accent)" opacity="0.2" />
        <path d="M1.5 3.5h5l1.5 1.5H14.5v8h-13z" />
        <path d="M2.5 7h12l-2 6h-10z" fill="var(--accent)" opacity="0.15" />
      </svg>
    );
  }
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2">
      <path d="M1.5 3.5h5l1.5 1.5H14.5v9h-13z" fill="var(--accent)" opacity="0.15" />
      <path d="M1.5 3.5h5l1.5 1.5H14.5v9h-13z" />
    </svg>
  );
}

function FileIcon({ path }: { path: string }) {
  const ext = path.split('.').pop()?.toLowerCase() ?? '';
  let color = 'var(--text-secondary)';

  const colorMap: Record<string, string> = {
    ts: '#3178c6', tsx: '#3178c6', js: '#f7df1e', jsx: '#f7df1e',
    rs: '#dea584', py: '#3572a5', go: '#00add8', rb: '#cc342d',
    json: '#a8b1b8', yaml: '#cb171e', yml: '#cb171e', toml: '#9c4221',
    md: '#519aba', css: '#563d7c', html: '#e34c26', scss: '#c6538c',
    sh: '#89e051', bash: '#89e051', sql: '#e38c00',
  };
  if (colorMap[ext]) color = colorMap[ext];

  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke={color} strokeWidth="1.2" className="flex-shrink-0">
      <path d="M3 1.5h6.5L13 5v9.5H3z" />
      <path d="M9.5 1.5V5H13" />
    </svg>
  );
}

// ── Indent guides ─────────────────────────────────────────────────────────

function IndentGuides({ depth }: { depth: number }) {
  if (depth === 0) return null;
  return (
    <div className="absolute top-0 bottom-0 pointer-events-none" style={{ left: 8 }}>
      {Array.from({ length: depth }, (_, i) => (
        <div
          key={i}
          className="absolute top-0 bottom-0"
          style={{
            left: i * 12,
            width: 1,
            backgroundColor: 'var(--border)',
            opacity: 0.5,
          }}
        />
      ))}
    </div>
  );
}

// ── TreeItem ──────────────────────────────────────────────────────────────

function TreeItem({
  node,
  depth,
  selectedPath,
  expandedPaths,
  loadedPaths,
  loadingPaths,
  onToggle,
  onSelect,
  onLoadChildren,
}: TreeItemProps) {
  const isExpanded = expandedPaths.has(node.path);
  const isSelected = node.path === selectedPath;
  const paddingLeft = 8 + depth * 12;
  const needsLoading = node.isDirectory && !node.children && !loadedPaths.has(node.path);
  const isLoading = loadingPaths.has(node.path);

  const handleFolderClick = () => {
    if (needsLoading && !isLoading) {
      onLoadChildren(node.path).then(() => {
        if (!isExpanded) onToggle(node.path);
      });
    } else {
      onToggle(node.path);
    }
  };

  if (node.isDirectory) {
    return (
      <div role="treeitem" aria-expanded={isExpanded}>
        <button
          onClick={handleFolderClick}
          className="w-full text-left py-1.5 pr-2 flex items-center gap-1 text-sm transition-colors relative"
          style={{
            paddingLeft,
            color: 'var(--text-secondary)',
            backgroundColor: 'transparent',
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = 'var(--bg-primary)';
            e.currentTarget.style.color = 'var(--text-primary)';
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = 'transparent';
            e.currentTarget.style.color = 'var(--text-secondary)';
          }}
        >
          <IndentGuides depth={depth} />
          <span
            className="w-3.5 h-3.5 flex items-center justify-center transition-transform flex-shrink-0"
            style={{
              color: 'var(--text-secondary)',
              transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)',
            }}
          >
            {isLoading ? <SpinnerIcon /> : <ChevronIcon />}
          </span>
          <span className="w-3.5 h-3.5 flex items-center justify-center flex-shrink-0">
            <FolderIcon open={isExpanded} />
          </span>
          <span className="truncate flex-1 min-w-0">{node.name}</span>
        </button>
        {isExpanded && node.children && (
          <div role="group">
            {node.children.map((child) => (
              <TreeItem
                key={child.path}
                node={child}
                depth={depth + 1}
                selectedPath={selectedPath}
                expandedPaths={expandedPaths}
                loadedPaths={loadedPaths}
                loadingPaths={loadingPaths}
                onToggle={onToggle}
                onSelect={onSelect}
                onLoadChildren={onLoadChildren}
              />
            ))}
          </div>
        )}
      </div>
    );
  }

  // File node
  const filePaddingLeft = paddingLeft + 16;

  return (
    <button
      onClick={() => onSelect(node.path)}
      role="treeitem"
      className="w-full text-left py-1.5 pr-2 flex items-center gap-1 text-sm transition-colors relative"
      style={{
        paddingLeft: filePaddingLeft,
        backgroundColor: isSelected
          ? 'var(--selection-bg, color-mix(in srgb, var(--accent) 20%, transparent))'
          : 'transparent',
        color: isSelected ? 'var(--selection-text, var(--accent))' : 'var(--text-primary)',
        fontWeight: isSelected ? 500 : 400,
      }}
      onMouseEnter={(e) => {
        if (!isSelected) {
          e.currentTarget.style.backgroundColor = 'var(--bg-primary)';
        }
      }}
      onMouseLeave={(e) => {
        if (!isSelected) {
          e.currentTarget.style.backgroundColor = 'transparent';
        }
      }}
    >
      <IndentGuides depth={depth} />
      <FileIcon path={node.path} />
      <span className="truncate flex-1 min-w-0">{node.name}</span>
    </button>
  );
}

// ── Main FileTree component ───────────────────────────────────────────────

function entriesToNodes(entries: FileEntry[], parentDir: string): FileTreeNode[] {
  return entries.map((e) => ({
    name: e.name,
    path: parentDir ? `${parentDir}/${e.name}` : e.name,
    isDirectory: e.is_dir,
    size: e.size,
    modified: e.modified,
    children: e.is_dir ? undefined : undefined, // lazy loaded
  }));
}

export function FileTree({ basePath, onFileSelect }: FileTreeProps) {
  const [rootNodes, setRootNodes] = useState<FileTreeNode[]>([]);
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [loadedPaths, setLoadedPaths] = useState<Set<string>>(new Set());
  const [loadingPaths, setLoadingPaths] = useState<Set<string>>(new Set());
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [rootLoaded, setRootLoaded] = useState(false);

  // Load root directory on mount / basePath change
  const loadRoot = useCallback(async () => {
    try {
      setError(null);
      const res = await fetchFileList(basePath);
      setRootNodes(entriesToNodes(res.entries, ''));
      setRootLoaded(true);
      setExpandedPaths(new Set());
      setLoadedPaths(new Set());
      setLoadingPaths(new Set());
      setSelectedPath(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load files');
    }
  }, [basePath]);

  // Load on first render / basePath change
  useMemo(() => {
    loadRoot();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [basePath]);

  // Lazy load children for a directory
  const loadChildren = useCallback(async (dirPath: string) => {
    if (loadedPaths.has(dirPath) || loadingPaths.has(dirPath)) return;

    setLoadingPaths((prev) => new Set(prev).add(dirPath));
    try {
      const res = await fetchFileList(basePath, dirPath);
      const children = entriesToNodes(res.entries, dirPath);

      // Patch children into the tree
      setRootNodes((prev) => patchChildren(prev, dirPath, children));
      setLoadedPaths((prev) => new Set(prev).add(dirPath));
    } catch (err) {
      console.error(`Failed to load ${dirPath}:`, err);
    } finally {
      setLoadingPaths((prev) => {
        const next = new Set(prev);
        next.delete(dirPath);
        return next;
      });
    }
  }, [basePath, loadedPaths, loadingPaths]);

  const toggleExpand = useCallback((path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const handleFileSelect = useCallback((path: string) => {
    setSelectedPath(path);
    onFileSelect(path);
  }, [onFileSelect]);

  if (error) {
    return (
      <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
        <p style={{ color: 'var(--error, #ef4444)' }}>{error}</p>
        <button
          onClick={loadRoot}
          className="mt-2 text-xs underline"
          style={{ color: 'var(--accent)' }}
        >
          Retry
        </button>
      </div>
    );
  }

  if (!rootLoaded) {
    return (
      <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
        Loading...
      </div>
    );
  }

  if (rootNodes.length === 0) {
    return (
      <div className="p-3 text-sm" style={{ color: 'var(--text-secondary)' }}>
        Empty directory
      </div>
    );
  }

  return (
    <div
      className="overflow-y-auto h-full"
      style={{ backgroundColor: 'var(--bg-secondary)' }}
      role="tree"
    >
      {rootNodes.map((node) => (
        <TreeItem
          key={node.path}
          node={node}
          depth={0}
          selectedPath={selectedPath}
          expandedPaths={expandedPaths}
          loadedPaths={loadedPaths}
          loadingPaths={loadingPaths}
          onToggle={toggleExpand}
          onSelect={handleFileSelect}
          onLoadChildren={loadChildren}
        />
      ))}
    </div>
  );
}

// ── Helpers ───────────────────────────────────────────────────────────────

/** Recursively patch children into the correct directory node */
function patchChildren(
  nodes: FileTreeNode[],
  targetPath: string,
  children: FileTreeNode[],
): FileTreeNode[] {
  return nodes.map((node) => {
    if (node.path === targetPath) {
      return { ...node, children };
    }
    if (node.isDirectory && node.children && targetPath.startsWith(node.path + '/')) {
      return { ...node, children: patchChildren(node.children, targetPath, children) };
    }
    return node;
  });
}
