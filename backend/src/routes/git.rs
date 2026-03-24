use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::state::AppState;

pub fn git_routes() -> Router<AppState> {
    Router::new()
        .route("/git/graph", get(git_graph))
        .route("/git/commit/{hash}", get(git_commit_details))
        .route("/git/diff", get(git_diff))
        .route("/git/status", get(git_status))
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Resolve a path, expanding `~` to `$HOME`.
fn expand_path(raw: &str) -> String {
    if raw.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &raw[1..]);
        }
    }
    raw.to_string()
}

/// Walk upward from `start` to find the git repository root.
fn find_git_root(start: &str) -> Option<String> {
    let mut path = std::path::PathBuf::from(start);
    loop {
        if path.join(".git").exists() {
            return Some(path.to_string_lossy().to_string());
        }
        if !path.pop() {
            return None;
        }
    }
}

/// Resolve and validate the git root from an optional `?path=` query param.
fn resolve_git_root(
    path: Option<String>,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let raw_path = match path {
        Some(p) if !p.is_empty() => p,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "path parameter required"})),
            ));
        }
    };
    let expanded = expand_path(&raw_path);
    let canonical = std::path::Path::new(&expanded)
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or(expanded);
    match find_git_root(&canonical) {
        Some(r) => Ok(r),
        None => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "not a git repository"})),
        )),
    }
}

// ── GET /git/graph ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitGraphParams {
    path: Option<String>,
    limit: Option<usize>,
    skip: Option<usize>,
}

async fn git_graph(Query(params): Query<GitGraphParams>) -> impl IntoResponse {
    let git_root = match resolve_git_root(params.path) {
        Ok(r) => r,
        Err(e) => return e,
    };

    let limit = params.limit.unwrap_or(50);
    let skip = params.skip.unwrap_or(0);

    // Request limit+1 to detect hasMore
    let format_str = "%H|%h|%an|%ae|%aI|%P|%D|%s";
    let output = tokio::process::Command::new("git")
        .args([
            "-C",
            &git_root,
            "log",
            "--all",
            &format!("--format={}", format_str),
            &format!("-n{}", limit + 1),
            &format!("--skip={}", skip),
        ])
        .output()
        .await;

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("git log failed: {}", stderr)})),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to run git: {}", e)})),
            );
        }
    };

    let mut commits: Vec<serde_json::Value> = Vec::new();
    for line in output.trim().lines() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(8, '|').collect();
        if parts.len() < 8 {
            continue;
        }

        let parents: Vec<&str> = if parts[5].is_empty() {
            Vec::new()
        } else {
            parts[5].split_whitespace().collect()
        };

        let refs: Vec<&str> = if parts[6].is_empty() {
            Vec::new()
        } else {
            parts[6].split(", ").map(|s| s.trim()).collect()
        };

        commits.push(serde_json::json!({
            "hash": parts[0],
            "shortHash": parts[1],
            "author": parts[2],
            "email": parts[3],
            "date": parts[4],
            "parents": parents,
            "refs": refs,
            "message": parts[7],
        }));
    }

    let has_more = commits.len() > limit;
    if has_more {
        commits.truncate(limit);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "data": {
                "commits": commits,
                "hasMore": has_more,
            }
        })),
    )
}

// ── GET /git/commit/{hash} ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitCommitParams {
    path: Option<String>,
}

async fn git_commit_details(
    Path(hash): Path<String>,
    Query(params): Query<GitCommitParams>,
) -> impl IntoResponse {
    let git_root = match resolve_git_root(params.path) {
        Ok(r) => r,
        Err(e) => return e,
    };

    // Get commit info with body
    let format_str = "%H|%h|%an|%ae|%aI|%P|%D|%s|%b";
    let output = tokio::process::Command::new("git")
        .args([
            "-C",
            &git_root,
            "log",
            "-1",
            &format!("--format={}", format_str),
            &hash,
        ])
        .output()
        .await;

    let commit_line = match output {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "commit not found"})),
                );
            }
            s
        }
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "commit not found"})),
            );
        }
    };

    let parts: Vec<&str> = commit_line.splitn(9, '|').collect();
    if parts.len() < 9 {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "failed to parse commit"})),
        );
    }

    let parents: Vec<&str> = if parts[5].is_empty() {
        Vec::new()
    } else {
        parts[5].split_whitespace().collect()
    };

    let refs: Vec<&str> = if parts[6].is_empty() {
        Vec::new()
    } else {
        parts[6].split(", ").map(|s| s.trim()).collect()
    };

    let body = parts[8].trim();

    // Get changed files via name-status
    let status_output = tokio::process::Command::new("git")
        .args([
            "-C",
            &git_root,
            "diff-tree",
            "--no-commit-id",
            "--name-status",
            "-r",
            &hash,
        ])
        .output()
        .await;

    // Get numstat for additions/deletions
    let numstat_output = tokio::process::Command::new("git")
        .args([
            "-C",
            &git_root,
            "diff-tree",
            "--no-commit-id",
            "--numstat",
            "-r",
            &hash,
        ])
        .output()
        .await;

    // Parse name-status into a map
    let mut status_map = std::collections::HashMap::new();
    if let Ok(o) = &status_output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.trim().lines() {
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() >= 2 {
                    let status = fields[0];
                    let path = if status.starts_with('R') && fields.len() >= 3 {
                        fields[2]
                    } else {
                        fields[1]
                    };
                    // Take just the first character of status (R100 -> R)
                    status_map.insert(
                        path.to_string(),
                        status.chars().next().unwrap_or('M').to_string(),
                    );
                }
            }
        }
    }

    // Parse numstat to build file list
    let mut files: Vec<serde_json::Value> = Vec::new();
    if let Ok(o) = &numstat_output {
        if o.status.success() {
            let text = String::from_utf8_lossy(&o.stdout);
            for line in text.trim().lines() {
                if line.is_empty() {
                    continue;
                }
                let fields: Vec<&str> = line.split('\t').collect();
                if fields.len() < 3 {
                    continue;
                }
                let additions: i64 = fields[0].parse().unwrap_or(0);
                let deletions: i64 = fields[1].parse().unwrap_or(0);
                let mut file_path = fields[2].to_string();

                // Handle renames: {old => new} or old => new
                if file_path.contains("=>") {
                    let after = file_path.split("=>").last().unwrap_or("").trim();
                    file_path = after.trim_end_matches('}').to_string();
                }

                let status = status_map
                    .get(&file_path)
                    .cloned()
                    .unwrap_or_else(|| "M".to_string());

                files.push(serde_json::json!({
                    "path": file_path,
                    "status": status,
                    "additions": additions,
                    "deletions": deletions,
                }));
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "data": {
                "hash": parts[0],
                "shortHash": parts[1],
                "author": parts[2],
                "email": parts[3],
                "date": parts[4],
                "parents": parents,
                "refs": refs,
                "message": parts[7],
                "body": if body.is_empty() { None } else { Some(body) },
                "files": files,
            }
        })),
    )
}

// ── GET /git/diff ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitDiffParams {
    path: Option<String>,
    base: Option<String>,
    file: Option<String>,
}

async fn git_diff(Query(params): Query<GitDiffParams>) -> impl IntoResponse {
    let git_root = match resolve_git_root(params.path) {
        Ok(r) => r,
        Err(e) => {
            let (status, json) = e;
            return (
                status,
                [("content-type", "application/json")],
                json.0.to_string(),
            );
        }
    };

    let base = params.base.unwrap_or_default();
    let file = params.file.unwrap_or_default();

    let mut args = vec!["-C".to_string(), git_root.clone(), "diff".to_string()];

    if !base.is_empty() {
        if base == "HEAD" {
            args.push("HEAD".to_string());
        } else {
            args.push(format!("{}^", base));
            args.push(base.clone());
        }
    }

    if !file.is_empty() {
        args.push("--".to_string());
        args.push(file.clone());
    }

    let output = tokio::process::Command::new("git")
        .args(&args)
        .output()
        .await;

    let diff_text = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => {
            // Fallback for first commit (no parent): use git show
            if !base.is_empty() && base != "HEAD" {
                let mut show_args = vec![
                    "-C".to_string(),
                    git_root,
                    "show".to_string(),
                    base,
                    "--format=".to_string(),
                ];
                if !file.is_empty() {
                    show_args.push("--".to_string());
                    show_args.push(file.clone());
                }
                match tokio::process::Command::new("git")
                    .args(&show_args)
                    .output()
                    .await
                {
                    Ok(o) if o.status.success() => {
                        String::from_utf8_lossy(&o.stdout).to_string()
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        }
    };

    let body = serde_json::json!({
        "data": {
            "diff": diff_text,
            "filePath": file,
        }
    });

    (
        StatusCode::OK,
        [("content-type", "application/json")],
        body.to_string(),
    )
}

// ── GET /git/status ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GitStatusParams {
    path: Option<String>,
}

async fn git_status(Query(params): Query<GitStatusParams>) -> impl IntoResponse {
    let git_root = match resolve_git_root(params.path) {
        Ok(r) => r,
        Err(e) => return e,
    };

    // --- git status --porcelain=v1 -b ---
    let status_output = tokio::process::Command::new("git")
        .args(["-C", &git_root, "status", "--porcelain=v1", "-b"])
        .output()
        .await;

    let status_text = match status_output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("git status failed: {}", stderr)})),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("failed to run git: {}", e)})),
            );
        }
    };

    let mut branch = String::new();
    let mut remote_branch: Option<String> = None;
    let mut staged: Vec<serde_json::Value> = Vec::new();
    let mut unstaged: Vec<serde_json::Value> = Vec::new();
    let mut untracked: Vec<String> = Vec::new();

    for line in status_text.lines() {
        if line.starts_with("## ") {
            // Parse branch line: "## main...origin/main" or "## HEAD (no branch)"
            let branch_info = &line[3..];
            if let Some(dots) = branch_info.find("...") {
                branch = branch_info[..dots].to_string();
                // Remote part may have trailing info like " [ahead 2, behind 1]"
                let rest = &branch_info[dots + 3..];
                if let Some(space) = rest.find(' ') {
                    remote_branch = Some(rest[..space].to_string());
                } else {
                    remote_branch = Some(rest.to_string());
                }
            } else {
                // No remote tracking
                if let Some(space) = branch_info.find(' ') {
                    branch = branch_info[..space].to_string();
                } else {
                    branch = branch_info.to_string();
                }
            }
            continue;
        }

        if line.len() < 4 {
            continue;
        }

        let index_status = line.as_bytes()[0] as char;
        let worktree_status = line.as_bytes()[1] as char;
        // File path starts at position 3
        let file_path = &line[3..];
        // Handle renames: "R  old -> new" -- use the new name
        let display_path = if let Some(arrow) = file_path.find(" -> ") {
            &file_path[arrow + 4..]
        } else {
            file_path
        };

        if index_status == '?' && worktree_status == '?' {
            untracked.push(display_path.to_string());
            continue;
        }

        // Staged changes (index column)
        if index_status != ' ' && index_status != '?' {
            let code = match index_status {
                'M' => "M",
                'A' => "A",
                'D' => "D",
                'R' => "R",
                'C' => "A", // copied, treat as added
                _ => "M",
            };
            staged.push(serde_json::json!({"path": display_path, "status": code}));
        }

        // Unstaged changes (worktree column)
        if worktree_status != ' ' && worktree_status != '?' {
            let code = match worktree_status {
                'M' => "M",
                'D' => "D",
                _ => "M",
            };
            unstaged.push(serde_json::json!({"path": display_path, "status": code}));
        }
    }

    // --- ahead/behind via rev-list ---
    let mut ahead: i64 = 0;
    let mut behind: i64 = 0;

    if remote_branch.is_some() {
        let revlist = tokio::process::Command::new("git")
            .args([
                "-C",
                &git_root,
                "rev-list",
                "--left-right",
                "--count",
                "HEAD...@{upstream}",
            ])
            .output()
            .await;

        if let Ok(o) = revlist {
            if o.status.success() {
                let text = String::from_utf8_lossy(&o.stdout);
                let parts: Vec<&str> = text.trim().split('\t').collect();
                if parts.len() == 2 {
                    ahead = parts[0].parse().unwrap_or(0);
                    behind = parts[1].parse().unwrap_or(0);
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "branch": branch,
            "remote_branch": remote_branch.unwrap_or_default(),
            "ahead": ahead,
            "behind": behind,
            "staged": staged,
            "unstaged": unstaged,
            "untracked": untracked,
        })),
    )
}
