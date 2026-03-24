use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

use super::{AdapterError, ChunkType, CliAdapter, OutputChunk};

// ---------------------------------------------------------------------------
// Claude adapter
// ---------------------------------------------------------------------------

/// Adapter that drives the `claude` CLI in streaming-JSON mode.
///
/// Spawns `claude -p --output-format stream-json` and reads JSONL from stdout.
pub struct ClaudeAdapter {
    /// Per-PID child-process handles, used for cancellation.
    children: Arc<Mutex<HashMap<u32, tokio::process::Child>>>,
    /// Maximum wall-clock time (seconds) before the process is killed.
    timeout_secs: u64,
}

impl ClaudeAdapter {
    /// Create a new adapter with the given timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs,
        }
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new(120)
    }
}

#[async_trait::async_trait]
impl CliAdapter for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    async fn spawn(
        &self,
        prompt: &str,
        agent_home: Option<&str>,
        workspace: Option<&str>,
    ) -> Result<mpsc::Receiver<OutputChunk>, AdapterError> {
        // Verify the CLI exists on PATH before spawning.
        let which = Command::new("which")
            .arg("claude")
            .output()
            .await
            .map_err(AdapterError::SpawnFailed)?;

        if !which.status.success() {
            return Err(AdapterError::CliNotFound("claude".into()));
        }

        // Build the command.
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg("--output-format")
            .arg("stream-json")
            .arg(prompt);

        if let Some(home) = agent_home {
            cmd.current_dir(home);
        }

        if let Some(ws) = workspace {
            cmd.arg("--add-dir").arg(ws);
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
                        Ok(None) => break,  // EOF
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
                    let chunks = parse_claude_event(&parsed);
                    for chunk in chunks {
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
            // Send SIGTERM via the nix crate-free path: kill(2) via libc.
            // tokio::process::Child::kill sends SIGKILL, so we use a manual
            // approach for the gentler SIGTERM.
            #[cfg(unix)]
            {
                // SAFETY: `process_id` was obtained from `tokio::process::Child::id()`
                // immediately after a successful spawn, so it is a valid PID.
                // SIGTERM is a standard signal. The child is still tracked in our
                // map, so the process is (or was) running under our control.
                let ret = unsafe {
                    libc::kill(process_id as libc::pid_t, libc::SIGTERM)
                };
                if ret == -1 {
                    tracing::warn!(
                        pid = process_id,
                        errno = std::io::Error::last_os_error().raw_os_error(),
                        "libc::kill(SIGTERM) failed for claude adapter"
                    );
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

/// Parse a single Claude stream-json event into zero or more [`OutputChunk`]s.
///
/// Claude's `--output-format stream-json` emits JSONL where each object has a
/// `type` field. The key event types we handle:
///
/// - `"assistant"` — contains a `content` array of blocks (`text`, `tool_use`,
///   `thinking`).
/// - `"content_block_delta"` — incremental text delta.
/// - `"content_block_start"` — start of a new content block.
/// - `"result"` / `"message_stop"` — end of the response.
/// - `"error"` — an error from the CLI.
fn parse_claude_event(event: &serde_json::Value) -> Vec<OutputChunk> {
    let mut chunks = Vec::new();

    let event_type = match event.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return chunks,
    };

    match event_type {
        // Full assistant message (non-streaming fallback).
        "assistant" => {
            if let Some(content_arr) = event.get("content").and_then(|v| v.as_array()) {
                for block in content_arr {
                    if let Some(chunk) = parse_content_block(block) {
                        chunks.push(chunk);
                    }
                }
            }
        }

        // Incremental content block start — may carry initial text.
        "content_block_start" => {
            if let Some(block) = event.get("content_block") {
                if let Some(chunk) = parse_content_block(block) {
                    chunks.push(chunk);
                }
            }
        }

        // Incremental text delta.
        "content_block_delta" => {
            if let Some(delta) = event.get("delta") {
                let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match delta_type {
                    "text_delta" => {
                        if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                            if !text.is_empty() {
                                chunks.push(OutputChunk {
                                    chunk_type: ChunkType::Text,
                                    content: text.to_owned(),
                                });
                            }
                        }
                    }
                    "thinking_delta" => {
                        if let Some(text) = delta.get("thinking").and_then(|v| v.as_str()) {
                            if !text.is_empty() {
                                chunks.push(OutputChunk {
                                    chunk_type: ChunkType::Thinking,
                                    content: text.to_owned(),
                                });
                            }
                        }
                    }
                    "input_json_delta" => {
                        if let Some(json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                            if !json.is_empty() {
                                chunks.push(OutputChunk {
                                    chunk_type: ChunkType::ToolUse,
                                    content: json.to_owned(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Error events from the CLI.
        "error" => {
            let msg = event
                .get("error")
                .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                .or_else(|| event.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error");
            chunks.push(OutputChunk {
                chunk_type: ChunkType::Error,
                content: msg.to_owned(),
            });
        }

        // End of message.
        "result" | "message_stop" => {
            // We don't emit Done here — the spawned task sends it after the
            // process exits, which is the authoritative signal.
        }

        _ => {
            // Ignore unknown event types (e.g. ping, message_start).
        }
    }

    chunks
}

/// Parse a single content block from a Claude assistant message.
fn parse_content_block(block: &serde_json::Value) -> Option<OutputChunk> {
    let block_type = block.get("type").and_then(|v| v.as_str())?;
    match block_type {
        "text" => {
            let text = block.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                None
            } else {
                Some(OutputChunk {
                    chunk_type: ChunkType::Text,
                    content: text.to_owned(),
                })
            }
        }
        "thinking" => {
            let text = block.get("thinking").and_then(|v| v.as_str()).unwrap_or("");
            if text.is_empty() {
                None
            } else {
                Some(OutputChunk {
                    chunk_type: ChunkType::Thinking,
                    content: text.to_owned(),
                })
            }
        }
        "tool_use" => {
            // Serialize the input object (or the whole block) as the content.
            let input = block.get("input").unwrap_or(block);
            Some(OutputChunk {
                chunk_type: ChunkType::ToolUse,
                content: serde_json::to_string(input).unwrap_or_default(),
            })
        }
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
    fn test_parse_assistant_text_event() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant",
            "content": [
                { "type": "text", "text": "Hello, world!" }
            ]
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "Hello, world!");
    }

    #[test]
    fn test_parse_content_block_delta_text() {
        let event: serde_json::Value = serde_json::json!({
            "type": "content_block_delta",
            "delta": { "type": "text_delta", "text": "partial " }
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "partial ");
    }

    #[test]
    fn test_parse_thinking_delta() {
        let event: serde_json::Value = serde_json::json!({
            "type": "content_block_delta",
            "delta": { "type": "thinking_delta", "thinking": "reasoning..." }
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Thinking));
        assert_eq!(chunks[0].content, "reasoning...");
    }

    #[test]
    fn test_parse_tool_use_delta() {
        let event: serde_json::Value = serde_json::json!({
            "type": "content_block_delta",
            "delta": { "type": "input_json_delta", "partial_json": "{\"key\":" }
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::ToolUse));
        assert_eq!(chunks[0].content, "{\"key\":");
    }

    #[test]
    fn test_parse_error_event() {
        let event: serde_json::Value = serde_json::json!({
            "type": "error",
            "error": { "message": "rate limited" }
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Error));
        assert_eq!(chunks[0].content, "rate limited");
    }

    #[test]
    fn test_parse_unknown_event_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "ping"
        });
        let chunks = parse_claude_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_message_stop_no_output() {
        let event: serde_json::Value = serde_json::json!({
            "type": "message_stop"
        });
        let chunks = parse_claude_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_content_block_start_with_text() {
        let event: serde_json::Value = serde_json::json!({
            "type": "content_block_start",
            "content_block": { "type": "text", "text": "start" }
        });
        let chunks = parse_claude_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "start");
    }

    #[test]
    fn test_default_timeout() {
        let adapter = ClaudeAdapter::default();
        assert_eq!(adapter.timeout_secs, 120);
    }
}
