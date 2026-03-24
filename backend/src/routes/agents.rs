use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub model: String,
    pub color: String,
    pub scope: Option<String>,
    pub system_prompt: Option<String>,
    pub workspace_path: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAgentRequest {
    pub name: String,
    pub model: String,
    pub color: String,
    pub scope: Option<String>,
    pub system_prompt: Option<String>,
    pub workspace_path: Option<String>,
}

#[derive(Deserialize)]
pub struct ListAgentsQuery {
    pub scope: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_agent(
    State(state): State<AppState>,
    Json(body): Json<CreateAgentRequest>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };
    let scope = body.scope.as_deref().unwrap_or("global");
    let system_prompt = body.system_prompt.as_deref().unwrap_or("");
    let workspace_path = body.workspace_path.as_deref();

    match db.create_agent(&body.name, &body.model, &body.color, scope, system_prompt, workspace_path) {
        Ok(agent) => (StatusCode::CREATED, Json(agent)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_agents(
    State(state): State<AppState>,
    Query(params): Query<ListAgentsQuery>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };
    let scope_filter = params.scope.as_deref();

    match db.list_agents(scope_filter) {
        Ok(agents) => (StatusCode::OK, Json(agents)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    match db.get_agent(&id) {
        Ok(Some(agent)) => (StatusCode::OK, Json(agent)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Agent not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn update_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateAgentRequest>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };
    let scope = body.scope.as_deref().unwrap_or("global");
    let system_prompt = body.system_prompt.as_deref().unwrap_or("");
    let workspace_path = body.workspace_path.as_deref();

    match db.update_agent(&id, &body.name, &body.model, &body.color, scope, system_prompt, workspace_path) {
        Ok(Some(agent)) => (StatusCode::OK, Json(agent)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Agent not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    match db.delete_agent(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Agent not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/agents", post(create_agent).get(list_agents))
        .route("/agents/{id}", get(get_agent).put(update_agent).delete(delete_agent))
}
