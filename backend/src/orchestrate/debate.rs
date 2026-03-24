// ---------------------------------------------------------------------------
// Debate orchestration engine
// ---------------------------------------------------------------------------
//
// Manages structured N-turn debates between agents.  The debate progresses
// through four phases:
//
//   Opening  ->  Rebuttal(1..=N)  ->  Closing  ->  Synthesis  ->  Complete
//
// Each phase iterates over participating agents (or the moderator for
// Synthesis).  The engine is a pure state machine — it does not perform I/O
// itself but tells the caller *which* agent speaks next and *what* prompt to
// give them.

// ---------------------------------------------------------------------------
// Config & phase types
// ---------------------------------------------------------------------------

/// Configuration for a structured debate.
#[derive(Debug, Clone)]
pub struct DebateConfig {
    /// The topic or proposition being debated.
    pub topic: String,
    /// Number of rebuttal rounds (default 3).
    pub num_rounds: usize,
    /// Agent ID of the moderator.  If `None`, the last participant moderates.
    pub moderator_agent_id: Option<String>,
    /// Ordered list of debating agent IDs (does NOT include the moderator
    /// unless the moderator is also a participant).
    pub participant_agent_ids: Vec<String>,
    /// Conversation this debate belongs to.
    pub conversation_id: String,
}

/// The current phase of a debate.
#[derive(Debug, Clone, PartialEq)]
pub enum DebatePhase {
    Opening,
    Rebuttal(usize), // 1-based round number
    Closing,
    Synthesis,
    Complete,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Pure state machine driving a structured debate.
pub struct DebateEngine {
    config: DebateConfig,
    phase: DebatePhase,
    /// Index into the current phase's agent list (0-based).
    turn_index: usize,
}

impl DebateEngine {
    /// Create a new engine positioned at the start of the Opening phase.
    pub fn new(config: DebateConfig) -> Self {
        Self {
            config,
            phase: DebatePhase::Opening,
            turn_index: 0,
        }
    }

    // -- accessors ----------------------------------------------------------

    /// The current phase.
    pub fn current_phase(&self) -> &DebatePhase {
        &self.phase
    }

    /// Returns `true` when the debate has finished all phases.
    pub fn is_complete(&self) -> bool {
        self.phase == DebatePhase::Complete
    }

    /// The resolved moderator agent ID.
    ///
    /// If the config supplies an explicit moderator, that is returned.
    /// Otherwise the last participant is used.
    fn moderator_id(&self) -> &str {
        self.config
            .moderator_agent_id
            .as_deref()
            .unwrap_or_else(|| {
                self.config
                    .participant_agent_ids
                    .last()
                    .map(|s| s.as_str())
                    .unwrap_or("")
            })
    }

    // -- turn management ----------------------------------------------------

    /// Which agent should speak next, or `None` if the debate is complete.
    pub fn next_agent_id(&self) -> Option<&str> {
        match &self.phase {
            DebatePhase::Opening | DebatePhase::Rebuttal(_) | DebatePhase::Closing => {
                self.config
                    .participant_agent_ids
                    .get(self.turn_index)
                    .map(|s| s.as_str())
            }
            DebatePhase::Synthesis => Some(self.moderator_id()),
            DebatePhase::Complete => None,
        }
    }

    /// Advance to the next turn (or phase).  Returns the *new* phase after
    /// advancing.
    ///
    /// Call this after the current agent has finished speaking.
    pub fn advance(&mut self) -> DebatePhase {
        match &self.phase {
            DebatePhase::Opening => {
                if self.turn_index + 1 < self.config.participant_agent_ids.len() {
                    self.turn_index += 1;
                } else {
                    // All participants gave opening statements.
                    if self.config.num_rounds > 0 {
                        self.phase = DebatePhase::Rebuttal(1);
                    } else {
                        self.phase = DebatePhase::Closing;
                    }
                    self.turn_index = 0;
                }
            }
            DebatePhase::Rebuttal(round) => {
                let round = *round;
                if self.turn_index + 1 < self.config.participant_agent_ids.len() {
                    self.turn_index += 1;
                } else if round < self.config.num_rounds {
                    self.phase = DebatePhase::Rebuttal(round + 1);
                    self.turn_index = 0;
                } else {
                    self.phase = DebatePhase::Closing;
                    self.turn_index = 0;
                }
            }
            DebatePhase::Closing => {
                if self.turn_index + 1 < self.config.participant_agent_ids.len() {
                    self.turn_index += 1;
                } else {
                    self.phase = DebatePhase::Synthesis;
                    self.turn_index = 0;
                }
            }
            DebatePhase::Synthesis => {
                self.phase = DebatePhase::Complete;
                self.turn_index = 0;
            }
            DebatePhase::Complete => {
                // Already done — no-op.
            }
        }
        self.phase.clone()
    }

    // -- prompt building ----------------------------------------------------

    /// Build the prompt string for the current turn.
    ///
    /// * `agent_name`     – display name of the agent about to speak.
    /// * `previous_turns` – all prior turn contents in order (used in
    ///   rebuttal prompts so the agent can reference earlier arguments).
    pub fn build_turn_prompt(&self, agent_name: &str, previous_turns: &[String]) -> String {
        match &self.phase {
            DebatePhase::Opening => {
                format!(
                    "{agent_name}, present your opening position on: {}",
                    self.config.topic,
                )
            }
            DebatePhase::Rebuttal(round) => {
                let context = previous_turns.join("\n\n---\n\n");
                format!(
                    "{agent_name}, this is rebuttal round {round}. \
                     Respond to the previous arguments:\n\n\
                     {context}\n\n\
                     Present your rebuttal.",
                )
            }
            DebatePhase::Closing => {
                format!(
                    "{agent_name}, give your closing statement on: {}",
                    self.config.topic,
                )
            }
            DebatePhase::Synthesis => {
                "As moderator, summarize all arguments and present a balanced conclusion."
                    .to_owned()
            }
            DebatePhase::Complete => String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a standard 3-participant, 2-round config.
    fn test_config() -> DebateConfig {
        DebateConfig {
            topic: "Rust vs Go".to_owned(),
            num_rounds: 2,
            moderator_agent_id: Some("mod-1".to_owned()),
            participant_agent_ids: vec![
                "agent-a".to_owned(),
                "agent-b".to_owned(),
                "agent-c".to_owned(),
            ],
            conversation_id: "conv-1".to_owned(),
        }
    }

    // -- phase progression --------------------------------------------------

    #[test]
    fn test_full_phase_progression() {
        let mut engine = DebateEngine::new(test_config());

        // Opening: 3 agents
        assert_eq!(*engine.current_phase(), DebatePhase::Opening);
        for _ in 0..3 {
            assert!(!engine.is_complete());
            engine.advance();
        }

        // Rebuttal round 1: 3 agents
        assert_eq!(*engine.current_phase(), DebatePhase::Rebuttal(1));
        for _ in 0..3 {
            engine.advance();
        }

        // Rebuttal round 2: 3 agents
        assert_eq!(*engine.current_phase(), DebatePhase::Rebuttal(2));
        for _ in 0..3 {
            engine.advance();
        }

        // Closing: 3 agents
        assert_eq!(*engine.current_phase(), DebatePhase::Closing);
        for _ in 0..3 {
            engine.advance();
        }

        // Synthesis: 1 turn (moderator)
        assert_eq!(*engine.current_phase(), DebatePhase::Synthesis);
        engine.advance();

        // Complete
        assert_eq!(*engine.current_phase(), DebatePhase::Complete);
        assert!(engine.is_complete());

        // Idempotent once complete
        engine.advance();
        assert!(engine.is_complete());
    }

    #[test]
    fn test_zero_rebuttal_rounds_skips_to_closing() {
        let mut config = test_config();
        config.num_rounds = 0;
        let mut engine = DebateEngine::new(config);

        // Opening: 3 agents
        assert_eq!(*engine.current_phase(), DebatePhase::Opening);
        for _ in 0..3 {
            engine.advance();
        }

        // Should jump straight to Closing (no rebuttal)
        assert_eq!(*engine.current_phase(), DebatePhase::Closing);
    }

    // -- agent ordering -----------------------------------------------------

    #[test]
    fn test_agent_ordering_opening() {
        let mut engine = DebateEngine::new(test_config());

        assert_eq!(engine.next_agent_id(), Some("agent-a"));
        engine.advance();
        assert_eq!(engine.next_agent_id(), Some("agent-b"));
        engine.advance();
        assert_eq!(engine.next_agent_id(), Some("agent-c"));
    }

    #[test]
    fn test_agent_ordering_rebuttal() {
        let mut engine = DebateEngine::new(test_config());

        // Skip through Opening (3 turns)
        for _ in 0..3 {
            engine.advance();
        }

        assert_eq!(*engine.current_phase(), DebatePhase::Rebuttal(1));
        assert_eq!(engine.next_agent_id(), Some("agent-a"));
        engine.advance();
        assert_eq!(engine.next_agent_id(), Some("agent-b"));
        engine.advance();
        assert_eq!(engine.next_agent_id(), Some("agent-c"));
    }

    #[test]
    fn test_moderator_speaks_in_synthesis() {
        let mut engine = DebateEngine::new(test_config());

        // Advance through Opening (3) + Rebuttal1 (3) + Rebuttal2 (3) + Closing (3)
        for _ in 0..12 {
            engine.advance();
        }

        assert_eq!(*engine.current_phase(), DebatePhase::Synthesis);
        assert_eq!(engine.next_agent_id(), Some("mod-1"));
    }

    #[test]
    fn test_moderator_defaults_to_last_participant() {
        let mut config = test_config();
        config.moderator_agent_id = None;
        let mut engine = DebateEngine::new(config);

        // Advance to Synthesis
        for _ in 0..12 {
            engine.advance();
        }

        assert_eq!(*engine.current_phase(), DebatePhase::Synthesis);
        assert_eq!(engine.next_agent_id(), Some("agent-c"));
    }

    #[test]
    fn test_next_agent_none_when_complete() {
        let mut engine = DebateEngine::new(test_config());

        // Advance to Complete (12 participant turns + 1 synthesis)
        for _ in 0..13 {
            engine.advance();
        }

        assert!(engine.is_complete());
        assert_eq!(engine.next_agent_id(), None);
    }

    // -- prompt building ----------------------------------------------------

    #[test]
    fn test_opening_prompt_includes_topic() {
        let engine = DebateEngine::new(test_config());

        let prompt = engine.build_turn_prompt("Alice", &[]);
        assert!(prompt.contains("opening position"));
        assert!(prompt.contains("Rust vs Go"));
        assert!(prompt.contains("Alice"));
    }

    #[test]
    fn test_rebuttal_prompt_includes_previous_turns() {
        let mut engine = DebateEngine::new(test_config());

        // Advance to Rebuttal(1)
        for _ in 0..3 {
            engine.advance();
        }
        assert_eq!(*engine.current_phase(), DebatePhase::Rebuttal(1));

        let previous = vec![
            "Alice: Rust is safe.".to_owned(),
            "Bob: Go is simple.".to_owned(),
        ];
        let prompt = engine.build_turn_prompt("Charlie", &previous);

        assert!(prompt.contains("rebuttal round 1"));
        assert!(prompt.contains("Alice: Rust is safe."));
        assert!(prompt.contains("Bob: Go is simple."));
        assert!(prompt.contains("Present your rebuttal"));
        assert!(prompt.contains("Charlie"));
    }

    #[test]
    fn test_closing_prompt_includes_topic() {
        let mut engine = DebateEngine::new(test_config());

        // Advance to Closing: Opening (3) + Rebuttal1 (3) + Rebuttal2 (3)
        for _ in 0..9 {
            engine.advance();
        }
        assert_eq!(*engine.current_phase(), DebatePhase::Closing);

        let prompt = engine.build_turn_prompt("Alice", &[]);
        assert!(prompt.contains("closing statement"));
        assert!(prompt.contains("Rust vs Go"));
        assert!(prompt.contains("Alice"));
    }

    #[test]
    fn test_synthesis_prompt() {
        let mut engine = DebateEngine::new(test_config());

        // Advance to Synthesis
        for _ in 0..12 {
            engine.advance();
        }
        assert_eq!(*engine.current_phase(), DebatePhase::Synthesis);

        let prompt = engine.build_turn_prompt("Moderator", &[]);
        assert!(prompt.contains("moderator"));
        assert!(prompt.contains("summarize"));
        assert!(prompt.contains("balanced conclusion"));
    }

    // -- edge cases ---------------------------------------------------------

    #[test]
    fn test_single_participant() {
        let config = DebateConfig {
            topic: "Monologue".to_owned(),
            num_rounds: 1,
            moderator_agent_id: None,
            participant_agent_ids: vec!["solo".to_owned()],
            conversation_id: "conv-2".to_owned(),
        };
        let mut engine = DebateEngine::new(config);

        // Opening: 1 turn
        assert_eq!(engine.next_agent_id(), Some("solo"));
        engine.advance();

        // Rebuttal(1): 1 turn
        assert_eq!(*engine.current_phase(), DebatePhase::Rebuttal(1));
        assert_eq!(engine.next_agent_id(), Some("solo"));
        engine.advance();

        // Closing: 1 turn
        assert_eq!(*engine.current_phase(), DebatePhase::Closing);
        engine.advance();

        // Synthesis: moderator defaults to last (only) participant
        assert_eq!(*engine.current_phase(), DebatePhase::Synthesis);
        assert_eq!(engine.next_agent_id(), Some("solo"));
        engine.advance();

        assert!(engine.is_complete());
    }

    #[test]
    fn test_advance_returns_new_phase() {
        let mut engine = DebateEngine::new(test_config());

        // First two advances stay in Opening
        let p = engine.advance();
        assert_eq!(p, DebatePhase::Opening);
        let p = engine.advance();
        assert_eq!(p, DebatePhase::Opening);
        // Third advance transitions to Rebuttal(1)
        let p = engine.advance();
        assert_eq!(p, DebatePhase::Rebuttal(1));
    }
}
