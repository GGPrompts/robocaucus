# Claude Code CLI Reference

> Anthropic's official CLI for Claude. Binary: `claude`

## Agent-Relevant CLI Flags

These are the flags that can be configured per-agent in the RoboCaucus agent creator.

### Core Execution

| Flag | Type | Description |
|------|------|-------------|
| `-p, --print` | boolean | Non-interactive mode (required for agent spawning) |
| `--model` | string | Model alias (`sonnet`, `opus`, `haiku`) or full name (`claude-sonnet-4-6`, `claude-opus-4-6`) |
| `--effort` | enum | Reasoning effort: `low`, `medium`, `high`, `max` (Opus 4.6 only) |
| `--verbose` | boolean | Show full turn-by-turn output |
| `--output-format` | enum | Output format: `text`, `json`, `stream-json` |

### System Prompt

| Flag | Type | Description |
|------|------|-------------|
| `--system-prompt` | string | Replace entire system prompt with custom text |
| `--system-prompt-file` | path | Replace system prompt from file |
| `--append-system-prompt` | string | Append to default prompt (preserves built-in capabilities) |
| `--append-system-prompt-file` | path | Append file contents to default prompt |

Note: `--system-prompt` and `--system-prompt-file` are mutually exclusive. Append flags can combine with either.

### Workspace & Directories

| Flag | Type | Description |
|------|------|-------------|
| `--add-dir` | path[] | Additional directories to allow tool access to |
| `-w, --worktree` | string? | Create isolated git worktree for session |

### Permissions & Tools

| Flag | Type | Description |
|------|------|-------------|
| `--permission-mode` | enum | `default`, `plan`, `acceptEdits`, `bypassPermissions`, `dontAsk`, `auto` |
| `--allowedTools` | string[] | Tools that run without permission prompts (e.g. `"Bash(git:*)" "Read"`) |
| `--disallowedTools` | string[] | Tools removed from model context entirely |
| `--tools` | string[] | Restrict available built-in tools (`""` = none, `"default"` = all, or tool names) |
| `--dangerously-skip-permissions` | boolean | Bypass all permission checks (sandboxed environments only) |

### MCP Servers

| Flag | Type | Description |
|------|------|-------------|
| `--mcp-config` | path[] | Load MCP servers from JSON files or strings |
| `--strict-mcp-config` | boolean | Only use MCP servers from `--mcp-config`, ignore all others |

### Session & Budget

| Flag | Type | Description |
|------|------|-------------|
| `--max-budget-usd` | number | Maximum dollar spend on API calls (print mode only) |
| `--max-turns` | number | Limit agentic turns (print mode only) |
| `--fallback-model` | string | Automatic fallback model when primary is overloaded |

### Advanced

| Flag | Type | Description |
|------|------|-------------|
| `--bare` | boolean | Minimal mode: skip hooks, LSP, plugins, CLAUDE.md discovery |
| `--json-schema` | JSON | Structured output validation schema |
| `--betas` | string[] | Beta headers for API requests |
| `--agent` | string | Use a named subagent for the session |
| `--agents` | JSON | Define custom subagents dynamically |

## Available Models

| Alias | Full Name | Notes |
|-------|-----------|-------|
| `opus` | `claude-opus-4-6` | Most capable, supports `--effort max` |
| `sonnet` | `claude-sonnet-4-6` | Fast and capable (default) |
| `haiku` | `claude-haiku-4-5` | Fastest, most affordable |

## Configuration Files

### Per-Agent Config Folder

| File | Purpose |
|------|---------|
| `CLAUDE.md` | Agent instructions (system prompt) |
| `.claude/settings.json` | MCP servers, allowed/disallowed tools, permissions |

### Settings JSON Structure

```json
{
  "permissions": {
    "allow": ["Bash(git:*)", "Read", "Glob", "Grep"],
    "deny": ["Bash(rm:*)"]
  },
  "mcpServers": {
    "server-name": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-name"],
      "env": {}
    }
  }
}
```

### Settings Scopes (precedence order)

1. CLI flags (highest)
2. `.claude/settings.local.json` (gitignored, machine-specific)
3. `.claude/settings.json` (project-level)
4. `~/.claude/settings.json` (user-level)
5. Managed settings (lowest)

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | API key for direct API auth |
| `CLAUDE_CODE_SIMPLE` | Set by `--bare` mode |
| `CLAUDE_MODEL` | Default model override |

## Current Adapter Usage

```
claude -p --verbose --output-format stream-json "<prompt>" [--add-dir <workspace>]
```

- `cwd` = agent_home (for CLAUDE.md / .claude/settings.json discovery)
- Workspace passed via `--add-dir`

## Sources

- [CLI Reference - Claude Code Docs](https://code.claude.com/docs/en/cli-reference)
- [Settings - Claude Code Docs](https://code.claude.com/docs/en/settings)
- `claude --help` output (local)
