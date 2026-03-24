import { useState, useRef, useEffect, useCallback } from 'react';
import type { Room, Agent } from '../types.ts';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface RoomMembersProps {
  room: Room;
  members: Agent[];
  allAgents: Agent[];
  onAddAgent: (agentId: string) => void;
  onRemoveAgent: (agentId: string) => void;
  onUpdateRoom: (updates: Partial<Room>) => void;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MODEL_LABELS: Record<Agent['model'], string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
  copilot: 'Copilot',
};

const ORCHESTRATION_MODES: { value: Room['orchestrationMode']; label: string }[] = [
  { value: 'manual', label: 'Manual' },
  { value: 'panel', label: 'Panel' },
  { value: 'debate', label: 'Debate' },
  { value: 'round_robin', label: 'Round-Robin' },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function RoomMembers({
  room,
  members,
  allAgents,
  onAddAgent,
  onRemoveAgent,
  onUpdateRoom,
}: RoomMembersProps) {
  // ---- State ----
  const [isEditingTitle, setIsEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState(room.title);
  const [showAgentPicker, setShowAgentPicker] = useState(false);
  const [showMemberList, setShowMemberList] = useState(false);
  const [hoveredAgentId, setHoveredAgentId] = useState<string | null>(null);

  const titleInputRef = useRef<HTMLInputElement>(null);
  const pickerRef = useRef<HTMLDivElement>(null);

  // ---- Sync title draft when room changes ----
  useEffect(() => {
    setTitleDraft(room.title);
  }, [room.title]);

  // ---- Focus title input when editing starts ----
  useEffect(() => {
    if (isEditingTitle) {
      titleInputRef.current?.focus();
      titleInputRef.current?.select();
    }
  }, [isEditingTitle]);

  // ---- Close agent picker on outside click ----
  useEffect(() => {
    if (!showAgentPicker) return;

    function handleClickOutside(e: MouseEvent) {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node)) {
        setShowAgentPicker(false);
      }
    }

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showAgentPicker]);

  // ---- Derived data ----
  const memberIds = new Set(members.map((a) => a.id));
  const availableAgents = allAgents.filter((a) => !memberIds.has(a.id));

  // ---- Handlers ----
  const commitTitle = useCallback(() => {
    setIsEditingTitle(false);
    const trimmed = titleDraft.trim();
    if (trimmed && trimmed !== room.title) {
      onUpdateRoom({ title: trimmed });
    } else {
      setTitleDraft(room.title);
    }
  }, [titleDraft, room.title, onUpdateRoom]);

  const handleTitleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        commitTitle();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        setTitleDraft(room.title);
        setIsEditingTitle(false);
      }
    },
    [commitTitle, room.title],
  );

  const handleAddAgent = useCallback(
    (agentId: string) => {
      onAddAgent(agentId);
      setShowAgentPicker(false);
    },
    [onAddAgent],
  );

  return (
    <div className="select-none border-b border-gray-800 bg-gray-900">
      {/* ===== Room Header Bar ===== */}
      <div className="flex items-center gap-3 px-4 py-2">
        {/* Room title — click to edit */}
        {isEditingTitle ? (
          <input
            ref={titleInputRef}
            value={titleDraft}
            onChange={(e) => setTitleDraft(e.target.value)}
            onBlur={commitTitle}
            onKeyDown={handleTitleKeyDown}
            className="min-w-0 flex-shrink-0 rounded bg-gray-800 px-2 py-0.5 text-sm font-semibold text-white outline-none ring-1 ring-indigo-500"
          />
        ) : (
          <button
            onClick={() => setIsEditingTitle(true)}
            className="truncate text-sm font-semibold text-white hover:text-indigo-400 transition-colors"
            title="Click to rename"
          >
            {room.title}
          </button>
        )}

        {/* Orchestration mode selector */}
        <select
          value={room.orchestrationMode}
          onChange={(e) =>
            onUpdateRoom({
              orchestrationMode: e.target.value as Room['orchestrationMode'],
            })
          }
          className="shrink-0 cursor-pointer rounded bg-gray-800 px-2 py-1 text-xs text-gray-300 outline-none ring-1 ring-gray-700 hover:ring-gray-600 focus:ring-indigo-500 transition-colors"
        >
          {ORCHESTRATION_MODES.map((mode) => (
            <option key={mode.value} value={mode.value}>
              {mode.label}
            </option>
          ))}
        </select>

        {/* Spacer */}
        <div className="flex-1" />

        {/* Member avatars */}
        <div className="flex items-center -space-x-1">
          {members.map((agent) => (
            <span
              key={agent.id}
              title={agent.name}
              className="inline-block h-6 w-6 rounded-full border-2 border-gray-900"
              style={{ backgroundColor: agent.color }}
            />
          ))}
        </div>

        {/* + Add Agent button */}
        <div className="relative" ref={pickerRef}>
          <button
            onClick={() => setShowAgentPicker((v) => !v)}
            className="flex items-center gap-1 rounded bg-gray-800 px-2 py-1 text-xs font-medium text-gray-400 ring-1 ring-gray-700 transition-colors hover:text-white hover:ring-gray-600"
          >
            <svg
              className="h-3 w-3"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
            </svg>
            Add Agent
          </button>

          {/* ---- Agent Picker Dropdown ---- */}
          {showAgentPicker && (
            <div className="absolute right-0 top-full z-50 mt-1 w-64 overflow-hidden rounded-lg border border-gray-700 bg-gray-800 shadow-lg">
              {availableAgents.length === 0 ? (
                <div className="px-3 py-3 text-center text-xs text-gray-500">
                  All agents are already in this room
                </div>
              ) : (
                <ul className="max-h-56 overflow-y-auto py-1">
                  {availableAgents.map((agent) => (
                    <li key={agent.id}>
                      <button
                        onClick={() => handleAddAgent(agent.id)}
                        className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-gray-300 transition-colors hover:bg-gray-700 hover:text-white"
                      >
                        <span
                          className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
                          style={{ backgroundColor: agent.color }}
                        />
                        <span className="truncate font-medium">{agent.name}</span>
                        <span className="ml-auto shrink-0 rounded bg-gray-700 px-1.5 py-0.5 text-[10px] leading-none font-medium text-gray-400">
                          {MODEL_LABELS[agent.model]}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>

        {/* Toggle member list */}
        <button
          onClick={() => setShowMemberList((v) => !v)}
          className="flex items-center gap-1 text-xs text-gray-500 transition-colors hover:text-gray-300"
          title={showMemberList ? 'Collapse member list' : 'Expand member list'}
        >
          <span>{members.length} member{members.length !== 1 ? 's' : ''}</span>
          <svg
            className={`h-3 w-3 transition-transform ${showMemberList ? 'rotate-180' : ''}`}
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      </div>

      {/* ===== Expandable Member List ===== */}
      {showMemberList && (
        <div className="border-t border-gray-800 px-4 py-2">
          <ul className="space-y-1">
            {members.map((agent) => (
              <li
                key={agent.id}
                className="group relative flex items-center gap-2 rounded px-2 py-1 transition-colors hover:bg-gray-800"
                onMouseEnter={() => setHoveredAgentId(agent.id)}
                onMouseLeave={() => setHoveredAgentId(null)}
              >
                {/* Color dot */}
                <span
                  className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
                  style={{ backgroundColor: agent.color }}
                />

                {/* Name */}
                <span className="truncate text-sm font-medium text-gray-300">
                  {agent.name}
                </span>

                {/* Model badge */}
                <span className="shrink-0 rounded bg-gray-700 px-1.5 py-0.5 text-[10px] leading-none font-medium text-gray-400">
                  {MODEL_LABELS[agent.model]}
                </span>

                {/* Spacer */}
                <div className="flex-1" />

                {/* Remove button */}
                <button
                  onClick={() => onRemoveAgent(agent.id)}
                  className="flex h-5 w-5 shrink-0 items-center justify-center rounded text-gray-600 opacity-0 transition-all hover:bg-red-900/40 hover:text-red-400 group-hover:opacity-100"
                  title={`Remove ${agent.name}`}
                >
                  <svg
                    className="h-3 w-3"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={2}
                  >
                    <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>

                {/* System prompt tooltip on hover */}
                {hoveredAgentId === agent.id && agent.systemPrompt && (
                  <div className="absolute bottom-full left-0 z-50 mb-2 max-w-xs rounded-lg border border-gray-700 bg-gray-850 bg-gray-950 px-3 py-2 shadow-lg">
                    <div className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
                      System Prompt
                    </div>
                    <p className="line-clamp-4 text-xs leading-relaxed text-gray-400">
                      {agent.systemPrompt}
                    </p>
                  </div>
                )}
              </li>
            ))}

            {members.length === 0 && (
              <li className="py-2 text-center text-xs text-gray-600">
                No agents in this room yet
              </li>
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
