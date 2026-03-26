use crate::adapter::claude::ClaudeAdapter;
use crate::adapter::codex::CodexAdapter;
use crate::adapter::copilot::CopilotAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::{AdapterError, CliAdapter, OutputChunk};
use crate::db::Agent;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Config & response types
// ---------------------------------------------------------------------------

/// Configuration for a panel ("Ask Everyone") fan-out.
#[derive(Debug, Clone)]
pub struct PanelConfig {
    /// The user prompt to send to every agent.
    pub prompt: String,
    /// The conversation this panel belongs to.
    #[allow(dead_code)]
    pub conversation_id: String,
    /// IDs of agents to query (informational; the actual agents + adapters are
    /// passed to [`spawn_panel`]).
    #[allow(dead_code)]
    pub agent_ids: Vec<String>,
}

/// Collected response from a single agent after the stream is fully consumed.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PanelResponse {
    pub agent_id: String,
    pub agent_name: String,
    pub chunks: Vec<OutputChunk>,
}

/// A single output chunk tagged with the agent that produced it.
///
/// The merged receiver returned by [`spawn_panel`] yields these so the caller
/// can tell which agent each piece of streaming output came from.
#[derive(Debug)]
pub struct TaggedChunk {
    pub agent_id: String,
    #[allow(dead_code)]
    pub agent_name: String,
    pub chunk: OutputChunk,
}

// ---------------------------------------------------------------------------
// Adapter selection
// ---------------------------------------------------------------------------

/// Return the appropriate [`CliAdapter`] for a model identifier.
///
/// Currently supported:
/// - `"claude"`  -> [`ClaudeAdapter`]
/// - `"codex"`   -> [`CodexAdapter`]
/// - `"copilot"` -> [`CopilotAdapter`]
/// - `"gemini"`  -> [`GeminiAdapter`]
///
/// Everything else yields [`AdapterError::CliNotFound`].
pub fn select_adapter(model: &str) -> Result<Box<dyn CliAdapter>, AdapterError> {
    match model {
        "claude" => Ok(Box::new(ClaudeAdapter::default())),
        "codex" => Ok(Box::new(CodexAdapter::default())),
        "copilot" => Ok(Box::new(CopilotAdapter::default())),
        "gemini" => Ok(Box::new(GeminiAdapter::default())),
        other => Err(AdapterError::CliNotFound(format!(
            "unsupported model: {other}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Panel fan-out
// ---------------------------------------------------------------------------

/// Spawn every agent's CLI adapter concurrently and merge output into one
/// [`mpsc::Receiver<TaggedChunk>`].
///
/// Each `(Agent, Box<dyn CliAdapter>)` pair is driven in its own `tokio::spawn`
/// task. Chunks arrive in the receiver in whatever order the underlying
/// processes produce them — there is no ordering guarantee across agents.
///
/// The receiver is closed once **all** agent streams have finished.
pub fn spawn_panel(
    config: &PanelConfig,
    agents_with_adapters: Vec<(Agent, Box<dyn CliAdapter>)>,
) -> mpsc::Receiver<TaggedChunk> {
    let (tx, rx) = mpsc::channel::<TaggedChunk>(128);
    let prompt = config.prompt.clone();

    for (agent, adapter) in agents_with_adapters {
        let tx = tx.clone();
        let prompt = prompt.clone();
        let agent_id = agent.id.clone();
        let agent_name = agent.name.clone();
        let agent_home = if agent.agent_home.is_empty() {
            None
        } else {
            Some(agent.agent_home.clone())
        };
        let workspace = agent.workspace_path.clone();

        tokio::spawn(async move {
            // Attempt to spawn the CLI process for this agent.
            let mut chunk_rx = match adapter
                .spawn(
                    &prompt,
                    agent_home.as_deref(),
                    workspace.as_deref(),
                )
                .await
            {
                Ok(rx) => rx,
                Err(e) => {
                    // Report the spawn failure as a tagged error chunk.
                    let _ = tx
                        .send(TaggedChunk {
                            agent_id,
                            agent_name,
                            chunk: OutputChunk {
                                chunk_type: crate::adapter::ChunkType::Error,
                                content: format!("Failed to spawn adapter: {e}"),
                            },
                        })
                        .await;
                    return;
                }
            };

            // Forward every OutputChunk as a TaggedChunk.
            while let Some(chunk) = chunk_rx.recv().await {
                if tx
                    .send(TaggedChunk {
                        agent_id: agent_id.clone(),
                        agent_name: agent_name.clone(),
                        chunk,
                    })
                    .await
                    .is_err()
                {
                    // Receiver dropped — stop forwarding.
                    break;
                }
            }
        });
    }

    // All per-agent `tx` clones are moved into their tasks.  The original `tx`
    // is dropped here, so the channel closes once every task finishes.
    rx
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_adapter_claude() {
        let adapter = select_adapter("claude").expect("should return ClaudeAdapter");
        assert_eq!(adapter.name(), "claude");
    }

    #[test]
    fn test_select_adapter_codex() {
        let adapter = select_adapter("codex").expect("should return CodexAdapter");
        assert_eq!(adapter.name(), "codex");
    }

    #[test]
    fn test_select_adapter_copilot() {
        let adapter = select_adapter("copilot").expect("should return CopilotAdapter");
        assert_eq!(adapter.name(), "copilot");
    }

    #[test]
    fn test_select_adapter_gemini() {
        let adapter = select_adapter("gemini").expect("should return GeminiAdapter");
        assert_eq!(adapter.name(), "gemini");
    }

    #[test]
    fn test_select_adapter_unknown_model() {
        match select_adapter("gpt-5") {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("unsupported model"),
                    "error message should mention unsupported model, got: {msg}"
                );
            }
            Ok(_) => panic!("select_adapter should fail for unknown model"),
        }
    }
}
