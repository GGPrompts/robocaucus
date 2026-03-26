//! Parse and serialize the `.agent.md` format.
//!
//! The format uses YAML frontmatter delimited by `---` lines, with the
//! markdown body serving as the agent's system prompt.
//!
//! ```text
//! ---
//! name: Editor
//! model: claude
//! color: "#b388ff"
//! scope: global
//! ---
//! You are a meticulous editor focused on clarity and voice.
//! ```

use crate::db;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum AgentMdError {
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("invalid YAML: {0}")]
    InvalidYaml(String),
    #[error("missing required field: {0}")]
    MissingField(String),
}

// ---------------------------------------------------------------------------
// Parsed data (no DB-generated fields like id/timestamps)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentMdData {
    pub name: String,
    pub model: String,
    pub color: String,
    pub scope: String,
    pub system_prompt: String,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse an `.agent.md` string into its component parts.
///
/// Expects content shaped like:
/// ```text
/// ---
/// key: value
/// ...
/// ---
/// body text
/// ```
pub fn parse_agent_md(content: &str) -> Result<AgentMdData, AgentMdError> {
    // Find the two `---` delimiters.
    let trimmed = content.trim_start();
    let rest = trimmed
        .strip_prefix("---")
        .ok_or(AgentMdError::MissingFrontmatter)?;

    // Find the closing `---`.
    let closing_pos = rest
        .find("\n---")
        .ok_or(AgentMdError::MissingFrontmatter)?;

    let yaml_block = &rest[..closing_pos];
    // Body starts after the closing `---` line.
    let after_closing = &rest[closing_pos + 4..]; // skip "\n---"
    // Strip the rest of the `---` line (possible trailing whitespace / newline).
    let body = after_closing
        .strip_prefix('\n')
        .unwrap_or(after_closing)
        .trim()
        .to_owned();

    // Parse the simple `key: value` YAML.
    let mut name: Option<String> = None;
    let mut model: Option<String> = None;
    let mut color: Option<String> = None;
    let mut scope: Option<String> = None;

    for line in yaml_block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| AgentMdError::InvalidYaml(format!("expected `key: value`, got: {line}")))?;
        let key = key.trim();
        let value = value.trim();
        // Strip optional surrounding quotes from the value.
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);

        match key {
            "name" => name = Some(value.to_owned()),
            "model" => model = Some(value.to_owned()),
            "color" => color = Some(value.to_owned()),
            "scope" => scope = Some(value.to_owned()),
            other => {
                return Err(AgentMdError::InvalidYaml(format!(
                    "unknown field: {other}"
                )));
            }
        }
    }

    let name = name.ok_or_else(|| AgentMdError::MissingField("name".into()))?;
    let model = model.ok_or_else(|| AgentMdError::MissingField("model".into()))?;
    let color = color.ok_or_else(|| AgentMdError::MissingField("color".into()))?;
    let scope = scope.unwrap_or_else(|| "global".to_owned());

    Ok(AgentMdData {
        name,
        model,
        color,
        scope,
        system_prompt: body,
    })
}

// ---------------------------------------------------------------------------
// Serializing
// ---------------------------------------------------------------------------

/// Render a `db::Agent` as an `.agent.md` string.
pub fn serialize_agent_md(agent: &db::Agent) -> String {
    format!(
        "---\nname: {}\nmodel: {}\ncolor: \"{}\"\nscope: {}\n---\n{}\n",
        agent.name, agent.model, agent.color, agent.scope, agent.system_prompt,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let input = "\
---
name: Editor
model: claude
color: \"#b388ff\"
scope: global
---
You are a meticulous editor focused on clarity and voice.
Push back on passive voice and jargon.
";
        let data = parse_agent_md(input).unwrap();
        assert_eq!(data.name, "Editor");
        assert_eq!(data.model, "claude");
        assert_eq!(data.color, "#b388ff");
        assert_eq!(data.scope, "global");
        assert_eq!(
            data.system_prompt,
            "You are a meticulous editor focused on clarity and voice.\nPush back on passive voice and jargon."
        );
    }

    #[test]
    fn missing_frontmatter() {
        let input = "No frontmatter here.";
        assert!(matches!(
            parse_agent_md(input),
            Err(AgentMdError::MissingFrontmatter)
        ));
    }

    #[test]
    fn missing_field() {
        let input = "---\nname: X\n---\nbody";
        let err = parse_agent_md(input).unwrap_err();
        assert!(err.to_string().contains("missing required field"));
    }

    #[test]
    fn serialize_round_trip() {
        let agent = db::Agent {
            id: "abc".into(),
            name: "Editor".into(),
            model: "claude".into(),
            provider: "claude".into(),
            agent_home: "".into(),
            color: "#b388ff".into(),
            scope: "global".into(),
            system_prompt: "Be helpful.".into(),
            workspace_path: None,
            cli_config: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let md = serialize_agent_md(&agent);
        let parsed = parse_agent_md(&md).unwrap();
        assert_eq!(parsed.name, agent.name);
        assert_eq!(parsed.model, agent.model);
        assert_eq!(parsed.color, agent.color);
        assert_eq!(parsed.scope, agent.scope);
        assert_eq!(parsed.system_prompt, agent.system_prompt);
    }

    #[test]
    fn scope_defaults_to_global() {
        let input = "---\nname: Bot\nmodel: claude\ncolor: \"#000\"\n---\nHello";
        let data = parse_agent_md(input).unwrap();
        assert_eq!(data.scope, "global");
    }
}
