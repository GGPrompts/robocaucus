pub mod agents;
pub mod conversations;
pub mod providers;

use axum::{routing::get, Router};
use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(agents::agent_routes())
        .merge(conversations::conversation_routes())
        .merge(providers::provider_routes())
}
