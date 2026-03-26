# OpenAI Codex CLI Reference

> OpenAI's coding agent CLI. Binary: `codex`

## Agent-Relevant CLI Flags

### Core Execution

| Flag | Type | Description |
|------|------|-------------|
| `exec` | subcommand | Non-interactive execution (required for agent spawning) |
| `-m, --model` | string | Model identifier (e.g. `o3`, `o4-mini`, `gpt-5.4`, `gpt-5-codex`) |
| `--json` | boolean | Print events to stdout as JSONL (exec mode) |
| `-o, --output-last-message` | path | Write last agent message to file |
| `--output-schema` | path | JSON Schema file describing model's final response shape |
| `--color` | enum | Color output: `always`, `never`, `auto` |

### Sandbox & Approvals

| Flag | Type | Description |
|------|------|-------------|
| `-s, --sandbox` | enum | Sandbox policy: `read-only`, `workspace-write`, `danger-full-access` |
| `-a, --ask-for-approval` | enum | Approval policy: `untrusted`, `on-request`, `never` |
| `--full-auto` | boolean | Convenience alias: `-a on-request --sandbox workspace-write` |
| `--dangerously-bypass-approvals-and-sandbox` | boolean | Skip all safety checks (externally sandboxed only) |

### Workspace & Directories

| Flag | Type | Description |
|------|------|-------------|
| `-C, --cd` | path | Set working directory for agent |
| `--add-dir` | path[] | Additional writable directories alongside primary workspace |
| `--skip-git-repo-check` | boolean | Allow running outside a git repository |

### Configuration

| Flag | Type | Description |
|------|------|-------------|
| `-c, --config` | key=value | Override config.toml values using dotted paths |
| `-p, --profile` | string | Configuration profile from config.toml |
| `--enable` | string | Enable a feature flag (repeatable) |
| `--disable` | string | Disable a feature flag (repeatable) |

### Model Tuning

| Flag | Type | Description |
|------|------|-------------|
| `--search` | boolean | Enable live web search tool |
| `-i, --image` | path[] | Attach image files to initial prompt |
| `--oss` | boolean | Use local open-source model provider (LM Studio/Ollama) |
| `--local-provider` | enum | Specify local provider: `lmstudio` or `ollama` |

### Session Management (exec mode)

| Flag | Type | Description |
|------|------|-------------|
| `--ephemeral` | boolean | Don't persist session files to disk |
| `resume` | subcommand | Resume previous exec session |

## Available Models

| Model | Notes |
|-------|-------|
| `o3` | Default reasoning model |
| `o4-mini` | Smaller reasoning model |
| `gpt-5-codex` | Optimized for code |
| `gpt-5.1-codex` | Updated codex model |
| `gpt-5.2-codex` | Latest codex variant |
| `gpt-5.4` | Latest GPT model |
| `gpt-5.4-mini` | Mini variant |

## Configuration File: `~/.codex/config.toml`

### Core Settings

```toml
model = "o3"
model_provider = "openai"
model_reasoning_effort = "high"       # minimal | low | medium | high | xhigh
model_reasoning_summary = "concise"   # auto | concise | detailed | none
model_verbosity = "medium"            # low | medium | high
sandbox_mode = "workspace-write"      # read-only | workspace-write | danger-full-access
approval_policy = "on-request"        # untrusted | on-request | never
service_tier = "fast"                 # flex | fast
personality = "pragmatic"             # none | friendly | pragmatic
web_search = "cached"                 # disabled | cached | live
```

### Agent Configuration

```toml
[agents.my_agent]
config_file = "path/to/agent.toml"
description = "Agent description"

[agents]
max_threads = 6
max_depth = 1
job_max_runtime_seconds = 1800
```

### Shell Environment

```toml
[shell_environment_policy]
inherit = "all"                       # all | core | none
exclude = ["SECRET_*"]
set = { EDITOR = "vim" }
```

### MCP Servers

```toml
[mcp_servers.my_server]
command = "npx"
enabled = true
required = false
startup_timeout_sec = 10
tool_timeout_sec = 60
enabled_tools = ["tool1", "tool2"]
disabled_tools = ["tool3"]
```

### Feature Flags

```toml
[features]
multi_agent = true
shell_tool = true
fast_mode = true
undo = false
smart_approvals = false
```

### Profiles

```toml
[profiles.fast]
service_tier = "fast"
model = "gpt-5.4"

[profiles.safe]
sandbox_mode = "read-only"
approval_policy = "untrusted"
```

### Per-Agent Config Folder

| File | Purpose |
|------|---------|
| `.codex/instructions.md` | Agent instructions (system prompt) |
| `.codex/config.toml` | Agent-level config overrides |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `CODEX_HOME` | Override config directory (default: `~/.codex`) |

## Current Adapter Usage

```
codex exec "<prompt>" --json --skip-git-repo-check [--add-dir <workspace>]
```

- `cwd` = agent_home (for `.codex/instructions.md` discovery)
- Workspace passed via `--add-dir`

## Sources

- [Command line options - Codex CLI](https://developers.openai.com/codex/cli/reference)
- [Configuration Reference - Codex](https://developers.openai.com/codex/config-reference)
- [Advanced Configuration - Codex](https://developers.openai.com/codex/config-advanced)
- `codex --help` / `codex exec --help` output (local)
