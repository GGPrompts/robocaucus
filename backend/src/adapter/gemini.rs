use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

use serde::Deserialize;

use super::{AdapterError, ChunkType, CliAdapter, OutputChunk};

// ---------------------------------------------------------------------------
// Provider-specific CLI config
// ---------------------------------------------------------------------------

/// Provider-specific CLI flag overrides for the Gemini adapter.
///
/// Deserialized from the agent's `cli_config` JSON column.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct GeminiConfig {
    /// Maps to `--approval-mode` (e.g. "auto", "ask", "deny").
    pub approval_mode: Option<String>,
    /// Maps to `--sandbox`.
    pub sandbox: Option<bool>,
    /// Each entry becomes a separate `-e` (extension) argument.
    pub extensions: Option<Vec<String>>,
    /// Each entry becomes a separate `--policy` argument.
    pub policy_paths: Option<Vec<String>>,
    /// Each entry becomes a separate `--allowed-mcp-server-names` argument.
    pub allowed_mcp_server_names: Option<Vec<String>>,
    /// Each entry becomes a separate `--include-directories` argument.
    pub include_directories: Option<Vec<String>>,
    /// Maps to `--raw-output`.
    pub raw_output: Option<bool>,
    /// Maps to `--debug`.
    pub debug: Option<bool>,
}

// ---------------------------------------------------------------------------
// Gemini adapter
// ---------------------------------------------------------------------------

/// Adapter that drives the Google `gemini` CLI in streaming-JSON mode.
///
/// Spawns `gemini -p "<prompt>" --output-format stream-json` and reads JSONL
/// from stdout. The Gemini CLI emits events with types: `init`, `message`,
/// `tool_use`, `tool_result`, `error`, and `result`.
pub struct GeminiAdapter {
    /// Per-PID child-process handles, used for cancellation.
    children: Arc<Mutex<HashMap<u32, tokio::process::Child>>>,
    /// Maximum wall-clock time (seconds) before the process is killed.
    timeout_secs: u64,
}

impl GeminiAdapter {
    /// Create a new adapter with the given timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs,
        }
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new(120)
    }
}

#[async_trait::async_trait]
impl CliAdapter for GeminiAdapter {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn spawn(
        &self,
        prompt: &str,
        agent_home: Option<&str>,
        workspace: Option<&str>,
        cli_config: Option<&str>,
    ) -> Result<mpsc::Receiver<OutputChunk>, AdapterError> {
        // Verify the CLI exists on PATH before spawning.
        let which = Command::new("which")
            .arg("gemini")
            .output()
            .await
            .map_err(AdapterError::SpawnFailed)?;

        if !which.status.success() {
            return Err(AdapterError::CliNotFound("gemini".into()));
        }

        // Parse provider-specific config (if any).
        let cfg: GeminiConfig = cli_config
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // Build the command: gemini -p "<prompt>" --output-format stream-json
        // Note: -p takes the prompt as its value (not a positional arg).
        let mut cmd = Command::new("gemini");
        cmd.arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("stream-json");

        if let Some(home) = agent_home {
            cmd.current_dir(home);
        }

        if let Some(ws) = workspace {
            cmd.arg("--include-directories").arg(ws);
        }

        // Apply provider-specific CLI flags from config.
        if let Some(ref mode) = cfg.approval_mode {
            cmd.arg("--approval-mode").arg(mode);
        }
        if cfg.sandbox == Some(true) {
            cmd.arg("--sandbox");
        }
        if let Some(ref exts) = cfg.extensions {
            for ext in exts {
                cmd.arg("-e").arg(ext);
            }
        }
        if let Some(ref policies) = cfg.policy_paths {
            for policy in policies {
                cmd.arg("--policy").arg(policy);
            }
        }
        if let Some(ref names) = cfg.allowed_mcp_server_names {
            for name in names {
                cmd.arg("--allowed-mcp-server-names").arg(name);
            }
        }
        if let Some(ref dirs) = cfg.include_directories {
            for dir in dirs {
                cmd.arg("--include-directories").arg(dir);
            }
        }
        if cfg.raw_output == Some(true) {
            cmd.arg("--raw-output");
        }
        if cfg.debug == Some(true) {
            cmd.arg("--debug");
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
                    let chunks = parse_gemini_event(&parsed);
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
            // Send SIGTERM via libc for a graceful shutdown.
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
                        "libc::kill(SIGTERM) failed for gemini adapter"
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

/// Parse a single Gemini stream-json event into zero or more [`OutputChunk`]s.
///
/// Gemini's `--output-format stream-json` emits JSONL where each object has a
/// `type` field. The event types (from `JsonStreamEventType`):
///
/// - `"init"`        — session start; contains `session_id` and `model`. Ignored.
/// - `"message"`     — text content from user or assistant. We emit assistant
///                     messages as `ChunkType::Text`. The `delta` field indicates
///                     whether this is an incremental chunk.
/// - `"tool_use"`    — the model is invoking a tool. Emitted as `ChunkType::ToolUse`.
/// - `"tool_result"` — result of a tool invocation. Ignored (informational).
/// - `"error"`       — an error from the CLI. Emitted as `ChunkType::Error`.
/// - `"result"`      — end of session with status and stats. Ignored (the spawned
///                     task sends `Done` after the process exits).
pub fn parse_gemini_event(event: &serde_json::Value) -> Vec<OutputChunk> {
    let mut chunks = Vec::new();

    let event_type = match event.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return chunks,
    };

    match event_type {
        // Assistant (or user) message content.
        "message" => {
            let role = event.get("role").and_then(|v| v.as_str()).unwrap_or("");
            // We only forward assistant messages to the caller.
            if role == "assistant" {
                let content = event.get("content").and_then(|v| v.as_str()).unwrap_or("");
                if !content.is_empty() {
                    chunks.push(OutputChunk {
                        chunk_type: ChunkType::Text,
                        content: content.to_owned(),
                    });
                }
            }
        }

        // Tool invocation by the model.
        "tool_use" => {
            let tool_name = event
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let parameters = event.get("parameters").unwrap_or(&serde_json::Value::Null);
            let content = serde_json::json!({
                "tool_name": tool_name,
                "parameters": parameters,
            });
            chunks.push(OutputChunk {
                chunk_type: ChunkType::ToolUse,
                content: serde_json::to_string(&content).unwrap_or_default(),
            });
        }

        // Error events from the CLI.
        "error" => {
            let msg = event
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            chunks.push(OutputChunk {
                chunk_type: ChunkType::Error,
                content: msg.to_owned(),
            });
        }

        // Result event with error status — surface the error.
        "result" => {
            let status = event.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status == "error" {
                let msg = event
                    .get("error")
                    .and_then(|e| e.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("session ended with error");
                chunks.push(OutputChunk {
                    chunk_type: ChunkType::Error,
                    content: msg.to_owned(),
                });
            }
            // Success result is ignored — Done is sent by the spawned task.
        }

        // init, tool_result, and unknown types are ignored.
        _ => {}
    }

    chunks
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_event_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "init",
            "timestamp": "2025-10-10T12:00:00.000Z",
            "session_id": "test-session-123",
            "model": "gemini-2.0-flash-exp"
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_assistant_message() {
        let event: serde_json::Value = serde_json::json!({
            "type": "message",
            "timestamp": "2025-10-10T12:00:01.000Z",
            "role": "assistant",
            "content": "Hello! How can I help you today?"
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "Hello! How can I help you today?");
    }

    #[test]
    fn test_parse_assistant_message_with_delta() {
        let event: serde_json::Value = serde_json::json!({
            "type": "message",
            "timestamp": "2025-10-10T12:00:01.000Z",
            "role": "assistant",
            "content": "partial text",
            "delta": true
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "partial text");
    }

    #[test]
    fn test_parse_user_message_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "message",
            "timestamp": "2025-10-10T12:00:00.000Z",
            "role": "user",
            "content": "What is 2+2?"
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_empty_assistant_message_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "message",
            "timestamp": "2025-10-10T12:00:01.000Z",
            "role": "assistant",
            "content": ""
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_tool_use_event() {
        let event: serde_json::Value = serde_json::json!({
            "type": "tool_use",
            "timestamp": "2025-10-10T12:00:02.000Z",
            "tool_name": "Read",
            "tool_id": "read-123",
            "parameters": { "file_path": "/path/to/file.txt" }
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::ToolUse));
        let parsed: serde_json::Value = serde_json::from_str(&chunks[0].content).unwrap();
        assert_eq!(parsed["tool_name"], "Read");
        assert_eq!(parsed["parameters"]["file_path"], "/path/to/file.txt");
    }

    #[test]
    fn test_parse_tool_result_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "tool_result",
            "timestamp": "2025-10-10T12:00:03.000Z",
            "tool_id": "read-123",
            "status": "success",
            "output": "File contents here"
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_error_event() {
        let event: serde_json::Value = serde_json::json!({
            "type": "error",
            "timestamp": "2025-10-10T12:00:04.000Z",
            "severity": "error",
            "message": "Loop detected, stopping execution"
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Error));
        assert_eq!(chunks[0].content, "Loop detected, stopping execution");
    }

    #[test]
    fn test_parse_error_event_warning() {
        let event: serde_json::Value = serde_json::json!({
            "type": "error",
            "timestamp": "2025-10-10T12:00:04.000Z",
            "severity": "warning",
            "message": "Rate limit approaching"
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Error));
        assert_eq!(chunks[0].content, "Rate limit approaching");
    }

    #[test]
    fn test_parse_result_success_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "result",
            "timestamp": "2025-10-10T12:00:05.000Z",
            "status": "success",
            "stats": {
                "total_tokens": 100,
                "input_tokens": 50,
                "output_tokens": 50,
                "cached": 0,
                "input": 50,
                "duration_ms": 1200,
                "tool_calls": 2,
                "models": {}
            }
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_result_error_surfaces_message() {
        let event: serde_json::Value = serde_json::json!({
            "type": "result",
            "timestamp": "2025-10-10T12:00:05.000Z",
            "status": "error",
            "error": {
                "type": "MaxSessionTurnsError",
                "message": "Maximum session turns exceeded"
            }
        });
        let chunks = parse_gemini_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Error));
        assert_eq!(chunks[0].content, "Maximum session turns exceeded");
    }

    #[test]
    fn test_parse_unknown_event_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "some_future_event",
            "timestamp": "2025-10-10T12:00:00.000Z"
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_event_without_type_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "data": "no type field"
        });
        let chunks = parse_gemini_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_default_timeout() {
        let adapter = GeminiAdapter::default();
        assert_eq!(adapter.timeout_secs, 120);
    }

    #[test]
    fn test_custom_timeout() {
        let adapter = GeminiAdapter::new(300);
        assert_eq!(adapter.timeout_secs, 300);
    }

    #[test]
    fn test_name() {
        let adapter = GeminiAdapter::default();
        assert_eq!(adapter.name(), "gemini");
    }
}
