// ---------------------------------------------------------------------------
// PR Review Tribunal
// ---------------------------------------------------------------------------
//
// Runs 3 AI models in parallel to review a GitHub PR, then debates findings
// across Opening -> Rebuttal -> Synthesis phases. The final output is a
// structured markdown comment posted to the PR.
//
// Endpoint: POST /api/pr-review
// Body:     { "owner": "...", "repo": "...", "pr_number": 123 }

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use tokio::time::{timeout, Duration};

use crate::adapter::{ChunkType, CliAdapter};
use crate::db::Agent;
use crate::orchestrate::debate::{DebateConfig, DebateEngine, DebatePhase};
use crate::orchestrate::panel::{select_adapter, spawn_panel, PanelConfig};
use crate::state::AppState;

const DEBATE_TURN_TIMEOUT: Duration = Duration::from_secs(120);

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PrReviewRequest {
    pub owner: String,
    pub repo: String,
    pub pr_number: u64,
}

#[derive(Serialize)]
struct PrReviewResponse {
    conversation_id: String,
    comment_url: Option<String>,
    markdown: String,
}

// ---------------------------------------------------------------------------
// Tribunal agent definitions
// ---------------------------------------------------------------------------

struct TribunalRole {
    name: &'static str,
    provider: &'static str,
    model: &'static str,
    color: &'static str,
    system_prompt: &'static str,
}

const TRIBUNAL_ROLES: &[TribunalRole] = &[
    TribunalRole {
        name: "Correctness Reviewer",
        provider: "claude",
        model: "sonnet",
        color: "#3b82f6",
        system_prompt: "You are a meticulous code correctness reviewer. Your role in this PR \
            review tribunal is to focus exclusively on:\n\
            - Logic errors and bugs\n\
            - Edge cases and off-by-one errors\n\
            - Incorrect assumptions about inputs or state\n\
            - Missing error handling\n\
            - Regression risks\n\n\
            Reference specific files and line numbers from the diff. \
            For each issue found, explain WHY it's wrong and suggest a fix. \
            Be specific and actionable — vague concerns are not helpful.",
    },
    TribunalRole {
        name: "Architecture Reviewer",
        provider: "gemini",
        model: "gemini-2.5-pro",
        color: "#22c55e",
        system_prompt: "You are a software architecture reviewer. Your role in this PR \
            review tribunal is to focus exclusively on:\n\
            - Design patterns and architectural fit\n\
            - Separation of concerns and modularity\n\
            - API design and naming conventions\n\
            - Maintainability and readability\n\
            - Scalability implications\n\
            - Whether the approach is the RIGHT approach, not just whether it works\n\n\
            Reference specific files and patterns from the diff. \
            Consider both local code quality and system-wide impact. \
            Suggest architectural improvements where warranted.",
    },
    TribunalRole {
        name: "Security Reviewer",
        provider: "codex",
        model: "o3",
        color: "#ef4444",
        system_prompt: "You are a security-focused code reviewer. Your role in this PR \
            review tribunal is to focus exclusively on:\n\
            - Injection vulnerabilities (SQL, command, path traversal)\n\
            - Authentication and authorization gaps\n\
            - Data validation and sanitization\n\
            - Secrets or credentials in code\n\
            - Unsafe operations and race conditions\n\
            - Dependency security concerns\n\n\
            Reference CWE IDs where applicable. \
            For each finding, rate severity (Critical/High/Medium/Low) \
            and provide a concrete remediation. \
            Do NOT flag theoretical risks that don't apply to this specific diff.",
    },
];

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

async fn pr_review_tribunal(
    State(state): State<AppState>,
    Json(req): Json<PrReviewRequest>,
) -> axum::response::Response {
    if req.owner.is_empty() || req.repo.is_empty() || req.pr_number == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "owner, repo must be non-empty and pr_number must be > 0"})),
        )
            .into_response();
    }

    let owner = req.owner;
    let repo = req.repo;
    let pr_number = req.pr_number;

    // ------------------------------------------------------------------
    // 1. Fetch PR diff via `gh`
    // ------------------------------------------------------------------
    let diff = match fetch_pr_diff(&owner, &repo, pr_number).await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": format!("Failed to fetch PR diff: {e}") })),
            )
                .into_response();
        }
    };

    if diff.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "PR diff is empty - nothing to review" })),
        )
            .into_response();
    }

    // ------------------------------------------------------------------
    // 2. Create conversation + agents in DB
    // ------------------------------------------------------------------
    let (conversation_id, agents) = {
        let db = match state.db() {
            Ok(db) => db,
            Err((status, msg)) => {
                return (status, Json(serde_json::json!({ "error": msg }))).into_response();
            }
        };

        let title = format!("PR Review: {owner}/{repo}#{pr_number}");
        let conversation = match db.create_conversation(&title, None, "tribunal") {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        };

        let mut agents: Vec<Agent> = Vec::new();
        for role in TRIBUNAL_ROLES {
            let agent = match db.create_agent(
                role.name,
                role.model,
                role.provider,
                "", // agent_home
                role.color,
                "global",
                role.system_prompt,
                None,
                None,
            ) {
                Ok(a) => a,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({ "error": e.to_string() })),
                    )
                        .into_response();
                }
            };

            if let Err(e) = db.add_agent_to_conversation(&conversation.id, &agent.id) {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
                    .into_response();
            }

            agents.push(agent);
        }

        (conversation.id, agents)
    }; // DB lock dropped here

    // ------------------------------------------------------------------
    // 3. Panel fan-out -- parallel initial reviews
    // ------------------------------------------------------------------
    let review_prompt = format!(
        "Review the following GitHub PR diff for {owner}/{repo}#{pr_number}.\n\n\
         Focus on your assigned area of expertise. Be specific, reference file names \
         and line numbers, and provide actionable feedback.\n\n\
         ```diff\n{diff}\n```"
    );

    let panel_config = PanelConfig {
        prompt: review_prompt,
        conversation_id: conversation_id.clone(),
        agent_ids: agents.iter().map(|a| a.id.clone()).collect(),
    };

    let agents_with_adapters: Vec<(Agent, Box<dyn CliAdapter>)> = {
        let mut pairs = Vec::new();
        for agent in &agents {
            let adapter = match select_adapter(&agent.provider) {
                Ok(a) => a,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": format!("Failed to create adapter for {}: {e}", agent.name)
                        })),
                    )
                        .into_response();
                }
            };
            pairs.push((agent.clone(), adapter));
        }
        pairs
    };

    let mut panel_rx = spawn_panel(&panel_config, agents_with_adapters);

    // Collect all panel responses grouped by agent.
    let mut reviews: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();
    for agent in &agents {
        reviews.insert(agent.id.clone(), (agent.name.clone(), String::new()));
    }

    while let Some(tagged) = panel_rx.recv().await {
        if matches!(tagged.chunk.chunk_type, ChunkType::Text) {
            if let Some((_name, content)) = reviews.get_mut(&tagged.agent_id) {
                content.push_str(&tagged.chunk.content);
            }
        }
    }

    // Save initial reviews as messages.
    {
        let db = match state.db() {
            Ok(db) => db,
            Err((status, msg)) => {
                return (status, Json(serde_json::json!({ "error": msg }))).into_response();
            }
        };
        for agent in &agents {
            if let Some((_name, content)) = reviews.get(&agent.id) {
                if !content.is_empty() {
                    if let Err(e) = db.create_message(
                        &conversation_id,
                        Some(&agent.id),
                        "assistant",
                        content,
                        Some(&agent.model),
                    ) {
                        tracing::warn!("Failed to save initial review message for agent {}: {e}", agent.id);
                    }
                }
            }
        }
    } // DB lock dropped here

    // Build ordered review texts for the debate phase.
    let initial_reviews: Vec<(String, String)> = agents
        .iter()
        .filter_map(|a| {
            reviews
                .get(&a.id)
                .filter(|(_, content)| !content.is_empty())
                .map(|(name, content)| (name.clone(), content.clone()))
        })
        .collect();

    // ------------------------------------------------------------------
    // 4. Debate phase -- Opening -> Rebuttal -> Synthesis
    // ------------------------------------------------------------------
    let debate_topic = format!(
        "Review findings for PR {owner}/{repo}#{pr_number}. \
         The initial independent reviews are:\n\n{}",
        initial_reviews
            .iter()
            .map(|(name, content)| format!("### {name}\n{content}"))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    );

    let debate_config = DebateConfig {
        topic: debate_topic,
        num_rounds: 1, // Single rebuttal round to keep token cost reasonable
        moderator_agent_id: Some(agents[0].id.clone()), // Claude moderates synthesis
        participant_agent_ids: agents.iter().map(|a| a.id.clone()).collect(),
        conversation_id: conversation_id.clone(),
    };

    let debate_transcript = run_debate(
        &state,
        &conversation_id,
        &agents,
        debate_config,
    )
    .await;

    // ------------------------------------------------------------------
    // 5. Format tribunal output as markdown
    // ------------------------------------------------------------------
    let markdown = format_tribunal_as_markdown(
        &owner,
        &repo,
        pr_number,
        &initial_reviews,
        &debate_transcript,
    );

    // ------------------------------------------------------------------
    // 6. Post comment to GitHub PR
    // ------------------------------------------------------------------
    let comment_url = match post_pr_comment(&owner, &repo, pr_number, &markdown).await {
        Ok(url) => Some(url),
        Err(e) => {
            tracing::warn!(
                "Failed to post PR comment to {}/{}{}: {}",
                owner, repo, pr_number, e,
            );
            None
        }
    };

    (
        StatusCode::OK,
        Json(PrReviewResponse {
            conversation_id,
            comment_url,
            markdown,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Debate runner (separated to keep handler manageable)
// ---------------------------------------------------------------------------

/// Run the debate engine to completion, returning a transcript of
/// `(phase_name, agent_name, content)` tuples.
async fn run_debate(
    state: &AppState,
    conversation_id: &str,
    agents: &[Agent],
    config: DebateConfig,
) -> Vec<(String, String, String)> {
    let mut engine = DebateEngine::new(config);
    let mut all_turns: Vec<String> = Vec::new();
    let mut transcript: Vec<(String, String, String)> = Vec::new();

    while !engine.is_complete() {
        let agent_id = match engine.next_agent_id() {
            Some(id) => id.to_owned(),
            None => break,
        };

        let phase_name = match engine.current_phase() {
            DebatePhase::Opening => "Opening".to_owned(),
            DebatePhase::Rebuttal(n) => format!("Rebuttal {n}"),
            DebatePhase::Closing => "Closing".to_owned(),
            DebatePhase::Synthesis => "Synthesis".to_owned(),
            DebatePhase::Complete => break,
        };

        // Clone agent data we need so no borrows span across await.
        let agent = match agents.iter().find(|a| a.id == agent_id) {
            Some(a) => a.clone(),
            None => {
                engine.advance();
                continue;
            }
        };

        let prompt = engine.build_turn_prompt(&agent.name, &all_turns);

        // Spawn single adapter for this turn.
        let adapter = match select_adapter(&agent.provider) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("Failed to create adapter for {} ({}): {e}", agent.name, agent.provider);
                engine.advance();
                continue;
            }
        };

        let agent_home: Option<String> = if agent.agent_home.is_empty() {
            None
        } else {
            Some(agent.agent_home.clone())
        };

        let mut turn_content = String::new();
        match timeout(
            DEBATE_TURN_TIMEOUT,
            adapter.spawn(&prompt, agent_home.as_deref(), None, agent.cli_config.as_deref()),
        )
        .await
        {
            Ok(Ok(mut rx)) => {
                while let Some(chunk) = rx.recv().await {
                    if matches!(chunk.chunk_type, ChunkType::Text) {
                        turn_content.push_str(&chunk.content);
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Adapter spawn error for {} during {}: {e}", agent.name, phase_name);
                turn_content = format!("[Adapter error: {e}]");
            }
            Err(_) => {
                tracing::warn!("Debate turn timed out for {} during {} ({}s limit)", agent.name, phase_name, DEBATE_TURN_TIMEOUT.as_secs());
                turn_content = format!("[Timeout after {}s]", DEBATE_TURN_TIMEOUT.as_secs());
            }
        }

        // Save debate turn as a message.
        {
            match state.db() {
                Ok(db) => {
                    if let Err(e) = db.create_message(
                        conversation_id,
                        Some(&agent.id),
                        "assistant",
                        &turn_content,
                        Some(&agent.model),
                    ) {
                        tracing::warn!("Failed to save debate turn message for agent {}: {e}", agent.id);
                    }
                }
                Err((_, msg)) => {
                    tracing::warn!("Failed to acquire DB for debate turn save: {msg}");
                }
            }
        }

        let turn_label = format!("[{} - {}]: {}", phase_name, agent.name, turn_content);
        all_turns.push(turn_label);
        transcript.push((phase_name, agent.name.clone(), turn_content));

        engine.advance();
    }

    transcript
}

// ---------------------------------------------------------------------------
// GitHub helpers
// ---------------------------------------------------------------------------

/// Fetch the unified diff for a PR using the `gh` CLI.
async fn fetch_pr_diff(owner: &str, repo: &str, pr_number: u64) -> Result<String, String> {
    let output = tokio::process::Command::new("gh")
        .args([
            "api",
            &format!("repos/{owner}/{repo}/pulls/{pr_number}/files"),
            "--jq",
            r#"[.[] | "diff --git a/\(.filename) b/\(.filename)\n\(.patch // "")"] | join("\n")"#,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run gh CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh api failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Post a markdown comment to a GitHub PR using the `gh` CLI.
async fn post_pr_comment(
    owner: &str,
    repo: &str,
    pr_number: u64,
    body: &str,
) -> Result<String, String> {
    let payload = serde_json::json!({ "body": body });

    let mut child = tokio::process::Command::new("gh")
        .args([
            "api",
            &format!("repos/{owner}/{repo}/issues/{pr_number}/comments"),
            "--method",
            "POST",
            "--input",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn gh CLI: {e}"))?;

    // Write JSON payload to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let payload_bytes = payload.to_string().into_bytes();
        stdin
            .write_all(&payload_bytes)
            .await
            .map_err(|e| format!("Failed to write to gh stdin: {e}"))?;
        // Drop stdin to signal EOF.
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for gh CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh api POST failed: {stderr}"));
    }

    // Extract the HTML URL from the response JSON.
    let response: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
            tracing::warn!("Failed to parse gh API response as JSON: {e}");
            serde_json::json!({})
        });
    let url = response
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();

    Ok(url)
}

// ---------------------------------------------------------------------------
// Markdown formatting
// ---------------------------------------------------------------------------

/// Format the tribunal results as a structured markdown comment.
///
/// Extracts:
/// - **High Confidence** -- points mentioned by 2+ reviewers
/// - **Contested** -- points where reviewers disagree
/// - **Final Verdict** -- synthesis from the debate moderator
fn format_tribunal_as_markdown(
    owner: &str,
    repo: &str,
    pr_number: u64,
    initial_reviews: &[(String, String)],
    debate_transcript: &[(String, String, String)],
) -> String {
    let mut md = String::new();

    // Header
    md.push_str(&format!(
        "# PR Review Tribunal: {owner}/{repo}#{pr_number}\n\n"
    ));
    md.push_str(
        "> Three AI reviewers independently analyzed this PR, \
         then debated their findings.\n\n",
    );

    // --- Individual Reviews ---
    md.push_str("## Independent Reviews\n\n");
    for (name, content) in initial_reviews {
        md.push_str(&format!("### {name}\n\n"));
        md.push_str(&format!("{content}\n\n"));
    }

    // --- Debate Summary ---
    // Extract synthesis (the last debate turn in Synthesis phase).
    let synthesis = debate_transcript
        .iter()
        .filter(|(phase, _, _)| phase == "Synthesis")
        .map(|(_, _, content)| content.as_str())
        .last();

    // Extract rebuttal/closing statements for the contested section.
    let closing_and_rebuttals: Vec<&(String, String, String)> = debate_transcript
        .iter()
        .filter(|(phase, _, _)| phase.starts_with("Rebuttal") || phase == "Closing")
        .collect();

    // --- High Confidence Section ---
    md.push_str("## High Confidence Findings\n\n");
    md.push_str(
        "> Points corroborated by multiple reviewers during the debate.\n\n",
    );
    if let Some(synth) = synthesis {
        md.push_str(synth);
        md.push_str("\n\n");
    } else {
        md.push_str("_No synthesis was produced._\n\n");
    }

    // --- Contested Section ---
    if !closing_and_rebuttals.is_empty() {
        md.push_str("## Contested Points\n\n");
        md.push_str(
            "> Areas where reviewers disagreed during rebuttal rounds.\n\n",
        );
        for (phase, agent_name, content) in &closing_and_rebuttals {
            md.push_str(&format!(
                "<details>\n<summary><b>{agent_name}</b> ({phase})</summary>\n\n\
                 {content}\n\n\
                 </details>\n\n"
            ));
        }
    }

    // --- Final Verdict ---
    md.push_str("## Final Verdict\n\n");
    if let Some(synth) = synthesis {
        md.push_str(synth);
    } else {
        md.push_str("_The tribunal did not reach a final verdict._");
    }
    md.push_str("\n\n---\n");
    md.push_str("_Generated by RoboCaucus PR Review Tribunal_\n");

    md
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn pr_review_routes() -> Router<AppState> {
    Router::new().route("/pr-review", post(pr_review_tribunal))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tribunal_markdown_basic() {
        let reviews = vec![
            (
                "Correctness Reviewer".to_owned(),
                "Found a bug on line 42.".to_owned(),
            ),
            (
                "Architecture Reviewer".to_owned(),
                "The module structure is clean.".to_owned(),
            ),
            (
                "Security Reviewer".to_owned(),
                "No security issues found.".to_owned(),
            ),
        ];

        let debate = vec![
            (
                "Opening".to_owned(),
                "Correctness Reviewer".to_owned(),
                "I found a critical bug.".to_owned(),
            ),
            (
                "Opening".to_owned(),
                "Architecture Reviewer".to_owned(),
                "Architecture looks solid.".to_owned(),
            ),
            (
                "Opening".to_owned(),
                "Security Reviewer".to_owned(),
                "No vulnerabilities detected.".to_owned(),
            ),
            (
                "Rebuttal 1".to_owned(),
                "Correctness Reviewer".to_owned(),
                "I stand by my bug finding.".to_owned(),
            ),
            (
                "Rebuttal 1".to_owned(),
                "Architecture Reviewer".to_owned(),
                "The bug seems valid but minor.".to_owned(),
            ),
            (
                "Rebuttal 1".to_owned(),
                "Security Reviewer".to_owned(),
                "Agreed, not a security concern.".to_owned(),
            ),
            (
                "Synthesis".to_owned(),
                "Correctness Reviewer".to_owned(),
                "Overall: one minor bug, solid architecture, no security issues.".to_owned(),
            ),
        ];

        let md = format_tribunal_as_markdown("acme", "widget", 42, &reviews, &debate);

        assert!(md.contains("# PR Review Tribunal: acme/widget#42"));
        assert!(md.contains("## Independent Reviews"));
        assert!(md.contains("### Correctness Reviewer"));
        assert!(md.contains("### Architecture Reviewer"));
        assert!(md.contains("### Security Reviewer"));
        assert!(md.contains("## High Confidence Findings"));
        assert!(md.contains("## Contested Points"));
        assert!(md.contains("## Final Verdict"));
        assert!(md.contains("one minor bug"));
        assert!(md.contains("RoboCaucus PR Review Tribunal"));
    }

    #[test]
    fn test_format_tribunal_markdown_no_synthesis() {
        let reviews = vec![(
            "Reviewer".to_owned(),
            "Some findings.".to_owned(),
        )];
        let debate = vec![(
            "Opening".to_owned(),
            "Reviewer".to_owned(),
            "Opening statement.".to_owned(),
        )];

        let md = format_tribunal_as_markdown("org", "repo", 1, &reviews, &debate);

        assert!(md.contains("No synthesis was produced"));
        assert!(md.contains("did not reach a final verdict"));
    }

    #[test]
    fn test_format_tribunal_markdown_empty_debate() {
        let reviews = vec![];
        let debate = vec![];

        let md = format_tribunal_as_markdown("org", "repo", 1, &reviews, &debate);

        assert!(md.contains("# PR Review Tribunal"));
        assert!(md.contains("## Independent Reviews"));
        assert!(md.contains("## Final Verdict"));
    }

    #[test]
    fn test_tribunal_roles_are_configured() {
        assert_eq!(TRIBUNAL_ROLES.len(), 3);
        assert_eq!(TRIBUNAL_ROLES[0].name, "Correctness Reviewer");
        assert_eq!(TRIBUNAL_ROLES[0].provider, "claude");
        assert_eq!(TRIBUNAL_ROLES[1].name, "Architecture Reviewer");
        assert_eq!(TRIBUNAL_ROLES[1].provider, "gemini");
        assert_eq!(TRIBUNAL_ROLES[2].name, "Security Reviewer");
        assert_eq!(TRIBUNAL_ROLES[2].provider, "codex");
    }
}
