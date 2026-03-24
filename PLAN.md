# RoboCaucus

**One app for all your AI subscriptions — and they can collaborate.**

Date: 2026-03-24
Status: Planning

---

## Vision

A beautiful, Discord-style app where your AI subscriptions (Claude, ChatGPT/Codex, Gemini, GitHub Copilot) share conversations in a group chat. Each AI can be configured as an "agent" with a specific role and personality. Agents join chat rooms and collaborate — debating ideas, reviewing plans, researching topics, or answering questions from different angles.

No terminal experience required. No API keys. No extra billing. Just the subscriptions you already pay for, working together in one place.

### Who Is This For?

**Writers and researchers** who pay for Claude Pro and ChatGPT Plus but juggle separate browser tabs, copy-pasting context between them. They want one place where their AIs can build on each other's ideas.

**Knowledge workers** who use Gemini Advanced for research and Claude for writing but have never opened a terminal. Their only interaction with the terminal is double-clicking a start script (or eventually an app icon).

**Developers** who already use AI coding CLIs but want them to collaborate — architecture debates, sprint planning with AI committee members, side-by-side comparisons of how different models approach the same problem.

### Core Thesis

People already pay for 2-4 AI subscriptions but use them in isolation. RoboCaucus is the first tool that lets those subscriptions talk to each other. Under the hood it orchestrates the CLI tools that come with each subscription. The user never needs to know that — they just see a polished chat interface where their AIs collaborate.

### The Experience Gap

Current options for non-technical users:
- **Claude Desktop / claude.ai** — one model, one conversation
- **ChatGPT app / chatgpt.com** — one model, one conversation
- **Gemini app / gemini.google.com** — one model, one conversation

They're paying for 3 subscriptions and getting 3 siloed experiences. RoboCaucus merges them into one, and adds collaboration on top.

---

## Target Subscriptions

| What Users See | What's Under the Hood | Subscription |
|---------------|----------------------|-------------|
| Claude | `claude -p --output-format stream-json` | Claude Pro / Max |
| ChatGPT | `codex exec` | ChatGPT Plus / Pro |
| Gemini | `gemini` (stdin) | Gemini Advanced |
| GitHub Copilot | `gh copilot -p` | GitHub Copilot |

Users never see the CLI commands. They pick "Claude" or "Gemini" from a dropdown when creating an agent. The backend handles the rest.

### Setup Experience

For non-technical users, setup is guided:
1. Download RoboCaucus (eventually: installer / Tauri app)
2. First-launch wizard detects which CLIs are installed
3. For missing CLIs: "You have a Claude subscription? Install the Claude CLI: [one-click link]"
4. Once CLIs are authenticated, RoboCaucus auto-detects them
5. Never touch the terminal again

For technical users: `git clone && ./start.sh`

---

## Layout

Discord-meets-VS-Code layout. Familiar to both chat app users and developers:

```
+--+--------------+-----------------------------------+
|  | My Projects  |                                   |
|  | robocaucus v |  Tab: #brainstorm | Tab: notes.md |
|  +--------------+                                   |
|  | Chats        |  You: Compare these two approaches|
|  |  #brainstorm |       to the intro paragraph      |
|  |  #research   |                                   |
|  |  #api-design |  Editor (Claude): The second      |
|  |              |  approach has stronger voice...    |
|  | Agents       |                                   |
|  |  Editor      |  Researcher (Gemini): I'd note    |
|  |  Researcher  |  that recent studies support...    |
|  |  Critic      |                                   |
|  |  + New Agent |  +-----------------------------+   |
|  |              |  | @Editor can you rework the  |   |
|  |              |  | opening with that in mind?  |   |
|  |              |  +-----------------------------+   |
+--+--------------+-----------------------------------+
```

### Workspace Selector (top of sidebar)
- Dropdown to pick a workspace context (project folder, research folder, writing folder)
- Switching workspace filters chats, file tree, and git graph
- CLIs spawn with that directory as context automatically
- Agents can be global (available everywhere) or workspace-scoped (project-specific)
- For non-coders: "My Projects" is just a folder picker — no terminal paths shown

### Left Sidebar (three modes via activity bar icons)
1. **Chats** — list of rooms/conversations + agent list (the primary view for most users)
2. **Files** — file tree for the selected workspace (for developers and researchers who want to browse/view files alongside chat)
3. **Git** — git graph, branches, working tree status (developer-only; hidden by default, toggled on in settings)

### Editor Area (tabbed)
- Chat tabs, file viewer tabs, and diff tabs are all first-class
- Split view supported (chat + file side by side)
- Content type determined by what you open from the sidebar
- Non-technical users may never leave the chat tab — and that's fine

---

## Agent System

An agent is a model + personality, not just a raw model. Same model can power multiple agents.

### Agent Creation (UI-First)

Non-technical users create agents through a form:
- **Name** — "Editor", "Researcher", "Fact Checker", "Devil's Advocate"
- **Model** — dropdown: Claude, ChatGPT, Gemini, Copilot (only shows installed/available)
- **Color** — color picker for chat messages
- **Personality** — textarea: "You are a meticulous editor focused on clarity and voice. Push back on passive voice and jargon."
- **Scope** — toggle: available everywhere vs this workspace only

Under the hood, this saves as an `.agent.md` file:

```yaml
# agents/editor.agent.md
name: Editor
model: claude
color: "#b388ff"
scope: global
---
You are a meticulous editor focused on clarity and voice.
Push back on passive voice and jargon. When reviewing text,
suggest specific rewrites, not just criticism.
```

Power users can edit these files directly. **SQLite is the source of truth** — the `.agent.md` export is a convenience for version control and sharing. Import/export, not two-way sync (avoids subtle sync bugs).

### Starter Agent Templates

Pre-built agents to get users started immediately:

**For writers:**
- Editor (Claude) — prose style, clarity, voice
- Researcher (Gemini) — fact-finding, citations, context
- Devil's Advocate — challenges assumptions, finds weak arguments

**For developers:**
- Architect (Claude) — system design, scalability
- Critic (Gemini) — code review, edge cases
- Builder (ChatGPT/Codex) — implementation, working code

**For researchers:**
- Analyst (Claude) — deep reasoning, synthesis
- Searcher (Gemini) — broad knowledge, cross-referencing
- Summarizer (ChatGPT) — distilling long content

Users can customize these or create their own from scratch.

### Agent Management
- Create/edit/delete from sidebar via form UI
- Drag agents into rooms to add them as participants
- Agents inherit workspace context when spawned
- Agent templates for quick setup (writer, developer, researcher presets)

---

## Orchestration Modes

Modes are per-room settings that control how agents respond. Accessible via simple UI controls (buttons/dropdown in the room header), not slash commands — though slash commands work too for power users.

### Default (Manual)
- Type a message, pick which agent responds via @mention or dropdown
- `@Editor can you rework the opening paragraph?`
- Most intuitive for new users — feels like a group chat

### Panel Mode (Compare)
- Fan out the same prompt to all room agents in parallel
- Show responses side by side (split view or stacked cards)
- Button: "Ask Everyone" or `/panel How should I approach this?`
- Great for: "I want to see how Claude and Gemini each handle this differently"

### Debate Mode
- Structured N-turn debate with one agent as moderator
- 3 rounds + synthesis: opening, rebuttal, closing, moderator summary
- Button: "Start Debate" with topic field and turn count
- Output: structured summary document
- Great for: decision-making, exploring tradeoffs, testing arguments

### Round-Robin
- Auto-rotate through agents for each response
- Good for brainstorming where you want all perspectives without directing
- Toggle: "Round-Robin" switch in room header

### Room Templates (developer-focused, hidden by default)

**Sprint Planning**
- Pre-configured room with role-based agents: PM, Tech Lead, QA, Security
- Feed in backlog, agents argue priority and sequencing
- Output: draft sprint plan pressure-tested from 4 angles

**T-Shirt Estimation**
- Agents debate context size per task
- XS/S -> Haiku, M -> Sonnet, L/XL -> Opus
- Output: issues tagged with model + estimated tokens, ready for execution

---

## Playbooks (Saved Orchestration Recipes)

In a single-model chat, saved prompts are just convenience. In a multi-agent context, a saved prompt is an **orchestration recipe** — it encodes who talks, in what order, how many rounds, and what the goal is. We call these **playbooks**.

### Format

YAML frontmatter for machine-readable orchestration config, markdown body for the human-readable prompt that gets injected. Same pattern as agent configs — power users edit files, everyone else uses the UI.

```yaml
# playbooks/code-review.playbook.md
name: Code Review
description: Multi-model code review from three angles + synthesis
icon: magnifying-glass
agents:
  - role: reviewer
    focus: "correctness, edge cases, bugs"
  - role: reviewer
    focus: "performance, scalability"
  - role: reviewer
    focus: "security, input validation"
  - role: synthesizer
    focus: "summarize all findings, prioritize by severity"
flow: round-robin-then-synthesize
input:
  - name: code
    type: text
    label: "Paste or attach the code to review"
---
Review the following code. Each reviewer examines it through
their specific lens. After all reviews are complete, the
synthesizer combines findings into a single prioritized
action list with severity ratings.
```

### How It Works

1. User clicks a playbook from the sidebar (or types `/playbook code-review`)
2. UI shows the input form (just the fields defined in `input:`)
3. User pastes their code / types their topic / attaches a file
4. RoboCaucus creates a room, assigns agents to roles, and runs the flow
5. User watches the orchestrated conversation unfold

The playbook defines the choreography. The user just provides the input.

### Flows

Flows define the turn structure. Built-in flow types:

- **round-robin-then-synthesize** — each agent takes one turn in order, final agent synthesizes
- **debate(turns: N)** — agents alternate for N rounds, moderator summarizes
- **parallel-then-compare** — all agents respond simultaneously, then critique each other
- **chain** — each agent builds on the previous one's output (plan -> critique -> implement)
- **free** — no structure, agents respond to @mentions (default room behavior)

### Starter Playbooks

**For everyone:**
- **Compare Approaches** — input: a problem. Each agent proposes a solution, then they critique each other's. Flow: parallel-then-compare.
- **Proposal Stress Test** — input: paste your proposal. Agents role-play stakeholders (investor, customer, engineer, skeptic) and poke holes. Flow: round-robin-then-synthesize.
- **Debate** — input: topic + stance A vs stance B. Configurable turns. Auto-moderator summary. Flow: debate(turns: 3).

**For writers:**
- **Draft Workshop** — input: paste your draft. Editor improves prose, researcher adds citations, critic challenges logic, author agent produces final revision. Flow: chain.
- **Research Brief** — input: topic/question. One agent reasons broadly, another fact-checks, third synthesizes into a structured summary. Flow: chain.

**For developers:**
- **Code Review** — input: code. Three reviewers (correctness, performance, security) + synthesizer. Flow: round-robin-then-synthesize.
- **Architecture Decision** — input: problem + constraints. Agents propose competing architectures, debate tradeoffs, moderator produces ADR. Flow: debate(turns: 3).
- **Sprint Planning** — input: backlog items. PM, Tech Lead, QA, Security agents argue priority. Flow: round-robin-then-synthesize.

### Playbook UI

- Sidebar section: "Playbooks" with categorized list
- Each playbook shows: icon, name, description, required agents
- Click to open input form -> fills in and hits "Run"
- Running playbook creates a room with the conversation visible in real-time
- Users can interrupt/redirect mid-playbook (it's still a chat, not a black box)
- Create custom playbooks via form UI or by editing .playbook.md files

---

## Provider Capability Matrix

Validate before building. These CLIs are the foundation — treat them as external dependencies, not guaranteed APIs.

| Capability | claude -p | codex exec | gemini | gh copilot -p |
|-----------|-----------|------------|--------|---------------|
| **Install friction** | `npm i -g @anthropic-ai/claude-code` | `npm i -g @openai/codex` | ❌ NO OFFICIAL CLI | Pre-installed via `gh copilot` wrapper |
| **Auth method** | OAuth via browser | OAuth via browser | N/A | GitHub OAuth via `gh auth` |
| **Streaming output** | `--output-format stream-json` (JSONL) | `--json` (JSONL with type events) | N/A | Unknown — likely plain text |
| **Cancellation** | Kill process (SIGTERM) | Kill process (SIGTERM) | N/A | Kill process (SIGTERM) |
| **Concurrent sessions** | Yes (standard process isolation) | Yes (per-session thread_id) | N/A | Likely yes (needs verification) |
| **Rate limits** | Subscription-tier dependent | Subscription-tier dependent | N/A | Subscription-tier dependent |
| **System prompt injection** | `--append-system-prompt` ✓ | Not supported ✗ | N/A | Unknown |
| **Context window** | Model-dependent (200k) | Model-dependent (gpt-5.4, o3) | N/A | Unknown |
| **Output format** | Structured JSONL with tool use, thinking, message events | JSONL with turn/item events + usage metadata | N/A | Unknown (likely plain text) |
| **CWD/workspace** | Implicit (cwd); `--add-dir` for extra context | `-C, --cd <DIR>` ✓ | N/A | Unknown |
| **TOS risk** | CLI is official, pipe mode documented | CLI is official, exec mode documented | N/A | Official — wrapper via gh |

**Validated 2026-03-24.** Claude and Codex are production-ready for MVP. Gemini has no official CLI — remove from MVP scope (consider API-based alternative post-MVP). GitHub Copilot is functional but less documented — mark as experimental/stretch goal.

### Codex CLI Details

Key flags: `codex exec [PROMPT]`, `--json` (JSONL output), `-C <DIR>` (working directory), `-m <MODEL>` (model selection), `-s <sandbox-mode>`. Output events: `thread.started`, `turn.started`, `item.completed` (with text), `turn.completed` (with usage). Has built-in MCP support.

---

## Data & Privacy

RoboCaucus doesn't send data anywhere new — it uses the same CLIs your subscriptions already provide. If you're already typing into claude.ai and chatgpt.com, you've accepted those providers' terms.

### What stays local
- **Conversations, messages, agents** — SQLite on your machine. Never leaves your disk.
- **Agent configs** — `.agent.md` files on your disk.

### What goes to providers
- **Your prompts + conversation context** — sent to whichever CLI the agent uses, same as typing directly into that provider's app.
- **File attachments** — sent to the agent's provider when you attach them. Same as pasting into ChatGPT.

### UI clarity
- Each agent shows its provider (Anthropic, OpenAI, Google, GitHub) via a small badge/icon
- Room member list makes it obvious which providers are in the conversation
- No extra confirmation dialogs — users chose these subscriptions, they know where their data goes

---

## Architecture

```
[React Frontend]  <--SSE/WS-->  [Rust Backend (Axum)]  --spawns-->  [claude -p]
                                                                      [codex exec]
                                                                      [gemini]
                                                                      [gh copilot -p]
                                       |
                                   [SQLite]
                              (conversations, messages, agents)
```

### Frontend — React 19 + Vite + Tailwind v4
- Streamdown for streaming markdown rendering (mermaid, math, syntax highlighting)
- 30 CSS-variable themes (from markdown-themes)
- SSE with reconnect buffering for streaming responses
- Split view for chat + file side by side

### Backend — Rust (Axum 0.8 + Tokio)
- CLI orchestrator: spawn subscription CLIs via tokio::process, stream stdout via SSE/WebSocket
- Agent config management (CRUD for .agent.md files)
- Conversation persistence (SQLite via rusqlite or sqlx)
- File tree + git graph API endpoints
- Multi-model context builder: each agent gets identity-aware context
- Process management with timeouts, abort, concurrent spawning
- tmux-backed process persistence (see below)

### tmux Persistence Layer

All CLI processes run inside tmux sessions so they survive browser closes, backend restarts, and network drops. The user never knows tmux exists — they just see that their agents kept working while they were away.

**Session naming:** All RoboCaucus sessions use the `rc-` prefix (e.g., `rc-conv-abc123-claude-architect`) so they're identifiable and don't collide with other tmux usage.

**Lifecycle:**
```
User sends message → Backend creates tmux session → Spawns CLI inside it
                                                     ↓
Browser closes → tmux session keeps running → CLI finishes → Output captured
                                                     ↓
Browser reopens → Backend reconciles tmux ls vs SQLite → Replays missed output via SSE
```

**Key operations (all via tokio::process::Command):**
- `tmux new-session -d -s rc-{id} -x 200 -y 50` — create detached session
- `tmux send-keys -t rc-{id} '{cli_command}' Enter` — run CLI in session
- `tmux capture-pane -t rc-{id} -p` — grab output
- `tmux ls -F '#{session_name}'` — list active sessions for reconciliation
- `tmux kill-session -t rc-{id}` — cleanup after completion

**Reconciliation on startup:**
1. Backend starts, queries `tmux ls` for all `rc-` sessions
2. Compares against SQLite conversation state
3. Orphaned sessions (no matching conversation) → offer cleanup
4. In-progress conversations with live sessions → reattach output streaming
5. Completed conversations with dead sessions → already persisted, no action

**Patterns borrowed from TabzChrome:**
- Prefix-based session naming for isolation
- Ghost detection (orphaned sessions that survived a crash)
- Reattachment on reconnect
- Rate-limited spawning to prevent runaway processes

**Why tmux over raw tokio::process:**
- Process survives backend crash/restart
- Process survives browser disconnect
- Debate mode with 4 agents and 3 turns = 12 sequential CLI invocations that might take minutes — user shouldn't have to keep the tab open
- Free session multiplexing — can inspect agent sessions manually via `tmux attach` if needed for debugging

### Data Model

**Conversations** (rooms)
- id, title, workspace_path, created_at, updated_at
- orchestration_mode (manual, panel, debate, round-robin)
- agent_ids[] (which agents are in this room)

**Messages**
- id, conversation_id, agent_id (nullable for user messages)
- role (user | assistant), content, timestamp
- model, usage metadata, cost, duration

**Agents**
- id, name, model, color, scope, system_prompt
- workspace_path (if workspace-scoped)

---

## Code Reuse Map

### Rust Backend — from PocketForge + CodeFactory

Both projects use Axum 0.8 + Tokio with nearly identical feature sets. PocketForge has cleaner module separation (routes/ directory); CodeFactory has everything in main.rs (3852 lines). Prefer PocketForge's structure.

| Component | Source | Files | What to Take |
|-----------|--------|-------|-------------|
| **Axum server scaffold** | PocketForge | `backend/src/main.rs` (384 lines) | Router setup, CORS, state injection, server startup |
| **AppState pattern** | PocketForge | `backend/src/state.rs` (56 lines) | Arc<Mutex<>> shared state with broadcast channels |
| **File tree API** | PocketForge | `backend/src/routes/files.rs` (1098 lines) | List, read, create, rename, delete, diff, search endpoints |
| **Git graph + operations** | PocketForge | `backend/src/routes/git.rs` (904 lines) | Graph visualization data, commit details, diff, status, stage/unstage |
| **WebSocket infrastructure** | PocketForge | `backend/src/ws.rs` (590 lines) | WS handler pattern, message serialization, biased select for priority |
| **Claude CLI spawning** | PocketForge | `backend/src/routes/claude.rs` (421 lines) | Process spawn, JSONL streaming, env var handling, session detection |
| **Process management** | Both | `backend/src/terminal.rs` | tokio::process::Command patterns, timeout handling, stdout/stderr capture |
| **Tracing + log broadcast** | PocketForge | `backend/src/log_layer.rs` (95 lines) | Custom tracing layer, ring buffer, broadcast channel |
| **Config management** | PocketForge | `backend/src/config.rs` (207 lines) | JSON config load/save pattern (adapt for agent configs) |
| **Beads integration** | PocketForge | `backend/src/routes/beads.rs` (139 lines) | CLI spawning for ggbd, JSON wrapping |

**What to add fresh in Rust:**
- SQLite layer (rusqlite or sqlx) for conversations, messages, agents
- Multi-CLI orchestrator module: adapter trait + implementations for claude/codex/gemini/copilot
- Agent config parser (.agent.md frontmatter + body)
- SSE endpoint for chat streaming (or adapt existing WebSocket pattern)
- @mention routing logic
- Orchestration mode engine (panel fan-out, debate turn management, round-robin)
- Context builder: identity-aware message history per agent (port logic from personal-homepage's conversation-multimodel.ts)

### React Frontend — from markdown-themes

| Component | Source File | What to Take |
|-----------|------------|-------------|
| **Streamdown + Mermaid + Math** | `src/components/MarkdownViewer.tsx` | Entire streaming markdown setup with theme-aware mermaid rendering |
| **Chat message rendering** | `src/components/chat/ChatMessage.tsx` | Message bubbles, tool-use blocks, thinking blocks — adapt for agent colors |
| **Chat input** | `src/components/chat/ChatInput.tsx` | Text input with auto-expand — add @mention autocomplete |
| **Chat panel** | `src/components/chat/ChatPanel.tsx` | Multi-tab conversation UI — adapt tabs to be rooms |
| **Chat state hook** | `src/hooks/useAIChat.ts` | SSE streaming, reconnect buffering, conversation lifecycle — adapt for multi-agent |
| **Chat context** | `src/context/AIChatContext.tsx` | Global state provider pattern |
| **30 themes** | `src/themes/*.css` + `src/themes/index.ts` | All CSS-variable theme files wholesale |
| **Git graph** | `src/components/git/GitGraph.tsx` | Commit history visualization |
| **Git working tree** | `src/components/git/WorkingTree.tsx` | Staged/unstaged file view |
| **Split view** | `src/components/SplitView.tsx` | Split pane container |
| **Tab management** | `src/hooks/useTabManager.ts` | Tab state management |
| **Code viewer** | `src/components/viewers/CodeViewer.tsx` | Shiki syntax highlighting + diff |
| **Diff viewer** | `src/components/viewers/DiffViewer.tsx` | Side-by-side diff display |
| **JSONL conversation viewer** | `src/components/viewers/ConversationMarkdownViewer.tsx` | JSONL log rendering with collapsible tool blocks |
| **API client** | `src/lib/api.ts` | Backend API client + WebSocket helpers — adapt endpoints |

**What to build fresh in React:**
- Discord-style sidebar (activity bar + rooms/files/git panels)
- Workspace selector dropdown
- Agent builder/editor UI
- @mention autocomplete in chat input
- Orchestration mode controls per room (panel, debate, round-robin toggles)
- Agent color-coded message rendering
- Room member management (add/remove agents)

### TypeScript Reference — from personal-homepage

These files are not directly reusable (wrong language for backend, wrong framework for frontend) but contain **logic to port**:

| Logic | Source | Port To |
|-------|--------|---------|
| **CLI adapter patterns** | `lib/model-invoker.ts` (362 lines) | Rust: adapter trait per CLI with spawn args, streaming, timeout |
| **Model registry** | `lib/models-registry.ts` (243 lines) | Rust: model definitions with metadata, pricing, CLI commands |
| **Identity-aware context** | `lib/ai/_archived/conversation-multimodel.ts` (358 lines) | Rust: buildModelContext() — each agent sees own msgs as assistant, others as labeled user msgs |
| **JSONL parsing** | `lib/ai/jsonl-parser.ts` (275 lines) | Rust: serde-based JSONL stream parser for Claude output format |

---

## What NOT to Bring

Features from markdown-themes / PocketForge / CodeFactory that are out of scope:

- **Terminal/PTY UI** — no embedded terminal or xterm.js; users have their own terminals. (tmux is used on the backend for process persistence, but is invisible to users)
- **File watcher / "follow AI edits"** — not watching files change in real-time
- **Workspace streaming detection** — no "AI is writing..." file mode
- **Termux API** — mobile-specific, not relevant
- **Server-side VT100 parsing** (alacritty_terminal) — no terminal emulation needed
- **Live reload WebSocket** — dev tooling, not product feature
- **Notes system** — out of scope
- **Process/port monitoring** — out of scope

---

## Competitive Landscape

**Nobody has built this for the terminal/local tooling space.**

### Closest tools (all single-model):
- **AIChat** (Rust, 9.6k stars) — 20+ providers but API-key based, one model at a time
- **Crush** (Go, Charm) — mid-session model switching but single-model chat
- **llm** (Python, Simon Willison) — great for scripting, sequential not side-by-side

### Web-based comparators (well-served, different market):
- LM Arena, TypingMind, T3 Chat, AiZolo, Multi Chats

### The gap:
No tool orchestrates subscription-based CLIs into a multi-agent group chat with identity-aware context. The pieces exist (every CLI has pipe mode, TUI/web frameworks are mature) but nobody has assembled them.

---

## Rendering Stack

### Streamdown (Vercel)
- Streaming markdown renderer, purpose-built for LLM output
- Handles incomplete markdown mid-stream
- Plugins: @streamdown/code (Shiki), @streamdown/mermaid, @streamdown/math (KaTeX)
- Already integrated in markdown-themes — proven setup

### Also noted for reference:
- **streamdown-rs** — Rust port, if any rendering moves server-side
- **Glow** (Charm) — terminal markdown renderer, not needed for web UI

---

## Demo-Worthy Moments

### For everyone:
- "Ask Everyone" on a question and seeing three different AI perspectives appear side by side
- `@Gemini what do you think about what Claude just said?` — and it actually has the context
- Creating a "Fact Checker" agent and watching it challenge your "Writer" agent's claims
- Switching from cyberpunk theme to zen theme mid-conversation
- A debate between two AIs about the best approach to your problem, with a third moderating

### For developers:
- Three AIs debating your architecture with mermaid diagrams inline
- Sprint planning committee arguing priorities while you drink coffee
- Split view: debate in left pane, the actual code file in right pane
- Panel comparing how Claude, Gemini, and Codex each implement the same function

### For writers/researchers:
- An Editor and Researcher collaborating on your draft — one critiques style, the other finds supporting evidence
- "Ask Everyone" on a thesis statement and comparing how each model strengthens it
- Debate mode: two AIs argue for and against your hypothesis while a third synthesizes

---

## Open Questions

- **TTS mode?** — Each agent gets a voice. Fun demo, low priority. Could use OpenAI TTS API or local piper/espeak.
- **Beads integration?** — Feed beads backlog into sprint planning rooms. Natural fit but not MVP.
- **Local models?** — Ollama/LM Studio adapter. Nice to have, not core value prop (core is subscription CLIs).
- **Export formats?** — Markdown, RFC template, sprint plan template. Post-MVP.
- **Auth/multi-user?** — Local-only for now. Single user, localhost.
- **rusqlite vs sqlx?** — rusqlite is simpler (sync, compile-time schema optional). sqlx has async + compile-time checked queries. Either works for this scale.
- **Tauri desktop app?** — Wrapping in Tauri gives a native app experience (double-click to launch, no terminal). Natural fit since backend is already Rust. Post-MVP but high impact for non-technical audience.
- **Onboarding flow?** — First-launch wizard that detects installed CLIs and guides authentication. Critical for non-technical users. Should be part of MVP or fast-follow.
- **File attachments in chat?** — Drag a file into chat to give agents context (like Discord). "Here's my draft, everyone review it." Would need to pipe file content into CLI prompts.

---

## MVP Scope

The minimum to be useful and demo-worthy. Focused on the chat experience — the thing that's new.

### MVP (the core loop)
1. Chat sidebar with room list + agent list
2. Create/join rooms with multiple agents
3. Agent creation via form UI (name, model, color, personality)
4. @mention routing to specific agents
5. Streaming responses with Streamdown rendering (markdown, code highlighting)
6. Two CLI adapters working (claude + one other)
7. One theme working (pick the best one)
8. SQLite persistence (conversations, messages, agents)
9. CLI detection (which subscriptions are available on this machine)

### Fast-Follow (makes it sticky)
- "Ask Everyone" panel mode
- 3-5 starter agent templates (writer, developer, researcher presets)
- Workspace selector
- 5+ themes
- Debate mode
- 2-3 starter playbooks (Compare Approaches, Draft Workshop, Code Review)

### Iteration (full vision)
- Full playbook system with custom creation UI and all flow types
- All starter playbooks (stress test, research brief, architecture decision, sprint planning)
- All 30 themes + mermaid + math rendering
- File tree + code viewer
- Git graph (developer mode)
- Onboarding wizard for CLI installation
- Round-robin mode
- Export conversations
- Tauri desktop app wrapper
- TTS mode
- Beads integration
- File attachments in chat
