use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::{Agent, Conversation, Message};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateConversationRequest {
    pub title: String,
    pub workspace_path: Option<String>,
    pub orchestration_mode: Option<String>,
    pub agent_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub orchestration_mode: Option<String>,
    pub agent_ids: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ConversationDetail {
    #[serde(flatten)]
    pub conversation: Conversation,
    pub agents: Vec<Agent>,
    pub messages: Vec<Message>,
}

#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub agent_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /conversations — create a new conversation room
async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<Conversation>), (StatusCode, String)> {
    let db = state.db()?;

    let mode = req.orchestration_mode.as_deref().unwrap_or("manual");

    let conv = db
        .create_conversation(&req.title, req.workspace_path.as_deref(), mode)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    // Attach agents if provided
    if let Some(agent_ids) = &req.agent_ids {
        for aid in agent_ids {
            db.add_agent_to_conversation(&conv.id, aid)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
        }
    }

    Ok((StatusCode::CREATED, Json(conv)))
}

/// GET /conversations — list all conversations
async fn list_conversations(
    State(state): State<AppState>,
) -> Result<Json<Vec<Conversation>>, (StatusCode, String)> {
    let db = state.db()?;

    let convs = db
        .list_conversations()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(Json(convs))
}

/// GET /conversations/:id — get a conversation with its agents and messages
async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ConversationDetail>, (StatusCode, String)> {
    let db = state.db()?;

    let conversation = db
        .get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
        })?;

    let agents = db
        .get_conversation_agents(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    let messages = db
        .list_messages(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(Json(ConversationDetail {
        conversation,
        agents,
        messages,
    }))
}

/// PUT /conversations/:id — update a conversation
async fn update_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateConversationRequest>,
) -> Result<Json<Conversation>, (StatusCode, String)> {
    let db = state.db()?;

    // Fetch existing conversation to fill in defaults for unchanged fields
    let existing = db
        .get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
        })?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let mode = req
        .orchestration_mode
        .as_deref()
        .unwrap_or(&existing.orchestration_mode);

    let conv = db
        .update_conversation(&id, title, mode)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
        })?;

    // Replace agent membership if agent_ids provided
    if let Some(agent_ids) = &req.agent_ids {
        db.remove_agents_from_conversation(&id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
        for aid in agent_ids {
            db.add_agent_to_conversation(&id, aid)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
        }
    }

    Ok(Json(conv))
}

/// DELETE /conversations/:id — delete a conversation and its related data
async fn delete_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db()?;

    let deleted = db
        .delete_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("Conversation {id} not found"),
        ))
    }
}

/// GET /conversations/:id/messages — list messages for a conversation
async fn list_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Message>>, (StatusCode, String)> {
    let db = state.db()?;

    // Verify conversation exists
    db.get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
        })?;

    let messages = db
        .list_messages(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(Json(messages))
}

/// POST /conversations/:id/messages — add a message to a conversation
async fn create_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<Message>), (StatusCode, String)> {
    let db = state.db()?;

    // Verify conversation exists
    db.get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Conversation {id} not found"),
            )
        })?;

    let role = if req.agent_id.is_some() {
        "assistant"
    } else {
        "user"
    };

    let msg = db
        .create_message(&id, req.agent_id.as_deref(), role, &req.content, None)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok((StatusCode::CREATED, Json(msg)))
}

// ---------------------------------------------------------------------------
// Conversation-Agent membership
// ---------------------------------------------------------------------------

/// POST /conversations/:id/agents/:agent_id — add an agent to a conversation
async fn add_agent(
    State(state): State<AppState>,
    Path((id, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db()?;

    // Verify conversation exists
    db.get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Conversation {id} not found")))?;

    // Verify agent exists
    db.get_agent(&agent_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Agent {agent_id} not found")))?;

    db.add_agent_to_conversation(&id, &agent_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /conversations/:id/agents/:agent_id — remove an agent from a conversation
async fn remove_agent(
    State(state): State<AppState>,
    Path((id, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let db = state.db()?;

    let removed = db
        .remove_agent_from_conversation(&id, &agent_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("Agent {agent_id} not in conversation {id}"),
        ))
    }
}

/// GET /conversations/:id/agents — list agents in a conversation
async fn list_conversation_agents(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Agent>>, (StatusCode, String)> {
    let db = state.db()?;

    // Verify conversation exists
    db.get_conversation(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Conversation {id} not found")))?;

    let agents = db
        .get_conversation_agents(&id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    Ok(Json(agents))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn conversation_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/conversations",
            get(list_conversations).post(create_conversation),
        )
        .route(
            "/conversations/{id}",
            get(get_conversation)
                .put(update_conversation)
                .delete(delete_conversation),
        )
        .route(
            "/conversations/{id}/messages",
            get(list_messages).post(create_message),
        )
        .route(
            "/conversations/{id}/agents",
            get(list_conversation_agents),
        )
        .route(
            "/conversations/{id}/agents/{agent_id}",
            axum::routing::post(add_agent).delete(remove_agent),
        )
}
