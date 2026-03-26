import { useState, useEffect } from 'react';
import { Loader2, Copy, Check, FilePlus, FileMinus, FileEdit, FileText } from 'lucide-react';
import { fetchCommitDetails, fetchGitDiff } from '../../lib/api';
import { DiffViewer } from './DiffViewer';

interface CommitFile {
  status: 'A' | 'M' | 'D' | 'R' | string;
  path: string;
}

interface CommitData {
  hash: string;
  shortHash: string;
  message: string;
  body: string | null;
  author: string | null;
  email: string | null;
  date: string | null;
  parents: string[];
  refs: string[];
  files: CommitFile[];
}

interface CommitViewProps {
  hash: string;
  repoPath: string;
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'A':
      return <FilePlus className="w-3.5 h-3.5" style={{ color: '#4ade80' }} />;
    case 'D':
      return <FileMinus className="w-3.5 h-3.5" style={{ color: '#f87171' }} />;
    case 'M':
      return <FileEdit className="w-3.5 h-3.5" style={{ color: '#fbbf24' }} />;
    case 'R':
      return <FileText className="w-3.5 h-3.5" style={{ color: '#60a5fa' }} />;
    default:
      return <FileText className="w-3.5 h-3.5" style={{ color: 'var(--text-secondary)' }} />;
  }
}

function getStatusLabel(status: string): string {
  switch (status) {
    case 'A': return 'Added';
    case 'D': return 'Deleted';
    case 'M': return 'Modified';
    case 'R': return 'Renamed';
    default: return status;
  }
}

export function CommitView({ hash, repoPath }: CommitViewProps) {
  const [data, setData] = useState<CommitData | null>(null);
  const [diff, setDiff] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        setLoading(true);
        setError(null);

        const [details, diffResult] = await Promise.all([
          fetchCommitDetails(repoPath, hash),
          fetchGitDiff(repoPath, hash),
        ]);

        if (!cancelled) {
          setData(details.data);
          // API returns { data: { diff: string, filePath: string } }
          const diffData = diffResult.data as unknown as { diff: string };
          setDiff(typeof diffData === 'string' ? diffData : diffData?.diff ?? '');
          setLoading(false);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load commit');
          setLoading(false);
        }
      }
    }

    load();
    return () => { cancelled = true; };
  }, [hash, repoPath]);

  const handleCopyHash = async () => {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(data.hash);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch { /* ignore */ }
  };

  if (loading) {
    return (
      <div className="flex flex-1 items-center justify-center" style={{ color: 'var(--text-secondary)' }}>
        <Loader2 className="w-5 h-5 animate-spin mr-2" />
        Loading commit...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm" style={{ color: '#f87171' }}>
        {error}
      </div>
    );
  }

  if (!data) return null;

  const formattedDate = data.date ? new Date(data.date).toLocaleString() : null;

  return (
    <div className="flex flex-1 flex-col overflow-hidden bg-[var(--bg-primary)]">
      {/* Commit header */}
      <div className="shrink-0 border-b border-[var(--border-primary)] px-6 py-4" style={{ backgroundColor: 'var(--bg-secondary)' }}>
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <h2 className="text-base font-semibold mb-1" style={{ color: 'var(--text-primary)' }}>
              {data.message}
            </h2>
            {data.body && (
              <p className="text-sm whitespace-pre-wrap mb-2" style={{ color: 'var(--text-secondary)' }}>
                {data.body}
              </p>
            )}
            <div className="flex items-center gap-3 text-xs" style={{ color: 'var(--text-muted)' }}>
              <span className="font-mono" style={{ color: 'var(--accent)' }}>{data.shortHash}</span>
              {data.author && <span>{data.author}</span>}
              {formattedDate && <span>{formattedDate}</span>}
              {data.refs?.length > 0 && data.refs.map((ref) => (
                <span key={ref} className="rounded px-1.5 py-0.5 text-[10px] font-medium" style={{ backgroundColor: 'var(--bg-primary)', color: 'var(--accent)', border: '1px solid var(--border-primary)' }}>
                  {ref}
                </span>
              ))}
            </div>
          </div>
          <button
            onClick={handleCopyHash}
            className="flex items-center gap-1.5 rounded px-2.5 py-1.5 text-xs shrink-0 transition-colors"
            style={{ backgroundColor: 'var(--bg-primary)', border: '1px solid var(--border-primary)', color: copied ? '#4ade80' : 'var(--text-secondary)' }}
          >
            {copied ? <><Check className="w-3.5 h-3.5" /> Copied!</> : <><Copy className="w-3.5 h-3.5" /> Copy Hash</>}
          </button>
        </div>

        {/* Files changed summary */}
        {data.files?.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-1.5">
            {data.files.map((file) => (
              <span
                key={file.path}
                className="flex items-center gap-1 rounded px-2 py-0.5 text-xs font-mono"
                style={{ backgroundColor: 'var(--bg-primary)', border: '1px solid var(--border-primary)', color: 'var(--text-secondary)' }}
                title={`${getStatusLabel(file.status)}: ${file.path}`}
              >
                {getStatusIcon(file.status)}
                {file.path.split('/').pop()}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Full diff */}
      <div className="flex-1 overflow-auto">
        {diff ? (
          <DiffViewer diff={diff} />
        ) : (
          <div className="p-4 text-sm" style={{ color: 'var(--text-muted)' }}>No diff available</div>
        )}
      </div>
    </div>
  );
}
