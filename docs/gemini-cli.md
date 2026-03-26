# Google Gemini CLI Reference

> Google's AI coding agent CLI. Binary: `gemini`

## Agent-Relevant CLI Flags

### Core Execution

| Flag | Type | Description |
|------|------|-------------|
| `-p, --prompt` | string | Non-interactive (headless) mode (required for agent spawning) |
| `-i, --prompt-interactive` | string | Execute prompt then continue interactively |
| `-m, --model` | string | Model alias or concrete name |
| `-o, --output-format` | enum | Output format: `text`, `json`, `stream-json` |
| `-d, --debug` | boolean | Run in debug mode (verbose logging) |

### Approval & Sandbox

| Flag | Type | Description |
|------|------|-------------|
| `--approval-mode` | enum | `default` (prompt), `auto_edit` (auto-approve edits), `yolo` (auto-approve all), `plan` (read-only) |
| `-y, --yolo` | boolean | Deprecated; use `--approval-mode=yolo` |
| `-s, --sandbox` | boolean | Run in sandboxed environment |

### Workspace & Directories

| Flag | Type | Description |
|------|------|-------------|
| `--include-directories` | path[] | Additional directories to include in workspace |

### Tools & Policy

| Flag | Type | Description |
|------|------|-------------|
| `--policy` | path[] | Additional policy files or directories to load |
| `--allowed-tools` | string[] | **DEPRECATED**: Use Policy Engine instead |
| `--allowed-mcp-server-names` | string[] | Restrict which MCP servers are active |

### Extensions & Skills

| Flag | Type | Description |
|------|------|-------------|
| `-e, --extensions` | string[] | Specific extensions to use (default: all) |
| `-l, --list-extensions` | boolean | List all available extensions and exit |

### Session Management

| Flag | Type | Description |
|------|------|-------------|
| `-r, --resume` | string | Resume session by ID or `"latest"` |
| `--list-sessions` | boolean | List available sessions for current project |
| `--delete-session` | string | Delete session by index |

### Output Control

| Flag | Type | Description |
|------|------|-------------|
| `--raw-output` | boolean | Disable output sanitization (allows ANSI escapes) |
| `--accept-raw-output-risk` | boolean | Suppress raw-output security warning |
| `--screen-reader` | boolean | Enable accessibility mode |

## Available Models

| Alias | Resolves To | Notes |
|-------|-------------|-------|
| `auto` | `gemini-2.5-pro` or `gemini-3-pro-preview` | Default, resolves to preview if enabled |
| `pro` | `gemini-2.5-pro` / `gemini-3-pro-preview` | Complex reasoning |
| `flash` | `gemini-2.5-flash` | Fast, balanced |
| `flash-lite` | `gemini-2.5-flash-lite` | Fastest, simple tasks |

## Configuration Files

### Hierarchy (precedence order, highest last)

1. Default values (hardcoded)
2. System defaults: `/etc/gemini-cli/system-defaults.json`
3. User settings: `~/.gemini/settings.json`
4. Project settings: `.gemini/settings.json`
5. System override: `/etc/gemini-cli/settings.json`
6. Environment variables
7. CLI arguments (highest)

### settings.json Structure

#### General Settings

```json
{
  "general": {
    "preferredEditor": "code",
    "vimMode": false,
    "defaultApprovalMode": "default",
    "devtools": false,
    "enableAutoUpdate": true,
    "enableNotifications": false,
    "retryFetchErrors": true,
    "maxAttempts": 10,
    "checkpointing": { "enabled": false },
    "plan": {
      "directory": "./plans",
      "modelRouting": true
    },
    "sessionRetention": {
      "enabled": true,
      "maxAge": "30d",
      "maxCount": null,
      "minRetention": "1d"
    }
  }
}
```

#### Model Settings

```json
{
  "model": {
    "name": "gemini-2.5-pro",
    "maxSessionTurns": -1,
    "compressionThreshold": 0.5,
    "disableLoopDetection": false,
    "skipNextSpeakerCheck": true,
    "summarizeToolOutput": null
  }
}
```

#### Model Config Aliases

```json
{
  "modelConfigs": {
    "aliases": {
      "my-custom": {
        "extends": "gemini-3-pro-preview",
        "temperature": 0.7
      }
    }
  }
}
```

#### UI Settings

```json
{
  "ui": {
    "theme": null,
    "autoThemeSwitching": true,
    "hideBanner": false,
    "hideFooter": false,
    "hideContextSummary": false,
    "hideTips": false,
    "showLineNumbers": true,
    "showCitations": false,
    "showModelInfoInChat": false,
    "showSpinner": true,
    "incrementalRendering": true,
    "useAlternateBuffer": false,
    "useBackgroundColor": true,
    "inlineThinkingMode": "off",
    "loadingPhrases": "tips",
    "errorVerbosity": "low",
    "accessibility": {
      "screenReader": false
    },
    "footer": {
      "items": null,
      "showLabels": true,
      "hideCWD": false,
      "hideSandboxStatus": false,
      "hideModelInfo": false,
      "hideContextPercentage": true
    }
  }
}
```

#### Policy Paths

```json
{
  "policyPaths": ["./policies/my-policy.yaml"],
  "adminPolicyPaths": []
}
```

#### Privacy & Billing

```json
{
  "privacy": {
    "usageStatisticsEnabled": true
  },
  "billing": {
    "overageStrategy": "ask"
  }
}
```

### Per-Agent Config Folder

| File | Purpose |
|------|---------|
| `GEMINI.md` | Agent instructions (system prompt) |
| `.gemini/settings.json` | Project-level settings overrides |

## MCP Server Management

```bash
# Add stdio server
gemini mcp add <name> <command> [--env KEY=value] [--scope user]

# Add HTTP server
gemini mcp add <name> <url> --transport http

# Add with specific tools
gemini mcp add <name> <command> --include-tools tool1,tool2

# Remove / list
gemini mcp remove <name>
gemini mcp list
```

## Extensions Management

```bash
gemini extensions install <source> [--ref <branch>] [--auto-update]
gemini extensions uninstall <name>
gemini extensions list
gemini extensions update <name> | --all
gemini extensions enable/disable <name>
gemini extensions link <path>    # For development
gemini extensions new <path>     # Create from template
```

## Skills Management

```bash
gemini skills list
gemini skills install <source>
gemini skills link <path>
gemini skills uninstall <name>
gemini skills enable/disable <name> | --all
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GEMINI_API_KEY` / `GOOGLE_API_KEY` | API key for authentication |
| `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` | Override system defaults file location |
| `GEMINI_CLI_SYSTEM_SETTINGS_PATH` | Override system settings file location |

## Current Adapter Usage

```
gemini -p "<prompt>" --output-format stream-json [--include-directories <workspace>]
```

- `cwd` = agent_home (for `GEMINI.md` discovery)
- Workspace passed via `--include-directories` (not `--add-dir`)

## Sources

- [Gemini CLI Documentation](https://geminicli.com/docs/)
- [Configuration Reference (GitHub)](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/configuration.md)
- [CLI Cheatsheet](https://geminicli.com/docs/cli/cli-reference/)
- `gemini --help` output (local)
