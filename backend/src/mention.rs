use crate::db::Agent;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MentionResult {
    /// Agents mentioned in the message.
    pub mentioned_agents: Vec<MentionedAgent>,
    /// The message with @mentions stripped (clean text for the agent).
    pub clean_content: String,
}

#[derive(Debug, Clone)]
pub struct MentionedAgent {
    pub agent_id: String,
    pub agent_name: String,
    /// Byte position of the `@` in the original text.
    pub start: usize,
    /// Byte position one past the end of the mention token in the original text.
    pub end: usize,
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// A raw mention token extracted from user text before agent matching.
#[derive(Debug)]
struct RawMention {
    /// The name string after `@` (without quotes).
    name: String,
    /// Byte offset of the `@` character.
    start: usize,
    /// Byte offset one past the end of the token (closing quote or last word char).
    end: usize,
}

/// Extract all `@Name` or `@"Quoted Name"` tokens from `text`.
fn extract_raw_mentions(text: &str) -> Vec<RawMention> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut mentions = Vec::new();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'@' {
            let start = i;
            i += 1; // skip '@'

            if i >= len {
                break;
            }

            if bytes[i] == b'"' {
                // Quoted mention: @"Some Name"
                i += 1; // skip opening quote
                let name_start = i;
                // Scan until closing quote or end of string.
                while i < len && bytes[i] != b'"' {
                    i += 1;
                }
                let name = &text[name_start..i];
                if i < len {
                    i += 1; // skip closing quote
                }
                if !name.is_empty() {
                    mentions.push(RawMention {
                        name: name.to_owned(),
                        start,
                        end: i,
                    });
                }
            } else if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
                // Unquoted mention: @Word (alphanumeric + underscore + hyphen)
                let name_start = i;
                while i < len
                    && (bytes[i].is_ascii_alphanumeric()
                        || bytes[i] == b'_'
                        || bytes[i] == b'-')
                {
                    i += 1;
                }
                let name = &text[name_start..i];
                if !name.is_empty() {
                    mentions.push(RawMention {
                        name: name.to_owned(),
                        start,
                        end: i,
                    });
                }
            }
            // else: stray '@' followed by whitespace/punctuation -- skip it
        } else {
            i += 1;
        }
    }

    mentions
}

/// Try to match a raw mention name against the available agents.
///
/// Matching priority:
/// 1. Exact match (case-insensitive)
/// 2. Prefix match (case-insensitive) -- the mention is a prefix of the agent name
///
/// Returns `None` if no agent matches.
fn match_agent<'a>(name: &str, agents: &'a [Agent]) -> Option<&'a Agent> {
    let lower = name.to_ascii_lowercase();

    // 1. Exact (case-insensitive)
    for agent in agents {
        if agent.name.to_ascii_lowercase() == lower {
            return Some(agent);
        }
    }

    // 2. Prefix (case-insensitive) -- only if unambiguous (single match)
    let prefix_matches: Vec<&Agent> = agents
        .iter()
        .filter(|a| a.name.to_ascii_lowercase().starts_with(&lower))
        .collect();

    if prefix_matches.len() == 1 {
        return Some(prefix_matches[0]);
    }

    None
}

/// Parse `@mentions` in `content`, resolving them against `agents`.
pub fn parse_mentions(content: &str, agents: &[Agent]) -> MentionResult {
    let raw = extract_raw_mentions(content);

    let mut mentioned: Vec<MentionedAgent> = Vec::new();
    // Collect byte ranges to strip (sorted by start, non-overlapping by construction).
    let mut strip_ranges: Vec<(usize, usize)> = Vec::new();

    for rm in &raw {
        if let Some(agent) = match_agent(&rm.name, agents) {
            // Avoid duplicates (same agent mentioned twice).
            if !mentioned.iter().any(|m| m.agent_id == agent.id) {
                mentioned.push(MentionedAgent {
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    start: rm.start,
                    end: rm.end,
                });
            }
            strip_ranges.push((rm.start, rm.end));
        }
    }

    // Build clean_content by removing matched mention spans.
    let clean = strip_spans(content, &strip_ranges);

    MentionResult {
        mentioned_agents: mentioned,
        clean_content: clean,
    }
}

/// Remove the given byte-ranges from `text` and collapse extra whitespace.
fn strip_spans(text: &str, ranges: &[(usize, usize)]) -> String {
    if ranges.is_empty() {
        return text.to_owned();
    }

    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    for &(start, end) in ranges {
        if start > cursor {
            out.push_str(&text[cursor..start]);
        }
        cursor = end;
    }
    if cursor < text.len() {
        out.push_str(&text[cursor..]);
    }

    // Collapse runs of whitespace into single spaces and trim.
    let collapsed: String = out
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    collapsed
}

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

/// Given a message and the room's agents, determine which agent(s) should
/// respond.
///
/// Priority:
/// 1. Explicit `@mention` in the message text
/// 2. The room's default agent (if provided)
/// 3. The first agent in `room_agents`
///
/// Returns `(target_agent_ids, cleaned_content)`.
pub fn route_message(
    content: &str,
    room_agents: &[Agent],
    default_agent_id: Option<&str>,
) -> (Vec<String>, String) {
    let result = parse_mentions(content, room_agents);

    if !result.mentioned_agents.is_empty() {
        let ids = result
            .mentioned_agents
            .iter()
            .map(|m| m.agent_id.clone())
            .collect();
        return (ids, result.clean_content);
    }

    // No explicit mention -- fall back.
    let fallback_id = if let Some(default_id) = default_agent_id {
        // Verify the default is actually in the room.
        if room_agents.iter().any(|a| a.id == default_id) {
            Some(default_id.to_owned())
        } else {
            room_agents.first().map(|a| a.id.clone())
        }
    } else {
        room_agents.first().map(|a| a.id.clone())
    };

    let ids = match fallback_id {
        Some(id) => vec![id],
        None => vec![],
    };

    (ids, result.clean_content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal Agent for testing.
    fn agent(id: &str, name: &str) -> Agent {
        Agent {
            id: id.to_owned(),
            name: name.to_owned(),
            model: "test-model".to_owned(),
            provider: String::new(),
            agent_home: String::new(),
            color: "#000000".to_owned(),
            scope: "global".to_owned(),
            system_prompt: String::new(),
            workspace_path: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    // -- extract_raw_mentions -----------------------------------------------

    #[test]
    fn raw_single_word() {
        let mentions = extract_raw_mentions("@Editor fix this");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].name, "Editor");
        assert_eq!(mentions[0].start, 0);
        assert_eq!(mentions[0].end, 7);
    }

    #[test]
    fn raw_quoted() {
        let mentions = extract_raw_mentions("Hey @\"Devil's Advocate\" what do you think?");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].name, "Devil's Advocate");
    }

    #[test]
    fn raw_multiple() {
        let mentions = extract_raw_mentions("@Editor @Researcher review this");
        assert_eq!(mentions.len(), 2);
        assert_eq!(mentions[0].name, "Editor");
        assert_eq!(mentions[1].name, "Researcher");
    }

    #[test]
    fn raw_no_mention() {
        let mentions = extract_raw_mentions("just a normal message");
        assert!(mentions.is_empty());
    }

    #[test]
    fn raw_stray_at() {
        let mentions = extract_raw_mentions("email me @ home");
        assert!(mentions.is_empty());
    }

    // -- match_agent --------------------------------------------------------

    #[test]
    fn exact_match() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let m = match_agent("Editor", &agents);
        assert!(m.is_some());
        assert_eq!(m.unwrap().id, "1");
    }

    #[test]
    fn exact_match_case_insensitive() {
        let agents = vec![agent("1", "Editor")];
        let m = match_agent("editor", &agents);
        assert!(m.is_some());
        assert_eq!(m.unwrap().id, "1");
    }

    #[test]
    fn prefix_match() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let m = match_agent("Edit", &agents);
        assert!(m.is_some());
        assert_eq!(m.unwrap().id, "1");
    }

    #[test]
    fn prefix_match_case_insensitive() {
        let agents = vec![agent("1", "Editor")];
        let m = match_agent("edit", &agents);
        assert!(m.is_some());
        assert_eq!(m.unwrap().id, "1");
    }

    #[test]
    fn ambiguous_prefix_returns_none() {
        // "Ed" matches both "Editor" and "Edward" -- ambiguous.
        let agents = vec![agent("1", "Editor"), agent("2", "Edward")];
        let m = match_agent("Ed", &agents);
        assert!(m.is_none());
    }

    #[test]
    fn no_match() {
        let agents = vec![agent("1", "Editor")];
        let m = match_agent("Xyz", &agents);
        assert!(m.is_none());
    }

    // -- parse_mentions (integration) ---------------------------------------

    #[test]
    fn parse_exact_mention() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("@Editor can you rework the opening?", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.mentioned_agents[0].agent_id, "1");
        assert_eq!(result.mentioned_agents[0].agent_name, "Editor");
        assert_eq!(result.clean_content, "can you rework the opening?");
    }

    #[test]
    fn parse_prefix_mention() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("@Edit fix this", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.mentioned_agents[0].agent_id, "1");
        assert_eq!(result.clean_content, "fix this");
    }

    #[test]
    fn parse_case_insensitive() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("@editor fix this", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.mentioned_agents[0].agent_id, "1");
    }

    #[test]
    fn parse_multiple_mentions() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let result = parse_mentions("@Editor @Researcher review this", &agents);
        assert_eq!(result.mentioned_agents.len(), 2);
        assert_eq!(result.clean_content, "review this");
    }

    #[test]
    fn parse_no_mention() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("review this", &agents);
        assert!(result.mentioned_agents.is_empty());
        assert_eq!(result.clean_content, "review this");
    }

    #[test]
    fn parse_quoted_name() {
        let agents = vec![agent("1", "Devil's Advocate")];
        let result =
            parse_mentions("@\"Devil's Advocate\" what do you think?", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.mentioned_agents[0].agent_name, "Devil's Advocate");
        assert_eq!(result.clean_content, "what do you think?");
    }

    #[test]
    fn parse_duplicate_mention_deduplicates() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("@Editor @Editor fix this", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.clean_content, "fix this");
    }

    #[test]
    fn parse_unmatched_mention_left_in_text() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("@Unknown fix this", &agents);
        assert!(result.mentioned_agents.is_empty());
        assert_eq!(result.clean_content, "@Unknown fix this");
    }

    #[test]
    fn parse_mention_mid_sentence() {
        let agents = vec![agent("1", "Editor")];
        let result = parse_mentions("Hey @Editor can you help?", &agents);
        assert_eq!(result.mentioned_agents.len(), 1);
        assert_eq!(result.clean_content, "Hey can you help?");
    }

    // -- route_message ------------------------------------------------------

    #[test]
    fn route_with_explicit_mention() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let (ids, clean) = route_message("@Editor fix this", &agents, Some("2"));
        assert_eq!(ids, vec!["1"]);
        assert_eq!(clean, "fix this");
    }

    #[test]
    fn route_falls_back_to_default() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let (ids, clean) = route_message("fix this", &agents, Some("2"));
        assert_eq!(ids, vec!["2"]);
        assert_eq!(clean, "fix this");
    }

    #[test]
    fn route_falls_back_to_first_agent() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let (ids, clean) = route_message("fix this", &agents, None);
        assert_eq!(ids, vec!["1"]);
        assert_eq!(clean, "fix this");
    }

    #[test]
    fn route_empty_agents() {
        let agents: Vec<Agent> = vec![];
        let (ids, clean) = route_message("fix this", &agents, None);
        assert!(ids.is_empty());
        assert_eq!(clean, "fix this");
    }

    #[test]
    fn route_default_not_in_room_falls_back_to_first() {
        let agents = vec![agent("1", "Editor")];
        let (ids, _) = route_message("fix this", &agents, Some("999"));
        assert_eq!(ids, vec!["1"]);
    }

    #[test]
    fn route_multiple_mentions() {
        let agents = vec![agent("1", "Editor"), agent("2", "Researcher")];
        let (ids, clean) =
            route_message("@Editor @Researcher review this", &agents, None);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"1".to_owned()));
        assert!(ids.contains(&"2".to_owned()));
        assert_eq!(clean, "review this");
    }
}
