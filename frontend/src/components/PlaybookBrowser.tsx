import { useState, useEffect } from 'react';
import type { Playbook } from '../types';
import { fetchPlaybooks, runPlaybook } from '../lib/api';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface PlaybookBrowserProps {
  onRunPlaybook: (conversationId: string) => void;
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Flow-type badge colors
// ---------------------------------------------------------------------------

const FLOW_TYPE_COLORS: Record<string, string> = {
  debate: 'bg-red-500/20 text-red-400',
  'parallel-then-compare': 'bg-blue-500/20 text-blue-400',
  'round-robin-then-synthesize': 'bg-green-500/20 text-green-400',
};

function flowBadgeClass(flowType: string): string {
  return FLOW_TYPE_COLORS[flowType] ?? 'bg-[var(--bg-surface)]/20 text-[var(--text-secondary)]';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function PlaybookBrowser({ onRunPlaybook, onClose }: PlaybookBrowserProps) {
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [runningId, setRunningId] = useState<string | null>(null);

  useEffect(() => {
    fetchPlaybooks()
      .then((pbs) => {
        setPlaybooks(pbs);
        setLoading(false);
      })
      // TODO: [code-review] e.message assumes Error instance — use e instanceof Error ? e.message : String(e) (85%)
      .catch((e) => {
        setError(e.message);
        setLoading(false);
      });
  }, []);

  async function handleRun(id: string) {
    setRunningId(id);
    setError(null);
    try {
      const result = await runPlaybook(id);
      onRunPlaybook(result.conversation_id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to run playbook');
      setRunningId(null);
    }
  }

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) onClose();
  }

  return (
    <div
      onClick={handleBackdropClick}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-[fadeIn_150ms_ease-out]"
    >
      <div className="w-full max-w-2xl rounded-xl bg-[var(--bg-primary)] shadow-2xl ring-1 ring-white/10 animate-[scaleIn_150ms_ease-out]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--border-primary)] px-6 py-4">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">Playbooks</h2>
          <button
            onClick={onClose}
            className="flex h-8 w-8 items-center justify-center rounded-lg text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
            aria-label="Close"
          >
            <svg
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="px-6 py-5">
          {loading && (
            <p className="text-center text-sm text-[var(--text-secondary)]">Loading playbooks...</p>
          )}

          {error && (
            <p className="mb-4 rounded-lg bg-red-500/10 px-3 py-2 text-sm text-red-400">
              {error}
            </p>
          )}

          {!loading && playbooks.length === 0 && !error && (
            <p className="text-center text-sm text-[var(--text-muted)]">No playbooks found.</p>
          )}

          {!loading && playbooks.length > 0 && (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {playbooks.map((pb) => (
                <div
                  key={pb.id}
                  className="flex flex-col rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)]/50 p-4 transition-colors hover:border-[var(--bg-surface)]"
                >
                  <div className="mb-2 flex items-start justify-between gap-2">
                    <h3 className="font-medium text-[var(--text-primary)]">{pb.name}</h3>
                    <span
                      className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium ${flowBadgeClass(pb.flowType)}`}
                    >
                      {pb.flowType}
                    </span>
                  </div>
                  <p className="mb-4 flex-1 text-xs text-[var(--text-secondary)]">{pb.description}</p>
                  <button
                    onClick={() => handleRun(pb.id)}
                    disabled={runningId !== null}
                    className="mt-auto w-full rounded-lg bg-[var(--accent)] px-3 py-1.5 text-sm font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--accent-hover)] disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    {runningId === pb.id ? 'Starting...' : 'Run'}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
