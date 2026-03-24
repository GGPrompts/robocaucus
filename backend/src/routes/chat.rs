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
    // 7. Spawn the adapter
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

    let mut rx = match adapter.spawn(&prompt, agent_home, workspace).await {
        Ok(rx) => rx,
        Err(e) => {
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
                            let _ = db.create_message(
                                &conv_id,
                                Some(&agent_id),
                                "assistant",
                                &full_response,
                                Some(&agent_model),
                            );
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
    });

    let stream = ReceiverStream::new(sse_rx);
    Sse::new(stream)
        .keep_alive(KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"))
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
        .route("/chat/stream/{conversation_id}", get(chat_reconnect))
}
