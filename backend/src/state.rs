use std::sync::{Arc, Mutex, MutexGuard};
use axum::http::StatusCode;
use tokio::sync::broadcast;

use crate::db::Database;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>, // For SSE broadcasting
    pub db: Arc<Mutex<Database>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            tx,
            db: Arc::new(Mutex::new(db)),
        }
    }

    /// Acquire the database mutex, returning an Axum-compatible error on poisoning.
    pub fn db(&self) -> Result<MutexGuard<'_, Database>, (StatusCode, String)> {
        self.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database lock error: {e}"))
        })
    }
}
