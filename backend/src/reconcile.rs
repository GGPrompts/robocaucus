// Wire into main.rs after creating AppState:
//   let tmux = tmux::TmuxManager::new();
//   let result = reconcile::reconcile(&db, &tmux).await;
//   tracing::info!("reconciliation: {:?}", result);

use crate::db::Database;
use crate::tmux::TmuxManager;

#[derive(Debug)]
pub struct ReconcileResult {
    pub active_sessions: Vec<String>,
    pub orphaned_killed: Vec<String>,
    pub missing_sessions: Vec<String>,
}

/// Reconcile tmux sessions with database state on startup.
///
/// 1. List all rc-* tmux sessions
/// 2. List all conversations from SQLite
/// 3. Kill orphaned sessions (tmux session exists, no matching conversation)
/// 4. Log missing sessions (conversation exists, no tmux session — expected for completed conversations)
/// 5. Return reconciliation result for logging
pub async fn reconcile(db: &Database, tmux: &TmuxManager) -> ReconcileResult {
    // Step 1: List all rc-* tmux sessions (full names like "rc-<uuid>").
    let tmux_sessions = match tmux.list_sessions().await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::warn!("reconcile: failed to list tmux sessions: {e}");
            return ReconcileResult {
                active_sessions: Vec::new(),
                orphaned_killed: Vec::new(),
                missing_sessions: Vec::new(),
            };
        }
    };

    // Step 2: List all conversations from SQLite.
    let conversations = match db.list_conversations() {
        Ok(convs) => convs,
        Err(e) => {
            tracing::warn!("reconcile: failed to list conversations: {e}");
            return ReconcileResult {
                active_sessions: Vec::new(),
                orphaned_killed: Vec::new(),
                missing_sessions: Vec::new(),
            };
        }
    };

    let conversation_ids: Vec<String> = conversations.iter().map(|c| c.id.clone()).collect();

    // Build a set of expected tmux session names from conversation IDs.
    let expected_names: std::collections::HashSet<String> = conversation_ids
        .iter()
        .map(|id| format!("rc-{id}"))
        .collect();

    // Build a set of actual tmux session names for quick lookup.
    let actual_names: std::collections::HashSet<String> =
        tmux_sessions.iter().cloned().collect();

    // Step 3: Kill orphaned sessions — tmux session exists but no matching conversation.
    // cleanup_orphans expects bare IDs (without rc- prefix).
    let orphaned_killed = match tmux.cleanup_orphans(&conversation_ids).await {
        Ok(killed) => {
            for name in &killed {
                tracing::warn!("reconcile: killed orphaned tmux session: {name}");
            }
            killed
        }
        Err(e) => {
            tracing::warn!("reconcile: failed to clean up orphans: {e}");
            Vec::new()
        }
    };

    // Step 4: Identify active sessions (tmux session exists AND has a matching conversation).
    let active_sessions: Vec<String> = tmux_sessions
        .iter()
        .filter(|name| expected_names.contains(name.as_str()))
        .cloned()
        .collect();

    // Step 5: Log missing sessions — conversation exists but no tmux session.
    // This is expected for completed/idle conversations.
    let missing_sessions: Vec<String> = conversation_ids
        .iter()
        .filter(|id| !actual_names.contains(&format!("rc-{id}")))
        .cloned()
        .collect();

    for id in &missing_sessions {
        tracing::info!("reconcile: conversation {id} has no tmux session (expected if completed)");
    }

    let result = ReconcileResult {
        active_sessions,
        orphaned_killed,
        missing_sessions,
    };

    tracing::info!(
        "reconcile complete: {} active, {} orphans killed, {} without sessions",
        result.active_sessions.len(),
        result.orphaned_killed.len(),
        result.missing_sessions.len(),
    );

    result
}
