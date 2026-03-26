import type { Agent, Message, Playbook, Room } from '../types.ts';

const API_BASE = '/api';

export function apiUrl(path: string, base?: string): string {
  return `${base ?? API_BASE}${path}`;
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

export interface ProviderInfo {
  id: string;
  name: string;
  available: boolean;
  version: string | null;
  cli_command: string;
}

export interface ProvidersResponse {
  providers: ProviderInfo[];
}

export async function fetchProviders(base?: string): Promise<ProvidersResponse> {
  const res = await fetch(apiUrl('/providers', base));
  return jsonOrThrow<ProvidersResponse>(res);
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

export interface AppConfig {
  default_workspace: string;
}

export async function fetchConfig(base?: string): Promise<AppConfig> {
  const res = await fetch(apiUrl('/config', base));
  return jsonOrThrow<AppConfig>(res);
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

export async function createAgent(
  name: string,
  model: string,
  provider: string,
  color: string,
  scope: string,
  systemPrompt: string,
  cliConfig?: Record<string, unknown>,
  base?: string,
): Promise<Agent> {
  const payload: Record<string, unknown> = { name, model, provider, color, scope, system_prompt: systemPrompt };
  if (cliConfig !== undefined) payload.cli_config = JSON.stringify(cliConfig);
  const res = await fetch(apiUrl('/agents', base), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
  return jsonOrThrow<Agent>(res);
}

export async function fetchAgentConfig(
  agentId: string,
  base?: string,
): Promise<{ path: string; content: string; format: string }> {
  const res = await fetch(apiUrl(`/agents/${agentId}/config`, base));
  return jsonOrThrow<{ path: string; content: string; format: string }>(res);
}

export async function saveAgentConfig(
  agentId: string,
  content: string,
  base?: string,
): Promise<void> {
  const res = await fetch(apiUrl(`/agents/${agentId}/config`, base), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`API ${res.status}: ${text}`);
  }
}

// ---------------------------------------------------------------------------
// Playbooks
// ---------------------------------------------------------------------------

export async function fetchPlaybooks(base?: string): Promise<Playbook[]> {
  const res = await fetch(apiUrl('/playbooks', base));
  return jsonOrThrow<Playbook[]>(res);
}

export async function fetchPlaybook(id: string, base?: string): Promise<Playbook> {
  const res = await fetch(apiUrl(`/playbooks/${id}`, base));
  return jsonOrThrow<Playbook>(res);
}

export async function runPlaybook(
  id: string,
  yamlContent?: string,
  base?: string,
): Promise<{ conversation_id: string }> {
  const hasBody = yamlContent !== undefined;
  const res = await fetch(apiUrl(`/playbooks/${id}/run`, base), {
    method: 'POST',
    ...(hasBody
      ? {
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ yaml_content: yamlContent }),
        }
      : {}),
  });
  return jsonOrThrow<{ conversation_id: string }>(res);
}

export async function createPlaybook(
  name: string,
  flowType: string,
  yamlContent: string,
  description: string,
  base?: string,
): Promise<Playbook> {
  const res = await fetch(apiUrl('/playbooks', base), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      name,
      flow_type: flowType,
      yaml_content: yamlContent,
      description,
    }),
  });
  return jsonOrThrow<Playbook>(res);
}

// ---------------------------------------------------------------------------
// Conversations
// ---------------------------------------------------------------------------

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

export interface ConversationDetail {
  id: string;
  title: string;
  workspacePath?: string;
  orchestrationMode: string;
  createdAt: string;
  updatedAt: string;
  agents: Agent[];
  messages: Message[];
}

export async function fetchConversation(
  id: string,
  base?: string,
): Promise<ConversationDetail> {
  const res = await fetch(apiUrl(`/conversations/${id}`, base));
  return jsonOrThrow<ConversationDetail>(res);
}

export async function updateConversation(
  id: string,
  updates: { title?: string; orchestration_mode?: string; agent_ids?: string[] },
  base?: string,
): Promise<Room> {
  const res = await fetch(apiUrl(`/conversations/${id}`, base), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  return jsonOrThrow<Room>(res);
}

export async function deleteConversation(
  id: string,
  base?: string,
): Promise<void> {
  const res = await fetch(apiUrl(`/conversations/${id}`, base), {
    method: 'DELETE',
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`API ${res.status}: ${text}`);
  }
}

export async function fetchConversationAgents(
  conversationId: string,
  base?: string,
): Promise<Agent[]> {
  const res = await fetch(apiUrl(`/conversations/${conversationId}/agents`, base));
  return jsonOrThrow<Agent[]>(res);
}

export async function addAgentToConversation(
  conversationId: string,
  agentId: string,
  base?: string,
): Promise<void> {
  const res = await fetch(apiUrl(`/conversations/${conversationId}/agents/${agentId}`, base), {
    method: 'POST',
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`API ${res.status}: ${text}`);
  }
}

export async function removeAgentFromConversation(
  conversationId: string,
  agentId: string,
  base?: string,
): Promise<void> {
  const res = await fetch(apiUrl(`/conversations/${conversationId}/agents/${agentId}`, base), {
    method: 'DELETE',
  });
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText);
    throw new Error(`API ${res.status}: ${text}`);
  }
}

// ---------------------------------------------------------------------------
// Orchestration: Panel & Debate (SSE endpoints — return raw Response for streaming)
// ---------------------------------------------------------------------------

export function startPanel(
  conversationId: string,
  content: string,
  base?: string,
): Promise<Response> {
  return fetch(apiUrl('/chat/panel', base), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ conversation_id: conversationId, content }),
  });
}

export function startDebate(
  conversationId: string,
  topic: string,
  numRounds?: number,
  base?: string,
): Promise<Response> {
  return fetch(apiUrl('/chat/debate', base), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      conversation_id: conversationId,
      topic,
      num_rounds: numRounds,
    }),
  });
}

// ── File API ──────────────────────────────────────────────────────────────

export interface FileEntry {
  name: string;
  is_dir: boolean;
  size: number;
  modified: number;
}

export interface FileListResponse {
  path: string;
  entries: FileEntry[];
}

export interface FileContentResponse {
  path: string;
  size: number;
  is_binary: boolean;
  extension: string;
  content: string;
}

export interface SearchResult {
  file: string;
  line: number;
  text: string;
  context_before: string[];
  context_after: string[];
}

export interface SearchResponse {
  results: SearchResult[];
}

export async function fetchFileList(
  path: string,
  dir?: string,
  base?: string,
): Promise<FileListResponse> {
  const params = new URLSearchParams({ path });
  if (dir) params.set('dir', dir);
  const res = await fetch(apiUrl(`/files/list?${params}`, base));
  return jsonOrThrow<FileListResponse>(res);
}

export async function fetchFileContent(
  path: string,
  file: string,
  base?: string,
): Promise<FileContentResponse> {
  const params = new URLSearchParams({ path, file });
  const res = await fetch(apiUrl(`/files/read?${params}`, base));
  return jsonOrThrow<FileContentResponse>(res);
}

export async function searchFiles(
  path: string,
  query: string,
  options?: { regex?: boolean; caseSensitive?: boolean; glob?: string },
  base?: string,
): Promise<SearchResponse> {
  const params = new URLSearchParams({ path, q: query });
  if (options?.regex) params.set('regex', 'true');
  if (options?.caseSensitive) params.set('case', 'true');
  if (options?.glob) params.set('glob', options.glob);
  const res = await fetch(apiUrl(`/search?${params}`, base));
  return jsonOrThrow<SearchResponse>(res);
}

// ---------------------------------------------------------------------------
// Git API
// ---------------------------------------------------------------------------

export async function fetchGitGraph(
  path: string,
  limit?: number,
  skip?: number,
  base?: string,
): Promise<{ data: { commits: any[]; hasMore: boolean } }> {
  const params = new URLSearchParams({ path });
  if (limit != null) params.set('limit', String(limit));
  if (skip != null) params.set('skip', String(skip));
  const res = await fetch(apiUrl(`/git/graph?${params}`, base));
  return jsonOrThrow(res);
}

export async function fetchCommitDetails(
  path: string,
  hash: string,
  base?: string,
): Promise<{ data: any }> {
  const params = new URLSearchParams({ path });
  const res = await fetch(apiUrl(`/git/commit/${hash}?${params}`, base));
  return jsonOrThrow(res);
}

export async function fetchGitDiff(
  path: string,
  diffBase?: string,
  file?: string,
  base?: string,
): Promise<{ data: string }> {
  const params = new URLSearchParams({ path });
  if (diffBase != null) params.set('base', diffBase);
  if (file != null) params.set('file', file);
  const res = await fetch(apiUrl(`/git/diff?${params}`, base));
  return jsonOrThrow(res);
}

export async function fetchGitStatus(
  path: string,
  base?: string,
): Promise<{ data: any }> {
  const params = new URLSearchParams({ path });
  const res = await fetch(apiUrl(`/git/status?${params}`, base));
  return jsonOrThrow(res);
}
