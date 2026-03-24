use axum::{
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::state::AppState;

pub fn file_routes() -> Router<AppState> {
    Router::new()
        .route("/files/list", get(files_list))
        .route("/files/read", get(files_read))
        .route("/search", get(search_query))
}

// ── Path helpers ──────────────────────────────────────────────────────────

fn expand_path(raw: &str) -> String {
    if raw.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &raw[1..]);
        }
    }
    raw.to_string()
}

/// Canonicalize `target` and verify it lives under `base`. Prevents directory traversal.
async fn validate_path_within_base(
    base: &str,
    target: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    let canonical_base = tokio::fs::canonicalize(base)
        .await
        .map_err(|e| format!("Invalid base path: {}", e))?;
    let canonical_target = tokio::fs::canonicalize(target)
        .await
        .map_err(|e| format!("Invalid path: {}", e))?;
    if !canonical_target.starts_with(&canonical_base) {
        return Err("Path escapes base directory".to_string());
    }
    Ok(canonical_target)
}

// ── List directory ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FilesListParams {
    path: Option<String>,
    dir: Option<String>,
}

async fn files_list(Query(params): Query<FilesListParams>) -> impl IntoResponse {
    let base = params.path.unwrap_or_else(|| "~".to_string());
    let expanded_base = expand_path(&base);
    let full_path = match params.dir {
        Some(ref d) if !d.is_empty() => {
            let p = std::path::PathBuf::from(&expanded_base).join(d);
            p.to_string_lossy().to_string()
        }
        _ => expanded_base.clone(),
    };

    let canonical =
        match validate_path_within_base(&expanded_base, std::path::Path::new(&full_path)).await {
            Ok(p) => p,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e})),
                );
            }
        };

    let mut entries = Vec::new();
    let mut read_dir = match tokio::fs::read_dir(&canonical).await {
        Ok(rd) => rd,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Cannot read directory: {}", e)})),
            );
        }
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        entries.push(serde_json::json!({
            "name": name,
            "is_dir": is_dir,
            "size": size,
            "modified": modified,
        }));
    }

    // Sort: directories first, then alphabetically (case-insensitive)
    entries.sort_by(|a, b| {
        let a_dir = a["is_dir"].as_bool().unwrap_or(false);
        let b_dir = b["is_dir"].as_bool().unwrap_or(false);
        match (b_dir).cmp(&a_dir) {
            std::cmp::Ordering::Equal => {
                let a_name = a["name"].as_str().unwrap_or("").to_lowercase();
                let b_name = b["name"].as_str().unwrap_or("").to_lowercase();
                a_name.cmp(&b_name)
            }
            other => other,
        }
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "path": canonical.to_string_lossy(),
            "entries": entries,
        })),
    )
}

// ── Read file ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct FilesReadParams {
    path: Option<String>,
    file: Option<String>,
}

async fn files_read(Query(params): Query<FilesReadParams>) -> impl IntoResponse {
    let base = params.path.unwrap_or_else(|| "~".to_string());
    let expanded_base = expand_path(&base);
    let file_name = match params.file {
        Some(ref f) if !f.is_empty() => f.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "file parameter required"})),
            );
        }
    };

    let full_path = std::path::PathBuf::from(&expanded_base).join(&file_name);
    let canonical = match validate_path_within_base(&expanded_base, &full_path).await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            );
        }
    };

    let meta = match tokio::fs::metadata(&canonical).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Cannot stat file: {}", e)})),
            );
        }
    };

    if meta.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Path is a directory, not a file"})),
        );
    }

    // Size limit: 2MB for text preview
    let max_size: u64 = 2 * 1024 * 1024;
    let is_binary;
    let content;

    if meta.len() > max_size {
        is_binary = true;
        content = String::new();
    } else {
        match tokio::fs::read(&canonical).await {
            Ok(bytes) => {
                // Simple binary detection: check first 8KB for null bytes
                let check_len = std::cmp::min(bytes.len(), 8192);
                let has_null = bytes[..check_len].contains(&0);
                if has_null {
                    is_binary = true;
                    content = String::new();
                } else {
                    is_binary = false;
                    content = String::from_utf8_lossy(&bytes).to_string();
                }
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Cannot read file: {}", e)})),
                );
            }
        }
    }

    let ext = canonical
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "path": canonical.to_string_lossy(),
            "size": meta.len(),
            "is_binary": is_binary,
            "extension": ext,
            "content": content,
        })),
    )
}

// ── Code search (ripgrep with grep fallback) ──────────────────────────────

#[derive(Deserialize)]
struct SearchParams {
    path: Option<String>,
    q: Option<String>,
    regex: Option<bool>,
    case: Option<bool>,
    glob: Option<String>,
}

async fn search_query(Query(params): Query<SearchParams>) -> impl IntoResponse {
    let query = match params.q {
        Some(ref q) if !q.is_empty() => q.clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "q parameter required"})),
            );
        }
    };

    let base = params.path.unwrap_or_else(|| "~".to_string());
    let expanded = expand_path(&base);
    let search_dir = match tokio::fs::canonicalize(&expanded).await {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid path: {}", e)})),
            );
        }
    };

    let use_regex = params.regex.unwrap_or(false);
    let case_sensitive = params.case.unwrap_or(false);
    let glob_pattern = params.glob.clone();
    let context_lines: usize = 3;

    // Try ripgrep first, fall back to grep
    let output = {
        let mut args: Vec<String> = vec![
            "--json".to_string(),
            "-C".to_string(),
            context_lines.to_string(),
            "--max-count".to_string(),
            "200".to_string(),
        ];

        if !case_sensitive {
            args.push("-i".to_string());
        }
        if !use_regex {
            args.push("-F".to_string());
        }
        if let Some(ref g) = glob_pattern {
            if !g.is_empty() {
                args.push("--glob".to_string());
                args.push(g.clone());
            }
        }
        args.push("--".to_string());
        args.push(query.clone());
        args.push(search_dir.clone());

        tokio::process::Command::new("rg")
            .args(&args)
            .output()
            .await
    };

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();

            if !o.status.success() && o.status.code() != Some(1) {
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("rg failed: {}", stderr)})),
                );
            }

            let results = parse_rg_json_output(&stdout, &search_dir);

            (
                StatusCode::OK,
                Json(serde_json::json!({"results": results})),
            )
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                match search_with_grep(
                    &query,
                    &search_dir,
                    use_regex,
                    case_sensitive,
                    &glob_pattern,
                )
                .await
                {
                    Ok(results) => (
                        StatusCode::OK,
                        Json(serde_json::json!({"results": results})),
                    ),
                    Err(err) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": err})),
                    ),
                }
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Failed to run rg: {}", e)})),
                )
            }
        }
    }
}

fn parse_rg_json_output(stdout: &str, search_dir: &str) -> Vec<serde_json::Value> {
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut context_before: Vec<String> = Vec::new();
    let mut last_match: Option<serde_json::Value> = None;
    let mut context_after: Vec<String> = Vec::new();
    let mut after_count: usize = 0;
    let context_size: usize = 3;

    for line in stdout.lines() {
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = parsed["type"].as_str().unwrap_or("");

        match msg_type {
            "match" => {
                if let Some(mut prev) = last_match.take() {
                    prev["context_after"] = serde_json::json!(context_after);
                    results.push(prev);
                }
                context_after = Vec::new();
                after_count = 0;

                let data = &parsed["data"];
                let file_path = data["path"]["text"].as_str().unwrap_or("");
                let line_number = data["line_number"].as_u64().unwrap_or(0);
                let text = data["lines"]["text"].as_str().unwrap_or("").trim_end();

                let rel_path = if file_path.starts_with(search_dir) {
                    let stripped = &file_path[search_dir.len()..];
                    if stripped.starts_with('/') {
                        &stripped[1..]
                    } else {
                        stripped
                    }
                } else {
                    file_path
                };

                last_match = Some(serde_json::json!({
                    "file": rel_path,
                    "line": line_number,
                    "text": text,
                    "context_before": context_before.clone(),
                    "context_after": [],
                }));
                context_before.clear();
            }
            "context" => {
                let data = &parsed["data"];
                let text = data["lines"]["text"].as_str().unwrap_or("").trim_end();

                if last_match.is_some() && after_count < context_size {
                    context_after.push(text.to_string());
                    after_count += 1;
                } else {
                    context_before.push(text.to_string());
                    if context_before.len() > context_size {
                        context_before.remove(0);
                    }
                }
            }
            "end" | "begin" | "summary" => {
                if msg_type == "end" || msg_type == "begin" {
                    if let Some(mut prev) = last_match.take() {
                        prev["context_after"] = serde_json::json!(context_after);
                        results.push(prev);
                    }
                    context_before.clear();
                    context_after = Vec::new();
                    after_count = 0;
                }
            }
            _ => {}
        }
    }

    if let Some(mut prev) = last_match.take() {
        prev["context_after"] = serde_json::json!(context_after);
        results.push(prev);
    }

    results
}

async fn search_with_grep(
    query: &str,
    search_dir: &str,
    use_regex: bool,
    case_sensitive: bool,
    _glob_pattern: &Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut args: Vec<String> = vec!["-rn".to_string()];

    if !case_sensitive {
        args.push("-i".to_string());
    }
    if !use_regex {
        args.push("-F".to_string());
    }
    args.push("-m".to_string());
    args.push("200".to_string());
    args.push("--".to_string());
    args.push(query.to_string());
    args.push(search_dir.to_string());

    let output = tokio::process::Command::new("grep")
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("grep failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut results: Vec<serde_json::Value> = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
            let file_path = parts[0];
            let line_num: u64 = parts[1].parse().unwrap_or(0);
            let text = parts[2].trim_end();

            let rel_path = if file_path.starts_with(search_dir) {
                let stripped = &file_path[search_dir.len()..];
                if stripped.starts_with('/') {
                    &stripped[1..]
                } else {
                    stripped
                }
            } else {
                file_path
            };

            results.push(serde_json::json!({
                "file": rel_path,
                "line": line_num,
                "text": text,
                "context_before": [],
                "context_after": [],
            }));
        }
    }

    Ok(results)
}
