import { useState } from 'react';
import type { Room, Agent } from '../types';

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const MOCK_ROOMS: (Room & { lastMessage: string; unread: boolean })[] = [
  {
    id: 'room-1',
    title: '#brainstorm',
    orchestrationMode: 'panel',
    createdAt: '2026-03-20T10:00:00Z',
    updatedAt: '2026-03-24T09:15:00Z',
    lastMessage: 'What if we tried a graph-based approach?',
    unread: true,
  },
  {
    id: 'room-2',
    title: '#research',
    orchestrationMode: 'round_robin',
    createdAt: '2026-03-21T14:30:00Z',
    updatedAt: '2026-03-24T08:45:00Z',
    lastMessage: 'Found three relevant papers on RAG pipelines',
    unread: false,
  },
  {
    id: 'room-3',
    title: '#api-design',
    orchestrationMode: 'debate',
    createdAt: '2026-03-22T09:00:00Z',
    updatedAt: '2026-03-23T17:30:00Z',
    lastMessage: 'REST vs GraphQL — let me outline the trade-offs',
    unread: true,
  },
];

const MOCK_AGENTS: Agent[] = [
  {
    id: 'agent-1',
    name: 'Editor',
    model: 'sonnet',
    provider: 'claude',
    color: '#a855f7', // purple
    scope: 'global',
    systemPrompt: '',
  },
  {
    id: 'agent-2',
    name: 'Researcher',
    model: 'gemini-2.5-pro',
    provider: 'gemini',
    color: '#22c55e', // green
    scope: 'global',
    systemPrompt: '',
  },
  {
    id: 'agent-3',
    name: 'Critic',
    model: 'gpt-4o',
    provider: 'copilot',
    color: '#14b8a6', // teal
    scope: 'workspace',
    systemPrompt: '',
  },
];

// ---------------------------------------------------------------------------
// Activity-bar mode type
// ---------------------------------------------------------------------------

type ActivityMode = 'chat' | 'files' | 'git';

// ---------------------------------------------------------------------------
// Model badge labels
// ---------------------------------------------------------------------------

const PROVIDER_LABELS: Record<Agent['provider'], string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
  copilot: 'Copilot',
};

// ---------------------------------------------------------------------------
// Component props
// ---------------------------------------------------------------------------

export interface SidebarProps {
  rooms?: (Room & { lastMessage: string; unread: boolean })[];
  agents?: Agent[];
  selectedRoomId?: string;
  onSelectRoom?: (room: Room) => void;
  onDeleteRoom?: (roomId: string) => void;
  onSelectAgent?: (agent: Agent) => void;
  onCreateRoom?: () => void;
  onCreateAgent?: () => void;
  onOpenPlaybooks?: () => void;
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

export default function Sidebar({
  rooms: propRooms,
  agents: propAgents,
  selectedRoomId,
  onSelectRoom,
  onDeleteRoom,
  onSelectAgent,
  onCreateRoom,
  onCreateAgent,
  onOpenPlaybooks,
}: SidebarProps) {
  const displayRooms = propRooms ?? MOCK_ROOMS;
  const displayAgents = propAgents ?? MOCK_AGENTS;
  const [activeMode, setActiveMode] = useState<ActivityMode>('chat');
  const [activeRoomId, setActiveRoomId] = useState<string>('');
  const effectiveActiveId = selectedRoomId !== undefined ? selectedRoomId : activeRoomId;

  // ---- handlers ----------------------------------------------------------

  function handleSelectRoom(room: Room) {
    setActiveRoomId(room.id);
    onSelectRoom?.(room);
  }

  // ---- render helpers ----------------------------------------------------

  const activityButtons: { mode: ActivityMode; icon: string; label: string }[] = [
    { mode: 'chat', icon: '\u{1F4AC}', label: 'Chat' },
    { mode: 'files', icon: '\u{1F4C1}', label: 'Files' },
    { mode: 'git', icon: '\u{1F500}', label: 'Git' },
  ];

  return (
    <aside className="flex h-screen select-none text-sm text-gray-300">
      {/* ===== Activity bar ===== */}
      <div className="flex w-12 shrink-0 flex-col items-center gap-1 bg-gray-950 pt-3">
        {activityButtons.map((btn) => (
          <button
            key={btn.mode}
            title={btn.label}
            onClick={() => setActiveMode(btn.mode)}
            className={`flex h-10 w-10 items-center justify-center rounded-lg text-lg transition-colors ${
              activeMode === btn.mode
                ? 'bg-gray-700 text-white'
                : 'text-gray-500 hover:bg-gray-800 hover:text-gray-300'
            }`}
          >
            {btn.icon}
          </button>
        ))}
      </div>

      {/* ===== Rooms / Agents panel ===== */}
      <div className="flex w-60 flex-col overflow-y-auto bg-gray-900 border-r border-gray-800">
        {/* Workspace selector */}
        <div className="flex h-12 items-center justify-between border-b border-gray-800 px-3">
          <span className="truncate font-semibold text-white">My Projects</span>
          <svg
            className="h-4 w-4 text-gray-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path strokeLinecap="round" strokeLinejoin="round" d="M19 9l-7 7-7-7" />
          </svg>
        </div>

        {activeMode !== 'chat' ? (
          <div className="flex flex-1 items-center justify-center p-4 text-xs text-gray-500">
            Coming soon
          </div>
        ) : (<>{/* ---- Chats section ---- */}
        <div className="px-2 pt-4">
          <div className="flex items-center justify-between px-1 pb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-gray-500">
              Chats
            </span>
            <button
              onClick={onCreateRoom}
              className="flex h-5 w-5 items-center justify-center rounded text-gray-500 hover:bg-gray-800 hover:text-gray-300"
              title="New room"
            >
              <svg
                className="h-3.5 w-3.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
              </svg>
            </button>
          </div>

          <ul className="space-y-0.5">
            {displayRooms.map((room) => {
              const isActive = room.id === effectiveActiveId;
              return (
                <li key={room.id} className="group/room relative">
                  <button
                    onClick={() => handleSelectRoom(room)}
                    className={`flex w-full flex-col rounded px-2 py-1.5 text-left transition-colors ${
                      isActive
                        ? 'bg-gray-800 text-white'
                        : 'hover:bg-gray-800/60 hover:text-white'
                    }`}
                  >
                    <span className="flex items-center gap-1.5">
                      <span className="truncate font-medium">{room.title}</span>
                      {room.unread && (
                        <span className="ml-auto h-2 w-2 shrink-0 rounded-full bg-indigo-500" />
                      )}
                    </span>
                    <span className="truncate text-xs text-gray-500 group-hover/room:text-gray-400">
                      {room.lastMessage}
                    </span>
                  </button>
                  {onDeleteRoom && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteRoom(room.id);
                      }}
                      className="absolute right-1 top-1/2 -translate-y-1/2 hidden rounded p-1 text-gray-500 hover:bg-gray-700 hover:text-red-400 group-hover/room:flex items-center justify-center"
                      title="Delete conversation"
                    >
                      <svg
                        className="h-3.5 w-3.5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={2}
                      >
                        <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        </div>

        {/* ---- Divider ---- */}
        <div className="mx-3 my-3 border-t border-gray-800" />

        {/* ---- Playbooks section ---- */}
        <div className="px-2">
          <button
            onClick={onOpenPlaybooks}
            className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left transition-colors hover:bg-gray-800/60 hover:text-white"
          >
            <svg
              className="h-4 w-4 shrink-0 text-gray-500"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.746 0 3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
            </svg>
            <span className="text-[11px] font-semibold uppercase tracking-wider text-gray-500">
              Playbooks
            </span>
          </button>
        </div>

        {/* ---- Divider ---- */}
        <div className="mx-3 my-3 border-t border-gray-800" />

        {/* ---- Agents section ---- */}
        <div className="px-2">
          <div className="flex items-center justify-between px-1 pb-1">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-gray-500">
              Agents
            </span>
            <button
              onClick={onCreateAgent}
              className="flex h-5 w-5 items-center justify-center rounded text-gray-500 hover:bg-gray-800 hover:text-gray-300"
              title="Add agent"
            >
              <svg
                className="h-3.5 w-3.5"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 4v16m8-8H4" />
              </svg>
            </button>
          </div>

          <ul className="space-y-0.5">
            {displayAgents.map((agent) => (
              <li key={agent.id}>
                <button
                  onClick={() => onSelectAgent?.(agent)}
                  className="flex w-full items-center gap-2 rounded px-2 py-1.5 text-left transition-colors hover:bg-gray-800/60 hover:text-white"
                >
                  {/* Color dot */}
                  <span
                    className="h-2.5 w-2.5 shrink-0 rounded-full"
                    style={{ backgroundColor: agent.color }}
                  />
                  <span className="truncate font-medium">{agent.name}</span>
                  <span className="ml-auto shrink-0 rounded bg-gray-800 px-1.5 py-0.5 text-[10px] font-medium text-gray-400">
                    {PROVIDER_LABELS[agent.provider]}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </div>
        </>)}
      </div>
    </aside>
  );
}
