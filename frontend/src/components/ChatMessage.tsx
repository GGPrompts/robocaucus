import { useMemo } from 'react';
import { Streamdown } from 'streamdown';
import { code } from '@streamdown/code';
import { mermaid } from '@streamdown/mermaid';
import { math } from '@streamdown/math';
import type { Agent, Message } from '../types.ts';

// Extend Message with optional streaming flag for local use
interface StreamingMessage extends Message {
  streaming?: boolean;
}

interface ChatMessageProps {
  message: StreamingMessage;
  agent?: Agent;
}

const MODEL_LABELS: Record<string, string> = {
  claude: 'Claude',
  codex: 'Codex',
  gemini: 'Gemini',
  copilot: 'Copilot',
};

function formatRelativeTime(timestamp: string): string {
  const now = Date.now();
  const then = new Date(timestamp).getTime();
  const diffSeconds = Math.floor((now - then) / 1000);

  if (diffSeconds < 60) return 'just now';
  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

function StreamingDots() {
  return (
    <div className="flex items-center gap-1 py-2">
      <span className="inline-block h-2 w-2 animate-pulse rounded-full bg-[var(--text-secondary)]" />
      <span
        className="inline-block h-2 w-2 animate-pulse rounded-full bg-[var(--text-secondary)]"
        style={{ animationDelay: '0.2s' }}
      />
      <span
        className="inline-block h-2 w-2 animate-pulse rounded-full bg-[var(--text-secondary)]"
        style={{ animationDelay: '0.4s' }}
      />
    </div>
  );
}

const plugins = { code, mermaid, math };

export default function ChatMessage({ message, agent }: ChatMessageProps) {
  const isUser = message.role === 'user';
  const isStreaming = message.streaming || (!message.content && message.role === 'assistant');

  const relativeTime = useMemo(
    () => formatRelativeTime(message.timestamp),
    [message.timestamp],
  );

  const modelLabel = agent ? MODEL_LABELS[agent.model] ?? agent.model : message.model;

  if (isUser) {
    return (
      <div className="flex justify-end px-4 py-1">
        <div className="max-w-[70%]">
          <div className="mb-0.5 text-right text-xs text-[var(--text-secondary)]">You</div>
          <div className="rounded-lg bg-[var(--bg-elevated)] px-3 py-2 text-sm text-[var(--text-primary)]">
            <div style={{ whiteSpace: 'pre-wrap' }}>{message.content}</div>
          </div>
          <div className="mt-0.5 text-right text-xs text-[var(--text-muted)]">
            {relativeTime}
          </div>
        </div>
      </div>
    );
  }

  // Agent / assistant message
  return (
    <div className="flex justify-start px-4 py-1">
      <div className="max-w-[80%]">
        {/* Header: color dot + name + model badge */}
        <div className="mb-0.5 flex items-center gap-2">
          {agent && (
            <>
              <span
                className="inline-block h-2.5 w-2.5 rounded-full"
                style={{ backgroundColor: agent.color }}
              />
              <span className="text-sm font-bold text-[var(--text-primary)]">
                {agent.name}
              </span>
            </>
          )}
          {modelLabel && (
            <span className="rounded-full bg-[var(--bg-surface)] px-2 py-0.5 text-[10px] leading-none font-medium text-[var(--text-secondary)]">
              {modelLabel}
            </span>
          )}
        </div>

        {/* Message body */}
        <div className="rounded-lg bg-[var(--bg-secondary)] px-3 py-2 text-sm text-[var(--text-primary)]">
          {isStreaming ? (
            <StreamingDots />
          ) : (
            <Streamdown plugins={plugins}>
              {message.content}
            </Streamdown>
          )}
        </div>

        {/* Timestamp */}
        <div className="mt-0.5 text-xs text-[var(--text-muted)]">{relativeTime}</div>
      </div>
    </div>
  );
}
