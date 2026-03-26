// Wire into main.rs after Database creation:
//   let seeded = templates::seed_starter_agents(&db).expect("failed to seed agents");
//   if seeded > 0 { tracing::info!("seeded {} starter agents", seeded); }

use crate::db::{agent_home_dir, Database};
use crate::scaffold::scaffold_agent_folder;

/// A starter agent template definition.
struct Template {
    name: &'static str,
    model: &'static str,
    provider: &'static str,
    color: &'static str,
    system_prompt: &'static str,
}

/// All 9 starter agents, organized in 3 sets (Writer, Developer, Researcher).
const STARTER_AGENTS: &[Template] = &[
    // -- Writer set ----------------------------------------------------------
    Template {
        name: "Editor",
        model: "sonnet",
        provider: "claude",
        color: "#b388ff",
        system_prompt: "You are a meticulous editor focused on clarity, voice, and style. \
            Push back on passive voice, jargon, and weak arguments. When reviewing text, \
            suggest specific rewrites, not just criticism.",
    },
    Template {
        name: "Researcher",
        model: "o3",
        provider: "codex",
        color: "#69f0ae",
        system_prompt: "You are a thorough researcher. Find supporting evidence, cite sources, \
            cross-reference claims, and provide context. When asked about a topic, go deep \
            — don't just skim the surface.",
    },
    Template {
        name: "Devil's Advocate",
        model: "sonnet",
        provider: "claude",
        color: "#ff8a80",
        system_prompt: "You are a devil's advocate. Challenge every assumption, find weak \
            arguments, and poke holes in reasoning. Be constructive but relentless — if \
            there's a flaw, find it.",
    },
    // -- Developer set -------------------------------------------------------
    Template {
        name: "Architect",
        model: "sonnet",
        provider: "claude",
        color: "#82b1ff",
        system_prompt: "You are a software architect. Think about system design, scalability, \
            maintainability, and trade-offs. When reviewing code or proposals, consider the \
            big picture — not just whether it works, but whether it's the right approach.",
    },
    Template {
        name: "Critic",
        model: "o3",
        provider: "codex",
        color: "#80cbc4",
        system_prompt: "You are a code critic. Review code for bugs, edge cases, performance \
            issues, and security vulnerabilities. Be specific about what's wrong and suggest \
            fixes.",
    },
    Template {
        name: "Builder",
        model: "o3",
        provider: "codex",
        color: "#ffab40",
        system_prompt: "You are a pragmatic builder. Write working code that solves the problem \
            directly. Prefer simplicity over cleverness. Include error handling and explain \
            your choices.",
    },
    // -- Researcher set ------------------------------------------------------
    Template {
        name: "Analyst",
        model: "sonnet",
        provider: "claude",
        color: "#ea80fc",
        system_prompt: "You are a deep analyst. Synthesize information, identify patterns, and \
            draw non-obvious conclusions. When given data or text, go beyond summarizing — \
            interpret and connect the dots.",
    },
    Template {
        name: "Searcher",
        model: "o3",
        provider: "codex",
        color: "#b9f6ca",
        system_prompt: "You are a broad knowledge searcher. Cast a wide net — find related \
            concepts, cross-domain connections, and alternative perspectives. Think laterally, \
            not just linearly.",
    },
    Template {
        name: "Summarizer",
        model: "o3",
        provider: "codex",
        color: "#ffe57f",
        system_prompt: "You are a concise summarizer. Distill long content into clear, structured \
            summaries. Use bullet points, headers, and hierarchy. Capture what matters, cut \
            what doesn't.",
    },
];

/// Seed the 9 starter agents into the database on first launch.
///
/// Returns the number of agents created. If any agents already exist in the
/// database, seeding is skipped entirely and `Ok(0)` is returned.
pub fn seed_starter_agents(db: &Database) -> Result<usize, rusqlite::Error> {
    // Check if any agents exist; if so, skip seeding.
    let existing = db.list_agents(None)?;
    if !existing.is_empty() {
        return Ok(0);
    }

    for tpl in STARTER_AGENTS {
        // Scaffold per-agent home directory with provider-specific config file
        let home = agent_home_dir(tpl.name);
        if let Err(e) = scaffold_agent_folder(tpl.provider, &home, tpl.system_prompt) {
            tracing::warn!("failed to scaffold starter agent '{}': {e}", tpl.name);
        }

        db.create_agent(
            tpl.name,
            tpl.model,
            tpl.provider,
            &home,
            tpl.color,
            "global",
            tpl.system_prompt,
            None,
            None,
        )?;
    }

    Ok(STARTER_AGENTS.len())
}

// ---------------------------------------------------------------------------
// Starter playbooks
// ---------------------------------------------------------------------------

struct PlaybookTemplate {
    name: &'static str,
    flow_type: &'static str,
    description: &'static str,
    yaml_content: &'static str,
}

const STARTER_PLAYBOOKS: &[PlaybookTemplate] = &[
    PlaybookTemplate {
        name: "Debate",
        flow_type: "debate",
        description: "Two agents argue opposing sides of a topic across multiple rounds.",
        yaml_content: "\
topic: \"{{TOPIC}}\"
rounds: 3
roles:
  - name: Advocate
    stance: for
    system_prompt: >
      You are the Advocate. Argue persuasively in favor of the topic.
      Support your points with evidence and reasoning.
  - name: Critic
    stance: against
    system_prompt: >
      You are the Critic. Argue persuasively against the topic.
      Find weaknesses, counter-examples, and alternative framings.
",
    },
    PlaybookTemplate {
        name: "Code Review Panel",
        flow_type: "parallel-then-compare",
        description: "Three reviewers examine code in parallel, then a comparison summarizes findings.",
        yaml_content: "\
artifact: \"{{CODE_OR_PR_LINK}}\"
roles:
  - name: Correctness Reviewer
    focus: correctness
    system_prompt: >
      Review the code for bugs, logic errors, and edge cases.
      Be specific about line numbers and suggest fixes.
  - name: Security Reviewer
    focus: security
    system_prompt: >
      Review the code for security vulnerabilities, injection risks,
      and unsafe patterns. Reference CWE IDs where applicable.
  - name: Performance Reviewer
    focus: performance
    system_prompt: >
      Review the code for performance issues, unnecessary allocations,
      N+1 queries, and scalability concerns. Suggest optimizations.
",
    },
    PlaybookTemplate {
        name: "Research & Synthesize",
        flow_type: "round-robin-then-synthesize",
        description: "A researcher explores the topic in rounds, then a synthesizer distills the findings.",
        yaml_content: "\
question: \"{{RESEARCH_QUESTION}}\"
rounds: 2
roles:
  - name: Researcher
    system_prompt: >
      You are a thorough researcher. Explore the question from multiple
      angles, cite sources, and surface non-obvious connections. Each
      round should go deeper than the last.
  - name: Synthesizer
    system_prompt: >
      You are a synthesizer. After the research rounds, distill the
      findings into a clear, structured summary with key takeaways,
      open questions, and recommended next steps.
",
    },
];

/// Seed the 3 starter playbooks into the database on first launch.
///
/// Returns the number of playbooks created. If any playbooks already exist in
/// the database, seeding is skipped entirely and `Ok(0)` is returned.
pub fn seed_starter_playbooks(db: &Database) -> Result<usize, rusqlite::Error> {
    let existing = db.list_playbooks()?;
    if !existing.is_empty() {
        return Ok(0);
    }

    for tpl in STARTER_PLAYBOOKS {
        db.create_playbook(tpl.name, tpl.flow_type, tpl.yaml_content, tpl.description)?;
    }

    Ok(STARTER_PLAYBOOKS.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_creates_nine_agents() {
        let db = Database::new_in_memory().unwrap();
        let count = seed_starter_agents(&db).unwrap();
        assert_eq!(count, 9);

        let agents = db.list_agents(None).unwrap();
        assert_eq!(agents.len(), 9);
    }

    #[test]
    fn test_seed_is_idempotent() {
        let db = Database::new_in_memory().unwrap();
        let first = seed_starter_agents(&db).unwrap();
        assert_eq!(first, 9);

        // Second call should skip seeding.
        let second = seed_starter_agents(&db).unwrap();
        assert_eq!(second, 0);

        let agents = db.list_agents(None).unwrap();
        assert_eq!(agents.len(), 9);
    }

    #[test]
    fn test_seed_creates_three_playbooks() {
        let db = Database::new_in_memory().unwrap();
        let count = seed_starter_playbooks(&db).unwrap();
        assert_eq!(count, 3);

        let playbooks = db.list_playbooks().unwrap();
        assert_eq!(playbooks.len(), 3);
    }

    #[test]
    fn test_playbook_seed_is_idempotent() {
        let db = Database::new_in_memory().unwrap();
        let first = seed_starter_playbooks(&db).unwrap();
        assert_eq!(first, 3);

        let second = seed_starter_playbooks(&db).unwrap();
        assert_eq!(second, 0);

        let playbooks = db.list_playbooks().unwrap();
        assert_eq!(playbooks.len(), 3);
    }
}
