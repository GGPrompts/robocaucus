use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::OnceLock;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

use crate::adapter::claude::ClaudeAdapter;
use crate::adapter::codex::CodexAdapter;
use crate::adapter::copilot::CopilotAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::{ChunkType, CliAdapter};
use crate::context::{self, ContextMessage};
use crate::mention;
use crate::orchestrate::debate::{DebateConfig, DebateEngine};
use crate::orchestrate::panel::{self, PanelConfig};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Reconnect buffer — stores the last N SSE events per conversation
// ---------------------------------------------------------------------------

const MAX_BUFFERED_EVENTS: usize = 100;

/// A single buffered SSE event, identified by a sequential id.
#[derive(Debug, Clone)]
struct BufferedEvent {
    id: u64,
    event_type: String,
    data: String,
}

/// Per-conversation ring buffer of recent SSE events.
type ConversationBuffer = VecDeque<BufferedEvent>;

/// Shared reconnect buffer (process-global singleton).
struct ReconnectBuffer {
    inner: Mutex<HashMap<String, ConversationBuffer>>,
}

impl ReconnectBuffer {
    fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Push an event into the buffer for a conversation, returning the assigned id.
    async fn push(&self, conversation_id: &str, event_type: &str, data: &str) -> u64 {
        let mut map = self.inner.lock().await;
        let buf = map
            .entry(conversation_id.to_owned())
            .or_insert_with(VecDeque::new);

        let next_id = buf.back().map_or(1, |e| e.id + 1);

        buf.push_back(BufferedEvent {
            id: next_id,
            event_type: event_type.to_owned(),
            data: data.to_owned(),
        });

        // Trim to keep only the last MAX_BUFFERED_EVENTS.
        while buf.len() > MAX_BUFFERED_EVENTS {
            buf.pop_front();
        }

        next_id
    }

    /// Return all events after `last_event_id` for a given conversation.
    async fn events_since(
        &self,
        conversation_id: &str,
        last_event_id: u64,
    ) -> Vec<BufferedEvent> {
        let map = self.inner.lock().await;
        match map.get(conversation_id) {
            Some(buf) => buf
                .iter()
                .filter(|e| e.id > last_event_id)
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }
}

/// Global singleton for the reconnect buffer.
fn reconnect_buffer() -> &'static ReconnectBuffer {
    static INSTANCE: OnceLock<ReconnectBuffer> = OnceLock::new();
    INSTANCE.get_or_init(ReconnectBuffer::new)
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ChatSendRequest {
    pub conversation_id: String,
    pub content: String,
    pub agent_id: Option<String>, // explicit target; if None, use @mention routing
}

#[derive(Deserialize)]
pub struct PanelRequest {
    pub conversation_id: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct DebateRequest {
    pub conversation_id: String,
    pub topic: String,
    pub num_rounds: Option<usize>,
}

#[derive(Deserialize)]
pub struct ReconnectQuery {
    pub last_event_id: Option<u64>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn chunk_type_to_event_name(ct: &ChunkType) -> &'static str {
    match ct {
        ChunkType::Text => "text",
        ChunkType::Thinking => "thinking",
        ChunkType::ToolUse => "tool_use",
        ChunkType::Error => "error",
        ChunkType::Done => "done",
    }
}

/// Join non-system context messages into a conversation prompt for the CLI adapter.
fn context_to_conversation_prompt(messages: &[ContextMessage]) -> String {
    let mut parts = Vec::new();
    for msg in messages {
        if msg.role == "system" {
            continue;
        }
        match msg.role.as_str() {
            "assistant" => parts.push(format!("[Assistant]: {}", msg.content)),
            "user" => parts.push(format!("[User]: {}", msg.content)),
            other => parts.push(format!("[{}]: {}", other, msg.content)),
        }
    }
    parts.join("\n\n")
}

// ---------------------------------------------------------------------------
// POST /chat/send — stream SSE response from an AI agent
// ---------------------------------------------------------------------------

async fn chat_send(
    State(state): State<AppState>,
    Json(req): Json<ChatSendRequest>,
) -> impl IntoResponse {
    let conversation_id = req.conversation_id.clone();

    // ------------------------------------------------------------------
    // 1. Validate conversation exists
    // ------------------------------------------------------------------
    let conversation = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        match db.get_conversation(&conversation_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!("Conversation '{}' not found", conversation_id),
                    }),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("Database error: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    // ------------------------------------------------------------------
    // 2. Save user message to DB
    // ------------------------------------------------------------------
    {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        if let Err(e) = db.create_message(
            &conversation_id,
            None, // user message has no agent_id
            "user",
            &req.content,
            None,
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("Failed to save user message: {e}"),
                }),
            )
                .into_response();
        }
    }

    // ------------------------------------------------------------------
    // 3. Determine target agent(s)
    // ------------------------------------------------------------------
    let room_agents = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.get_conversation_agents(&conversation_id)
            .unwrap_or_default()
    };

    let (target_agent_ids, _clean_content) = if let Some(ref explicit_id) = req.agent_id {
        (vec![explicit_id.clone()], req.content.clone())
    } else {
        let (ids, clean) = mention::route_message(&req.content, &room_agents, None);
        (ids, clean)
    };

    if target_agent_ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "No target agent: provide agent_id or use @mention".to_owned(),
            }),
        )
            .into_response();
    }

    // For now we target the first resolved agent (multi-agent fan-out is future work).
    let target_agent_id = &target_agent_ids[0];

    // ------------------------------------------------------------------
    // 4. Get target agent from DB
    // ------------------------------------------------------------------
    let target_agent = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        match db.get_agent(target_agent_id) {
            Ok(Some(a)) => a,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!("Agent '{}' not found", target_agent_id),
                    }),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("Database error: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    // ------------------------------------------------------------------
    // 5. Build identity-aware context
    // ------------------------------------------------------------------
    let messages = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.list_messages(&conversation_id).unwrap_or_default()
    };

    let all_agents = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.get_conversation_agents(&conversation_id)
            .unwrap_or_default()
    };

    let context_messages = context::build_agent_context(
        &messages,
        &all_agents,
        &target_agent,
        None, // room-level system prompt could be added later
        Some(50),
    );

    // ------------------------------------------------------------------
    // 6. Select CLI adapter based on agent provider
    // ------------------------------------------------------------------
    let provider = if target_agent.provider.is_empty() {
        // Backwards compat: fall back to model field for legacy agents
        target_agent.model.as_str()
    } else {
        target_agent.provider.as_str()
    };
    let adapter: Box<dyn CliAdapter> = match provider {
        "claude" => Box::new(ClaudeAdapter::new(120)),
        "codex" => Box::new(CodexAdapter::new(120)),
        "copilot" => Box::new(CopilotAdapter::new(120)),
        "gemini" => Box::new(GeminiAdapter::new(120)),
        other => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(ErrorBody {
                    error: format!(
                        "Unsupported provider '{}': only 'claude', 'codex', 'copilot', and 'gemini' are supported",
                        other
                    ),
                }),
            )
                .into_response();
        }
    };

    // ------------------------------------------------------------------
    // 7. Spawn the adapter (optionally wrapped in a tmux session)
    // ------------------------------------------------------------------
    let prompt = context_to_conversation_prompt(&context_messages);
    let agent_home = if target_agent.agent_home.is_empty() {
        None
    } else {
        Some(target_agent.agent_home.as_str())
    };
    let workspace = conversation
        .workspace_path
        .as_deref()
        .or(target_agent.workspace_path.as_deref());

    // Create a tmux session for tracking this interaction (conversation + agent).
    // The session ID encodes both so reconciliation can map sessions to conversations.
    let tmux_session_id = format!("{}-{}", conversation_id, target_agent.id);
    let tmux_active = if let Some(ref tmux) = state.tmux {
        match tmux.create_session(&tmux_session_id, 200, 50).await {
            Ok(_session) => {
                tracing::debug!(
                    "tmux session created for conv={} agent={}",
                    conversation_id,
                    target_agent.id,
                );
                true
            }
            Err(e) => {
                tracing::warn!("failed to create tmux session: {e} — falling back to direct spawn");
                false
            }
        }
    } else {
        false
    };

    let mut rx = match adapter.spawn(&prompt, agent_home, workspace).await {
        Ok(rx) => rx,
        Err(e) => {
            // Clean up tmux session on spawn failure.
            if tmux_active {
                if let Some(ref tmux) = state.tmux {
                    let _ = tmux.kill_session(&tmux_session_id).await;
                }
            }
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("Failed to spawn adapter: {e}"),
                }),
            )
                .into_response();
        }
    };

    // ------------------------------------------------------------------
    // 8. Build SSE stream
    // ------------------------------------------------------------------
    let conv_id = conversation_id.clone();
    let db = state.db.clone();
    let broadcast_tx = state.tx.clone();
    let agent_id = target_agent.id.clone();
    let agent_model = target_agent.model.clone();
    let buf = reconnect_buffer();

    // Clone tmux handle for the spawned task so it can clean up the session on completion.
    let tmux_for_task = if tmux_active { state.tmux.clone() } else { None };
    let tmux_session_id_for_task = tmux_session_id.clone();

    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);

    // Fire-and-forget: the JoinHandle is intentionally dropped. The spawned
    // task drives the SSE stream independently; if it panics, tokio logs it and
    // the client sees a broken stream — no server-wide impact.
    tokio::spawn(async move {
        let mut full_response = String::new();
        let mut had_text = false;

        while let Some(chunk) = rx.recv().await {
            let event_name = chunk_type_to_event_name(&chunk.chunk_type);

            // Accumulate text content for the final DB save.
            if matches!(chunk.chunk_type, ChunkType::Text) {
                full_response.push_str(&chunk.content);
                had_text = true;
            }

            let data = serde_json::json!({
                "content": chunk.content,
                "agent_id": agent_id,
                "conversation_id": conv_id,
            })
            .to_string();

            // Push to reconnect buffer and get the sequential id.
            let event_id = buf.push(&conv_id, event_name, &data).await;

            let sse_event = Event::default()
                .event(event_name)
                .id(event_id.to_string())
                .data(&data);

            if sse_tx.send(Ok(sse_event)).await.is_err() {
                // Client disconnected.
                break;
            }

            // Broadcast for any listeners on the broadcast channel.
            let _ = broadcast_tx.send(format!(
                "event:{event_name} conv:{conv_id} data:{data}"
            ));

            // On Done: save the accumulated response to DB.
            if matches!(chunk.chunk_type, ChunkType::Done) {
                if had_text {
                    match db.lock() {
                        Ok(db) => {
                            if let Err(e) = db.create_message(
                                &conv_id,
                                Some(&agent_id),
                                "assistant",
                                &full_response,
                                Some(&agent_model),
                            ) {
                                tracing::error!(
                                    "Failed to save assistant message for conversation {}: {e}",
                                    conv_id
                                );
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to acquire DB lock to save assistant message for conversation {}: {e}",
                                conv_id
                            );
                        }
                    }
                }
                break;
            }
        }

        // Clean up the tmux session now that the adapter has finished.
        if let Some(ref tmux) = tmux_for_task {
            if let Err(e) = tmux.kill_session(&tmux_session_id_for_task).await {
                tracing::debug!("tmux session cleanup for {}: {e}", tmux_session_id_for_task);
            }
        }
    });

    let stream = ReceiverStream::new(sse_rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"))
        .into_response()
}

// ---------------------------------------------------------------------------
// POST /chat/panel — "Ask Everyone" fan-out to all conversation agents
// ---------------------------------------------------------------------------

async fn chat_panel(
    State(state): State<AppState>,
    Json(req): Json<PanelRequest>,
) -> impl IntoResponse {
    let conversation_id = req.conversation_id.clone();

    // 1. Validate conversation exists
    let conversation = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        match db.get_conversation(&conversation_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!("Conversation '{}' not found", conversation_id),
                    }),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("Database error: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    // 2. Save user message to DB
    {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        if let Err(e) = db.create_message(
            &conversation_id,
            None,
            "user",
            &req.content,
            None,
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("Failed to save user message: {e}"),
                }),
            )
                .into_response();
        }
    }

    // 3. Load all agents assigned to this conversation
    let room_agents = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.get_conversation_agents(&conversation_id)
            .unwrap_or_default()
    };

    if room_agents.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "No agents assigned to this conversation".to_owned(),
            }),
        )
            .into_response();
    }

    // 4. Build adapters for each agent
    let mut agents_with_adapters: Vec<(crate::db::Agent, Box<dyn CliAdapter>)> = Vec::new();
    for agent in &room_agents {
        let provider = if agent.provider.is_empty() {
            agent.model.as_str()
        } else {
            agent.provider.as_str()
        };
        match panel::select_adapter(provider) {
            Ok(adapter) => {
                agents_with_adapters.push((agent.clone(), adapter));
            }
            Err(e) => {
                tracing::warn!(
                    "Skipping agent '{}' ({}): {e}",
                    agent.name,
                    provider
                );
            }
        }
    }

    if agents_with_adapters.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "No agents have supported providers".to_owned(),
            }),
        )
            .into_response();
    }

    // 5. Build context-aware prompt
    let messages = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.list_messages(&conversation_id).unwrap_or_default()
    };

    // Use recent conversation context if available, otherwise the raw prompt
    let context_prompt = if messages.len() > 1 {
        let recent: Vec<_> = messages.iter().rev().take(20).collect();
        let mut parts = Vec::new();
        for msg in recent.iter().rev() {
            let prefix = if msg.agent_id.is_some() {
                "[Agent]"
            } else {
                "[User]"
            };
            parts.push(format!("{prefix}: {}", msg.content));
        }
        parts.join("\n\n")
    } else {
        req.content.clone()
    };

    let agent_ids: Vec<String> = agents_with_adapters
        .iter()
        .map(|(a, _)| a.id.clone())
        .collect();
    let panel_config = PanelConfig {
        prompt: context_prompt,
        conversation_id: conversation_id.clone(),
        agent_ids,
    };

    // 6. Spawn the panel
    let mut rx = panel::spawn_panel(&panel_config, agents_with_adapters);

    // 7. Build SSE stream from tagged chunks
    let conv_id = conversation_id.clone();
    let db = state.db.clone();
    let broadcast_tx = state.tx.clone();
    let buf = reconnect_buffer();
    let _conv = conversation; // keep alive for workspace_path

    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(128);

    tokio::spawn(async move {
        // Track per-agent accumulated text for DB saves
        let mut agent_texts: HashMap<String, String> = HashMap::new();

        while let Some(tagged) = rx.recv().await {
            let event_name = chunk_type_to_event_name(&tagged.chunk.chunk_type);

            if matches!(tagged.chunk.chunk_type, ChunkType::Text) {
                agent_texts
                    .entry(tagged.agent_id.clone())
                    .or_default()
                    .push_str(&tagged.chunk.content);
            }

            let data = serde_json::json!({
                "content": tagged.chunk.content,
                "agent_id": tagged.agent_id,
                "agent_name": tagged.agent_name,
                "conversation_id": conv_id,
            })
            .to_string();

            let event_id = buf.push(&conv_id, event_name, &data).await;

            let sse_event = Event::default()
                .event(event_name)
                .id(event_id.to_string())
                .data(&data);

            if sse_tx.send(Ok(sse_event)).await.is_err() {
                break;
            }

            let _ = broadcast_tx.send(format!(
                "event:{event_name} conv:{conv_id} data:{data}"
            ));

            // On Done for a specific agent: save their accumulated response
            if matches!(tagged.chunk.chunk_type, ChunkType::Done) {
                if let Some(text) = agent_texts.get(&tagged.agent_id) {
                    if !text.is_empty() {
                        match db.lock() {
                            Ok(db) => {
                                let _ = db.create_message(
                                    &conv_id,
                                    Some(&tagged.agent_id),
                                    "assistant",
                                    text,
                                    None,
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to save panel response for agent {}: {e}",
                                    tagged.agent_id
                                );
                            }
                        }
                    }
                }
            }
        }

        // Send a final "done" event so the frontend knows the panel is complete
        let done_data = serde_json::json!({
            "content": "",
            "conversation_id": conv_id,
        })
        .to_string();
        let event_id = buf.push(&conv_id, "done", &done_data).await;
        let done_event = Event::default()
            .event("done")
            .id(event_id.to_string())
            .data(&done_data);
        let _ = sse_tx.send(Ok(done_event)).await;
    });

    let stream = ReceiverStream::new(sse_rx);
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

// ---------------------------------------------------------------------------
// POST /chat/debate — structured multi-turn debate between agents
// ---------------------------------------------------------------------------

async fn chat_debate(
    State(state): State<AppState>,
    Json(req): Json<DebateRequest>,
) -> impl IntoResponse {
    let conversation_id = req.conversation_id.clone();
    // TODO: [code-review] Validate num_rounds upper bound (e.g., max 20) to prevent DoS (85%)
    let num_rounds = req.num_rounds.unwrap_or(2);

    // 1. Validate conversation
    let conversation = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        match db.get_conversation(&conversation_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ErrorBody {
                        error: format!("Conversation '{}' not found", conversation_id),
                    }),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("Database error: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    // 2. Save the debate topic as a user message
    {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        // TODO: [code-review] Validate topic length (e.g., max 10000 chars) (82%)
        let topic_msg = format!("[Debate Topic]: {}", req.topic);
        if let Err(e) = db.create_message(
            &conversation_id,
            None,
            "user",
            &topic_msg,
            None,
        ) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("Failed to save debate topic: {e}"),
                }),
            )
                .into_response();
        }
    }

    // 3. Load conversation agents
    let room_agents = {
        let db = match state.db() {
            Ok(db) => db,
            Err((_status, msg)) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: msg }),
                )
                    .into_response();
            }
        };
        db.get_conversation_agents(&conversation_id)
            .unwrap_or_default()
    };

    if room_agents.len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "Debate requires at least 2 agents in the conversation".to_owned(),
            }),
        )
            .into_response();
    }

    // 4. Build agent lookup map
    let agent_map: HashMap<String, crate::db::Agent> = room_agents
        .iter()
        .map(|a| (a.id.clone(), a.clone()))
        .collect();

    let participant_ids: Vec<String> = room_agents.iter().map(|a| a.id.clone()).collect();

    let debate_config = DebateConfig {
        topic: req.topic.clone(),
        num_rounds,
        moderator_agent_id: None, // last participant moderates synthesis
        participant_agent_ids: participant_ids,
        conversation_id: conversation_id.clone(),
    };

    // 5. Build SSE stream
    let conv_id = conversation_id.clone();
    let db = state.db.clone();
    let broadcast_tx = state.tx.clone();
    let buf = reconnect_buffer();
    let conv_for_task = conversation;

    let (sse_tx, sse_rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(128);

    tokio::spawn(async move {
        let mut engine = DebateEngine::new(debate_config);
        let mut previous_turns: Vec<String> = Vec::new();

        while !engine.is_complete() {
            let agent_id = match engine.next_agent_id() {
                Some(id) => id.to_owned(),
                None => break,
            };

            let agent = match agent_map.get(&agent_id) {
                Some(a) => a.clone(),
                None => {
                    tracing::error!("Debate agent {} not found in map", agent_id);
                    engine.advance();
                    continue;
                }
            };

            // Build a phase label for the SSE event metadata
            let phase_label = match engine.current_phase() {
                crate::orchestrate::debate::DebatePhase::Opening => "Opening".to_owned(),
                crate::orchestrate::debate::DebatePhase::Rebuttal(n) => {
                    format!("Rebuttal Round {n}")
                }
                crate::orchestrate::debate::DebatePhase::Closing => "Closing".to_owned(),
                crate::orchestrate::debate::DebatePhase::Synthesis => "Synthesis".to_owned(),
                crate::orchestrate::debate::DebatePhase::Complete => "Complete".to_owned(),
            };

            // Send a phase marker so the frontend can render section headers
            let phase_data = serde_json::json!({
                "content": format!("\n\n--- {}: {} ---\n\n", phase_label, agent.name),
                "agent_id": agent.id,
                "agent_name": agent.name,
                "conversation_id": conv_id,
                "phase": phase_label,
            })
            .to_string();

            let phase_event_id = buf.push(&conv_id, "text", &phase_data).await;
            let phase_event = Event::default()
                .event("text")
                .id(phase_event_id.to_string())
                .data(&phase_data);
            if sse_tx.send(Ok(phase_event)).await.is_err() {
                break;
            }

            // Build the prompt for this turn
            let prompt = engine.build_turn_prompt(&agent.name, &previous_turns);

            // Select adapter and spawn
            let provider = if agent.provider.is_empty() {
                agent.model.as_str()
            } else {
                agent.provider.as_str()
            };

            let adapter: Box<dyn CliAdapter> = match provider {
                "claude" => Box::new(ClaudeAdapter::new(120)),
                "codex" => Box::new(CodexAdapter::new(120)),
                "copilot" => Box::new(CopilotAdapter::new(120)),
                "gemini" => Box::new(GeminiAdapter::new(120)),
                other => {
                    let err_data = serde_json::json!({
                        "content": format!(
                            "Unsupported provider '{}' for agent '{}'",
                            other, agent.name
                        ),
                        "agent_id": agent.id,
                        "conversation_id": conv_id,
                    })
                    .to_string();
                    let eid = buf.push(&conv_id, "error", &err_data).await;
                    let evt = Event::default()
                        .event("error")
                        .id(eid.to_string())
                        .data(&err_data);
                    let _ = sse_tx.send(Ok(evt)).await;
                    engine.advance();
                    continue;
                }
            };

            let agent_home = if agent.agent_home.is_empty() {
                None
            } else {
                Some(agent.agent_home.as_str())
            };
            let workspace = conv_for_task
                .workspace_path
                .as_deref()
                .or(agent.workspace_path.as_deref());

            let mut chunk_rx = match adapter.spawn(&prompt, agent_home, workspace).await {
                Ok(rx) => rx,
                Err(e) => {
                    let err_data = serde_json::json!({
                        "content": format!(
                            "Failed to spawn adapter for {}: {e}",
                            agent.name
                        ),
                        "agent_id": agent.id,
                        "conversation_id": conv_id,
                    })
                    .to_string();
                    let eid = buf.push(&conv_id, "error", &err_data).await;
                    let evt = Event::default()
                        .event("error")
                        .id(eid.to_string())
                        .data(&err_data);
                    let _ = sse_tx.send(Ok(evt)).await;
                    engine.advance();
                    continue;
                }
            };

            // Stream this agent's response
            let mut turn_text = String::new();
            while let Some(chunk) = chunk_rx.recv().await {
                let event_name = chunk_type_to_event_name(&chunk.chunk_type);

                if matches!(chunk.chunk_type, ChunkType::Text) {
                    turn_text.push_str(&chunk.content);
                }

                let data = serde_json::json!({
                    "content": chunk.content,
                    "agent_id": agent.id,
                    "agent_name": agent.name,
                    "conversation_id": conv_id,
                    "phase": phase_label,
                })
                .to_string();

                let event_id = buf.push(&conv_id, event_name, &data).await;
                let sse_event = Event::default()
                    .event(event_name)
                    .id(event_id.to_string())
                    .data(&data);

                if sse_tx.send(Ok(sse_event)).await.is_err() {
                    return; // client disconnected
                }

                let _ = broadcast_tx.send(format!(
                    "event:{event_name} conv:{conv_id} data:{data}"
                ));

                if matches!(chunk.chunk_type, ChunkType::Done) {
                    break;
                }
            }

            // Save this turn to DB
            if !turn_text.is_empty() {
                let saved_content = format!("[{}] {}", phase_label, turn_text);
                match db.lock() {
                    Ok(db) => {
                        let _ = db.create_message(
                            &conv_id,
                            Some(&agent.id),
                            "assistant",
                            &saved_content,
                            None,
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to save debate turn for agent {}: {e}",
                            agent.id
                        );
                    }
                }
                previous_turns.push(format!("{}: {}", agent.name, turn_text));
            }

            engine.advance();
        }

        // Send final done event
        let done_data = serde_json::json!({
            "content": "",
            "conversation_id": conv_id,
        })
        .to_string();
        let event_id = buf.push(&conv_id, "done", &done_data).await;
        let done_event = Event::default()
            .event("done")
            .id(event_id.to_string())
            .data(&done_data);
        let _ = sse_tx.send(Ok(done_event)).await;
    });

    let stream = ReceiverStream::new(sse_rx);
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

// ---------------------------------------------------------------------------
// GET /chat/stream/:conversation_id?last_event_id=N — reconnect replay
// ---------------------------------------------------------------------------

async fn chat_reconnect(
    State(_state): State<AppState>,
    Path(conversation_id): Path<String>,
    Query(query): Query<ReconnectQuery>,
) -> impl IntoResponse {
    let last_id = query.last_event_id.unwrap_or(0);
    let buf = reconnect_buffer();

    let missed = buf.events_since(&conversation_id, last_id).await;

    // Build a one-shot SSE stream that replays missed events then closes.
    let capacity = missed.len().max(1) + 1;
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(capacity);

    tokio::spawn(async move {
        for evt in missed {
            let sse_event = Event::default()
                .event(&evt.event_type)
                .id(evt.id.to_string())
                .data(&evt.data);

            if tx.send(Ok(sse_event)).await.is_err() {
                return;
            }
        }
        // Signal that replay is complete.
        let done = Event::default().event("replay_done").data("{}");
        let _ = tx.send(Ok(done)).await;
    });

    let stream = ReceiverStream::new(rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"))
        .into_response()
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn chat_routes() -> Router<AppState> {
    Router::new()
        .route("/chat/send", post(chat_send))
        .route("/chat/panel", post(chat_panel))
        .route("/chat/debate", post(chat_debate))
        .route("/chat/stream/{conversation_id}", get(chat_reconnect))
}
