pub mod claude;
pub mod codex;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single chunk of output from a CLI adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputChunk {
    pub chunk_type: ChunkType,
    pub content: String,
}

/// Discriminator for the kind of content carried by an [`OutputChunk`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Text,
    Thinking,
    ToolUse,
    Error,
    Done,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("CLI not found: {0}")]
    CliNotFound(String),

    #[error("Process spawn failed: {0}")]
    SpawnFailed(#[from] std::io::Error),

    #[error("Process timed out after {0}s")]
    Timeout(u64),

    #[error("Process cancelled")]
    Cancelled,

    #[error("Parse error: {0}")]
    ParseError(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over a subscription CLI (Claude, Codex, etc.).
///
/// Each adapter knows how to spawn the CLI, stream structured output chunks
/// through an [`mpsc`] channel, and cancel a running process.
#[async_trait::async_trait]
pub trait CliAdapter: Send + Sync {
    /// Human-readable name of the backing CLI (e.g. "claude").
    fn name(&self) -> &str;

    /// Spawn the CLI with the given prompt and return a receiver that yields
    /// [`OutputChunk`]s as the process produces output.
    ///
    /// * `prompt`        – the user/orchestrator prompt to send.
    /// * `system_prompt` – optional system-level instruction.
    /// * `cwd`           – optional working directory for the child process.
    async fn spawn(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<mpsc::Receiver<OutputChunk>, AdapterError>;

    /// Send SIGTERM to the process identified by `process_id`.
    async fn cancel(&self, process_id: u32) -> Result<(), AdapterError>;
}
