use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

use super::{AdapterError, ChunkType, CliAdapter, OutputChunk};

// ---------------------------------------------------------------------------
// Codex adapter
// ---------------------------------------------------------------------------

/// Adapter that drives the OpenAI `codex` CLI in JSONL mode.
///
/// Spawns `codex exec "<prompt>" --json` and reads JSONL from stdout.
pub struct CodexAdapter {
    /// Per-PID child-process handles, used for cancellation.
    children: Arc<Mutex<HashMap<u32, tokio::process::Child>>>,
    /// Maximum wall-clock time (seconds) before the process is killed.
    timeout_secs: u64,
}

impl CodexAdapter {
    /// Create a new adapter with the given timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs,
        }
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new(120)
    }
}

#[async_trait::async_trait]
impl CliAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "codex"
    }

    async fn spawn(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<mpsc::Receiver<OutputChunk>, AdapterError> {
        // Verify the CLI exists on PATH before spawning.
        let which = Command::new("which")
            .arg("codex")
            .output()
            .await
            .map_err(AdapterError::SpawnFailed)?;

        if !which.status.success() {
            return Err(AdapterError::CliNotFound("codex".into()));
        }

        // Codex has no system prompt flag — prepend it to the prompt instead.
        let effective_prompt = match system_prompt {
            Some(sp) => format!("[System: {sp}]\n\n{prompt}"),
            None => prompt.to_owned(),
        };

        // Build the command: codex exec "<prompt>" --json
        let mut cmd = Command::new("codex");
        cmd.arg("exec").arg(&effective_prompt).arg("--json");

        if let Some(dir) = cwd {
            cmd.arg("-C").arg(dir);
        }

        // We only need stdout; inherit stderr so operator can see diagnostics.
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn().map_err(AdapterError::SpawnFailed)?;

        let pid = child
            .id()
            .expect("child should have a PID immediately after spawn");

        // Take ownership of stdout before storing the child.
        let stdout = child
            .stdout
            .take()
            .expect("stdout should be piped after spawn");

        // Store the child so we can cancel it later.
        {
            let mut map = self.children.lock().await;
            map.insert(pid, child);
        }

        // Channel for streaming chunks back to the caller.
        let (tx, rx) = mpsc::channel::<OutputChunk>(64);

        let children = Arc::clone(&self.children);
        let timeout_secs = self.timeout_secs;

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            let read_loop = async {
                loop {
                    let line = match lines.next_line().await {
                        Ok(Some(line)) => line,
                        Ok(None) => break, // EOF
                        Err(e) => {
                            let _ = tx
                                .send(OutputChunk {
                                    chunk_type: ChunkType::Error,
                                    content: format!("IO error reading stdout: {e}"),
                                })
                                .await;
                            break;
                        }
                    };

                    if line.trim().is_empty() {
                        continue;
                    }

                    // Parse each JSONL line.
                    let parsed: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx
                                .send(OutputChunk {
                                    chunk_type: ChunkType::Error,
                                    content: format!("JSON parse error: {e} — raw: {line}"),
                                })
                                .await;
                            continue;
                        }
                    };

                    // Extract chunks from the event.
                    if let Some(chunk) = parse_codex_event(&parsed) {
                        if tx.send(chunk).await.is_err() {
                            // Receiver dropped; stop processing.
                            return;
                        }
                    }
                }
            };

            // Apply the timeout to the whole read loop.
            let timed_out = timeout(Duration::from_secs(timeout_secs), read_loop)
                .await
                .is_err();

            if timed_out {
                let _ = tx
                    .send(OutputChunk {
                        chunk_type: ChunkType::Error,
                        content: format!("Process timed out after {timeout_secs}s"),
                    })
                    .await;

                // Kill the child on timeout.
                let mut map = children.lock().await;
                if let Some(mut child) = map.remove(&pid) {
                    let _ = child.kill().await;
                }
            }

            // Wait for the child to finish and clean up.
            {
                let mut map = children.lock().await;
                if let Some(mut child) = map.remove(&pid) {
                    let _ = child.wait().await;
                }
            }

            // Signal completion.
            let _ = tx
                .send(OutputChunk {
                    chunk_type: ChunkType::Done,
                    content: String::new(),
                })
                .await;
        });

        Ok(rx)
    }

    async fn cancel(&self, process_id: u32) -> Result<(), AdapterError> {
        let mut map = self.children.lock().await;
        if let Some(mut child) = map.remove(&process_id) {
            // Send SIGTERM via libc for a graceful shutdown.
            #[cfg(unix)]
            {
                // Safety: sending a signal to a known PID.
                unsafe {
                    libc::kill(process_id as libc::pid_t, libc::SIGTERM);
                }
            }
            #[cfg(not(unix))]
            {
                // On non-Unix, fall back to hard kill.
                let _ = child.kill().await;
            }
            let _ = child.wait().await;
            Ok(())
        } else {
            // Process already finished or unknown — treat as success.
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// JSONL event parsing
// ---------------------------------------------------------------------------

/// Parse a single Codex JSONL event into an optional [`OutputChunk`].
///
/// Codex `exec --json` emits JSONL where each object has a `type` field.
/// The key event types:
///
/// - `"item.completed"` with `item.type == "agent_message"` — the agent's
///   text response lives in `item.text`.
/// - `"turn.completed"` — usage metadata; ignored.
/// - `"thread.started"`, `"turn.started"` — lifecycle events; ignored.
fn parse_codex_event(event: &serde_json::Value) -> Option<OutputChunk> {
    let event_type = event.get("type").and_then(|v| v.as_str())?;

    match event_type {
        "item.completed" => {
            let item = event.get("item")?;
            let item_type = item.get("type").and_then(|v| v.as_str())?;

            if item_type == "agent_message" {
                let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                if !text.is_empty() {
                    return Some(OutputChunk {
                        chunk_type: ChunkType::Text,
                        content: text.to_owned(),
                    });
                }
            }

            None
        }
        // All other event types (thread.started, turn.started, turn.completed,
        // etc.) are ignored.
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_item_completed_agent_message() {
        let event: serde_json::Value = serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_0",
                "type": "agent_message",
                "text": "Hello from Codex!"
            }
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert!(matches!(chunk.chunk_type, ChunkType::Text));
        assert_eq!(chunk.content, "Hello from Codex!");
    }

    #[test]
    fn test_parse_item_completed_non_agent_message() {
        let event: serde_json::Value = serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_1",
                "type": "tool_call",
                "name": "shell"
            }
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_turn_completed_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "turn.completed",
            "usage": { "input_tokens": 100, "output_tokens": 50 }
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_thread_started_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "thread.started",
            "thread_id": "019d1e5f-abcd-0000-0000-000000000000"
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_turn_started_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "turn.started"
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_empty_agent_message_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "item.completed",
            "item": {
                "id": "item_0",
                "type": "agent_message",
                "text": ""
            }
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_parse_unknown_event_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "some.future.event"
        });
        let chunk = parse_codex_event(&event);
        assert!(chunk.is_none());
    }

    #[test]
    fn test_default_timeout() {
        let adapter = CodexAdapter::default();
        assert_eq!(adapter.timeout_secs, 120);
    }

    #[test]
    fn test_custom_timeout() {
        let adapter = CodexAdapter::new(300);
        assert_eq!(adapter.timeout_secs, 300);
    }

    #[test]
    fn test_name() {
        let adapter = CodexAdapter::default();
        assert_eq!(adapter.name(), "codex");
    }
}
