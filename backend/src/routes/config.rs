use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ConfigResponse {
    pub default_workspace: String,
}

async fn get_config() -> Json<ConfigResponse> {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Json(ConfigResponse {
        default_workspace: cwd,
    })
}

pub fn config_routes() -> Router<AppState> {
    Router::new().route("/config", get(get_config))
}
