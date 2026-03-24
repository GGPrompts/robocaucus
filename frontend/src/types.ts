export interface Room {
  id: string;
  title: string;
  workspacePath?: string;
  orchestrationMode: 'manual' | 'panel' | 'debate' | 'round_robin';
  createdAt: string;
  updatedAt: string;
}

export interface Agent {
  id: string;
  name: string;
  model: 'claude' | 'codex' | 'gemini' | 'copilot';
  color: string;
  scope: 'global' | 'workspace';
  systemPrompt: string;
  workspacePath?: string;
}

export interface Message {
  id: string;
  conversationId: string;
  agentId?: string;
  role: 'user' | 'assistant';
  content: string;
  model?: string;
  timestamp: string;
  usageJson?: string;
}
