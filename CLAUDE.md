# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is RoboCaucus?

A multi-agent chat platform where multiple AI agents (backed by CLI tools: Claude, Codex, Gemini, Copilot) participate in conversations together. Users create agents with distinct personas stored in per-agent config folders, add them to conversations, and interact via a web UI with @mention routing, SSE streaming, orchestration modes, playbooks, and a developer sidebar with git/file browsing.

## Build & Run

```bash
# Start both backend and frontend (idempotent, checks ports first)
./start.sh

# Backend only (Rust/Axum, port 7331)
cargo run -p backend

# Frontend only (React/Vite, port 7330, proxies /api to backend)
cd frontend && npm run dev

# Run all Rust tests (backend + common)
cargo test

# Run a specific test
cargo test --package backend test_create_and_list_conversations

# Frontend lint
cd frontend && npm run lint

# Frontend build (TypeScript check + Vite build)
cd frontend && npm run build
```

## Architecture

**Rust workspace** with three crates:
- `backend/` — Axum HTTP server, SQLite database, CLI adapters, orchestration engines
- `common/` — Shared types (currently just a `Message` struct; most types live in `backend::db`)
- `tui/` — Ratatui terminal UI (placeholder, not yet wired up)

**Frontend** — React 19 + TypeScript + Vite + Tailwind CSS 4

### Backend modules (backend/src/)

- `db.rs` — SQLite via rusqlite (WAL mode). Domain structs: `Conversation`, `Agent`, `Message`, `Playbook`. Join table `conversation_agents`. Manual ISO-8601 timestamps (no chrono dependency).
- `state.rs` — `AppState` holds `Arc<Mutex<Database>>` + `broadcast::Sender` for SSE. Use `state.db()` helper which returns an Axum-compatible error on mutex poisoning.
- `scaffold.rs` — Agent folder scaffolding. Creates `~/.robocaucus/agents/{name}/` with provider-specific instruction files (CLAUDE.md, .codex/instructions.md, GEMINI.md, .copilot-instructions.md).
- `routes/` — Axum route handlers:
  - `conversations.rs` — CRUD for conversations
  - `agents.rs` — CRUD for agents + `GET/PUT /api/agents/{id}/config` for per-agent MCP/tool config files
  - `chat.rs` — `POST /api/chat/send` (SSE streaming), `GET /api/chat/stream/:id` (reconnect replay). Process-global `ReconnectBuffer` for SSE event replay.
  - `playbooks.rs` — CRUD for playbooks + `POST /api/playbooks/{id}/run` (creates conversation, assigns agents from YAML roles)
  - `git.rs` — Read-only git endpoints: graph (paginated log), commit details, diff, status. Ported from PocketForge, uses spawned git CLI.
  - `files.rs` — File browser: directory listing, file read (with binary detection), ripgrep-powered search. Ported from PocketForge.
  - `providers.rs` — Provider/model detection
  - `agentmd.rs` — Agent markdown management
- `adapter/` — `CliAdapter` trait abstracts over AI CLIs. Signature: `spawn(prompt, agent_home, workspace)`. Agent's folder is set as cwd for native config discovery; workspace passed via `--add-dir`/`-C`/`--include-directories`.
  - `claude.rs` — Claude CLI (`-p --output-format stream-json`). Discovers CLAUDE.md from agent_home.
  - `codex.rs` — Codex CLI (`exec --json`). Discovers .codex/instructions.md from agent_home.
  - `gemini.rs` — Gemini CLI (`-p --output-format stream-json`). Discovers GEMINI.md from agent_home.
  - `copilot.rs` — Copilot CLI (`-p --output-format json --allow-all-tools`). Uses --config-dir for agent_home.
- `orchestrate/` — Orchestration engines:
  - `panel.rs` — "Ask Everyone" fan-out: spawns all agents concurrently, merges output into tagged chunks. `select_adapter()` maps provider → adapter.
  - `debate.rs` — Pure state machine (`DebateEngine`): Opening → Rebuttal(1..N) → Closing → Synthesis → Complete.
- `context.rs` — `build_agent_context()` creates per-agent message context where own messages are "assistant" role, other agents' messages get `[Name responded]:` prefix.
- `mention.rs` — `@mention` parsing: `@Name` and `@"Quoted Name"`, case-insensitive exact/prefix matching, deduplication, routing fallback.
- `tmux.rs` — `TmuxManager` manages `rc-` prefixed tmux sessions.
- `reconcile.rs` — Startup reconciliation between tmux sessions and DB state.
- `templates.rs` — Starter agent templates and starter playbooks (Debate, Code Review Panel, Research & Synthesize).

### Frontend structure (frontend/src/)

- `App.tsx` — Main layout: `Sidebar | ChatPanel | DevSidebar(optional)`. Manages theme state (localStorage), playbook modal, dev sidebar toggle.
- `hooks/useChat.ts` — SSE streaming hook with reconnection support.
- `lib/api.ts` — REST client for all backend endpoints (conversations, agents, chat, playbooks, git, files, search).
- `lib/graphLayout.ts` — Rail-based git graph layout algorithm (ported from markdown-themes).
- `types.ts` — TypeScript interfaces: `Room`, `Agent` (provider + model variant), `Message`, `Playbook`.
- `themes/` — 31 CSS variable themes (ported from markdown-themes) with Shiki syntax highlighting integration.
- `components/`:
  - `Sidebar.tsx` — Conversations list, agents list, playbooks button
  - `ChatInput.tsx`, `ChatMessage.tsx` — Chat UI with @mention support
  - `AgentBuilder.tsx` — Provider-aware agent creation/editing with conditional fields per CLI, MCP config editor
  - `ThemeSelector.tsx` — Dropdown theme picker with color previews, keyboard navigation
  - `PlaybookBrowser.tsx` — Grid of playbook cards with run button
  - `DevSidebar.tsx` — Right-side panel with Files/Git/Search tabs
  - `FileTree.tsx` — Hierarchical file browser with lazy loading
  - `CodeViewer.tsx` — Shiki-powered syntax highlighting (40+ languages, CSS variable themes)
  - `git/` — Git visualization: GitGraph, GitGraphCanvas (Canvas 2D), GitGraphRow, CommitDetails, ChangesTree, DiffViewer, StatusBadge

### Agent folder architecture

Each agent gets a folder at `~/.robocaucus/agents/{name}/` containing CLI-native config files:
- Claude agents: `CLAUDE.md` + `.claude/settings.json`
- Codex agents: `.codex/instructions.md` + `.codex/config.toml`
- Gemini agents: `GEMINI.md`
- Copilot agents: `.copilot-instructions.md`

The adapter spawns with `cwd = agent_home` (for native config discovery) and passes the workspace via CLI flags. Agent instructions live in files, not CLI args.

### Data flow: sending a message

1. Frontend `useChat.sendMessage()` → `POST /api/chat/send` with SSE response
2. Backend saves user message to DB, resolves target agent via explicit `agent_id` or `@mention` routing
3. `build_agent_context()` creates identity-aware context window (last 50 messages)
4. CLI adapter spawns with `cwd = agent_home`, workspace via `--add-dir`
5. Output chunks stream as SSE events to frontend, accumulated text saved to DB on `Done`

## Key patterns

- **Database access**: Always use `state.db()?` which safely handles mutex poisoning. Hold the lock briefly, don't hold across `.await`.
- **SSE events**: types are `text`, `thinking`, `tool_use`, `error`, `done`. Each gets a sequential id for reconnection.
- **Agent model**: `provider` field = CLI name (claude/codex/gemini/copilot), `model` field = variant (sonnet/opus/o3/gemini-2.5-pro). `agent_home` = path to agent's config folder.
- **Adapter trait**: `spawn(prompt, agent_home, workspace)` — agent_home sets cwd for config discovery, workspace passed via CLI flags.
- **Ports**: Backend 7331, Frontend 7330. Vite proxies `/api` requests to the backend.
- **DB file**: `robocaucus.db` (SQLite WAL) created in the project root.
- **Themes**: CSS variable system. Each theme is a `.css` file in `frontend/src/themes/`. Shiki syntax highlighting reads `--shiki-*` variables.
