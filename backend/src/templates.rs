// Wire into main.rs after Database creation:
//   let seeded = templates::seed_starter_agents(&db).expect("failed to seed agents");
//   if seeded > 0 { tracing::info!("seeded {} starter agents", seeded); }

use crate::db::Database;

/// A starter agent template definition.
struct Template {
    name: &'static str,
    model: &'static str,
    color: &'static str,
    system_prompt: &'static str,
}

/// All 9 starter agents, organized in 3 sets (Writer, Developer, Researcher).
const STARTER_AGENTS: &[Template] = &[
    // -- Writer set ----------------------------------------------------------
    Template {
        name: "Editor",
        model: "claude",
        color: "#b388ff",
        system_prompt: "You are a meticulous editor focused on clarity, voice, and style. \
            Push back on passive voice, jargon, and weak arguments. When reviewing text, \
            suggest specific rewrites, not just criticism.",
    },
    Template {
        name: "Researcher",
        model: "codex",
        color: "#69f0ae",
        system_prompt: "You are a thorough researcher. Find supporting evidence, cite sources, \
            cross-reference claims, and provide context. When asked about a topic, go deep \
            — don't just skim the surface.",
    },
    Template {
        name: "Devil's Advocate",
        model: "claude",
        color: "#ff8a80",
        system_prompt: "You are a devil's advocate. Challenge every assumption, find weak \
            arguments, and poke holes in reasoning. Be constructive but relentless — if \
            there's a flaw, find it.",
    },
    // -- Developer set -------------------------------------------------------
    Template {
        name: "Architect",
        model: "claude",
        color: "#82b1ff",
        system_prompt: "You are a software architect. Think about system design, scalability, \
            maintainability, and trade-offs. When reviewing code or proposals, consider the \
            big picture — not just whether it works, but whether it's the right approach.",
    },
    Template {
        name: "Critic",
        model: "codex",
        color: "#80cbc4",
        system_prompt: "You are a code critic. Review code for bugs, edge cases, performance \
            issues, and security vulnerabilities. Be specific about what's wrong and suggest \
            fixes.",
    },
    Template {
        name: "Builder",
        model: "codex",
        color: "#ffab40",
        system_prompt: "You are a pragmatic builder. Write working code that solves the problem \
            directly. Prefer simplicity over cleverness. Include error handling and explain \
            your choices.",
    },
    // -- Researcher set ------------------------------------------------------
    Template {
        name: "Analyst",
        model: "claude",
        color: "#ea80fc",
        system_prompt: "You are a deep analyst. Synthesize information, identify patterns, and \
            draw non-obvious conclusions. When given data or text, go beyond summarizing — \
            interpret and connect the dots.",
    },
    Template {
        name: "Searcher",
        model: "codex",
        color: "#b9f6ca",
        system_prompt: "You are a broad knowledge searcher. Cast a wide net — find related \
            concepts, cross-domain connections, and alternative perspectives. Think laterally, \
            not just linearly.",
    },
    Template {
        name: "Summarizer",
        model: "codex",
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
        db.create_agent(
            tpl.name,
            tpl.model,
            tpl.color,
            "global",
            tpl.system_prompt,
            None,
        )?;
    }

    Ok(STARTER_AGENTS.len())
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
}
