pub mod agentmd;
pub mod agents;
pub mod chat;
pub mod conversations;
pub mod files;
pub mod git;
pub mod playbooks;
pub mod providers;

use axum::{routing::get, Router};
use crate::state::AppState;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(agents::agent_routes())
        .merge(agentmd::agentmd_routes())
        .merge(chat::chat_routes())
        .merge(conversations::conversation_routes())
        .merge(providers::provider_routes())
        .merge(git::git_routes())
        .merge(files::file_routes())
        .merge(playbooks::playbook_routes())
}
