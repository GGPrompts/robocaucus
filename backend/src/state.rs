use std::sync::{Arc, Mutex};
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
}
