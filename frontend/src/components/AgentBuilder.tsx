import { useState } from 'react';
import type { Agent } from '../types.ts';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface AgentBuilderProps {
  agent?: Agent;
  availableModels?: string[];
  onSave: (agent: Omit<Agent, 'id'>) => void;
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ALL_MODELS: { value: Agent['model']; label: string }[] = [
  { value: 'claude', label: 'Claude' },
  { value: 'codex', label: 'ChatGPT / Codex' },
  { value: 'gemini', label: 'Gemini' },
  { value: 'copilot', label: 'Copilot' },
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
  availableModels,
  onSave,
  onClose,
}: AgentBuilderProps) {
  // Form state, pre-filled when editing
  const [name, setName] = useState(agent?.name ?? '');
  const [model, setModel] = useState<Agent['model'] | ''>(agent?.model ?? '');
  const [color, setColor] = useState(agent?.color ?? PALETTE[0].hex);
  const [systemPrompt, setSystemPrompt] = useState(agent?.systemPrompt ?? '');
  const [scope, setScope] = useState<Agent['scope']>(agent?.scope ?? 'global');

  // Validation
  const [errors, setErrors] = useState<{ name?: string; model?: string }>({});

  // Filter models if availableModels prop is provided
  const models = availableModels
    ? ALL_MODELS.filter((m) => availableModels.includes(m.value))
    : ALL_MODELS;

  // ---- Handlers -----------------------------------------------------------

  function handleSave() {
    const nextErrors: typeof errors = {};
    if (!name.trim()) nextErrors.name = 'Name is required';
    if (!model) nextErrors.model = 'Model is required';

    if (Object.keys(nextErrors).length > 0) {
      setErrors(nextErrors);
      return;
    }

    onSave({
      name: name.trim(),
      model: model as Agent['model'],
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
      <div className="w-full max-w-lg rounded-xl bg-gray-900 shadow-2xl ring-1 ring-white/10 animate-[scaleIn_150ms_ease-out]">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-gray-800 px-6 py-4">
          <h2 className="text-lg font-semibold text-white">
            {isEditing ? 'Edit Agent' : 'Create Agent'}
          </h2>
          <button
            onClick={onClose}
            className="flex h-8 w-8 items-center justify-center rounded-lg text-gray-400 transition-colors hover:bg-gray-800 hover:text-white"
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
            <label htmlFor="agent-name" className="mb-1.5 block text-sm font-medium text-gray-300">
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
              className={`w-full rounded-lg border bg-gray-800 px-3 py-2 text-sm text-white placeholder-gray-500 outline-none transition-colors focus:ring-2 focus:ring-indigo-500/50 ${
                errors.name ? 'border-red-500' : 'border-gray-700 focus:border-indigo-500'
              }`}
            />
            {errors.name && (
              <p className="mt-1 text-xs text-red-400">{errors.name}</p>
            )}
          </div>

          {/* Model */}
          <div>
            <label htmlFor="agent-model" className="mb-1.5 block text-sm font-medium text-gray-300">
              Model
            </label>
            <select
              id="agent-model"
              value={model}
              onChange={(e) => {
                setModel(e.target.value as Agent['model']);
                if (errors.model) setErrors((prev) => ({ ...prev, model: undefined }));
              }}
              className={`w-full appearance-none rounded-lg border bg-gray-800 px-3 py-2 text-sm text-white outline-none transition-colors focus:ring-2 focus:ring-indigo-500/50 ${
                errors.model ? 'border-red-500' : 'border-gray-700 focus:border-indigo-500'
              } ${!model ? 'text-gray-500' : ''}`}
            >
              <option value="" disabled>
                Select a model...
              </option>
              {models.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
            </select>
            {errors.model && (
              <p className="mt-1 text-xs text-red-400">{errors.model}</p>
            )}
          </div>

          {/* Color */}
          <div>
            <span className="mb-1.5 block text-sm font-medium text-gray-300">Color</span>
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

          {/* Personality / System Prompt */}
          <div>
            <label htmlFor="agent-prompt" className="mb-1.5 block text-sm font-medium text-gray-300">
              Personality
            </label>
            <textarea
              id="agent-prompt"
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
              placeholder="Describe this agent's role and personality..."
              rows={4}
              className="w-full resize-none rounded-lg border border-gray-700 bg-gray-800 px-3 py-2 text-sm text-white placeholder-gray-500 outline-none transition-colors focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/50"
            />
          </div>

          {/* Scope toggle */}
          <div className="flex items-center justify-between">
            <div>
              <span className="block text-sm font-medium text-gray-300">Scope</span>
              <span className="text-xs text-gray-500">
                {scope === 'global' ? 'Available everywhere' : 'This workspace only'}
              </span>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={scope === 'global'}
              onClick={() => setScope(scope === 'global' ? 'workspace' : 'global')}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full transition-colors ${
                scope === 'global' ? 'bg-indigo-500' : 'bg-gray-600'
              }`}
            >
              <span
                className={`inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ${
                  scope === 'global' ? 'translate-x-6' : 'translate-x-1'
                }`}
              />
            </button>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 border-t border-gray-800 px-6 py-4">
          <button
            onClick={onClose}
            className="rounded-lg px-4 py-2 text-sm font-medium text-gray-300 transition-colors hover:bg-gray-800 hover:text-white"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-indigo-500"
          >
            {isEditing ? 'Save Changes' : 'Create Agent'}
          </button>
        </div>
      </div>
    </div>
  );
}
