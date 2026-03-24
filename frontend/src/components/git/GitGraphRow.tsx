import { ChevronRight, ChevronDown } from 'lucide-react';
import type { GraphNode } from '../../lib/graphLayout';
import { CommitDetails } from './CommitDetails';

interface GitGraphRowProps {
  node: GraphNode;
  rowHeight: number;
  isSelected?: boolean;
  isExpanded?: boolean;
  repoPath: string;
  onClick?: () => void;
  onFileClick?: (path: string, status: string) => void;
}

/**
 * Format a date string as relative time (e.g., "2 hours ago", "3 days ago")
 */
function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSeconds = Math.floor(diffMs / 1000);
  const diffMinutes = Math.floor(diffSeconds / 60);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);
  const diffWeeks = Math.floor(diffDays / 7);
  const diffMonths = Math.floor(diffDays / 30);
  const diffYears = Math.floor(diffDays / 365);

  if (diffSeconds < 60) return 'just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  if (diffWeeks < 4) return `${diffWeeks}w ago`;
  if (diffMonths < 12) return `${diffMonths}mo ago`;
  return `${diffYears}y ago`;
}

/**
 * Parse refs to separate branches, tags, and HEAD
 */
function parseRefs(refs: string[]): { branches: string[]; tags: string[]; isHead: boolean } {
  const branches: string[] = [];
  const tags: string[] = [];
  let isHead = false;

  for (const ref of refs) {
    if (ref === 'HEAD') {
      isHead = true;
    } else if (ref.startsWith('HEAD -> ')) {
      isHead = true;
      branches.push(ref.replace('HEAD -> ', ''));
    } else if (ref.startsWith('tag: ')) {
      tags.push(ref.replace('tag: ', ''));
    } else if (ref.startsWith('origin/')) {
      // Remote branches - show with different styling
      branches.push(ref);
    } else {
      branches.push(ref);
    }
  }

  return { branches, tags, isHead };
}

/**
 * Get author initials for avatar
 */
function getInitials(author: string): string {
  const parts = author.split(' ').filter(Boolean);
  if (parts.length === 0) return '?';
  if (parts.length === 1) return parts[0].charAt(0).toUpperCase();
  return (parts[0].charAt(0) + parts[parts.length - 1].charAt(0)).toUpperCase();
}

export function GitGraphRow({
  node,
  rowHeight,
  isSelected = false,
  isExpanded = false,
  repoPath,
  onClick,
  onFileClick,
}: GitGraphRowProps) {
  const { branches, tags, isHead } = parseRefs(node.refs);
  const isMerge = node.parents.length > 1;
  const relativeTime = formatRelativeTime(node.date);

  // Truncate message to fit in row
  const maxMessageLength = 60;
  const truncatedMessage =
    node.message.length > maxMessageLength
      ? node.message.slice(0, maxMessageLength) + '...'
      : node.message;

  return (
    <div className="git-graph-row-wrapper">
      <div
        className="git-graph-row flex items-center gap-3 px-3 cursor-pointer transition-colors"
        style={{
          height: rowHeight,
          backgroundColor: isExpanded
            ? 'color-mix(in srgb, var(--accent) 10%, var(--bg-primary))'
            : isSelected
              ? 'color-mix(in srgb, var(--accent) 15%, var(--bg-primary))'
              : 'transparent',
          borderBottom: isExpanded ? 'none' : '1px solid var(--border)',
        }}
        onClick={onClick}
        onMouseEnter={(e) => {
          if (!isSelected && !isExpanded) {
            e.currentTarget.style.backgroundColor = 'var(--bg-secondary)';
          }
        }}
        onMouseLeave={(e) => {
          if (!isSelected && !isExpanded) {
            e.currentTarget.style.backgroundColor = 'transparent';
          }
        }}
      >
        {/* Expand/collapse indicator */}
        <span style={{ color: 'var(--text-secondary)' }}>
          {isExpanded ? (
            <ChevronDown className="w-4 h-4" />
          ) : (
            <ChevronRight className="w-4 h-4" />
          )}
        </span>
      {/* Short hash */}
      <span
        className="font-mono text-xs shrink-0"
        style={{
          color: isHead ? 'var(--accent)' : 'var(--text-secondary)',
          fontWeight: isHead ? 600 : 400,
        }}
      >
        {node.shortHash}
      </span>

      {/* Refs (branches and tags) */}
      {(branches.length > 0 || tags.length > 0) && (
        <div className="flex items-center gap-1 shrink-0">
          {branches.map((branch) => (
            <span
              key={branch}
              className="git-graph-ref px-1.5 py-0.5 text-xs rounded"
              style={{
                backgroundColor: branch.startsWith('origin/')
                  ? 'color-mix(in srgb, var(--text-secondary) 15%, transparent)'
                  : 'color-mix(in srgb, var(--accent) 20%, transparent)',
                color: branch.startsWith('origin/')
                  ? 'var(--text-secondary)'
                  : 'var(--accent)',
                border: '1px solid',
                borderColor: branch.startsWith('origin/')
                  ? 'color-mix(in srgb, var(--text-secondary) 30%, transparent)'
                  : 'color-mix(in srgb, var(--accent) 40%, transparent)',
              }}
            >
              {branch}
            </span>
          ))}
          {tags.map((tag) => (
            <span
              key={tag}
              className="git-graph-ref px-1.5 py-0.5 text-xs rounded"
              style={{
                backgroundColor: 'color-mix(in srgb, #fbbf24 20%, transparent)',
                color: '#fbbf24',
                border: '1px solid color-mix(in srgb, #fbbf24 40%, transparent)',
              }}
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      {/* Commit message */}
      <span
        className="flex-1 text-sm truncate"
        style={{ color: 'var(--text-primary)' }}
        title={node.message}
      >
        {isMerge && (
          <span className="text-xs mr-1.5" style={{ color: 'var(--text-secondary)' }}>
            [merge]
          </span>
        )}
        {truncatedMessage}
      </span>

      {/* Author */}
      <div
        className="flex items-center gap-1.5 shrink-0"
        title={node.author}
      >
        <span
          className="w-5 h-5 rounded-full flex items-center justify-center text-xs font-medium"
          style={{
            backgroundColor: 'var(--bg-secondary)',
            color: 'var(--text-secondary)',
            border: '1px solid var(--border)',
          }}
        >
          {getInitials(node.author)}
        </span>
        <span
          className="text-xs max-w-[100px] truncate hidden sm:inline"
          style={{ color: 'var(--text-secondary)' }}
        >
          {node.author.split(' ')[0]}
        </span>
      </div>

      {/* Relative time */}
      <span
        className="text-xs shrink-0 w-16 text-right"
        style={{ color: 'var(--text-secondary)' }}
        title={new Date(node.date).toLocaleString()}
      >
        {relativeTime}
      </span>
      </div>

      {/* Expanded commit details */}
      {isExpanded && (
        <CommitDetails
          hash={node.hash}
          repoPath={repoPath}
          onFileClick={onFileClick}
        />
      )}
    </div>
  );
}
