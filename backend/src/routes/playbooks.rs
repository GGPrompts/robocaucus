use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::db::agent_home_dir;
use crate::scaffold::scaffold_agent_folder;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// YAML role extraction
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PlaybookYaml {
    roles: Option<Vec<PlaybookRole>>,
}

#[derive(Deserialize)]
struct PlaybookRole {
    name: String,
    system_prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePlaybookRequest {
    pub name: String,
    pub flow_type: String,
    pub yaml_content: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePlaybookRequest {
    pub name: String,
    pub flow_type: String,
    pub yaml_content: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct RunPlaybookRequest {
    /// Optional YAML content override with user-filled placeholder values.
    /// If provided, this is used instead of the stored playbook YAML.
    pub yaml_content: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn create_playbook(
    State(state): State<AppState>,
    Json(body): Json<CreatePlaybookRequest>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };
    let description = body.description.as_deref().unwrap_or("");

    match db.create_playbook(&body.name, &body.flow_type, &body.yaml_content, description) {
        Ok(pb) => (StatusCode::CREATED, Json(pb)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_playbooks(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    match db.list_playbooks() {
        Ok(playbooks) => (StatusCode::OK, Json(playbooks)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_playbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    match db.get_playbook(&id) {
        Ok(Some(pb)) => (StatusCode::OK, Json(pb)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Playbook not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn update_playbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePlaybookRequest>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };
    let description = body.description.as_deref().unwrap_or("");

    match db.update_playbook(&id, &body.name, &body.flow_type, &body.yaml_content, description) {
        Ok(Some(pb)) => (StatusCode::OK, Json(pb)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Playbook not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_playbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => return (status, Json(serde_json::json!({ "error": msg }))).into_response(),
    };

    match db.delete_playbook(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Playbook not found" })),
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
// Run a playbook: create conversation + agents
// ---------------------------------------------------------------------------

/// Color palette for auto-created agents.
const ROLE_COLORS: &[&str] = &[
    "#a855f7", "#3b82f6", "#22c55e", "#14b8a6", "#f97316",
    "#ef4444", "#ec4899", "#eab308",
];

async fn run_playbook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<RunPlaybookRequest>>,
) -> impl IntoResponse {
    let db = match state.db() {
        Ok(db) => db,
        Err((status, msg)) => {
            return (status, Json(serde_json::json!({ "error": msg }))).into_response()
        }
    };

    // 1. Fetch the playbook
    let playbook = match db.get_playbook(&id) {
        Ok(Some(pb)) => pb,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Playbook not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // Use provided yaml_content override (with filled placeholders) or fall back to stored content
    let yaml_content = body
        .and_then(|b| b.0.yaml_content)
        .unwrap_or(playbook.yaml_content);

    // 2. Create a conversation with the playbook's name and flow type
    let conversation = match db.create_conversation(&playbook.name, None, &playbook.flow_type) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // 3. Parse YAML to extract roles
    let parsed: PlaybookYaml = match serde_yaml::from_str(&yaml_content) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Invalid playbook YAML: {e}") })),
            )
                .into_response()
        }
    };

    // 4. For each role, create an agent and add it to the conversation
    if let Some(roles) = parsed.roles {
        for (i, role) in roles.iter().enumerate() {
            let color = ROLE_COLORS[i % ROLE_COLORS.len()];
            let prompt = role.system_prompt.as_deref().unwrap_or("");
            let provider = "claude";

            // Scaffold the agent home directory so the CLI discovers the agent's persona
            let home = agent_home_dir(&role.name);
            if let Err(e) = scaffold_agent_folder(provider, &home, prompt) {
                tracing::warn!("failed to scaffold agent '{}': {e}", role.name);
            }

            let agent = match db.create_agent(
                &role.name,
                "sonnet",  // default model
                provider,
                &home,
                color,
                "global",
                prompt,
                None,
                None,
            ) {
                Ok(a) => a,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                        .into_response()
                }
            };

            if let Err(e) = db.add_agent_to_conversation(&conversation.id, &agent.id) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        }
    }

    // 5. Return the new conversation ID
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "conversation_id": conversation.id })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn playbook_routes() -> Router<AppState> {
    Router::new()
        .route("/playbooks", post(create_playbook).get(list_playbooks))
        .route("/playbooks/{id}", get(get_playbook).put(update_playbook).delete(delete_playbook))
        .route("/playbooks/{id}/run", post(run_playbook))
}
