import { useState, useEffect } from 'react';
import type { Agent } from '../types.ts';
import { fetchAgentConfig, saveAgentConfig } from '../lib/api.ts';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface AgentBuilderProps {
  agent?: Agent;
  onSave: (agent: Omit<Agent, 'id'>) => void;
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const PROVIDERS: {
  value: Agent['provider'];
  label: string;
  description: string;
  defaultModel: string;
  variants: string[];
}[] = [
  { value: 'claude', label: 'Claude', description: 'Anthropic Claude CLI', defaultModel: 'sonnet', variants: ['sonnet', 'opus', 'haiku'] },
  { value: 'codex', label: 'Codex', description: 'OpenAI Codex CLI', defaultModel: 'o3', variants: ['o3', 'o4-mini', 'gpt-5.4'] },
  { value: 'gemini', label: 'Gemini', description: 'Google Gemini CLI', defaultModel: 'gemini-2.5-pro', variants: ['gemini-2.5-pro'] },
  { value: 'copilot', label: 'Copilot', description: 'GitHub Copilot CLI', defaultModel: 'gpt-4o', variants: ['gpt-4o'] },
];

const PALETTE = [
  { name: 'purple', hex: '#a855f7' },
  { name: 'blue', hex: '#3b82f6' },
  { name: 'green', hex: '#22c55e' },
  { name: 'teal', hex: '#14b8a6' },
  { name: 'orange', hex: '#f97316' },
  { name: 'red', hex: '#ef4444' },
  { name: 'pink', hex: '#ec4899' },
  { name: 'yellow', hex: '#eab308' },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function AgentBuilder({
  agent,
  onSave,
  onClose,
}: AgentBuilderProps) {
  // Form state, pre-filled when editing
  const [name, setName] = useState(agent?.name ?? '');
  const [provider, setProvider] = useState<Agent['provider'] | ''>(agent?.provider ?? '');
  const [model, setModel] = useState(agent?.model ?? '');
  const [color, setColor] = useState(agent?.color ?? PALETTE[0].hex);
  const [systemPrompt, setSystemPrompt] = useState(agent?.systemPrompt ?? '');
  const [scope, setScope] = useState<Agent['scope']>(agent?.scope ?? 'global');

  // Config editor state
  const [configOpen, setConfigOpen] = useState(false);
  const [configContent, setConfigContent] = useState('');
  const [configPath, setConfigPath] = useState('');
  const [configFormat, setConfigFormat] = useState('');
  const [configLoading, setConfigLoading] = useState(false);
  const [configMsg, setConfigMsg] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // Load config when section is expanded
  useEffect(() => {
    if (!configOpen || !agent?.id) return;
    setConfigLoading(true);
    setConfigMsg(null);
    fetchAgentConfig(agent.id)
      .then((data) => {
        setConfigContent(data.content);
        setConfigPath(data.path);
        setConfigFormat(data.format);
      })
      .catch((e) => setConfigMsg({ type: 'error', text: e.message }))
      .finally(() => setConfigLoading(false));
  }, [configOpen, agent?.id]);

  async function handleSaveConfig() {
    if (!agent?.id) return;
    setConfigMsg(null);
    try {
      await saveAgentConfig(agent.id, configContent);
      setConfigMsg({ type: 'success', text: 'Config saved successfully' });
    } catch (e: unknown) {
      setConfigMsg({ type: 'error', text: e instanceof Error ? e.message : 'Save failed' });
    }
  }

  // Validation
  const [errors, setErrors] = useState<{ name?: string; provider?: string }>({});

  // Derive provider info for the selected provider
  const selectedProviderInfo = PROVIDERS.find((p) => p.value === provider);

  // ---- Handlers -----------------------------------------------------------

  function handleSave() {
    const nextErrors: typeof errors = {};
    if (!name.trim()) nextErrors.name = 'Name is required';
    if (!provider) nextErrors.provider = 'Provider is required';

    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }

    onSave({
      name: name.trim(),
      provider: provider as Agent['provider'],
      model: model.trim() || (selectedProviderInfo?.defaultModel ?? ''),
      color,
      scope,
      systemPrompt,
    });
  }

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) onClose();
  }

  // ---- Render -------------------------------------------------------------

  const isEditing = Boolean(agent);

  return (
    /* Backdrop */
    <div
      onClick={handleBackdropClick}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-[fadeIn_150ms_ease-out]"
    >
      {/* Panel */}
      <div className="w-full max-w-lg rounded-xl bg-[var(--bg-primary)] shadow-2xl ring-1 ring-white/10 animate-[scaleIn_150ms_ease-out]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[var(--border-primary)] px-6 py-4">
          <h2 className="text-lg font-semibold text-[var(--text-primary)]">
            {isEditing ? 'Edit Agent' : 'Create Agent'}
          </h2>
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
        <div className="space-y-5 px-6 py-5">
          {/* Name */}
          <div>
            <label htmlFor="agent-name" className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">
              Name
            </label>
            <input
              id="agent-name"
              type="text"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                if (errors.name) setErrors((prev) => ({ ...prev, name: undefined }));
              }}
              placeholder="e.g. Editor, Researcher, Critic"
              className={`w-full rounded-lg border bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:ring-2 focus:ring-[var(--ring-accent)] ${
                errors.name ? 'border-red-500' : 'border-[var(--border-secondary)] focus:border-[var(--accent-hover)]'
              }`}
            />
            {errors.name && (
              <p className="mt-1 text-xs text-red-400">{errors.name}</p>
            )}
          </div>

          {/* Provider */}
          <div>
            <label className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">
              Provider
            </label>
            <div className="grid grid-cols-2 gap-2">
              {PROVIDERS.map((p) => (
                <button
                  key={p.value}
                  type="button"
                  onClick={() => {
                    setProvider(p.value);
                    setModel(p.defaultModel);
                    if (errors.provider) setErrors((prev) => ({ ...prev, provider: undefined }));
                  }}
                  className={`flex flex-col rounded-lg border px-3 py-2 text-left text-sm transition-colors ${
                    provider === p.value
                      ? 'border-[var(--accent-hover)] bg-[var(--accent-subtle)] text-[var(--text-primary)]'
                      : 'border-[var(--border-secondary)] bg-[var(--bg-secondary)] text-[var(--text-secondary)] hover:border-[var(--bg-surface)] hover:text-[var(--text-primary)]'
                  }`}
                >
                  <span className="font-medium">{p.label}</span>
                  <span className="text-[11px] text-[var(--text-muted)]">{p.description}</span>
                </button>
              ))}
            </div>
            {errors.provider && (
              <p className="mt-1 text-xs text-red-400">{errors.provider}</p>
            )}
          </div>

          {/* Model variant (shown after provider selection) */}
          {provider && selectedProviderInfo && (
            <div>
              <label htmlFor="agent-model" className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">
                Model
              </label>
              <select
                id="agent-model"
                value={model || selectedProviderInfo.defaultModel}
                onChange={(e) => setModel(e.target.value)}
                className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
              >
                {selectedProviderInfo.variants.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Color */}
          <div>
            <span className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">Color</span>
            <div className="flex gap-2">
              {PALETTE.map((swatch) => (
                <button
                  key={swatch.hex}
                  type="button"
                  title={swatch.name}
                  onClick={() => setColor(swatch.hex)}
                  className="flex h-8 w-8 items-center justify-center rounded-full transition-transform hover:scale-110"
                  style={{ backgroundColor: swatch.hex }}
                >
                  {color === swatch.hex && (
                    <svg
                      className="h-4 w-4 text-white drop-shadow-md"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                      strokeWidth={3}
                    >
                      <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                </button>
              ))}
            </div>
          </div>

          {/* Instructions / System Prompt */}
          <div>
            <label htmlFor="agent-prompt" className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">
              Instructions
            </label>
            <textarea
              id="agent-prompt"
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              placeholder="Describe this agent's role and behavior..."
              rows={4}
              className="w-full resize-none rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
            />
            <p className="mt-1 text-xs text-[var(--text-muted)]">
              Instructions that shape this agent's behavior. Saved to the agent's config folder and discovered natively by the CLI.
            </p>
          </div>

          {/* Scope toggle */}
          <div className="flex items-center justify-between">
            <div>
              <span className="block text-sm font-medium text-[var(--text-secondary)]">Scope</span>
              <span className="text-xs text-[var(--text-muted)]">
                {scope === 'global'
                  ? 'Global: available in all conversations across workspaces'
                  : 'Workspace: only available when working in a specific project directory'}
              </span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={scope === 'global'}
              onClick={() => setScope(scope === 'global' ? 'workspace' : 'global')}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                scope === 'global' ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
              }`}
            >
              <span
                className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                  scope === 'global' ? 'translate-x-6' : 'translate-x-1'
                }`}
              />
            </button>
          </div>

          {/* MCP Servers & Tool Permissions — only for existing agents */}
          {isEditing && agent?.id && (
            <div className="rounded-lg border border-[var(--border-secondary)]">
              <button
                type="button"
                onClick={() => setConfigOpen((v) => !v)}
                className="flex w-full items-center justify-between px-3 py-2 text-sm font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] transition-colors"
              >
                <span>MCP Servers &amp; Tool Permissions</span>
                <svg
                  className={`h-4 w-4 transition-transform ${configOpen ? 'rotate-180' : ''}`}
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  strokeWidth={2}
                >
                  <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
                </svg>
              </button>

              {configOpen && (
                <div className="border-t border-[var(--border-secondary)] px-3 py-3 space-y-2">
                  {configPath && (
                    <p className="text-xs text-[var(--text-muted)] break-all">{configPath} ({configFormat})</p>
                  )}

                  {configLoading ? (
                    <p className="text-xs text-[var(--text-secondary)]">Loading config...</p>
                  ) : (
                    <>
                      <textarea
                        value={configContent}
                        onChange={(e) => {
                          setConfigContent(e.target.value);
                          setConfigMsg(null);
                        }}
                        rows={10}
                        className="w-full resize-y rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-xs font-mono text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                        placeholder={`Paste or edit your ${configFormat === 'toml' ? 'TOML' : 'JSON'} config here...`}
                      />
                      <div className="flex items-center gap-3">
                        <button
                          type="button"
                          onClick={handleSaveConfig}
                          className="rounded-lg bg-[var(--accent)] px-3 py-1.5 text-xs font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--accent-hover)]"
                        >
                          Save Config
                        </button>
                        {configMsg && (
                          <span className={`text-xs ${configMsg.type === 'success' ? 'text-green-400' : 'text-red-400'}`}>
                            {configMsg.text}
                          </span>
                        )}
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 border-t border-[var(--border-primary)] px-6 py-4">
          <button
            onClick={onClose}
            className="rounded-lg px-4 py-2 text-sm font-medium text-[var(--text-secondary)] transition-colors hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--accent-hover)]"
          >
            {isEditing ? 'Save Changes' : 'Create Agent'}
          </button>
        </div>
      </div>
    </div>
  );
}
