use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use crate::agentmd;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// POST /agents/import  — import from .agent.md text/plain body
// ---------------------------------------------------------------------------

async fn import_agent(
    State(state): State<AppState>,
    body: String,
) -> impl IntoResponse {
    // Parse the .agent.md content.
    let data = match agentmd::parse_agent_md(&body) {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    // Persist to DB.
    let db = state.db.lock().unwrap();
    match db.create_agent(
        &data.name,
        &data.model,
        &data.color,
        &data.scope,
        &data.system_prompt,
        None,
    ) {
        Ok(agent) => (StatusCode::CREATED, Json(agent)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// GET /agents/:id/export  — export as .agent.md text/plain
// ---------------------------------------------------------------------------

async fn export_agent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();

    match db.get_agent(&id) {
        Ok(Some(agent)) => {
            let md = agentmd::serialize_agent_md(&agent);
            let filename = format!("{}.agent.md", agent.name.to_lowercase().replace(' ', "-"));
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_owned()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{filename}\""),
                    ),
                ],
                md,
            )
                .into_response()
        }
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

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn agentmd_routes() -> Router<AppState> {
    Router::new()
        .route("/agents/import", post(import_agent))
        .route("/agents/{id}/export", get(export_agent))
}
