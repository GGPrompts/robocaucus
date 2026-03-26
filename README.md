# RoboCaucus

**One app for all your AI subscriptions -- and they can collaborate.**

RoboCaucus is a multi-agent chat platform where AI agents backed by your existing CLI subscriptions (Claude, ChatGPT/Codex, Gemini, GitHub Copilot) share conversations in a group chat. Each agent has a distinct persona and can debate, review, research, or brainstorm alongside other agents and you.

No API keys. No extra billing. Just the subscriptions you already pay for, working together.

## Features

- **Multi-agent conversations** -- Add multiple AI agents to a conversation, each with its own personality and role
- **@mention routing** -- Direct messages to specific agents with `@Editor` or `@"Devil's Advocate"`
- **Ask Everyone (Panel mode)** -- Fan out a prompt to all agents simultaneously and compare responses
- **Structured Debate** -- Multi-turn debates with Opening, Rebuttal, Closing, and Synthesis phases
- **Playbooks** -- Saved orchestration recipes with input forms (e.g., Code Review, Debate, Research)
- **9 starter agents** -- Pre-built personas for writing, development, and research, seeded on first launch
- **4 CLI adapters** -- Claude, Codex, Gemini, and GitHub Copilot
- **31 themes** -- Full CSS variable theming across the entire UI
- **Tabbed editor** -- Multiple conversations and file viewers open as tabs
- **Developer sidebar** -- File tree, git graph, and ripgrep search
- **Workspace management** -- Switch between project directories with a persistent recent list
- **tmux persistence** -- CLI processes survive browser closes and backend restarts
- **Streaming markdown** -- Streamdown with syntax highlighting, mermaid diagrams, and math rendering

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v20+)
- At least one AI CLI installed and authenticated:
  - `claude` ([Claude Code](https://claude.ai/code))
  - `codex` ([OpenAI Codex](https://github.com/openai/codex))
  - `gemini` ([Gemini CLI](https://github.com/google-gemini/gemini-cli))
  - `gh copilot` ([GitHub Copilot](https://github.com/github/gh-copilot))

### Run

```bash
# Clone and start (checks ports, idempotent)
git clone <repo-url> && cd robocaucus
./start.sh

# Or run backend and frontend separately:
cargo run -p backend          # Port 7331
cd frontend && npm run dev    # Port 7330 (proxies /api to backend)
```

Open [http://localhost:7330](http://localhost:7330). On first launch, 9 starter agents and 3 playbooks are seeded automatically.

## Architecture

```
[React Frontend]  <--SSE-->  [Rust Backend (Axum)]  --spawns-->  [claude -p]
     :7330                         :7331                          [codex exec]
                                    |                             [gemini]
                                [SQLite]                          [gh copilot -p]
                            (WAL mode, local)
                                    |
                              [tmux sessions]
                            (process persistence)
```

**Backend** -- Rust workspace (Axum 0.8 + Tokio + rusqlite). CLI adapters spawn processes with `cwd = agent_home` for native config discovery. Orchestration engines handle panel fan-out and structured debate. tmux provides optional process persistence.

**Frontend** -- React 19 + TypeScript + Vite + Tailwind CSS 4. Streamdown for streaming markdown. 31 CSS variable themes. SSE with reconnection buffering.

See [CLAUDE.md](CLAUDE.md) for detailed module documentation.

## Project Structure

```
backend/src/
  adapter/          # CLI adapter trait + implementations (claude, codex, gemini, copilot)
  orchestrate/      # Panel fan-out and debate state machine
  routes/           # Axum HTTP handlers (conversations, agents, chat, playbooks, git, files, config, providers, pr-review)
  db.rs             # SQLite schema and queries
  state.rs          # Shared app state (DB + SSE broadcast + TmuxManager)
  templates.rs      # Starter agents and playbooks
  tmux.rs           # tmux session management
  reconcile.rs      # Startup tmux reconciliation

frontend/src/
  components/       # React components (Sidebar, ChatInput/Message, AgentBuilder, TabBar, DevSidebar, FileTree, git/, etc.)
  hooks/useChat.ts  # SSE streaming (single, panel, debate)
  lib/api.ts        # Backend API client
  themes/           # 31 CSS variable theme files
  types.ts          # TypeScript interfaces
```

## CLI Adapters

Each agent is backed by a subscription CLI. The adapter spawns the CLI with the agent's config folder as `cwd` (for native config discovery) and the workspace directory via CLI flags:

| Provider | CLI Command | Config File | Workspace Flag |
|----------|-------------|-------------|----------------|
| Claude | `claude -p --verbose --output-format stream-json` | `CLAUDE.md` | `--add-dir` |
| Codex | `codex exec --json` | `.codex/instructions.md` | `-C` |
| Gemini | `gemini -p --output-format stream-json` | `GEMINI.md` | `--include-directories` |
| Copilot | `gh copilot -p --output-format json` | `.copilot-instructions.md` | `--config-dir` |

## Development

```bash
# Run all tests
cargo test

# Run specific test
cargo test --package backend test_create_and_list_conversations

# Frontend lint
cd frontend && npm run lint

# Frontend type check + build
cd frontend && npm run build
```

## Status

MVP and Fast-Follow milestones are complete. See [PLAN.md](PLAN.md) for the full roadmap with status markers.

**Done:** Chat with multi-agent conversations, @mention routing, panel mode, debate mode, playbooks with input forms, 9 starter agents, 31 themes, tabbed editor, file tree, git graph, workspace management, tmux persistence, CLI detection, conversation delete, mermaid/math rendering, PR review tribunal.

**Remaining:** Round-robin mode, playbook custom creation UI, onboarding wizard, export conversations, Tauri desktop app, TTS mode, file attachments.

## License

Private -- not yet licensed for distribution.
