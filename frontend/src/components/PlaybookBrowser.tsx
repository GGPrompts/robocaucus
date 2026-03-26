import { useState, useEffect, useRef } from 'react';
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
// Placeholder extraction
// ---------------------------------------------------------------------------

/** Extract unique {{PLACEHOLDER}} tokens from YAML content. */
function extractPlaceholders(yaml: string): string[] {
  const regex = /\{\{([A-Z_][A-Z0-9_]*)\}\}/g;
  const seen = new Set<string>();
  const result: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = regex.exec(yaml)) !== null) {
    const token = match[1];
    if (!seen.has(token)) {
      seen.add(token);
      result.push(token);
    }
  }
  return result;
}

/** Convert a PLACEHOLDER_TOKEN to a human-readable label. */
function tokenToLabel(token: string): string {
  return token
    .split('_')
    .map((w) => w.charAt(0) + w.slice(1).toLowerCase())
    .join(' ');
}

/** Replace all {{TOKEN}} occurrences with user-provided values. */
function fillPlaceholders(yaml: string, values: Record<string, string>): string {
  return yaml.replace(/\{\{([A-Z_][A-Z0-9_]*)\}\}/g, (_, token: string) => {
    return values[token] ?? '';
  });
}

// ---------------------------------------------------------------------------
// Placeholder Input Modal (inline sub-component)
// ---------------------------------------------------------------------------

interface PlaceholderModalProps {
  playbook: Playbook;
  placeholders: string[];
  onSubmit: (filledYaml: string) => void;
  onCancel: () => void;
}

function PlaceholderModal({ playbook, placeholders, onSubmit, onCancel }: PlaceholderModalProps) {
  const [values, setValues] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    for (const p of placeholders) {
      initial[p] = '';
    }
    return initial;
  });
  const firstInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    firstInputRef.current?.focus();
  }, []);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const filled = fillPlaceholders(playbook.yamlContent, values);
    onSubmit(filled);
  }

  const allFilled = placeholders.every((p) => values[p].trim().length > 0);

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) onCancel();
  }

  return (
    <div
      onClick={handleBackdropClick}
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-[fadeIn_150ms_ease-out]"
    >
      <div className="w-full max-w-md rounded-xl bg-[var(--bg-primary)] shadow-2xl ring-1 ring-white/10 animate-[scaleIn_150ms_ease-out]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--border-primary)] px-6 py-4">
          <div>
            <h3 className="text-base font-semibold text-[var(--text-primary)]">
              {playbook.name}
            </h3>
            <p className="mt-0.5 text-xs text-[var(--text-secondary)]">
              Fill in the details to run this playbook
            </p>
          </div>
          <button
            onClick={onCancel}
            className="flex h-8 w-8 items-center justify-center rounded-lg text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
            aria-label="Cancel"
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

        {/* Form */}
        <form onSubmit={handleSubmit} className="px-6 py-5">
          <div className="space-y-4">
            {placeholders.map((token, i) => (
              <div key={token}>
                <label
                  htmlFor={`ph-${token}`}
                  className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]"
                >
                  {tokenToLabel(token)}
                </label>
                <input
                  ref={i === 0 ? firstInputRef : undefined}
                  id={`ph-${token}`}
                  type="text"
                  value={values[token]}
                  onChange={(e) =>
                    setValues((prev) => ({ ...prev, [token]: e.target.value }))
                  }
                  onKeyDown={(e) => {
                    if (e.key === 'Escape') {
                      e.preventDefault();
                      onCancel();
                    }
                  }}
                  placeholder={`Enter ${tokenToLabel(token).toLowerCase()}...`}
                  className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent)] focus:ring-1 focus:ring-[var(--accent)]"
                />
              </div>
            ))}
          </div>

          {/* Actions */}
          <div className="mt-6 flex items-center justify-end gap-3">
            <button
              type="button"
              onClick={onCancel}
              className="rounded-lg px-4 py-2 text-sm font-medium text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!allFilled}
              className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--accent-hover)] disabled:cursor-not-allowed disabled:opacity-50"
            >
              Run Playbook
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function PlaybookBrowser({ onRunPlaybook, onClose }: PlaybookBrowserProps) {
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [runningId, setRunningId] = useState<string | null>(null);

  // Placeholder modal state: which playbook is pending input
  const [pendingPlaybook, setPendingPlaybook] = useState<Playbook | null>(null);
  const [pendingPlaceholders, setPendingPlaceholders] = useState<string[]>([]);

  useEffect(() => {
    fetchPlaybooks()
      .then((pbs) => {
        setPlaybooks(pbs);
        setLoading(false);
      })
      .catch((e) => {
        setError(e instanceof Error ? e.message : String(e));
        setLoading(false);
      });
  }, []);

  /** Initiate a playbook run -- shows placeholder modal if needed, otherwise runs immediately. */
  function handleRun(pb: Playbook) {
    const placeholders = extractPlaceholders(pb.yamlContent);
    if (placeholders.length > 0) {
      // Show placeholder input modal
      setPendingPlaybook(pb);
      setPendingPlaceholders(placeholders);
    } else {
      // No placeholders -- run immediately
      executeRun(pb.id);
    }
  }

  /** Execute the actual run API call with optional filled YAML content. */
  async function executeRun(id: string, filledYaml?: string) {
    setRunningId(id);
    setError(null);
    setPendingPlaybook(null);
    setPendingPlaceholders([]);
    try {
      const result = await runPlaybook(id, filledYaml);
      onRunPlaybook(result.conversation_id);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to run playbook');
      setRunningId(null);
    }
  }

  function handlePlaceholderSubmit(filledYaml: string) {
    if (!pendingPlaybook) return;
    executeRun(pendingPlaybook.id, filledYaml);
  }

  function handlePlaceholderCancel() {
    setPendingPlaybook(null);
    setPendingPlaceholders([]);
  }

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) onClose();
  }

  return (
    <>
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
                      onClick={() => handleRun(pb)}
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

      {/* Placeholder input modal -- rendered above the playbook browser */}
      {pendingPlaybook && pendingPlaceholders.length > 0 && (
        <PlaceholderModal
          playbook={pendingPlaybook}
          placeholders={pendingPlaceholders}
          onSubmit={handlePlaceholderSubmit}
          onCancel={handlePlaceholderCancel}
        />
      )}
    </>
  );
}
