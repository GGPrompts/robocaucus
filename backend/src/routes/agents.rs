use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::agent_home_dir;
use crate::scaffold::scaffold_agent_folder;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / query types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub model: String,
    pub provider: Option<String>,
    pub color: String,
    pub scope: Option<String>,
    pub system_prompt: Option<String>,
    pub workspace_path: Option<String>,
    pub cli_config: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateAgentRequest {
    pub name: String,
    pub model: String,
    pub provider: Option<String>,
    #[allow(dead_code)]
    pub agent_home: Option<String>,
    pub color: String,
    pub scope: Option<String>,
    pub system_prompt: Option<String>,
    pub workspace_path: Option<String>,
    pub cli_config: Option<String>,
}

#[derive(Deserialize)]
pub struct ListAgentsQuery {
    pub scope: Option<String>,
}

#[derive(Serialize)]
pub struct AgentConfigResponse {
    pub path: String,
    pub content: String,
    pub format: String,
}

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub content: String,
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
    let provider = body.provider.as_deref().unwrap_or("");
    let scope = body.scope.as_deref().unwrap_or("global");
    let system_prompt = body.system_prompt.as_deref().unwrap_or("");
    let workspace_path = body.workspace_path.as_deref();

    // Compute agent home directory from the agent name.
    let agent_home = agent_home_dir(&body.name);

    // Scaffold the provider-specific instruction file if a provider is set.
    if !provider.is_empty() {
        if let Err(e) = scaffold_agent_folder(provider, &agent_home, system_prompt) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("scaffold failed: {e}") })),
            )
                .into_response();
        }
    }

    match db.create_agent(&body.name, &body.model, provider, &agent_home, &body.color, scope, system_prompt, workspace_path, body.cli_config.as_deref()) {
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
    let provider = body.provider.as_deref().unwrap_or("");
    // Ignore user-supplied agent_home to prevent path traversal; recompute from name.
    let agent_home = agent_home_dir(&body.name);
    let agent_home = agent_home.as_str();
    let scope = body.scope.as_deref().unwrap_or("global");
    let system_prompt = body.system_prompt.as_deref().unwrap_or("");
    let workspace_path = body.workspace_path.as_deref();

    match db.update_agent(&id, &body.name, &body.model, provider, agent_home, &body.color, scope, system_prompt, workspace_path, body.cli_config.as_deref()) {
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

/// Validate that agent_home is within the expected base directory (~/.robocaucus/agents/).
fn validate_agent_home(agent_home: &str) -> Result<(), &'static str> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned());
    let expected_base = format!("{home}/.robocaucus/agents/");
    let canonical = std::path::Path::new(agent_home)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| agent_home.to_string());
    let canonical_base = std::path::Path::new(&expected_base)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(expected_base);
    if !canonical.starts_with(&canonical_base) {
        return Err("agent_home path escapes allowed directory");
    }
    Ok(())
}

/// Return the config file path and format for a given provider and agent_home.
fn config_path_and_format(provider: &str, agent_home: &str) -> Option<(std::path::PathBuf, &'static str)> {
    let base = std::path::Path::new(agent_home);
    match provider {
        "claude" => Some((base.join(".claude/settings.json"), "json")),
        "codex" => Some((base.join(".codex/config.toml"), "toml")),
        "gemini" => Some((base.join(".gemini/settings.json"), "json")),
        "copilot" => Some((base.join(".copilot/mcp-config.json"), "json")),
        _ => None,
    }
}

async fn get_agent_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    let agent = match db.get_agent(&id) {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Agent not found" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    };

    if let Err(msg) = validate_agent_home(&agent.agent_home) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": msg }))).into_response();
    }

    let (path, format) = match config_path_and_format(&agent.provider, &agent.agent_home) {
        Some(v) => v,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Unknown provider" }))).into_response(),
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    };

    (StatusCode::OK, Json(AgentConfigResponse {
        path: path.to_string_lossy().into_owned(),
        content,
        format: format.to_string(),
    })).into_response()
}

async fn update_agent_config(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    let agent = match db.get_agent(&id) {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Agent not found" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))).into_response(),
    };

    if let Err(msg) = validate_agent_home(&agent.agent_home) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "error": msg }))).into_response();
    }

    let (path, _format) = match config_path_and_format(&agent.provider, &agent.agent_home) {
        Some(v) => v,
        None => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Unknown provider" }))).into_response(),
    };

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to create directories: {e}") }))).into_response();
        }
    }

    if let Err(e) = std::fs::write(&path, &body.content) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": format!("Failed to write config: {e}") }))).into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn agent_routes() -> Router<AppState> {
    Router::new()
        .route("/agents", post(create_agent).get(list_agents))
        .route("/agents/{id}", get(get_agent).put(update_agent).delete(delete_agent))
        .route("/agents/{id}/config", get(get_agent_config).put(update_agent_config))
}
