import type { Agent, Message, Room } from '../types.ts';

const API_BASE = '/api';

export function apiUrl(path: string, base?: string): string {
  return `${base ?? API_BASE}${path}`;
}

async function jsonOrThrow<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`API ${res.status}: ${text}`);
  }
  return res.json() as Promise<T>;
}

export async function fetchMessages(
  conversationId: string,
  base?: string,
): Promise<Message[]> {
  const res = await fetch(apiUrl(`/conversations/${conversationId}/messages`, base));
  return jsonOrThrow<Message[]>(res);
}

export async function fetchConversations(base?: string): Promise<Room[]> {
  const res = await fetch(apiUrl('/conversations', base));
  return jsonOrThrow<Room[]>(res);
}

export async function fetchAgents(base?: string): Promise<Agent[]> {
  const res = await fetch(apiUrl('/agents', base));
  return jsonOrThrow<Agent[]>(res);
}

export async function createConversation(
  title: string,
  agentIds?: string[],
  base?: string,
): Promise<Room> {
  const res = await fetch(apiUrl('/conversations', base), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ title, agent_ids: agentIds }),
  });
  return jsonOrThrow<Room>(res);
}
