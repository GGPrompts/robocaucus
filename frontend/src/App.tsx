import { useState, useEffect, useRef, useCallback } from 'react';
import { Code2 } from 'lucide-react';
import Sidebar from './components/Sidebar';
import ChatMessage from './components/ChatMessage';
import ChatInput from './components/ChatInput';
import AgentBuilder from './components/AgentBuilder';
import PlaybookBrowser from './components/PlaybookBrowser';
import RoomMembers from './components/RoomMembers';
import { DevSidebar } from './components/DevSidebar';
import { ThemeSelector } from './components/ThemeSelector';
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
import type { Room, Agent } from './types';

// TODO: [code-review] localStorage.getItem() can throw in private/incognito — wrap in try/catch (85%)
function getInitialTheme(): ThemeId {
  const stored = localStorage.getItem('robocaucus-theme');
  if (stored && themes.some((t) => t.id === stored)) return stored as ThemeId;
  return 'noir';
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
  const { messages, streamingMessage, sendMessage, isStreaming, error } = useChat({
    conversationId: room.id,
  });

  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingMessage]);

  const agentMap = Object.fromEntries(allAgents.map((a) => [a.id, a]));

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
  const [selectedRoom, setSelectedRoom] = useState<Room | null>(null);
  const [roomMembers, setRoomMembers] = useState<Agent[]>([]);
  const [showAgentBuilder, setShowAgentBuilder] = useState(false);
  const [showPlaybooks, setShowPlaybooks] = useState(false);
  const [showDevSidebar, setShowDevSidebar] = useState(false);
  const [theme, setTheme] = useState<ThemeId>(getInitialTheme);
  const [defaultWorkspace, setDefaultWorkspace] = useState('');

  function handleThemeChange(newTheme: ThemeId) {
    setTheme(newTheme);
    localStorage.setItem('robocaucus-theme', newTheme);
  }

  const themeClassName = themes.find((t) => t.id === theme)?.className ?? '';

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
    if (!selectedRoom) {
      setRoomMembers([]);
      return;
    }
    fetchConversationAgents(selectedRoom.id)
      .then(setRoomMembers)
      .catch(() => setRoomMembers([]));
  }, [selectedRoom?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleSelectRoom = useCallback((room: Room) => {
    setSelectedRoom(room);
  }, []);

  async function handleCreateRoom() {
    try {
      // Include all current agents in the new conversation
      const agentIds = agents.map((a) => a.id);
      const room = await createConversation('New Chat', agentIds);
      setRooms((prev) => [{ ...room, lastMessage: '', unread: false }, ...prev]);
      setSelectedRoom(room);
    } catch (e) {
      console.error('Failed to create conversation', e);
    }
  }

  async function handleRunPlaybook(conversationId: string) {
    setShowPlaybooks(false);
    // Refresh conversations and select the new one
    try {
      const convs = await fetchConversations();
      setRooms(convs.map((r) => ({ ...r, lastMessage: '', unread: false })));
      const newRoom = convs.find((c) => c.id === conversationId);
      if (newRoom) setSelectedRoom(newRoom);
      // Refresh agents since playbook run may have created new ones
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
        // Re-fetch members to stay in sync with backend
        const members = await fetchConversationAgents(selectedRoom.id);
        setRoomMembers(members);
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
        // Re-fetch members to stay in sync with backend
        const members = await fetchConversationAgents(selectedRoom.id);
        setRoomMembers(members);
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
        setSelectedRoom(updated);
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
        if (selectedRoom?.id === roomId) {
          setSelectedRoom(null);
        }
      } catch (e) {
        console.error('Failed to delete conversation', e);
      }
    },
    [selectedRoom],
  );

  return (
    <div className={`flex h-screen overflow-hidden ${themeClassName}`}>
      <Sidebar
        rooms={rooms}
        agents={agents}
        selectedRoomId={selectedRoom?.id}
        onSelectRoom={handleSelectRoom}
        onDeleteRoom={handleDeleteRoom}
        onCreateRoom={handleCreateRoom}
        onCreateAgent={() => setShowAgentBuilder(true)}
        onOpenPlaybooks={() => setShowPlaybooks(true)}
      />
      {selectedRoom ? (
        <ChatPanel
          room={selectedRoom}
          members={roomMembers}
          allAgents={agents}
          theme={theme}
          onThemeChange={handleThemeChange}
          showDevSidebar={showDevSidebar}
          onToggleDevSidebar={() => setShowDevSidebar((v) => !v)}
          onAddAgent={handleAddAgent}
          onRemoveAgent={handleRemoveAgent}
          onUpdateRoom={handleUpdateRoom}
        />
      ) : (
        <EmptyState onCreateRoom={handleCreateRoom} theme={theme} onThemeChange={handleThemeChange} />
      )}
      {showDevSidebar && selectedRoom && (
        <DevSidebar
          workspacePath={selectedRoom.workspacePath || defaultWorkspace}
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
