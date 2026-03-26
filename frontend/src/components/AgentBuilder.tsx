import { useState, useEffect } from 'react';
import type { Agent } from '../types.ts';
import { fetchAgentConfig, saveAgentConfig, fetchProviders } from '../lib/api.ts';
import type { ProviderInfo } from '../lib/api.ts';
import { TagInput, PathListInput, CollapsibleSection } from './ui';

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
  { value: 'codex', label: 'Codex', description: 'OpenAI Codex CLI', defaultModel: 'o3', variants: ['o3', 'o4-mini', 'gpt-5-codex', 'gpt-5.1-codex', 'gpt-5.2-codex', 'gpt-5.4', 'gpt-5.4-mini'] },
  { value: 'gemini', label: 'Gemini', description: 'Google Gemini CLI', defaultModel: 'auto', variants: ['auto', 'pro', 'flash', 'flash-lite'] },
  { value: 'copilot', label: 'Copilot', description: 'GitHub Copilot CLI', defaultModel: 'claude-sonnet-4.6', variants: ['claude-sonnet-4.6', 'claude-opus-4.6', 'claude-haiku-4.5', 'gpt-5.4', 'gpt-5.2', 'gpt-5.1-codex', 'gpt-5.4-mini', 'gpt-4.1', 'gemini-3-pro-preview'] },
];

const CUSTOM_MODEL_SENTINEL = '__custom__';

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

  // CLI config state — provider-specific flags
  const [cliConfig, setCliConfig] = useState<Record<string, unknown>>(
    agent?.cliConfig ?? {}
  );

  function updateConfig(key: string, value: unknown) {
    setCliConfig(prev => {
      const next = { ...prev };
      if (value === '' || value === false || value === undefined || value === null || (Array.isArray(value) && value.length === 0)) {
        delete next[key];
      } else {
        next[key] = value;
      }
      return next;
    });
  }

  // Custom model state: detect if existing agent uses a model not in the known variants
  const initProvider = PROVIDERS.find((p) => p.value === agent?.provider);
  const isInitialCustom = Boolean(agent?.model && initProvider && !initProvider.variants.includes(agent.model));
  const [customModel, setCustomModel] = useState(isInitialCustom ? (agent?.model ?? '') : '');
  const [useCustomModel, setUseCustomModel] = useState(isInitialCustom);

  // Provider availability from backend CLI detection
  const [providerAvailability, setProviderAvailability] = useState<Record<string, ProviderInfo>>({});
  const [availabilityLoaded, setAvailabilityLoaded] = useState(false);

  useEffect(() => {
    fetchProviders()
      .then((data) => {
        const map: Record<string, ProviderInfo> = {};
        for (const p of data.providers) {
          map[p.id] = p;
        }
        setProviderAvailability(map);
      })
      .catch(() => {
        // If the endpoint fails, treat all providers as available (graceful degradation)
      })
      .finally(() => setAvailabilityLoaded(true));
  }, []);

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

    const resolvedModel = useCustomModel ? customModel.trim() : (model.trim() || (selectedProviderInfo?.defaultModel ?? ''));

    onSave({
      name: name.trim(),
      provider: provider as Agent['provider'],
      model: resolvedModel,
      color,
      scope,
      systemPrompt,
      cliConfig: Object.keys(cliConfig).length > 0 ? cliConfig : undefined,
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
        <div className="space-y-5 px-6 py-5 max-h-[80vh] overflow-y-auto">
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
              {PROVIDERS.map((p) => {
                const info = providerAvailability[p.value];
                const isAvailable = !availabilityLoaded || !info || info.available;
                return (
                  <button
                    key={p.value}
                    type="button"
                    onClick={() => {
                      setProvider(p.value);
                      setModel(p.defaultModel);
                      setUseCustomModel(false);
                      setCustomModel('');
                      setCliConfig({});
                      if (errors.provider) setErrors((prev) => ({ ...prev, provider: undefined }));
                    }}
                    title={!isAvailable ? `${info.cli_command} not found on PATH` : undefined}
                    className={`relative flex flex-col rounded-lg border px-3 py-2 text-left text-sm transition-colors ${
                      provider === p.value
                        ? 'border-[var(--accent-hover)] bg-[var(--accent-subtle)] text-[var(--text-primary)]'
                        : !isAvailable
                          ? 'border-[var(--border-secondary)] bg-[var(--bg-secondary)] text-[var(--text-muted)] opacity-50'
                          : 'border-[var(--border-secondary)] bg-[var(--bg-secondary)] text-[var(--text-secondary)] hover:border-[var(--bg-surface)] hover:text-[var(--text-primary)]'
                    }`}
                  >
                    <span className="font-medium">{p.label}</span>
                    <span className="text-[11px] text-[var(--text-muted)]">
                      {!isAvailable ? 'Not installed' : info?.version ? `${p.description} (${info.version})` : p.description}
                    </span>
                  </button>
                );
              })}
            </div>
            {provider && availabilityLoaded && providerAvailability[provider] && !providerAvailability[provider].available && (
              <p className="mt-1.5 text-xs text-yellow-400">
                CLI not detected on this machine. The agent will be created but may fail to respond until the CLI is installed.
              </p>
            )}
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
                value={useCustomModel ? CUSTOM_MODEL_SENTINEL : (model || selectedProviderInfo.defaultModel)}
                onChange={(e) => {
                  if (e.target.value === CUSTOM_MODEL_SENTINEL) {
                    setUseCustomModel(true);
                    setCustomModel('');
                  } else {
                    setUseCustomModel(false);
                    setCustomModel('');
                    setModel(e.target.value);
                  }
                }}
                className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
              >
                {selectedProviderInfo.variants.map((v) => (
                  <option key={v} value={v}>
                    {v}
                  </option>
                ))}
                <option value={CUSTOM_MODEL_SENTINEL}>Custom...</option>
              </select>
              {useCustomModel && (
                <input
                  type="text"
                  value={customModel}
                  onChange={(e) => setCustomModel(e.target.value)}
                  placeholder="Enter custom model name"
                  className="mt-2 w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                  autoFocus
                />
              )}
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

          {/* Provider-specific CLI config */}
          {provider === 'claude' && (
            <div className="space-y-2">
              <CollapsibleSection title="Execution" defaultOpen={true}>
                <div className="space-y-3">
                  {/* effort */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Effort</label>
                    <select
                      value={(cliConfig.effort as string) ?? ''}
                      onChange={(e) => updateConfig('effort', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="low">low</option>
                      <option value="medium">medium</option>
                      <option value="high">high</option>
                      <option value="max">max (Opus only)</option>
                    </select>
                  </div>
                  {/* permission_mode */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Permission Mode</label>
                    <select
                      value={(cliConfig.permission_mode as string) ?? ''}
                      onChange={(e) => updateConfig('permission_mode', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="default">default</option>
                      <option value="plan">plan</option>
                      <option value="acceptEdits">acceptEdits</option>
                      <option value="dontAsk">dontAsk</option>
                      <option value="auto">auto</option>
                    </select>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Budget & Limits">
                <div className="space-y-3">
                  {/* max_budget_usd */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Max Budget (USD)</label>
                    <input
                      type="number"
                      step={0.5}
                      min={0}
                      value={(cliConfig.max_budget_usd as number) ?? ''}
                      onChange={(e) => updateConfig('max_budget_usd', e.target.value ? Number(e.target.value) : '')}
                      placeholder="No limit"
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                  {/* max_turns */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Max Turns</label>
                    <input
                      type="number"
                      min={1}
                      value={(cliConfig.max_turns as number) ?? ''}
                      onChange={(e) => updateConfig('max_turns', e.target.value ? Number(e.target.value) : '')}
                      placeholder="No limit"
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                  {/* fallback_model */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Fallback Model</label>
                    <input
                      type="text"
                      value={(cliConfig.fallback_model as string) ?? ''}
                      onChange={(e) => updateConfig('fallback_model', e.target.value)}
                      placeholder="e.g. sonnet"
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Tools & Permissions">
                <div className="space-y-3">
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Allowed Tools</label>
                    <TagInput
                      value={(cliConfig.allowed_tools as string[]) ?? []}
                      onChange={(v) => updateConfig('allowed_tools', v)}
                      placeholder='e.g. Bash(git:*), Read'
                    />
                  </div>
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Disallowed Tools</label>
                    <TagInput
                      value={(cliConfig.disallowed_tools as string[]) ?? []}
                      onChange={(v) => updateConfig('disallowed_tools', v)}
                      placeholder='e.g. Bash(rm:*)'
                    />
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Advanced">
                <div className="space-y-3">
                  {/* append_system_prompt */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Append System Prompt</label>
                    <textarea
                      value={(cliConfig.append_system_prompt as string) ?? ''}
                      onChange={(e) => updateConfig('append_system_prompt', e.target.value)}
                      placeholder="Additional prompt appended to defaults"
                      rows={3}
                      className="w-full resize-none rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                  {/* bare */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Bare mode</span>
                      <span className="text-xs text-[var(--text-muted)]">Skip hooks, plugins, CLAUDE.md discovery</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.bare)}
                      onClick={() => updateConfig('bare', !cliConfig.bare)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.bare ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.bare ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>
            </div>
          )}

          {provider === 'codex' && (
            <div className="space-y-2">
              <CollapsibleSection title="Execution" defaultOpen={true}>
                <div className="space-y-3">
                  {/* sandbox */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Sandbox</label>
                    <select
                      value={(cliConfig.sandbox as string) ?? ''}
                      onChange={(e) => updateConfig('sandbox', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="read-only">read-only</option>
                      <option value="workspace-write">workspace-write</option>
                      <option value="danger-full-access">danger-full-access</option>
                    </select>
                  </div>
                  {/* approval_policy */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Approval Policy</label>
                    <select
                      value={(cliConfig.approval_policy as string) ?? ''}
                      onChange={(e) => updateConfig('approval_policy', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="untrusted">untrusted</option>
                      <option value="on-request">on-request</option>
                      <option value="never">never</option>
                    </select>
                  </div>
                  {/* full_auto */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Full Auto</span>
                      <span className="text-xs text-[var(--text-muted)]">Sets workspace-write sandbox + on-request approval</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.full_auto)}
                      onClick={() => {
                        const next = !cliConfig.full_auto;
                        if (next) {
                          setCliConfig(prev => ({
                            ...prev,
                            full_auto: true,
                            sandbox: 'workspace-write',
                            approval_policy: 'on-request',
                          }));
                        } else {
                          updateConfig('full_auto', false);
                        }
                      }}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.full_auto ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.full_auto ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Model Tuning">
                <div className="space-y-3">
                  {/* reasoning_effort */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Reasoning Effort</label>
                    <select
                      value={(cliConfig.reasoning_effort as string) ?? ''}
                      onChange={(e) => updateConfig('reasoning_effort', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="minimal">minimal</option>
                      <option value="low">low</option>
                      <option value="medium">medium</option>
                      <option value="high">high</option>
                      <option value="xhigh">xhigh</option>
                    </select>
                  </div>
                  {/* reasoning_summary */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Reasoning Summary</label>
                    <select
                      value={(cliConfig.reasoning_summary as string) ?? ''}
                      onChange={(e) => updateConfig('reasoning_summary', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="auto">auto</option>
                      <option value="concise">concise</option>
                      <option value="detailed">detailed</option>
                      <option value="none">none</option>
                    </select>
                  </div>
                  {/* service_tier */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Service Tier</label>
                    <select
                      value={(cliConfig.service_tier as string) ?? ''}
                      onChange={(e) => updateConfig('service_tier', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="flex">flex</option>
                      <option value="fast">fast</option>
                    </select>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Tools & Search">
                <div className="space-y-3">
                  {/* web_search */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Web Search</label>
                    <select
                      value={(cliConfig.web_search as string) ?? ''}
                      onChange={(e) => updateConfig('web_search', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="disabled">disabled</option>
                      <option value="cached">cached</option>
                      <option value="live">live</option>
                    </select>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Advanced">
                <div className="space-y-3">
                  {/* profile */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Profile</label>
                    <input
                      type="text"
                      value={(cliConfig.profile as string) ?? ''}
                      onChange={(e) => updateConfig('profile', e.target.value)}
                      placeholder="Config profile name"
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                  {/* config_overrides */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Config Overrides</label>
                    <textarea
                      value={((cliConfig.config_overrides as string[]) ?? []).join('\n')}
                      onChange={(e) => {
                        const lines = e.target.value.split('\n').filter((l) => l.trim().length > 0);
                        updateConfig('config_overrides', lines);
                      }}
                      placeholder="key=value (one per line)"
                      rows={3}
                      className="w-full resize-none rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    />
                  </div>
                </div>
              </CollapsibleSection>
            </div>
          )}

          {provider === 'copilot' && (
            <div className="space-y-2">
              <CollapsibleSection title="Execution" defaultOpen={true}>
                <div className="space-y-3">
                  {/* reasoning_effort */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Reasoning Effort</label>
                    <select
                      value={(cliConfig.reasoning_effort as string) ?? ''}
                      onChange={(e) => updateConfig('reasoning_effort', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="low">low</option>
                      <option value="medium">medium</option>
                      <option value="high">high</option>
                      <option value="xhigh">xhigh</option>
                    </select>
                  </div>
                  {/* autopilot */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Autopilot</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.autopilot)}
                      onClick={() => updateConfig('autopilot', !cliConfig.autopilot)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.autopilot ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.autopilot ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                  {/* max_autopilot_continues — only shown when autopilot is on */}
                  {Boolean(cliConfig.autopilot) && (
                    <div>
                      <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Max Autopilot Continues</label>
                      <input
                        type="number"
                        min={1}
                        value={(cliConfig.max_autopilot_continues as number) ?? ''}
                        onChange={(e) => updateConfig('max_autopilot_continues', e.target.value ? Number(e.target.value) : '')}
                        placeholder="No limit"
                        className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                      />
                    </div>
                  )}
                  {/* no_ask_user */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">No ask user</span>
                      <span className="text-xs text-[var(--text-muted)]">Agent works fully autonomously</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.no_ask_user)}
                      onClick={() => updateConfig('no_ask_user', !cliConfig.no_ask_user)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.no_ask_user ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.no_ask_user ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Permissions">
                <div className="space-y-3">
                  {/* permission_preset */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Permission Preset</label>
                    <select
                      value={(cliConfig.permission_preset as string) ?? ''}
                      onChange={(e) => updateConfig('permission_preset', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="default">default</option>
                      <option value="allow-all-tools">allow-all-tools</option>
                      <option value="allow-all">allow-all</option>
                    </select>
                    <p className="mt-1 text-xs text-[var(--text-muted)]">Current adapter already uses --allow-all-tools by default</p>
                  </div>
                  {/* allowed_tools */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Allowed Tools</label>
                    <TagInput
                      value={(cliConfig.allowed_tools as string[]) ?? []}
                      onChange={(v) => updateConfig('allowed_tools', v)}
                      placeholder='e.g. shell(git:*), write'
                      helpText='Patterns: shell(cmd), write, MyMCP(tool)'
                    />
                  </div>
                  {/* denied_tools */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Denied Tools</label>
                    <TagInput
                      value={(cliConfig.denied_tools as string[]) ?? []}
                      onChange={(v) => updateConfig('denied_tools', v)}
                    />
                  </div>
                  {/* secret_env_vars */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Secret Env Vars</label>
                    <TagInput
                      value={(cliConfig.secret_env_vars as string[]) ?? []}
                      onChange={(v) => updateConfig('secret_env_vars', v)}
                      placeholder='e.g. API_KEY, SECRET'
                    />
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="MCP Servers">
                <div className="space-y-3">
                  {/* disable_builtin_mcps */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Disable Built-in MCPs</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.disable_builtin_mcps)}
                      onClick={() => updateConfig('disable_builtin_mcps', !cliConfig.disable_builtin_mcps)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.disable_builtin_mcps ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.disable_builtin_mcps ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                  {/* enable_all_github_mcp_tools */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Enable All GitHub MCP Tools</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.enable_all_github_mcp_tools)}
                      onClick={() => updateConfig('enable_all_github_mcp_tools', !cliConfig.enable_all_github_mcp_tools)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.enable_all_github_mcp_tools ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.enable_all_github_mcp_tools ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>
            </div>
          )}

          {provider === 'gemini' && (
            <div className="space-y-2">
              <CollapsibleSection title="Execution" defaultOpen={true}>
                <div className="space-y-3">
                  {/* approval_mode */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Approval Mode</label>
                    <select
                      value={(cliConfig.approval_mode as string) ?? ''}
                      onChange={(e) => updateConfig('approval_mode', e.target.value)}
                      className="w-full rounded-lg border border-[var(--border-secondary)] bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)] outline-none transition-colors focus:border-[var(--accent-hover)] focus:ring-2 focus:ring-[var(--ring-accent)]"
                    >
                      <option value="">Default</option>
                      <option value="default">default</option>
                      <option value="auto_edit">auto_edit</option>
                      <option value="yolo">yolo</option>
                      <option value="plan">plan</option>
                    </select>
                  </div>
                  {/* sandbox */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Sandbox</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.sandbox)}
                      onClick={() => updateConfig('sandbox', !cliConfig.sandbox)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.sandbox ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.sandbox ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Extensions & Policy">
                <div className="space-y-3">
                  {/* extensions */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Extensions</label>
                    <TagInput
                      value={(cliConfig.extensions as string[]) ?? []}
                      onChange={(v) => updateConfig('extensions', v)}
                      placeholder="Extension names"
                    />
                  </div>
                  {/* policy_paths */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Policy Paths</label>
                    <PathListInput
                      value={(cliConfig.policy_paths as string[]) ?? []}
                      onChange={(v) => updateConfig('policy_paths', v)}
                      placeholder="/path/to/policy"
                    />
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="MCP Servers">
                <div className="space-y-3">
                  {/* allowed_mcp_server_names */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Allowed MCP Server Names</label>
                    <TagInput
                      value={(cliConfig.allowed_mcp_server_names as string[]) ?? []}
                      onChange={(v) => updateConfig('allowed_mcp_server_names', v)}
                      placeholder="Server names"
                    />
                  </div>
                </div>
              </CollapsibleSection>

              <CollapsibleSection title="Advanced">
                <div className="space-y-3">
                  {/* include_directories */}
                  <div>
                    <label className="mb-1 block text-xs font-medium text-[var(--text-secondary)]">Include Directories</label>
                    <PathListInput
                      value={(cliConfig.include_directories as string[]) ?? []}
                      onChange={(v) => updateConfig('include_directories', v)}
                      placeholder="/additional/dir"
                    />
                  </div>
                  {/* raw_output */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Raw Output</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.raw_output)}
                      onClick={() => updateConfig('raw_output', !cliConfig.raw_output)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.raw_output ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.raw_output ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                  {/* debug */}
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="block text-xs font-medium text-[var(--text-secondary)]">Debug</span>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(cliConfig.debug)}
                      onClick={() => updateConfig('debug', !cliConfig.debug)}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                        cliConfig.debug ? 'bg-[var(--accent-hover)]' : 'bg-[var(--bg-surface)]'
                      }`}
                    >
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                          cliConfig.debug ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </CollapsibleSection>
            </div>
          )}

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
