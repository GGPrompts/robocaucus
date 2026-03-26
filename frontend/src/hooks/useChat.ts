import { useState, useRef, useCallback, useEffect } from 'react';
import type { Message } from '../types.ts';
import { apiUrl, fetchMessages } from '../lib/api.ts';

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

export interface UseChatOptions {
  conversationId: string;
  apiBase?: string; // defaults to '/api'
}

export interface StreamingMessage extends Message {
  streaming: boolean;
}

export interface UseChatReturn {
  messages: Message[];
  streamingMessage: StreamingMessage | null;
  sendMessage: (content: string, agentId?: string) => Promise<void>;
  isStreaming: boolean;
  error: string | null;
  clearError: () => void;
}

// ---------------------------------------------------------------------------
// SSE line parser – works with chunked ReadableStream text
// ---------------------------------------------------------------------------

interface SSEEvent {
  id?: string;
  event: string;
  data: string;
}

/**
 * Incrementally parses SSE frames from an accumulating buffer string.
 * Returns parsed events and the remaining unparsed buffer.
 */
function parseSSEBuffer(buffer: string): { events: SSEEvent[]; rest: string } {
  const events: SSEEvent[] = [];
  // SSE frames are separated by a blank line (\n\n)
  let idx: number;
  let remaining = buffer;

  while ((idx = remaining.indexOf('\n\n')) !== -1) {
    const frame = remaining.slice(0, idx);
    remaining = remaining.slice(idx + 2);

    let id: string | undefined;
    let event = 'message';
    let data = '';

    for (const line of frame.split('\n')) {
      if (line.startsWith('id:')) {
        id = line.slice(3).trim();
      } else if (line.startsWith('event:')) {
        event = line.slice(6).trim();
      } else if (line.startsWith('data:')) {
        // Accumulate data lines (spec allows multiple)
        data += (data ? '\n' : '') + line.slice(5).trim();
      }
      // Ignore comments (lines starting with ':') and unknown fields
    }

    if (event || data) {
      events.push({ id, event, data });
    }
  }

  return { events, rest: remaining };
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useChat(options: UseChatOptions): UseChatReturn {
  const { conversationId, apiBase } = options;

  const [messages, setMessages] = useState<Message[]>([]);
  const [streamingMessage, setStreamingMessage] = useState<StreamingMessage | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Track the last SSE event id for reconnection
  const lastEventIdRef = useRef<string | null>(null);

  // Abort controller for in-flight requests
  const abortRef = useRef<AbortController | null>(null);

  // Guard against state updates after unmount
  const mountedRef = useRef(true);

  const clearError = useCallback(() => setError(null), []);

  // ------------------------------------------------------------------
  // Load existing messages on mount / conversationId change
  // ------------------------------------------------------------------
  useEffect(() => {
    mountedRef.current = true;
    let cancelled = false;

    async function load() {
      try {
        const msgs = await fetchMessages(conversationId, apiBase);
        if (!cancelled) {
          setMessages(msgs);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Failed to load messages');
        }
      }
    }

    load();

    return () => {
      cancelled = true;
      mountedRef.current = false;
      // Abort any in-flight SSE stream
      abortRef.current?.abort();
    };
  }, [conversationId, apiBase]);

  // ------------------------------------------------------------------
  // Core SSE stream reader (shared by send & reconnect)
  // ------------------------------------------------------------------
  const readSSEStream = useCallback(
    async (
      response: Response,
      signal: AbortSignal,
      /** The message being built up. Pass initial content for reconnect. */
      initialContent: string,
      initialMessageBase: Omit<StreamingMessage, 'content' | 'streaming'>,
    ) => {
      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      let content = initialContent;

      try {
        // eslint-disable-next-line no-constant-condition
        while (true) {
          if (signal.aborted) break;

          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const { events, rest } = parseSSEBuffer(buffer);
          buffer = rest;

          for (const sse of events) {
            if (signal.aborted || !mountedRef.current) break;

            // Track last event id for reconnection
            if (sse.id) {
              lastEventIdRef.current = sse.id;
            }

            switch (sse.event) {
              case 'text': {
                const parsed = safeParse(sse.data);
                const chunk = typeof parsed === 'string' ? parsed : (parsed as { content?: string })?.content ?? sse.data;
                content += chunk;
                setStreamingMessage({
                  ...initialMessageBase,
                  content,
                  streaming: true,
                });
                break;
              }

              case 'thinking': {
                // Append thinking text wrapped in a collapsible marker
                const parsed = safeParse(sse.data);
                const thought = typeof parsed === 'string' ? parsed : (parsed as { content?: string })?.content ?? sse.data;
                content += `\n\n<details><summary>Thinking...</summary>\n\n${thought}\n\n</details>\n\n`;
                setStreamingMessage({
                  ...initialMessageBase,
                  content,
                  streaming: true,
                });
                break;
              }

              case 'tool_use': {
                // Show tool usage inline
                const parsed = safeParse(sse.data);
                const toolName = (parsed as { name?: string })?.name ?? 'tool';
                content += `\n\n> Using tool: **${toolName}**\n\n`;
                setStreamingMessage({
                  ...initialMessageBase,
                  content,
                  streaming: true,
                });
                break;
              }

              case 'done': {
                // Finalize the message
                const finalMessage: Message = {
                  ...initialMessageBase,
                  content,
                };
                setMessages((prev) => [...prev, finalMessage]);
                setStreamingMessage(null);
                setIsStreaming(false);
                return; // Done reading this stream
              }

              case 'error': {
                const parsed = safeParse(sse.data);
                const errMsg = typeof parsed === 'string' ? parsed : (parsed as { content?: string })?.content ?? sse.data;
                setError(errMsg);
                setStreamingMessage(null);
                setIsStreaming(false);
                return;
              }

              default:
                // Unknown event type, ignore
                break;
            }
          }
        }

        // Stream ended without a 'done' event – finalize what we have
        if (mountedRef.current && content) {
          const finalMessage: Message = {
            ...initialMessageBase,
            content,
          };
          setMessages((prev) => [...prev, finalMessage]);
          setStreamingMessage(null);
          setIsStreaming(false);
        }
      } catch (err) {
        if (signal.aborted) return;
        if (mountedRef.current) {
          setError(err instanceof Error ? err.message : 'Stream read failed');
          // Attempt reconnection
          attemptReconnect(content, initialMessageBase);
        }
      } finally {
        reader.releaseLock();
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  // ------------------------------------------------------------------
  // Reconnection logic
  // ------------------------------------------------------------------
  const attemptReconnect = useCallback(
    async (
      currentContent: string,
      messageBase: Omit<StreamingMessage, 'content' | 'streaming'>,
    ) => {
      if (!mountedRef.current) return;
      if (!lastEventIdRef.current) return;

      const controller = new AbortController();
      abortRef.current = controller;

      try {
        const url = apiUrl(
          `/chat/stream/${conversationId}?last_event_id=${encodeURIComponent(lastEventIdRef.current)}`,
          apiBase,
        );
        const response = await fetch(url, { signal: controller.signal });

        if (!response.ok || !response.body) {
          // Reconnection failed – finalize with what we have
          if (mountedRef.current && currentContent) {
            setMessages((prev) => [...prev, { ...messageBase, content: currentContent }]);
            setStreamingMessage(null);
            setIsStreaming(false);
          }
          return;
        }

        await readSSEStream(response, controller.signal, currentContent, messageBase);
      } catch {
        // Reconnection failed entirely
        if (mountedRef.current) {
          if (currentContent) {
            setMessages((prev) => [...prev, { ...messageBase, content: currentContent }]);
          }
          setStreamingMessage(null);
          setIsStreaming(false);
        }
      }
    },
    [conversationId, apiBase, readSSEStream],
  );

  // ------------------------------------------------------------------
  // sendMessage
  // ------------------------------------------------------------------
  const sendMessage = useCallback(
    async (content: string, agentId?: string) => {
      if (isStreaming) return;

      setError(null);
      setIsStreaming(true);

      // Optimistic user message
      const userMessage: Message = {
        id: `temp-${Date.now()}`,
        conversationId,
        role: 'user',
        content,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, userMessage]);

      // Prepare streaming message scaffold
      const assistantBase = {
        id: `stream-${Date.now()}`,
        conversationId,
        role: 'assistant' as const,
        agentId,
        timestamp: new Date().toISOString(),
      };

      setStreamingMessage({
        ...assistantBase,
        content: '',
        streaming: true,
      });

      // Abort any previous request
      abortRef.current?.abort();
      const controller = new AbortController();
      abortRef.current = controller;

      try {
        const body = JSON.stringify({
          conversation_id: conversationId,
          content,
          agent_id: agentId,
        });

        const response = await fetch(apiUrl('/chat/send', apiBase), {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body,
          signal: controller.signal,
        });

        if (!response.ok) {
          const errText = await response.text().catch(() => response.statusText);
          throw new Error(`Send failed (${response.status}): ${errText}`);
        }

        if (!response.body) {
          throw new Error('Response has no body – SSE streaming not supported');
        }

        await readSSEStream(response, controller.signal, '', assistantBase);
      } catch (err) {
        if (controller.signal.aborted) return;
        if (mountedRef.current) {
          setError(err instanceof Error ? err.message : 'Failed to send message');
          setStreamingMessage(null);
          setIsStreaming(false);
        }
      }
    },
    [conversationId, apiBase, isStreaming, readSSEStream],
  );

  return {
    messages,
    streamingMessage,
    sendMessage,
    isStreaming,
    error,
    clearError,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function safeParse(data: string): unknown {
  try {
    return JSON.parse(data);
  } catch {
    return data;
  }
}
