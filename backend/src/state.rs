use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<String>, // For SSE broadcasting
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }
}
