import { useState, useEffect, useRef } from 'react';
import { Code2 } from 'lucide-react';
import Sidebar from './components/Sidebar';
import ChatMessage from './components/ChatMessage';
import ChatInput from './components/ChatInput';
import AgentBuilder from './components/AgentBuilder';
import PlaybookBrowser from './components/PlaybookBrowser';
import { DevSidebar } from './components/DevSidebar';
import { ThemeSelector } from './components/ThemeSelector';
import { useChat } from './hooks/useChat';
import { fetchConversations, fetchAgents, createConversation, createAgent } from './lib/api';
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

function ChatPanel({ room, agents, theme, onThemeChange, showDevSidebar, onToggleDevSidebar }: { room: Room; agents: Agent[]; theme: ThemeId; onThemeChange: (t: ThemeId) => void; showDevSidebar: boolean; onToggleDevSidebar: () => void }) {
  const { messages, streamingMessage, sendMessage, isStreaming, error } = useChat({
    conversationId: room.id,
  });

  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, streamingMessage]);

  const agentMap = Object.fromEntries(agents.map((a) => [a.id, a]));

  return (
    <div className="flex flex-1 flex-col overflow-hidden bg-gray-900">
      {/* Header */}
      <div className="flex h-12 shrink-0 items-center border-b border-gray-800 px-4 gap-3">
        <span className="font-semibold text-white">{room.title}</span>
        <span className="rounded bg-gray-800 px-2 py-0.5 text-[11px] text-gray-400 capitalize">
          {room.orchestrationMode}
        </span>
        <div className="ml-auto flex items-center gap-3">
          <button
            onClick={onToggleDevSidebar}
            className={`rounded p-1.5 transition-colors ${
              showDevSidebar
                ? 'bg-indigo-600 text-white'
                : 'text-gray-400 hover:bg-gray-800 hover:text-white'
            }`}
            title="Toggle developer sidebar"
          >
            <Code2 size={16} />
          </button>
          <ThemeSelector currentTheme={theme} onThemeChange={onThemeChange} />
          {agents.length > 0 && (
            <div className="flex items-center gap-1.5">
              {agents.map((a) => (
                <span
                  key={a.id}
                  title={a.name}
                  className="inline-flex h-5 items-center gap-1 rounded-full bg-gray-800 px-2 text-[10px] text-gray-400"
                >
                  <span
                    className="inline-block h-1.5 w-1.5 rounded-full"
                    style={{ backgroundColor: a.color }}
                  />
                  {a.name}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto py-4">
        {messages.length === 0 && !streamingMessage && (
          <div className="flex h-full items-center justify-center text-sm text-gray-500">
            {agents.length === 0
              ? 'No agents in this conversation. Create an agent first.'
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
      <div className="shrink-0 border-t border-gray-800 p-4">
        <ChatInput
          agents={agents}
          onSend={(text, mentionedAgentIds) => {
            // Use explicitly @mentioned agent, or fall back to first agent in conversation
            const agentId = mentionedAgentIds[0] ?? agents[0]?.id;
            sendMessage(text, agentId);
          }}
          isSending={isStreaming}
          placeholder={
            agents.length === 0
              ? 'Add agents to this conversation to start chatting...'
              : agents.length === 1
              ? `Message ${agents[0].name}...`
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
    <div className="flex flex-1 flex-col items-center justify-center bg-gray-900 text-gray-500">
      <div className="absolute top-3 right-4">
        <ThemeSelector currentTheme={theme} onThemeChange={onThemeChange} />
      </div>
      <p className="mb-4 text-sm">Select a conversation or start a new one</p>
      <button
        onClick={onCreateRoom}
        className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-500"
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
  const [showAgentBuilder, setShowAgentBuilder] = useState(false);
  const [showPlaybooks, setShowPlaybooks] = useState(false);
  const [showDevSidebar, setShowDevSidebar] = useState(false);
  const [theme, setTheme] = useState<ThemeId>(getInitialTheme);

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

  return (
    <div className={`flex h-screen overflow-hidden ${themeClassName}`}>
      <Sidebar
        rooms={rooms}
        agents={agents}
        selectedRoomId={selectedRoom?.id}
        onSelectRoom={setSelectedRoom}
        onCreateRoom={handleCreateRoom}
        onCreateAgent={() => setShowAgentBuilder(true)}
        onOpenPlaybooks={() => setShowPlaybooks(true)}
      />
      {selectedRoom ? (
        <ChatPanel
          room={selectedRoom}
          agents={agents}
          theme={theme}
          onThemeChange={handleThemeChange}
          showDevSidebar={showDevSidebar}
          onToggleDevSidebar={() => setShowDevSidebar((v) => !v)}
        />
      ) : (
        <EmptyState onCreateRoom={handleCreateRoom} theme={theme} onThemeChange={handleThemeChange} />
      )}
      {showDevSidebar && selectedRoom && (
        <DevSidebar
          workspacePath={selectedRoom.workspacePath ?? ''}
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
