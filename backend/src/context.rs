use std::collections::HashMap;

use serde::Serialize;

use crate::db;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ContextMessage {
    pub role: String,    // "user", "assistant", or "system"
    pub content: String,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build context messages for a specific agent from conversation history.
///
/// Each agent sees the conversation from its own perspective:
/// - Messages authored by `target_agent` become `role: "assistant"`.
/// - Messages from the human user (`agent_id` is `None`) stay `role: "user"`.
/// - Messages from *other* agents become `role: "user"` with a prefix so the
///   target agent knows who said what.
///
/// An optional `max_messages` parameter limits the history to the most recent
/// N messages (the system prompt is always prepended and does not count toward
/// the limit).
pub fn build_agent_context(
    messages: &[db::Message],
    agents: &[db::Agent],
    target_agent: &db::Agent,
    system_prompt: Option<&str>,
    max_messages: Option<usize>,
) -> Vec<ContextMessage> {
    // Pre-build a lookup map: agent_id -> agent name
    let agent_names: HashMap<&str, &str> = agents
        .iter()
        .map(|a| (a.id.as_str(), a.name.as_str()))
        .collect();

    let mut result: Vec<ContextMessage> = Vec::new();

    // -----------------------------------------------------------------
    // 1. System prompt (always first, if provided or if the agent has one)
    // -----------------------------------------------------------------
    let identity = if target_agent.system_prompt.is_empty() {
        format!("You are {}.", target_agent.name)
    } else {
        format!("You are {}. {}", target_agent.name, target_agent.system_prompt)
    };

    let system_content = match system_prompt {
        Some(sp) => format!("{}\n\n{}", sp, identity),
        None => identity,
    };

    result.push(ContextMessage {
        role: "system".to_owned(),
        content: system_content,
    });

    // -----------------------------------------------------------------
    // 2. Conversation messages (optionally truncated to last N)
    // -----------------------------------------------------------------
    let window: &[db::Message] = match max_messages {
        Some(n) if n < messages.len() => &messages[messages.len() - n..],
        _ => messages,
    };

    for msg in window {
        let ctx_msg = match &msg.agent_id {
            // Message from the target agent itself -> assistant
            Some(aid) if aid == &target_agent.id => ContextMessage {
                role: "assistant".to_owned(),
                content: msg.content.clone(),
            },
            // Message from another agent -> user with attribution prefix
            Some(aid) => {
                let name = agent_names
                    .get(aid.as_str())
                    .copied()
                    .unwrap_or("Unknown Agent");
                ContextMessage {
                    role: "user".to_owned(),
                    content: format!("[{} responded]:\n{}", name, msg.content),
                }
            }
            // Human user message (agent_id is None) -> user as-is
            None => ContextMessage {
                role: "user".to_owned(),
                content: msg.content.clone(),
            },
        };
        result.push(ctx_msg);
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Agent, Message};

    fn make_agent(id: &str, name: &str, prompt: &str) -> Agent {
        Agent {
            id: id.to_owned(),
            name: name.to_owned(),
            model: "claude".to_owned(),
            color: "#000000".to_owned(),
            scope: "global".to_owned(),
            system_prompt: prompt.to_owned(),
            workspace_path: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    fn make_message(agent_id: Option<&str>, content: &str) -> Message {
        Message {
            id: "msg-1".to_owned(),
            conversation_id: "conv-1".to_owned(),
            agent_id: agent_id.map(|s| s.to_owned()),
            role: if agent_id.is_some() {
                "assistant"
            } else {
                "user"
            }
            .to_owned(),
            content: content.to_owned(),
            model: None,
            timestamp: "2026-01-01T00:00:00Z".to_owned(),
            usage_json: None,
        }
    }

    #[test]
    fn test_own_messages_become_assistant() {
        let alice = make_agent("a1", "Alice", "");
        let agents = vec![alice.clone()];
        let messages = vec![
            make_message(None, "Hello Alice"),
            make_message(Some("a1"), "Hi there!"),
        ];

        let ctx = build_agent_context(&messages, &agents, &alice, None, None);

        // system + 2 messages
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx[0].role, "system");
        assert_eq!(ctx[1].role, "user");
        assert_eq!(ctx[1].content, "Hello Alice");
        assert_eq!(ctx[2].role, "assistant");
        assert_eq!(ctx[2].content, "Hi there!");
    }

    #[test]
    fn test_other_agent_messages_prefixed() {
        let alice = make_agent("a1", "Alice", "");
        let bob = make_agent("a2", "Bob", "");
        let agents = vec![alice.clone(), bob.clone()];
        let messages = vec![
            make_message(None, "Hello everyone"),
            make_message(Some("a2"), "I am Bob"),
            make_message(Some("a1"), "I am Alice"),
        ];

        // From Alice's perspective
        let ctx = build_agent_context(&messages, &agents, &alice, None, None);

        assert_eq!(ctx.len(), 4);
        assert_eq!(ctx[2].role, "user");
        assert_eq!(ctx[2].content, "[Bob responded]:\nI am Bob");
        assert_eq!(ctx[3].role, "assistant");
        assert_eq!(ctx[3].content, "I am Alice");

        // From Bob's perspective
        let ctx_bob = build_agent_context(&messages, &agents, &bob, None, None);

        assert_eq!(ctx_bob[2].role, "assistant");
        assert_eq!(ctx_bob[2].content, "I am Bob");
        assert_eq!(ctx_bob[3].role, "user");
        assert_eq!(ctx_bob[3].content, "[Alice responded]:\nI am Alice");
    }

    #[test]
    fn test_system_prompt_composition() {
        let alice = make_agent("a1", "Alice", "Be concise.");
        let agents = vec![alice.clone()];
        let messages = vec![];

        let ctx = build_agent_context(
            &messages,
            &agents,
            &alice,
            Some("You are in a multi-agent conversation."),
            None,
        );

        assert_eq!(ctx.len(), 1);
        assert_eq!(ctx[0].role, "system");
        assert_eq!(
            ctx[0].content,
            "You are in a multi-agent conversation.\n\nYou are Alice. Be concise."
        );
    }

    #[test]
    fn test_system_prompt_without_agent_prompt() {
        let alice = make_agent("a1", "Alice", "");
        let agents = vec![alice.clone()];
        let messages = vec![];

        let ctx = build_agent_context(&messages, &agents, &alice, None, None);

        assert_eq!(ctx[0].content, "You are Alice.");
    }

    #[test]
    fn test_max_messages_truncation() {
        let alice = make_agent("a1", "Alice", "");
        let agents = vec![alice.clone()];
        let messages = vec![
            make_message(None, "msg1"),
            make_message(None, "msg2"),
            make_message(None, "msg3"),
            make_message(None, "msg4"),
            make_message(None, "msg5"),
        ];

        let ctx = build_agent_context(&messages, &agents, &alice, None, Some(2));

        // system + last 2 messages
        assert_eq!(ctx.len(), 3);
        assert_eq!(ctx[1].content, "msg4");
        assert_eq!(ctx[2].content, "msg5");
    }

    #[test]
    fn test_max_messages_larger_than_history() {
        let alice = make_agent("a1", "Alice", "");
        let agents = vec![alice.clone()];
        let messages = vec![make_message(None, "only one")];

        let ctx = build_agent_context(&messages, &agents, &alice, None, Some(100));

        // system + 1 message (no panic from over-sized limit)
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[1].content, "only one");
    }

    #[test]
    fn test_unknown_agent_fallback() {
        let alice = make_agent("a1", "Alice", "");
        // agents list does NOT include the author of the message
        let agents = vec![alice.clone()];
        let messages = vec![make_message(Some("unknown-id"), "mystery")];

        let ctx = build_agent_context(&messages, &agents, &alice, None, None);

        assert_eq!(ctx[1].role, "user");
        assert_eq!(ctx[1].content, "[Unknown Agent responded]:\nmystery");
    }
}
