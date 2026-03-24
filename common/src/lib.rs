use serde::{Deserialize, Serialize};

/// Placeholder shared types for RoboCaucus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}
