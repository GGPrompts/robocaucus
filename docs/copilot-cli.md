# GitHub Copilot CLI Reference

> GitHub's AI coding agent CLI. Binary: `copilot`

## Agent-Relevant CLI Flags

### Core Execution

| Flag | Type | Description |
|------|------|-------------|
| `-p, --prompt` | string | Non-interactive mode (required for agent spawning) |
| `--model` | string | AI model to use |
| `--effort, --reasoning-effort` | enum | Reasoning effort: `low`, `medium`, `high`, `xhigh` |
| `--output-format` | enum | Output format: `text` (default), `json` (JSONL) |
| `-s, --silent` | boolean | Output only agent response, no stats (useful for scripting) |

### Permissions & Tools

| Flag | Type | Description |
|------|------|-------------|
| `--allow-all` / `--yolo` | boolean | Enable all permissions (tools + paths + URLs) |
| `--allow-all-tools` | boolean | Allow all tools without confirmation |
| `--allow-all-paths` | boolean | Disable file path verification |
| `--allow-all-urls` | boolean | Allow all URL access |
| `--allow-tool` | pattern[] | Allow specific tools (e.g. `shell(git:*)`, `write`) |
| `--deny-tool` | pattern[] | Deny specific tools (always takes precedence) |
| `--allow-url` | pattern[] | Allow specific URLs/domains |
| `--deny-url` | pattern[] | Deny specific URLs/domains |
| `--available-tools` | string[] | Only these tools visible to model |
| `--excluded-tools` | string[] | These tools hidden from model |
| `--no-ask-user` | boolean | Disable ask_user tool (fully autonomous) |

#### Tool Permission Patterns

```
shell(command)      # Exact shell command match
shell(git:*)        # All git subcommands
write               # File creation/modification tools
MyMCP(tool_name)    # Specific MCP server tool
MyMCP               # All tools from MCP server
url(domain)         # URL access (protocol-aware)
```

### Workspace & Directories

| Flag | Type | Description |
|------|------|-------------|
| `--add-dir` | path[] | Additional directories for file access |
| `--disallow-temp-dir` | boolean | Prevent automatic temp directory access |
| `--config-dir` | path | Set configuration directory (default: `~/.copilot`) |

### MCP Servers

| Flag | Type | Description |
|------|------|-------------|
| `--additional-mcp-config` | JSON/path | Additional MCP servers (augments `~/.copilot/mcp-config.json`) |
| `--disable-builtin-mcps` | boolean | Disable all built-in MCP servers |
| `--disable-mcp-server` | string | Disable specific MCP server by name |
| `--add-github-mcp-tool` | string | Enable specific GitHub MCP tool |
| `--add-github-mcp-toolset` | string | Enable GitHub MCP toolset |
| `--enable-all-github-mcp-tools` | boolean | Enable all GitHub MCP server tools |

### Custom Instructions

| Flag | Type | Description |
|------|------|-------------|
| `--no-custom-instructions` | boolean | Disable loading AGENTS.md and related files |
| `--agent` | string | Specify a custom agent |

### Session & Autopilot

| Flag | Type | Description |
|------|------|-------------|
| `--autopilot` | boolean | Enable autonomous continuation in prompt mode |
| `--max-autopilot-continues` | number | Max continuation messages in autopilot (default: unlimited) |
| `--continue` | boolean | Resume most recent session |
| `--resume` | string? | Resume by session ID |

### Output & Display

| Flag | Type | Description |
|------|------|-------------|
| `--no-color` | boolean | Disable color output |
| `--stream` | enum | Streaming mode: `on`, `off` |
| `--screen-reader` | boolean | Enable screen reader optimizations |
| `--plain-diff` | boolean | Disable rich diff rendering |
| `--no-alt-screen` | boolean | Disable alternate screen buffer |

### Security

| Flag | Type | Description |
|------|------|-------------|
| `--secret-env-vars` | string[] | Environment variables to redact from output |

## Available Models

| Model | Notes |
|-------|-------|
| `claude-sonnet-4.6` | Anthropic Sonnet |
| `claude-opus-4.6` | Anthropic Opus |
| `claude-haiku-4.5` | Anthropic Haiku |
| `gpt-5.4` | Latest OpenAI |
| `gpt-5.2` | OpenAI |
| `gpt-5.1-codex` | Code-optimized |
| `gpt-5.4-mini` | Mini variant |
| `gpt-4.1` | Legacy |
| `gemini-3-pro-preview` | Google Gemini |

## Configuration File: `~/.copilot/config.json`

### Core Settings

```json
{
  "model": "claude-sonnet-4.6",
  "theme": "auto",
  "alt_screen": true,
  "auto_update": true,
  "banner": "once",
  "beep": true,
  "stream": true,
  "render_markdown": true,
  "mouse": true,
  "experimental": false,
  "log_level": "default",
  "screen_reader": false,
  "includeCoAuthoredBy": true,
  "respectGitignore": true,
  "compact_paste": true,
  "copy_on_select": true,
  "update_terminal_title": true,
  "disableAllHooks": false,
  "trusted_folders": ["/path/to/project"],
  "allowed_urls": ["github.com", "*.github.com"],
  "denied_urls": []
}
```

### Status Line Configuration

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.copilot/status.sh",
    "padding": 2
  }
}
```

### IDE Integration

```json
{
  "ide": {
    "auto_connect": true,
    "open_diff_on_edit": true
  }
}
```

### Per-Agent Config Folder

| File | Purpose |
|------|---------|
| `.copilot-instructions.md` | Agent instructions (system prompt) |
| `~/.copilot/config.json` | Global config |
| `~/.copilot/mcp-config.json` | MCP server definitions |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `COPILOT_GITHUB_TOKEN` | Auth token (highest precedence) |
| `GH_TOKEN` | GitHub token (medium precedence) |
| `GITHUB_TOKEN` | GitHub token (lowest precedence) |
| `COPILOT_MODEL` | Default model override |
| `COPILOT_ALLOW_ALL` | Enable all permissions when `"true"` |
| `COPILOT_HOME` | Override config directory (default: `~/.copilot`) |
| `COPILOT_AUTO_UPDATE` | Enable/disable auto-updates |
| `COPILOT_CUSTOM_INSTRUCTIONS_DIRS` | Additional instruction directories (comma-separated) |
| `COPILOT_EDITOR` | Editor for interactive editing |
| `COPILOT_OFFLINE` | Enable offline mode when `"true"` |
| `GH_HOST` | GitHub hostname (default: `github.com`) |
| `NO_COLOR` | Disable color output |
| `PLAIN_DIFF` | Disable rich diff rendering |
| `USE_BUILTIN_RIPGREP` | Use bundled vs system ripgrep |

## Current Adapter Usage

```
copilot -p "<prompt>" --output-format json --allow-all-tools [--add-dir <workspace>]
```

- `cwd` = agent_home (for `.copilot-instructions.md` discovery)
- `--config-dir` can point to agent_home for full config isolation
- Workspace passed via `--add-dir`

## Sources

- [CLI Command Reference - GitHub Docs](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference)
- [Configure Copilot CLI - GitHub Docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/configure-copilot-cli)
- [DeepWiki Flags Reference](https://deepwiki.com/github/copilot-cli/5.6-command-line-flags-reference)
- `copilot --help` / `copilot help config` / `copilot help permissions` / `copilot help environment` output (local)
