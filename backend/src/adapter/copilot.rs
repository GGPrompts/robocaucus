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

/// Provider-specific CLI flag overrides for the Copilot adapter.
///
/// Deserialized from the agent's `cli_config` JSON column.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct CopilotConfig {
    /// Maps to `--effort` (e.g. "low", "medium", "high").
    pub reasoning_effort: Option<String>,
    /// Maps to `--autopilot`.
    pub autopilot: Option<bool>,
    /// Maps to `--max-autopilot-continues`.
    pub max_autopilot_continues: Option<u32>,
    /// Maps to `--no-ask-user`.
    pub no_ask_user: Option<bool>,
    /// Maps to permission presets: "allow-all" -> `--allow-all`,
    /// "allow-all-tools" -> `--allow-all-tools`.
    pub permission_preset: Option<String>,
    /// Each entry becomes a separate `--allow-tool` argument.
    pub allowed_tools: Option<Vec<String>>,
    /// Each entry becomes a separate `--deny-tool` argument.
    pub denied_tools: Option<Vec<String>>,
    /// Each entry becomes a separate `--allow-url` argument.
    pub allowed_urls: Option<Vec<String>>,
    /// Each entry becomes a separate `--deny-url` argument.
    pub denied_urls: Option<Vec<String>>,
    /// Each entry becomes a separate `--secret-env-vars` argument.
    pub secret_env_vars: Option<Vec<String>>,
    /// Maps to `--disable-builtin-mcps`.
    pub disable_builtin_mcps: Option<bool>,
    /// Maps to `--enable-all-github-mcp-tools`.
    pub enable_all_github_mcp_tools: Option<bool>,
    /// Maps to `--no-custom-instructions`.
    pub no_custom_instructions: Option<bool>,
    /// Maps to `--stream` (e.g. "full", "diff").
    pub stream: Option<String>,
}

// ---------------------------------------------------------------------------
// Copilot adapter
// ---------------------------------------------------------------------------

/// Adapter that drives the GitHub `copilot` CLI in JSONL mode.
///
/// Spawns `copilot -p "<prompt>" --output-format json --allow-all-tools` and
/// reads JSONL from stdout. The Copilot CLI emits events with types such as
/// `assistant.message_delta`, `assistant.reasoning_delta`, `assistant.message`,
/// `assistant.tool_use`, `result`, etc.
pub struct CopilotAdapter {
    /// Per-PID child-process handles, used for cancellation.
    children: Arc<Mutex<HashMap<u32, tokio::process::Child>>>,
    /// Maximum wall-clock time (seconds) before the process is killed.
    timeout_secs: u64,
}

impl CopilotAdapter {
    /// Create a new adapter with the given timeout (seconds).
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            children: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs,
        }
    }
}

impl Default for CopilotAdapter {
    fn default() -> Self {
        Self::new(120)
    }
}

#[async_trait::async_trait]
impl CliAdapter for CopilotAdapter {
    fn name(&self) -> &str {
        "copilot"
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
            .arg("copilot")
            .output()
            .await
            .map_err(AdapterError::SpawnFailed)?;

        if !which.status.success() {
            return Err(AdapterError::CliNotFound("copilot".into()));
        }

        // Parse provider-specific config (if any).
        let cfg: CopilotConfig = cli_config
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // Build the command: copilot -p "<prompt>" --output-format json
        // --allow-all-tools is the default unless overridden by permission_preset.
        let mut cmd = Command::new("copilot");
        cmd.arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("json");

        // Apply permission preset or default to --allow-all-tools.
        match cfg.permission_preset.as_deref() {
            Some("allow-all") => { cmd.arg("--allow-all"); }
            Some("allow-all-tools") | None => { cmd.arg("--allow-all-tools"); }
            Some(other) => {
                tracing::warn!("Unknown copilot permission_preset '{}', defaulting to --allow-all-tools", other);
                cmd.arg("--allow-all-tools");
            }
        }

        if let Some(home) = agent_home {
            cmd.current_dir(home);
        }

        if let Some(ws) = workspace {
            cmd.arg("--add-dir").arg(ws);
        }

        // Apply provider-specific CLI flags from config.
        if let Some(ref effort) = cfg.reasoning_effort {
            cmd.arg("--effort").arg(effort);
        }
        if cfg.autopilot == Some(true) {
            cmd.arg("--autopilot");
        }
        if let Some(max) = cfg.max_autopilot_continues {
            cmd.arg("--max-autopilot-continues").arg(max.to_string());
        }
        if cfg.no_ask_user == Some(true) {
            cmd.arg("--no-ask-user");
        }
        if let Some(ref tools) = cfg.allowed_tools {
            for tool in tools {
                cmd.arg("--allow-tool").arg(tool);
            }
        }
        if let Some(ref tools) = cfg.denied_tools {
            for tool in tools {
                cmd.arg("--deny-tool").arg(tool);
            }
        }
        if let Some(ref urls) = cfg.allowed_urls {
            for url in urls {
                cmd.arg("--allow-url").arg(url);
            }
        }
        if let Some(ref urls) = cfg.denied_urls {
            for url in urls {
                cmd.arg("--deny-url").arg(url);
            }
        }
        if let Some(ref vars) = cfg.secret_env_vars {
            for var in vars {
                cmd.arg("--secret-env-vars").arg(var);
            }
        }
        if cfg.disable_builtin_mcps == Some(true) {
            cmd.arg("--disable-builtin-mcps");
        }
        if cfg.enable_all_github_mcp_tools == Some(true) {
            cmd.arg("--enable-all-github-mcp-tools");
        }
        if cfg.no_custom_instructions == Some(true) {
            cmd.arg("--no-custom-instructions");
        }
        if let Some(ref stream_mode) = cfg.stream {
            cmd.arg("--stream").arg(stream_mode);
        }

        // We only need stdout; inherit stderr so operator can see diagnostics.
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::inherit());

        let mut child = cmd.spawn().map_err(AdapterError::SpawnFailed)?;

        let pid = child.id().ok_or_else(|| {
            AdapterError::SpawnFailed(std::io::Error::other(
                "child process exited before PID could be read",
            ))
        })?;

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
                    let chunks = parse_copilot_event(&parsed);
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
                        "libc::kill(SIGTERM) failed for copilot adapter"
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

/// Parse a single Copilot JSONL event into zero or more [`OutputChunk`]s.
///
/// The Copilot CLI `--output-format json` emits JSONL where each object has a
/// `type` field. The key event types we handle:
///
/// - `"assistant.message_delta"` — incremental text delta. The text is in
///   `data.deltaContent`.
/// - `"assistant.reasoning_delta"` — incremental reasoning/thinking delta. The
///   text is in `data.deltaContent`.
/// - `"assistant.message"` — full assistant message (non-streaming fallback).
///   The text is in `data.content`. Also contains `data.reasoningText` for
///   thinking.
/// - `"assistant.tool_use"` — tool invocation by the model.
/// - `"result"` — end of session, with `exitCode`. Non-zero exit codes are
///   surfaced as errors.
///
/// Ephemeral session/init events, `user.message`, `assistant.turn_start`,
/// `assistant.turn_end`, and `assistant.reasoning` are ignored.
pub fn parse_copilot_event(event: &serde_json::Value) -> Vec<OutputChunk> {
    let mut chunks = Vec::new();

    let event_type = match event.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return chunks,
    };

    match event_type {
        // Incremental text delta from the assistant.
        "assistant.message_delta" => {
            if let Some(data) = event.get("data") {
                if let Some(text) = data.get("deltaContent").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        chunks.push(OutputChunk {
                            chunk_type: ChunkType::Text,
                            content: text.to_owned(),
                        });
                    }
                }
            }
        }

        // Incremental reasoning/thinking delta.
        "assistant.reasoning_delta" => {
            if let Some(data) = event.get("data") {
                if let Some(text) = data.get("deltaContent").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        chunks.push(OutputChunk {
                            chunk_type: ChunkType::Thinking,
                            content: text.to_owned(),
                        });
                    }
                }
            }
        }

        // Full assistant message (non-streaming / final).
        // We only use this if we didn't already get deltas — but emitting it
        // is safe because the caller accumulates text.
        "assistant.message" => {
            if let Some(data) = event.get("data") {
                if let Some(text) = data.get("content").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        chunks.push(OutputChunk {
                            chunk_type: ChunkType::Text,
                            content: text.to_owned(),
                        });
                    }
                }

                // Surface tool requests embedded in the message.
                if let Some(tool_requests) = data.get("toolRequests").and_then(|v| v.as_array()) {
                    for tool_req in tool_requests {
                        if let Some(tool_name) =
                            tool_req.get("toolName").and_then(|v| v.as_str())
                        {
                            let parameters = tool_req
                                .get("parameters")
                                .unwrap_or(&serde_json::Value::Null);
                            let content = serde_json::json!({
                                "tool_name": tool_name,
                                "parameters": parameters,
                            });
                            chunks.push(OutputChunk {
                                chunk_type: ChunkType::ToolUse,
                                content: serde_json::to_string(&content).unwrap_or_default(),
                            });
                        }
                    }
                }
            }
        }

        // Explicit tool use event (if emitted separately from the message).
        "assistant.tool_use" => {
            if let Some(data) = event.get("data") {
                let tool_name = data
                    .get("toolName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let parameters = data
                    .get("parameters")
                    .unwrap_or(&serde_json::Value::Null);
                let content = serde_json::json!({
                    "tool_name": tool_name,
                    "parameters": parameters,
                });
                chunks.push(OutputChunk {
                    chunk_type: ChunkType::ToolUse,
                    content: serde_json::to_string(&content).unwrap_or_default(),
                });
            }
        }

        // End of session.
        "result" => {
            let exit_code = event
                .get("exitCode")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if exit_code != 0 {
                chunks.push(OutputChunk {
                    chunk_type: ChunkType::Error,
                    content: format!("Copilot session ended with exit code {exit_code}"),
                });
            }
            // Success result is ignored — Done is sent by the spawned task.
        }

        // All other events are ignored: session.*, user.message,
        // assistant.turn_start, assistant.turn_end, assistant.reasoning, etc.
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
    fn test_parse_message_delta() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.message_delta",
            "data": {
                "messageId": "abc-123",
                "deltaContent": "Hello, world!"
            },
            "id": "evt-1",
            "timestamp": "2026-01-01T00:00:00.000Z"
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "Hello, world!");
    }

    #[test]
    fn test_parse_message_delta_empty_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.message_delta",
            "data": {
                "messageId": "abc-123",
                "deltaContent": ""
            }
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_reasoning_delta() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.reasoning_delta",
            "data": {
                "reasoningId": "r-123",
                "deltaContent": "Thinking about the problem..."
            },
            "id": "evt-2",
            "timestamp": "2026-01-01T00:00:00.000Z",
            "ephemeral": true
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Thinking));
        assert_eq!(chunks[0].content, "Thinking about the problem...");
    }

    #[test]
    fn test_parse_reasoning_delta_empty_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.reasoning_delta",
            "data": {
                "reasoningId": "r-123",
                "deltaContent": ""
            }
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_full_assistant_message() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg-456",
                "content": "Here is the full response.",
                "toolRequests": [],
                "interactionId": "int-789"
            },
            "id": "evt-3",
            "timestamp": "2026-01-01T00:00:01.000Z"
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Text));
        assert_eq!(chunks[0].content, "Here is the full response.");
    }

    #[test]
    fn test_parse_assistant_message_with_tool_requests() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg-456",
                "content": "",
                "toolRequests": [
                    {
                        "toolName": "read_file",
                        "parameters": { "path": "/tmp/test.txt" }
                    }
                ]
            }
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::ToolUse));
        let parsed: serde_json::Value = serde_json::from_str(&chunks[0].content).unwrap();
        assert_eq!(parsed["tool_name"], "read_file");
        assert_eq!(parsed["parameters"]["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_parse_tool_use_event() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.tool_use",
            "data": {
                "toolName": "shell",
                "parameters": { "command": "ls -la" }
            },
            "id": "evt-4",
            "timestamp": "2026-01-01T00:00:02.000Z"
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::ToolUse));
        let parsed: serde_json::Value = serde_json::from_str(&chunks[0].content).unwrap();
        assert_eq!(parsed["tool_name"], "shell");
        assert_eq!(parsed["parameters"]["command"], "ls -la");
    }

    #[test]
    fn test_parse_result_success_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "result",
            "timestamp": "2026-01-01T00:00:05.000Z",
            "sessionId": "session-abc",
            "exitCode": 0,
            "usage": {
                "premiumRequests": 1,
                "totalApiDurationMs": 5000,
                "sessionDurationMs": 8000
            }
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_result_error_exit_code() {
        let event: serde_json::Value = serde_json::json!({
            "type": "result",
            "timestamp": "2026-01-01T00:00:05.000Z",
            "sessionId": "session-abc",
            "exitCode": 1
        });
        let chunks = parse_copilot_event(&event);
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0].chunk_type, ChunkType::Error));
        assert!(chunks[0].content.contains("exit code 1"));
    }

    #[test]
    fn test_parse_session_events_ignored() {
        for event_type in &[
            "session.mcp_server_status_changed",
            "session.mcp_servers_loaded",
            "session.tools_updated",
        ] {
            let event: serde_json::Value = serde_json::json!({
                "type": event_type,
                "data": {},
                "ephemeral": true
            });
            let chunks = parse_copilot_event(&event);
            assert!(
                chunks.is_empty(),
                "event type '{event_type}' should be ignored"
            );
        }
    }

    #[test]
    fn test_parse_user_message_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "user.message",
            "data": {
                "content": "Hello"
            }
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_turn_events_ignored() {
        for event_type in &["assistant.turn_start", "assistant.turn_end"] {
            let event: serde_json::Value = serde_json::json!({
                "type": event_type,
                "data": { "turnId": "0" }
            });
            let chunks = parse_copilot_event(&event);
            assert!(
                chunks.is_empty(),
                "event type '{event_type}' should be ignored"
            );
        }
    }

    #[test]
    fn test_parse_reasoning_full_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "type": "assistant.reasoning",
            "data": {
                "reasoningId": "r-123",
                "content": "Full reasoning text"
            },
            "ephemeral": true
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_parse_event_without_type_ignored() {
        let event: serde_json::Value = serde_json::json!({
            "data": "no type field"
        });
        let chunks = parse_copilot_event(&event);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_default_timeout() {
        let adapter = CopilotAdapter::default();
        assert_eq!(adapter.timeout_secs, 120);
    }

    #[test]
    fn test_custom_timeout() {
        let adapter = CopilotAdapter::new(300);
        assert_eq!(adapter.timeout_secs, 300);
    }

    #[test]
    fn test_name() {
        let adapter = CopilotAdapter::default();
        assert_eq!(adapter.name(), "copilot");
    }
}
