import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { Code2, Users, Swords, FileCode } from 'lucide-react';
import Sidebar from './components/Sidebar';
import ChatMessage from './components/ChatMessage';
import ChatInput from './components/ChatInput';
import AgentBuilder from './components/AgentBuilder';
import PlaybookBrowser from './components/PlaybookBrowser';
import RoomMembers from './components/RoomMembers';
import { DevSidebar } from './components/DevSidebar';
import { ThemeSelector } from './components/ThemeSelector';
import { TabBar } from './components/TabBar';
import { CodeViewer } from './components/CodeViewer';
import { useChat } from './hooks/useChat';
import {
  fetchConversations,
  fetchAgents,
  createConversation,
  createAgent,
  fetchConfig,
  fetchConversationAgents,
  addAgentToConversation,
  removeAgentFromConversation,
  updateConversation,
  deleteConversation,
} from './lib/api';
import { themes, type ThemeId } from './themes';
import type { Room, Agent, EditorTab } from './types';

// TODO: [code-review] localStorage.getItem() can throw in private/incognito — wrap in try/catch (85%)
function getInitialTheme(): ThemeId {
  try {
    const stored = localStorage.getItem('robocaucus-theme');
    if (stored && themes.some((t) => t.id === stored)) return stored as ThemeId;
  } catch { /* ignore */ }
  return 'noir';
}

const RECENT_WORKSPACES_KEY = 'robocaucus-recent-workspaces';
const ACTIVE_WORKSPACE_KEY = 'robocaucus-active-workspace';

function loadRecentWorkspaces(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_WORKSPACES_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed.filter((x): x is string => typeof x === 'string');
    }
  } catch { /* ignore */ }
  return [];
}

function saveRecentWorkspaces(paths: string[]) {
  try {
    localStorage.setItem(RECENT_WORKSPACES_KEY, JSON.stringify(paths));
  } catch { /* ignore */ }
}

function loadActiveWorkspace(): string | null {
  try {
    return localStorage.getItem(ACTIVE_WORKSPACE_KEY);
  } catch { /* ignore */ }
  return null;
}

function saveActiveWorkspace(path: string) {
  try {
    localStorage.setItem(ACTIVE_WORKSPACE_KEY, path);
  } catch { /* ignore */ }
}

type RoomWithMeta = Room & { lastMessage: string; unread: boolean };

// ---------------------------------------------------------------------------
// Chat panel
// ---------------------------------------------------------------------------

interface ChatPanelProps {
  room: Room;
  members: Agent[];
  allAgents: Agent[];
  theme: ThemeId;
  onThemeChange: (t: ThemeId) => void;
  showDevSidebar: boolean;
  onToggleDevSidebar: () => void;
  onAddAgent: (agentId: string) => void;
  onRemoveAgent: (agentId: string) => void;
  onUpdateRoom: (updates: Partial<Room>) => void;
}

function ChatPanel({
  room,
  members,
  allAgents,
  theme,
  onThemeChange,
  showDevSidebar,
  onToggleDevSidebar,
  onAddAgent,
  onRemoveAgent,
  onUpdateRoom,
}: ChatPanelProps) {
  const {
    messages,
    streamingMessage,
    sendMessage,
    startPanelStream,
    startDebateStream,
    isStreaming,
    error,
  } = useChat({
    conversationId: room.id,
  });

  const [showDebateInput, setShowDebateInput] = useState(false);
  const [debateTopic, setDebateTopic] = useState('');
  const debateInputRef = useRef<HTMLInputElement>(null);

  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingMessage]);

  useEffect(() => {
    if (showDebateInput) {
      debateInputRef.current?.focus();
    }
  }, [showDebateInput]);

  const agentMap = Object.fromEntries(allAgents.map((a) => [a.id, a]));

  const handleAskEveryone = useCallback(() => {
    const question = window.prompt('Ask all agents:');
    if (question?.trim()) {
      startPanelStream(question.trim());
    }
  }, [startPanelStream]);

  const handleStartDebate = useCallback(() => {
    if (!debateTopic.trim()) return;
    startDebateStream(debateTopic.trim());
    setDebateTopic('');
    setShowDebateInput(false);
  }, [debateTopic, startDebateStream]);

  return (
    <div className="flex flex-1 flex-col overflow-hidden bg-[var(--bg-primary)]">
      {/* Room Members header (title, orchestration, add/remove agents) */}
      <RoomMembers
        room={room}
        members={members}
        allAgents={allAgents}
        onAddAgent={onAddAgent}
        onRemoveAgent={onRemoveAgent}
        onUpdateRoom={onUpdateRoom}
      />

      {/* Toolbar row */}
      <div className="flex h-10 shrink-0 items-center border-b border-[var(--border-primary)] px-4 gap-3">
        {/* Orchestration buttons */}
        {members.length >= 1 && (
          <button
            onClick={handleAskEveryone}
            disabled={isStreaming || members.length < 1}
            className="flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition-colors bg-[var(--bg-secondary)] text-[var(--text-secondary)] ring-1 ring-[var(--border-secondary)] hover:text-[var(--text-primary)] hover:ring-[var(--accent)] disabled:opacity-40 disabled:cursor-not-allowed"
            title="Send the same prompt to all agents in this conversation"
          >
            <Users size={14} />
            Ask Everyone
          </button>
        )}
        {members.length >= 2 && (
          <button
            onClick={() => setShowDebateInput((v) => !v)}
            disabled={isStreaming}
            className={`flex items-center gap-1.5 rounded px-2.5 py-1 text-xs font-medium transition-colors ring-1 disabled:opacity-40 disabled:cursor-not-allowed ${
              showDebateInput
                ? 'bg-[var(--accent)] text-[var(--text-primary)] ring-[var(--accent)]'
                : 'bg-[var(--bg-secondary)] text-[var(--text-secondary)] ring-[var(--border-secondary)] hover:text-[var(--text-primary)] hover:ring-[var(--accent)]'
            }`}
            title="Start a structured debate between agents (requires 2+ agents)"
          >
            <Swords size={14} />
            Start Debate
          </button>
        )}

        <div className="flex-1" />
        <button
          onClick={onToggleDevSidebar}
          className={`rounded p-1.5 transition-colors ${
            showDevSidebar
              ? 'bg-[var(--accent)] text-[var(--text-primary)]'
              : 'text-[var(--text-muted)] hover:bg-[var(--bg-secondary)] hover:text-[var(--text-primary)]'
          }`}
          title="Toggle developer sidebar"
        >
          <Code2 size={16} />
        </button>
        <ThemeSelector currentTheme={theme} onThemeChange={onThemeChange} />
      </div>

      {/* Debate topic input row (shown when Start Debate is clicked) */}
      {showDebateInput && (
        <div className="flex shrink-0 items-center gap-2 border-b border-[var(--border-primary)] bg-[var(--bg-secondary)] px-4 py-2">
          <label className="text-xs font-medium text-[var(--text-muted)] whitespace-nowrap">
            Debate topic:
          </label>
          <input
            ref={debateInputRef}
            value={debateTopic}
            onChange={(e) => setDebateTopic(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                handleStartDebate();
              } else if (e.key === 'Escape') {
                setShowDebateInput(false);
                setDebateTopic('');
              }
            }}
            placeholder="e.g. Should we use microservices or a monolith?"
            className="flex-1 rounded bg-[var(--bg-primary)] px-2.5 py-1 text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] outline-none ring-1 ring-[var(--border-secondary)] focus:ring-[var(--accent)]"
          />
          <button
            onClick={handleStartDebate}
            disabled={!debateTopic.trim() || isStreaming}
            className="rounded bg-[var(--accent)] px-3 py-1 text-xs font-medium text-[var(--text-primary)] hover:bg-[var(--accent-hover)] disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Go
          </button>
          <button
            onClick={() => {
              setShowDebateInput(false);
              setDebateTopic('');
            }}
            className="rounded px-2 py-1 text-xs text-[var(--text-muted)] hover:text-[var(--text-secondary)]"
          >
            Cancel
          </button>
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto py-4">
        {messages.length === 0 && !streamingMessage && (
          <div className="flex h-full items-center justify-center text-sm text-[var(--text-muted)]">
            {members.length === 0
              ? 'No agents in this conversation. Add an agent to start chatting.'
              : 'No messages yet. Start the conversation.'}
          </div>
        )}
        {messages.map((msg) => (
          <ChatMessage
            key={msg.id}
            message={msg}
            agent={msg.agentId ? agentMap[msg.agentId] : undefined}
          />
        ))}
        {streamingMessage && (
          <ChatMessage
            message={streamingMessage}
            agent={streamingMessage.agentId ? agentMap[streamingMessage.agentId] : undefined}
          />
        )}
        {error && (
          <div className="px-4 py-2 text-xs text-red-400">{error}</div>
        )}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <div className="shrink-0 border-t border-[var(--border-primary)] p-4">
        <ChatInput
          agents={members}
          onSend={(text, mentionedAgentIds) => {
            // Use explicitly @mentioned agent, or fall back to first member in conversation
            const agentId = mentionedAgentIds[0] ?? members[0]?.id;
            sendMessage(text, agentId);
          }}
          isSending={isStreaming}
          placeholder={
            members.length === 0
              ? 'Add agents to this conversation to start chatting...'
              : members.length === 1
              ? `Message ${members[0].name}...`
              : 'Message the group... (use @mention to target a specific agent)'
          }
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

function EmptyState({ onCreateRoom, theme, onThemeChange }: { onCreateRoom: () => void; theme: ThemeId; onThemeChange: (t: ThemeId) => void }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center bg-[var(--bg-primary)] text-[var(--text-muted)]">
      <div className="absolute top-3 right-4">
        <ThemeSelector currentTheme={theme} onThemeChange={onThemeChange} />
      </div>
      <p className="mb-4 text-sm">Select a conversation or start a new one</p>
      <button
        onClick={onCreateRoom}
        className="rounded-lg bg-[var(--accent)] px-4 py-2 text-sm font-medium text-[var(--text-primary)] hover:bg-[var(--accent-hover)]"
      >
        New Conversation
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  const [rooms, setRooms] = useState<RoomWithMeta[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [roomMembersMap, setRoomMembersMap] = useState<Record<string, Agent[]>>({});
  const [showAgentBuilder, setShowAgentBuilder] = useState(false);
  const [showPlaybooks, setShowPlaybooks] = useState(false);
  const [showDevSidebar, setShowDevSidebar] = useState(false);
  const [theme, setTheme] = useState<ThemeId>(getInitialTheme);
  const [defaultWorkspace, setDefaultWorkspace] = useState('');
  const [activeWorkspace, setActiveWorkspace] = useState<string | null>(loadActiveWorkspace);
  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>(loadRecentWorkspaces);

  // ---- Tab state -----------------------------------------------------------
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? null,
    [tabs, activeTabId],
  );

  // Derive selectedRoom from the active chat tab
  const selectedRoom = useMemo(() => {
    if (!activeTab || activeTab.type !== 'chat') return null;
    return rooms.find((r) => r.id === activeTab.roomId) ?? null;
  }, [activeTab, rooms]);

  // ---- Tab helpers ---------------------------------------------------------

  const openChatTab = useCallback((room: Room) => {
    const tabId = `chat-${room.id}`;
    setTabs((prev) => {
      if (prev.some((t) => t.id === tabId)) return prev;
      return [...prev, { id: tabId, type: 'chat', title: room.title, roomId: room.id }];
    });
    setActiveTabId(tabId);
  }, []);

  // Exposed for future use (e.g. DevSidebar file clicks, search results)
  const openFileTab = useCallback((filePath: string) => {
    const tabId = `file-${filePath}`;
    const fileName = filePath.split('/').pop() ?? filePath;
    setTabs((prev) => {
      if (prev.some((t) => t.id === tabId)) return prev;
      return [...prev, { id: tabId, type: 'file', title: fileName, filePath }];
    });
    setActiveTabId(tabId);
  }, []);

  // Make openFileTab available on the window for dev tools / external callers
  useEffect(() => {
    (window as unknown as Record<string, unknown>).__openFileTab = openFileTab;
    return () => {
      delete (window as unknown as Record<string, unknown>).__openFileTab;
    };
  }, [openFileTab]);

  const handleCloseTab = useCallback((tabId: string) => {
    setTabs((prev) => {
      const idx = prev.findIndex((t) => t.id === tabId);
      const next = prev.filter((t) => t.id !== tabId);
      // If we closed the active tab, activate the nearest remaining tab
      setActiveTabId((currentActive) => {
        if (currentActive !== tabId) return currentActive;
        if (next.length === 0) return null;
        // Prefer the tab to the left, else the one that slid into this index
        const newIdx = Math.min(idx, next.length - 1);
        return next[newIdx].id;
      });
      return next;
    });
  }, []);

  // Derive display tabs with up-to-date room titles (avoids setState-in-effect)
  const displayTabs = useMemo(
    () =>
      tabs.map((tab) => {
        if (tab.type !== 'chat' || !tab.roomId) return tab;
        const room = rooms.find((r) => r.id === tab.roomId);
        if (room && room.title !== tab.title) {
          return { ...tab, title: room.title };
        }
        return tab;
      }),
    [tabs, rooms],
  );

  // ---- Theme ---------------------------------------------------------------

  function handleThemeChange(newTheme: ThemeId) {
    setTheme(newTheme);
    try {
      localStorage.setItem('robocaucus-theme', newTheme);
    } catch { /* ignore — private/incognito may block storage */ }
  }

  const themeClassName = themes.find((t) => t.id === theme)?.className ?? '';

  // ---- Workspace management ------------------------------------------------

  // The effective workspace: user-selected > default from backend config
  const effectiveWorkspace = activeWorkspace || defaultWorkspace;

  // Build the deduplicated recent-workspaces list for the dropdown.
  // Always includes the default workspace (from backend config) as the first
  // entry so the user can easily switch back to it.
  const allRecentWorkspaces = useMemo(() => {
    const combined: string[] = [];
    if (defaultWorkspace) combined.push(defaultWorkspace);
    for (const ws of recentWorkspaces) {
      if (ws && !combined.includes(ws)) combined.push(ws);
    }
    return combined;
  }, [defaultWorkspace, recentWorkspaces]);

  const handleWorkspaceChange = useCallback(
    (path: string) => {
      setActiveWorkspace(path);
      saveActiveWorkspace(path);
      // Add to recent list (deduplicated, most recent first, cap at 10)
      setRecentWorkspaces((prev) => {
        const next = [path, ...prev.filter((w) => w !== path)].slice(0, 10);
        saveRecentWorkspaces(next);
        return next;
      });
    },
    [],
  );

  // When defaultWorkspace arrives from the backend and no active workspace
  // has been chosen yet, seed the active workspace with the default.
  useEffect(() => {
    if (defaultWorkspace && !activeWorkspace) {
      setActiveWorkspace(defaultWorkspace);
    }
  }, [defaultWorkspace]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Data fetching -------------------------------------------------------

  useEffect(() => {
    fetchConversations()
      .then((convs) => setRooms(convs.map((r) => ({ ...r, lastMessage: '', unread: false }))))
      .catch(() => {});
    fetchAgents().then(setAgents).catch(() => {});
    fetchConfig()
      .then((cfg) => setDefaultWorkspace(cfg.default_workspace))
      .catch(() => {});
  }, []);

  // Fetch conversation-specific agents when the selected room changes
  useEffect(() => {
    if (!selectedRoom) return;
    // Skip fetch if we already have members cached for this room
    if (roomMembersMap[selectedRoom.id]) return;
    fetchConversationAgents(selectedRoom.id)
      .then((members) =>
        setRoomMembersMap((prev) => ({ ...prev, [selectedRoom.id]: members })),
      )
      .catch(() =>
        setRoomMembersMap((prev) => ({ ...prev, [selectedRoom.id]: [] })),
      );
  }, [selectedRoom?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Room selection (from sidebar) opens/focuses a tab -------------------

  const handleSelectRoom = useCallback(
    (room: Room) => {
      openChatTab(room);
    },
    [openChatTab],
  );

  async function handleCreateRoom() {
    try {
      const agentIds = agents.map((a) => a.id);
      const room = await createConversation('New Chat', agentIds);
      setRooms((prev) => [{ ...room, lastMessage: '', unread: false }, ...prev]);
      openChatTab(room);
    } catch (e) {
      console.error('Failed to create conversation', e);
    }
  }

  async function handleRunPlaybook(conversationId: string) {
    setShowPlaybooks(false);
    try {
      const convs = await fetchConversations();
      setRooms(convs.map((r) => ({ ...r, lastMessage: '', unread: false })));
      const newRoom = convs.find((c) => c.id === conversationId);
      if (newRoom) openChatTab(newRoom);
      const updatedAgents = await fetchAgents();
      setAgents(updatedAgents);
    } catch (e) {
      console.error('Failed to refresh after playbook run', e);
    }
  }

  async function handleCreateAgent(data: Omit<Agent, 'id'>) {
    try {
      const agent = await createAgent(
        data.name,
        data.model,
        data.provider,
        data.color,
        data.scope,
        data.systemPrompt,
      );
      setAgents((prev) => [...prev, agent]);
      setShowAgentBuilder(false);
    } catch (e) {
      console.error('Failed to create agent', e);
    }
  }

  const handleAddAgent = useCallback(
    async (agentId: string) => {
      if (!selectedRoom) return;
      try {
        await addAgentToConversation(selectedRoom.id, agentId);
        const members = await fetchConversationAgents(selectedRoom.id);
        setRoomMembersMap((prev) => ({ ...prev, [selectedRoom.id]: members }));
      } catch (e) {
        console.error('Failed to add agent to conversation', e);
      }
    },
    [selectedRoom],
  );

  const handleRemoveAgent = useCallback(
    async (agentId: string) => {
      if (!selectedRoom) return;
      try {
        await removeAgentFromConversation(selectedRoom.id, agentId);
        const members = await fetchConversationAgents(selectedRoom.id);
        setRoomMembersMap((prev) => ({ ...prev, [selectedRoom.id]: members }));
      } catch (e) {
        console.error('Failed to remove agent from conversation', e);
      }
    },
    [selectedRoom],
  );

  const handleUpdateRoom = useCallback(
    async (updates: Partial<Room>) => {
      if (!selectedRoom) return;
      try {
        const payload: { title?: string; orchestration_mode?: string } = {};
        if (updates.title !== undefined) payload.title = updates.title;
        if (updates.orchestrationMode !== undefined)
          payload.orchestration_mode = updates.orchestrationMode;

        const updated = await updateConversation(selectedRoom.id, payload);
        setRooms((prev) =>
          prev.map((r) =>
            r.id === updated.id ? { ...r, ...updated } : r,
          ),
        );
      } catch (e) {
        console.error('Failed to update conversation', e);
      }
    },
    [selectedRoom],
  );

  const handleDeleteRoom = useCallback(
    async (roomId: string) => {
      if (!window.confirm('Delete this conversation? This cannot be undone.')) return;
      try {
        await deleteConversation(roomId);
        setRooms((prev) => prev.filter((r) => r.id !== roomId));
        // Close the tab for the deleted room
        const tabId = `chat-${roomId}`;
        handleCloseTab(tabId);
        // Clean up cached members
        setRoomMembersMap((prev) => {
          const next = { ...prev };
          delete next[roomId];
          return next;
        });
      } catch (e) {
        console.error('Failed to delete conversation', e);
      }
    },
    [handleCloseTab],
  );

  // ---- Collect all open chat tabs' rooms for rendering (hidden/shown) ------

  const openChatRooms = useMemo(() => {
    const chatTabs = tabs.filter((t) => t.type === 'chat' && t.roomId);
    return chatTabs
      .map((t) => ({
        tab: t,
        room: rooms.find((r) => r.id === t.roomId),
      }))
      .filter((entry): entry is { tab: EditorTab; room: RoomWithMeta } => !!entry.room);
  }, [tabs, rooms]);

  // ---- Render --------------------------------------------------------------

  return (
    <div className={`flex h-screen overflow-hidden ${themeClassName}`}>
      <Sidebar
        rooms={rooms}
        agents={agents}
        selectedRoomId={selectedRoom?.id}
        workspacePath={selectedRoom?.workspacePath || effectiveWorkspace}
        recentWorkspaces={allRecentWorkspaces}
        onSelectRoom={handleSelectRoom}
        onDeleteRoom={handleDeleteRoom}
        onCreateRoom={handleCreateRoom}
        onCreateAgent={() => setShowAgentBuilder(true)}
        onOpenPlaybooks={() => setShowPlaybooks(true)}
        onWorkspaceChange={handleWorkspaceChange}
      />

      {/* Main editor area with tab bar */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <TabBar
          tabs={displayTabs}
          activeTabId={activeTabId}
          onSelectTab={setActiveTabId}
          onCloseTab={handleCloseTab}
        />

        {/* Tab content area */}
        {tabs.length === 0 ? (
          <EmptyState onCreateRoom={handleCreateRoom} theme={theme} onThemeChange={handleThemeChange} />
        ) : (
          <div className="relative flex flex-1 overflow-hidden">
            {/* Render all open chat panels (hidden when not active) to preserve state */}
            {openChatRooms.map(({ tab, room }) => (
              <div
                key={tab.id}
                className={`absolute inset-0 flex flex-col ${
                  tab.id === activeTabId ? '' : 'invisible pointer-events-none'
                }`}
              >
                <ChatPanel
                  room={room}
                  members={roomMembersMap[room.id] ?? []}
                  allAgents={agents}
                  theme={theme}
                  onThemeChange={handleThemeChange}
                  showDevSidebar={showDevSidebar}
                  onToggleDevSidebar={() => setShowDevSidebar((v) => !v)}
                  onAddAgent={handleAddAgent}
                  onRemoveAgent={handleRemoveAgent}
                  onUpdateRoom={handleUpdateRoom}
                />
              </div>
            ))}

            {/* File tab content */}
            {activeTab?.type === 'file' && activeTab.filePath && (
              <div className="flex flex-1 flex-col overflow-hidden bg-[var(--bg-primary)]">
                <div className="flex shrink-0 items-center gap-2 border-b border-[var(--border-primary)] px-4 py-2">
                  <FileCode size={14} className="shrink-0 text-[var(--text-muted)]" />
                  <span className="truncate text-xs font-mono text-[var(--text-secondary)]">
                    {activeTab.filePath}
                  </span>
                  <div className="flex-1" />
                  <ThemeSelector currentTheme={theme} onThemeChange={handleThemeChange} />
                </div>
                <div className="flex-1 overflow-auto">
                  <CodeViewer filePath={activeTab.filePath} basePath={effectiveWorkspace} />
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      {showDevSidebar && selectedRoom && (
        <DevSidebar
          workspacePath={selectedRoom.workspacePath || effectiveWorkspace}
          onClose={() => setShowDevSidebar(false)}
        />
      )}
      {showAgentBuilder && (
        <AgentBuilder
          onSave={handleCreateAgent}
          onClose={() => setShowAgentBuilder(false)}
        />
      )}
      {showPlaybooks && (
        <PlaybookBrowser
          onRunPlaybook={handleRunPlaybook}
          onClose={() => setShowPlaybooks(false)}
        />
      )}
    </div>
  );
}
