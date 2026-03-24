use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::time::Duration;
use tokio::process::Command;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub version: Option<String>,
    pub cli_command: String,
}

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<Provider>,
}

// ---------------------------------------------------------------------------
// CLI detection helpers
// ---------------------------------------------------------------------------

/// Specification for a CLI provider to detect.
struct CliSpec {
    id: &'static str,
    name: &'static str,
    command: &'static str,
    args: &'static [&'static str],
}

const CLI_SPECS: &[CliSpec] = &[
    CliSpec {
        id: "claude",
        name: "Claude",
        command: "claude",
        args: &["--version"],
    },
    CliSpec {
        id: "codex",
        name: "ChatGPT/Codex",
        command: "codex",
        args: &["--version"],
    },
    CliSpec {
        id: "gemini",
        name: "Gemini",
        command: "gemini",
        args: &["--version"],
    },
    CliSpec {
        id: "copilot",
        name: "GitHub Copilot",
        command: "gh",
        args: &["copilot", "--version"],
    },
];

const CHECK_TIMEOUT: Duration = Duration::from_secs(5);

/// Run `<command> <args>` with a timeout and return the first line that looks
/// like a version string, or the full trimmed stdout if non-empty.
async fn try_version_command(command: &str, args: &[&str]) -> Option<String> {
    let result = tokio::time::timeout(CHECK_TIMEOUT, async {
        Command::new(command)
            .args(args)
            .output()
            .await
    })
    .await;

    let output = match result {
        Ok(Ok(output)) if output.status.success() => output,
        _ => return None,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Some CLIs print version to stdout, others to stderr.
    let text = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Return the first non-empty line as the version string.
    Some(
        trimmed
            .lines()
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_string(),
    )
}

/// Check if a command is on PATH via `which`.
async fn try_which(command: &str) -> bool {
    let result = tokio::time::timeout(CHECK_TIMEOUT, async {
        Command::new("which")
            .arg(command)
            .output()
            .await
    })
    .await;

    matches!(result, Ok(Ok(output)) if output.status.success())
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

async fn list_providers() -> impl IntoResponse {
    // Run all CLI checks concurrently using JoinSet.
    let mut set = tokio::task::JoinSet::new();

    for spec in CLI_SPECS {
        let id = spec.id;
        let name = spec.name;
        let command = spec.command;
        let args: Vec<&'static str> = spec.args.to_vec();

        set.spawn(async move {
            let cli_command = if id == "copilot" {
                "gh copilot".to_string()
            } else {
                command.to_string()
            };

            // First attempt: run `<command> --version`
            if let Some(version) = try_version_command(command, &args).await {
                return Provider {
                    id: id.to_string(),
                    name: name.to_string(),
                    available: true,
                    version: Some(version),
                    cli_command,
                };
            }

            // Fallback: check if the command exists via `which`
            if try_which(command).await {
                return Provider {
                    id: id.to_string(),
                    name: name.to_string(),
                    available: true,
                    version: None,
                    cli_command,
                };
            }

            Provider {
                id: id.to_string(),
                name: name.to_string(),
                available: false,
                version: None,
                cli_command,
            }
        });
    }

    // Collect results and sort by the original CLI_SPECS order.
    let mut providers = Vec::with_capacity(CLI_SPECS.len());
    while let Some(result) = set.join_next().await {
        if let Ok(provider) = result {
            providers.push(provider);
        }
    }

    // Restore deterministic ordering (claude, codex, gemini, copilot).
    let order: Vec<&str> = CLI_SPECS.iter().map(|s| s.id).collect();
    providers.sort_by_key(|p| {
        order.iter().position(|&id| id == p.id).unwrap_or(usize::MAX)
    });

    (
        StatusCode::OK,
        Json(ProvidersResponse { providers }),
    )
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn provider_routes() -> Router<AppState> {
    Router::new().route("/providers", get(list_providers))
}
