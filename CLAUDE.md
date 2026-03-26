# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is RoboCaucus?

A multi-agent chat platform where multiple AI agents (backed by CLI tools: Claude, Codex, Gemini, Copilot) participate in conversations together. Users create agents with distinct personas stored in per-agent config folders, add them to conversations, and interact via a web UI with @mention routing, SSE streaming, orchestration modes (panel fan-out, structured debate), playbooks with input forms, a tabbed editor area, and developer sidebars with git/file browsing.

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
- `backend/` — Axum HTTP server, SQLite database, CLI adapters, orchestration engines, tmux persistence
- `common/` — Shared types (currently just a `Message` struct; most types live in `backend::db`)
- `tui/` — Ratatui terminal UI (placeholder, not yet wired up)

**Frontend** — React 19 + TypeScript + Vite + Tailwind CSS 4

### Backend modules (backend/src/)

- `db.rs` — SQLite via rusqlite (WAL mode). Domain structs: `Conversation`, `Agent`, `Message`, `Playbook`. Join table `conversation_agents` with `add_agent_to_conversation`, `get_conversation_agents`, `remove_agent_from_conversation`. Manual ISO-8601 timestamps (no chrono dependency).
- `state.rs` — `AppState` holds `Arc<Mutex<Database>>` + `broadcast::Sender` for SSE + `Option<Arc<TmuxManager>>` for session persistence. Use `state.db()` helper which returns an Axum-compatible error on mutex poisoning.
- `scaffold.rs` — Agent folder scaffolding. Creates `~/.robocaucus/agents/{name}/` with provider-specific instruction files (CLAUDE.md, .codex/instructions.md, GEMINI.md, .copilot-instructions.md).
- `templates.rs` — 9 starter agent templates and 3 starter playbooks (Debate, Code Review Panel, Research & Synthesize). Seeded idempotently at startup via `seed_starter_agents()` and `seed_starter_playbooks()`.
- `tmux.rs` — `TmuxManager` manages `rc-` prefixed tmux sessions. Graceful degradation when tmux is not installed.
- `reconcile.rs` — Startup reconciliation between tmux sessions and DB state. Detects orphans, reattaches live sessions, logs status.
- `routes/` — Axum route handlers:
  - `conversations.rs` — CRUD for conversations + `POST/DELETE/GET /api/conversations/{id}/agents/{agent_id}` for agent-conversation management.
  - `agents.rs` — CRUD for agents + `GET/PUT /api/agents/{id}/config` for per-agent MCP/tool config files.
  - `chat.rs` — `POST /api/chat/send` (single-agent SSE streaming), `POST /api/chat/panel` (fan-out to all conversation agents), `POST /api/chat/debate` (structured multi-turn debate with phases), `GET /api/chat/stream/:id` (reconnect replay). Process-global `ReconnectBuffer` for SSE event replay. Tmux session lifecycle integrated into spawn flow.
  - `playbooks.rs` — CRUD for playbooks + `POST /api/playbooks/{id}/run` (creates conversation, assigns agents from YAML roles). Accepts optional `yaml_content` body for user-filled placeholder values.
  - `config.rs` — `GET /api/config` returns `{ default_workspace }` (backend's cwd) for frontend fallback.
  - `providers.rs` — `GET /api/providers` detects installed CLI tools, returns availability + version info.
  - `pr_review.rs` — `POST /api/pr-review` runs 3-model parallel PR review with debate synthesis.
  - `git.rs` — Read-only git endpoints: graph (paginated log), commit details, diff, status. Uses spawned git CLI.
  - `files.rs` — File browser: directory listing, file read (with binary detection), ripgrep-powered search.
  - `agentmd.rs` — Agent markdown management.
- `adapter/` — `CliAdapter` trait abstracts over AI CLIs. Signature: `spawn(prompt, agent_home, workspace)`. Agent's folder is set as cwd for native config discovery; workspace passed via `--add-dir`/`-C`/`--include-directories`.
  - `claude.rs` — Claude CLI (`-p --verbose --output-format stream-json`). Handles both non-verbose and verbose (nested `.message.content[]`) response formats.
  - `codex.rs` — Codex CLI (`exec --json`). Discovers .codex/instructions.md from agent_home.
  - `gemini.rs` — Gemini CLI (`-p --output-format stream-json`). Discovers GEMINI.md from agent_home.
  - `copilot.rs` — Copilot CLI (`-p --output-format json --allow-all-tools`). Uses --config-dir for agent_home.
- `orchestrate/` — Orchestration engines:
  - `panel.rs` — "Ask Everyone" fan-out: spawns all agents concurrently, merges output into tagged chunks. `select_adapter()` maps provider to adapter.
  - `debate.rs` — Pure state machine (`DebateEngine`): Opening -> Rebuttal(1..N) -> Closing -> Synthesis -> Complete. Each phase spawns the appropriate agent.
- `context.rs` — `build_agent_context()` creates per-agent message context where own messages are "assistant" role, other agents' messages get `[Name responded]:` prefix.
- `mention.rs` — `@mention` parsing: `@Name` and `@"Quoted Name"`, case-insensitive exact/prefix matching, deduplication, routing fallback.

### Frontend structure (frontend/src/)

- `App.tsx` — Main layout: `ActivityBar | Sidebar | TabBar + EditorArea | DevSidebar(optional)`. Manages tabs (chat/file), workspace state (localStorage-backed with backend default fallback), theme state, agent-conversation membership, orchestration controls (Ask Everyone, Start Debate), and playbook modal.
- `hooks/useChat.ts` — SSE streaming hook with reconnection support. Includes `sendMessage` (single agent), `startPanelStream` (multi-agent fan-out), `startDebateStream` (structured debate), and multi-agent SSE reader.
- `lib/api.ts` — REST client for all backend endpoints: conversations (CRUD + agent management), agents (CRUD + provider detection), chat (send/panel/debate), playbooks (CRUD + run with optional yaml_content), config, git, files, search.
- `lib/graphLayout.ts` — Rail-based git graph layout algorithm (ported from markdown-themes).
- `types.ts` — TypeScript interfaces: `Room`, `Agent` (provider + model variant), `Message`, `Playbook`, `EditorTab` (chat/file tab type).
- `themes/` — 31 CSS variable themes (ported from markdown-themes) with Shiki syntax highlighting integration. Themes control all UI colors via a CSS variable bridge that auto-derives missing variables with `color-mix()`.
- `index.css` — `:root` CSS variable defaults for all UI chrome (backgrounds, text, accents, borders) plus theme bridge block.
- `components/`:
  - `Sidebar.tsx` — Activity bar (Chat/Files/Git modes) + content panels. Chat mode: conversation list, agent list, playbooks. Files mode: FileTree. Git mode: GitGraph. Includes WorkspaceSelector and conversation delete (trash icon on hover).
  - `TabBar.tsx` — Horizontal tab bar for editor area. Supports chat and file tabs with icons, close buttons, active highlighting.
  - `ChatInput.tsx`, `ChatMessage.tsx` — Chat UI with @mention autocomplete (scoped to conversation members). ChatMessage uses Streamdown with code, mermaid, and math plugins.
  - `AgentBuilder.tsx` — Provider-aware agent creation/editing with provider-dependent model dropdown, CLI detection (greys out unavailable providers with "Not installed" label), MCP config editor.
  - `RoomMembers.tsx` — Add/remove agents from conversations via API-wired callbacks.
  - `ThemeSelector.tsx` — Dropdown theme picker with color previews, keyboard navigation.
  - `PlaybookBrowser.tsx` — Grid of playbook cards with run button. Parses `{{PLACEHOLDER}}` tokens from YAML and shows modal input form before running.
  - `WorkspaceSelector.tsx` — Dropdown workspace picker with recent projects (localStorage), manual path entry, backend default.
  - `DevSidebar.tsx` — Right-side panel with Files/Git/Search tabs.
  - `FileTree.tsx` — Hierarchical file browser with lazy loading.
  - `CodeViewer.tsx` — Shiki-powered syntax highlighting (40+ languages, CSS variable themes).
  - `git/` — Git visualization: GitGraph, GitGraphCanvas (Canvas 2D), GitGraphRow, CommitDetails, ChangesTree, DiffViewer, StatusBadge.

### Agent folder architecture

Each agent gets a folder at `~/.robocaucus/agents/{name}/` containing CLI-native config files:
- Claude agents: `CLAUDE.md` + `.claude/settings.json`
- Codex agents: `.codex/instructions.md` + `.codex/config.toml`
- Gemini agents: `GEMINI.md`
- Copilot agents: `.copilot-instructions.md`

The adapter spawns with `cwd = agent_home` (for native config discovery) and passes the workspace via CLI flags. Agent instructions live in files, not CLI args. 9 starter agents are scaffolded on first launch.

### Data flow: sending a message

**Single agent (default):**
1. Frontend `useChat.sendMessage()` -> `POST /api/chat/send` with SSE response
2. Backend saves user message to DB, resolves target agent via explicit `agent_id` or `@mention` routing
3. `build_agent_context()` creates identity-aware context window (last 50 messages)
4. CLI adapter spawns inside tmux session (if available), with `cwd = agent_home`, workspace via `--add-dir`
5. Output chunks stream as SSE events to frontend, accumulated text saved to DB on `Done`

**Panel mode (Ask Everyone):**
1. Frontend `useChat.startPanelStream()` -> `POST /api/chat/panel`
2. Backend loads all conversation agents, creates adapters, calls `spawn_panel()`
3. All agents run concurrently, output merged into tagged SSE chunks with `agent_name`
4. Each agent's accumulated text saved separately to DB

**Debate mode:**
1. Frontend `useChat.startDebateStream()` -> `POST /api/chat/debate`
2. Backend creates `DebateEngine`, drives through Opening -> Rebuttal(N) -> Closing -> Synthesis
3. Each phase spawns the appropriate agent, streams via SSE with phase markers
4. Turn text saved to DB with `[Phase]` prefix

## Key patterns

- **Database access**: Always use `state.db()?` which safely handles mutex poisoning. Hold the lock briefly, don't hold across `.await`.
- **SSE events**: types are `text`, `thinking`, `tool_use`, `error`, `done`. Each gets a sequential id for reconnection. Multi-agent events include `agent_name` and `agent_id`.
- **Agent model**: `provider` field = CLI name (claude/codex/gemini/copilot), `model` field = variant (sonnet/opus/o3/gemini-2.5-pro). `agent_home` = path to agent's config folder.
- **Adapter trait**: `spawn(prompt, agent_home, workspace)` -- agent_home sets cwd for config discovery, workspace passed via CLI flags.
- **Tmux persistence**: CLI processes run inside `rc-` prefixed tmux sessions. TmuxManager is optional in AppState -- graceful fallback when tmux is not installed. Reconciliation on startup detects orphaned/live sessions.
- **CSS variable theming**: All UI colors use CSS variables (--bg-primary, --text-primary, --accent, etc.) defined in `:root` with defaults. Theme files override variables. A bridge block using `[class*="theme-"]` auto-derives missing variables via `color-mix()` so themes don't need to define every variable.
- **Ports**: Backend 7331, Frontend 7330. Vite proxies `/api` requests to the backend.
- **DB file**: `robocaucus.db` (SQLite WAL) created in the project root.
- **Startup seeding**: `seed_starter_agents()` and `seed_starter_playbooks()` run idempotently on startup. Agent config folders are scaffolded for each seeded agent.
