import {
  useState,
  useRef,
  useEffect,
  useCallback,
  type KeyboardEvent,
  type ChangeEvent,
} from 'react';
import type { Agent } from '../types';

interface ChatInputProps {
  agents: Agent[];
  onSend: (text: string, mentionedAgentIds: string[]) => void;
  isSending?: boolean;
  placeholder?: string;
}

const MODEL_BADGES: Record<Agent['model'], string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
  copilot: 'Copilot',
};

export default function ChatInput({
  agents,
  onSend,
  isSending = false,
  placeholder = 'Message the group...',
}: ChatInputProps) {
  const [text, setText] = useState('');
  const [mentionQuery, setMentionQuery] = useState<string | null>(null);
  const [mentionStart, setMentionStart] = useState<number>(0);
  const [highlightIndex, setHighlightIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // ---- Auto-resize textarea ----
  const resizeTextarea = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }, []);

  useEffect(() => {
    resizeTextarea();
  }, [text, resizeTextarea]);

  // ---- Mention detection ----
  const detectMention = useCallback(
    (value: string, cursorPos: number) => {
      // Walk backwards from cursor to find an unmatched @
      const before = value.slice(0, cursorPos);
      const match = /@([A-Za-z0-9_-]*)$/.exec(before);
      if (match) {
        setMentionQuery(match[1].toLowerCase());
        setMentionStart(cursorPos - match[0].length);
        setHighlightIndex(0);
      } else {
        setMentionQuery(null);
      }
    },
    [],
  );

  const handleChange = useCallback(
    (e: ChangeEvent<HTMLTextAreaElement>) => {
      const value = e.target.value;
      setText(value);
      detectMention(value, e.target.selectionStart ?? value.length);
    },
    [detectMention],
  );

  // ---- Filtered agents for dropdown ----
  const filteredAgents =
    mentionQuery !== null
      ? agents.filter((a) => a.name.toLowerCase().includes(mentionQuery))
      : [];

  // Keep highlight in bounds
  useEffect(() => {
    if (highlightIndex >= filteredAgents.length) {
      setHighlightIndex(Math.max(0, filteredAgents.length - 1));
    }
  }, [filteredAgents.length, highlightIndex]);

  // Scroll highlighted item into view
  useEffect(() => {
    if (!dropdownRef.current) return;
    const item = dropdownRef.current.children[highlightIndex] as
      | HTMLElement
      | undefined;
    item?.scrollIntoView({ block: 'nearest' });
  }, [highlightIndex]);

  // ---- Insert mention ----
  const insertMention = useCallback(
    (agent: Agent) => {
      const before = text.slice(0, mentionStart);
      const after = text.slice(
        mentionStart + 1 + (mentionQuery?.length ?? 0),
      );
      const newText = `${before}@${agent.name} ${after}`;
      setText(newText);
      setMentionQuery(null);

      // Restore focus + cursor
      requestAnimationFrame(() => {
        const el = textareaRef.current;
        if (!el) return;
        const pos = before.length + agent.name.length + 2; // @Name + space
        el.focus();
        el.setSelectionRange(pos, pos);
      });
    },
    [text, mentionStart, mentionQuery],
  );

  // ---- Collect mentioned agent ids from final text ----
  const collectMentionedIds = useCallback(
    (value: string): string[] => {
      const ids: string[] = [];
      for (const agent of agents) {
        if (value.includes(`@${agent.name}`)) {
          ids.push(agent.id);
        }
      }
      return ids;
    },
    [agents],
  );

  // ---- Send ----
  const send = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed || isSending) return;
    onSend(trimmed, collectMentionedIds(trimmed));
    setText('');
    setMentionQuery(null);

    // Reset textarea height
    requestAnimationFrame(() => {
      const el = textareaRef.current;
      if (el) {
        el.style.height = 'auto';
      }
    });
  }, [text, isSending, onSend, collectMentionedIds]);

  // ---- Keyboard handling ----
  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      const dropdownOpen = mentionQuery !== null && filteredAgents.length > 0;

      if (dropdownOpen) {
        if (e.key === 'ArrowDown') {
          e.preventDefault();
          setHighlightIndex((i) =>
            i < filteredAgents.length - 1 ? i + 1 : 0,
          );
          return;
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault();
          setHighlightIndex((i) =>
            i > 0 ? i - 1 : filteredAgents.length - 1,
          );
          return;
        }
        if (e.key === 'Enter') {
          e.preventDefault();
          const agent = filteredAgents[highlightIndex];
          if (agent) insertMention(agent);
          return;
        }
        if (e.key === 'Escape') {
          e.preventDefault();
          setMentionQuery(null);
          return;
        }
      }

      // Enter to send (no shift)
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        send();
      }
    },
    [mentionQuery, filteredAgents, highlightIndex, insertMention, send],
  );

  const canSend = text.trim().length > 0 && !isSending;
  const showDropdown = mentionQuery !== null && filteredAgents.length > 0;

  return (
    <div className="relative w-full">
      {/* Mention Autocomplete Dropdown */}
      {showDropdown && (
        <div
          ref={dropdownRef}
          className="absolute bottom-full left-0 z-50 mb-1 max-h-48 w-64 overflow-y-auto rounded-lg border border-gray-600 bg-gray-800 py-1 shadow-lg"
        >
          {filteredAgents.map((agent, i) => (
            <button
              key={agent.id}
              type="button"
              className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm ${
                i === highlightIndex
                  ? 'bg-gray-700 text-white'
                  : 'text-gray-300 hover:bg-gray-700/50'
              }`}
              onMouseEnter={() => setHighlightIndex(i)}
              onMouseDown={(e) => {
                // Prevent textarea blur
                e.preventDefault();
                insertMention(agent);
              }}
            >
              {/* Color dot */}
              <span
                className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
                style={{ backgroundColor: agent.color }}
              />
              {/* Name */}
              <span className="truncate font-medium">{agent.name}</span>
              {/* Model badge */}
              <span className="ml-auto shrink-0 rounded bg-gray-600 px-1.5 py-0.5 text-[10px] leading-none text-gray-400">
                {MODEL_BADGES[agent.model]}
              </span>
            </button>
          ))}
        </div>
      )}

      {/* Input row */}
      <div className="flex items-end gap-2 rounded-lg border border-gray-700 bg-gray-800 px-3 py-2">
        <textarea
          ref={textareaRef}
          value={text}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={isSending}
          rows={1}
          className="max-h-[200px] min-h-[24px] flex-1 resize-none bg-transparent text-sm leading-6 text-gray-300 placeholder-gray-500 outline-none disabled:opacity-50"
        />

        {/* Send button */}
        <button
          type="button"
          disabled={!canSend}
          onClick={send}
          className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-indigo-600 text-white transition-colors hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {isSending ? (
            /* Loading spinner */
            <svg
              className="h-4 w-4 animate-spin"
              viewBox="0 0 24 24"
              fill="none"
            >
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"
              />
            </svg>
          ) : (
            /* Send arrow */
            <svg
              className="h-4 w-4"
              viewBox="0 0 20 20"
              fill="currentColor"
            >
              <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z" />
            </svg>
          )}
        </button>
      </div>
    </div>
  );
}
