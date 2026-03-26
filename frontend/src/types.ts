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
  model: string;
  provider: 'claude' | 'codex' | 'gemini' | 'copilot';
  agentHome?: string;
  color: string;
  scope: 'global' | 'workspace';
  systemPrompt: string;
  workspacePath?: string;
}

export interface Playbook {
  id: string;
  name: string;
  flowType: string;
  yamlContent: string;
  description: string;
  createdAt: string;
  updatedAt: string;
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

export interface EditorTab {
  id: string;
  type: 'chat' | 'file';
  title: string;
  roomId?: string;
  filePath?: string;
}
