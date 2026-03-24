use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Domain structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub workspace_path: Option<String>,
    pub orchestration_mode: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub model: String,
    pub color: String,
    pub scope: String,
    pub system_prompt: String,
    pub workspace_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub agent_id: Option<String>,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub timestamp: String,
    pub usage_json: Option<String>,
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a database at `path`, run migrations, and enable WAL.
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    /// Create an in-memory database – useful for tests.
    pub fn new_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    // -- private helpers ----------------------------------------------------

    fn init(&self) -> SqlResult<()> {
        // Enable WAL mode for better concurrent read performance.
        self.conn.pragma_update(None, "journal_mode", "wal")?;

        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS conversations (
                id                TEXT PRIMARY KEY,
                title             TEXT NOT NULL,
                workspace_path    TEXT,
                orchestration_mode TEXT NOT NULL DEFAULT 'manual',
                created_at        TEXT NOT NULL,
                updated_at        TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agents (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                model          TEXT NOT NULL,
                color          TEXT NOT NULL,
                scope          TEXT NOT NULL DEFAULT 'global',
                system_prompt  TEXT NOT NULL DEFAULT '',
                workspace_path TEXT,
                created_at     TEXT NOT NULL,
                updated_at     TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(id),
                agent_id        TEXT REFERENCES agents(id),
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                model           TEXT,
                timestamp       TEXT NOT NULL,
                usage_json      TEXT
            );

            CREATE TABLE IF NOT EXISTS conversation_agents (
                conversation_id TEXT NOT NULL REFERENCES conversations(id),
                agent_id        TEXT NOT NULL REFERENCES agents(id),
                PRIMARY KEY (conversation_id, agent_id)
            );

            -- Indexes
            CREATE INDEX IF NOT EXISTS idx_messages_conv_ts
                ON messages(conversation_id, timestamp);

            CREATE INDEX IF NOT EXISTS idx_agents_scope_ws
                ON agents(scope, workspace_path);

            CREATE INDEX IF NOT EXISTS idx_conv_agents_agent
                ON conversation_agents(agent_id);
            ",
        )?;

        Ok(())
    }

    fn now_iso8601() -> String {
        // Use a simple UTC timestamp derived from std::time.  We avoid pulling
        // in the `chrono` or `time` crate by formatting manually.  The format
        // produced is ISO-8601-ish: "2026-03-24T12:34:56Z" (always UTC).
        use std::time::SystemTime;
        let dur = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs();
        // Break epoch seconds into date/time components (UTC).
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;

        // Civil date from day count (algorithm from Howard Hinnant).
        let z = days as i64 + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            y, m, d, hours, minutes, seconds,
        )
    }

    // -----------------------------------------------------------------------
    // Conversations CRUD
    // -----------------------------------------------------------------------

    pub fn create_conversation(
        &self,
        title: &str,
        workspace_path: Option<&str>,
        mode: &str,
    ) -> SqlResult<Conversation> {
        let id = Uuid::new_v4().to_string();
        let now = Self::now_iso8601();
        self.conn.execute(
            "INSERT INTO conversations (id, title, workspace_path, orchestration_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, title, workspace_path, mode, now, now],
        )?;
        Ok(Conversation {
            id,
            title: title.to_owned(),
            workspace_path: workspace_path.map(|s| s.to_owned()),
            orchestration_mode: mode.to_owned(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_conversations(&self) -> SqlResult<Vec<Conversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, workspace_path, orchestration_mode, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                workspace_path: row.get(2)?,
                orchestration_mode: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Agents CRUD
    // -----------------------------------------------------------------------

    pub fn create_agent(
        &self,
        name: &str,
        model: &str,
        color: &str,
        scope: &str,
        system_prompt: &str,
        workspace_path: Option<&str>,
    ) -> SqlResult<Agent> {
        let id = Uuid::new_v4().to_string();
        let now = Self::now_iso8601();
        self.conn.execute(
            "INSERT INTO agents (id, name, model, color, scope, system_prompt, workspace_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, name, model, color, scope, system_prompt, workspace_path, now, now],
        )?;
        Ok(Agent {
            id,
            name: name.to_owned(),
            model: model.to_owned(),
            color: color.to_owned(),
            scope: scope.to_owned(),
            system_prompt: system_prompt.to_owned(),
            workspace_path: workspace_path.map(|s| s.to_owned()),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_agents(&self, scope_filter: Option<&str>) -> SqlResult<Vec<Agent>> {
        let (sql, filter_params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            match scope_filter {
                Some(scope) => (
                    "SELECT id, name, model, color, scope, system_prompt, workspace_path, created_at, updated_at
                     FROM agents WHERE scope = ?1 ORDER BY name"
                        .to_owned(),
                    vec![Box::new(scope.to_owned()) as Box<dyn rusqlite::types::ToSql>],
                ),
                None => (
                    "SELECT id, name, model, color, scope, system_prompt, workspace_path, created_at, updated_at
                     FROM agents ORDER BY name"
                        .to_owned(),
                    vec![],
                ),
            };

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(filter_params.iter()), |row| {
            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                model: row.get(2)?,
                color: row.get(3)?,
                scope: row.get(4)?,
                system_prompt: row.get(5)?,
                workspace_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Messages CRUD
    // -----------------------------------------------------------------------

    pub fn create_message(
        &self,
        conversation_id: &str,
        agent_id: Option<&str>,
        role: &str,
        content: &str,
        model: Option<&str>,
    ) -> SqlResult<Message> {
        let id = Uuid::new_v4().to_string();
        let now = Self::now_iso8601();
        self.conn.execute(
            "INSERT INTO messages (id, conversation_id, agent_id, role, content, model, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, conversation_id, agent_id, role, content, model, now],
        )?;
        Ok(Message {
            id,
            conversation_id: conversation_id.to_owned(),
            agent_id: agent_id.map(|s| s.to_owned()),
            role: role.to_owned(),
            content: content.to_owned(),
            model: model.map(|s| s.to_owned()),
            timestamp: now,
            usage_json: None,
        })
    }

    pub fn list_messages(&self, conversation_id: &str) -> SqlResult<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, agent_id, role, content, model, timestamp, usage_json
             FROM messages WHERE conversation_id = ?1 ORDER BY timestamp ASC",
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                agent_id: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                model: row.get(5)?,
                timestamp: row.get(6)?,
                usage_json: row.get(7)?,
            })
        })?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Conversation-Agents join table
    // -----------------------------------------------------------------------

    pub fn add_agent_to_conversation(
        &self,
        conversation_id: &str,
        agent_id: &str,
    ) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO conversation_agents (conversation_id, agent_id)
             VALUES (?1, ?2)",
            params![conversation_id, agent_id],
        )?;
        Ok(())
    }

    pub fn get_conversation_agents(&self, conversation_id: &str) -> SqlResult<Vec<Agent>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, a.name, a.model, a.color, a.scope, a.system_prompt,
                    a.workspace_path, a.created_at, a.updated_at
             FROM agents a
             INNER JOIN conversation_agents ca ON ca.agent_id = a.id
             WHERE ca.conversation_id = ?1
             ORDER BY a.name",
        )?;
        let rows = stmt.query_map(params![conversation_id], |row| {
            Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                model: row.get(2)?,
                color: row.get(3)?,
                scope: row.get(4)?,
                system_prompt: row.get(5)?,
                workspace_path: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list_conversations() {
        let db = Database::new_in_memory().unwrap();
        let conv = db
            .create_conversation("Test chat", None, "manual")
            .unwrap();
        assert_eq!(conv.title, "Test chat");
        assert_eq!(conv.orchestration_mode, "manual");

        let convs = db.list_conversations().unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].id, conv.id);
    }

    #[test]
    fn test_create_and_list_agents() {
        let db = Database::new_in_memory().unwrap();
        let agent = db
            .create_agent("Claude", "claude", "#7c3aed", "global", "You are helpful.", None)
            .unwrap();
        assert_eq!(agent.name, "Claude");

        let all = db.list_agents(None).unwrap();
        assert_eq!(all.len(), 1);

        let global = db.list_agents(Some("global")).unwrap();
        assert_eq!(global.len(), 1);

        let ws = db.list_agents(Some("workspace")).unwrap();
        assert!(ws.is_empty());
    }

    #[test]
    fn test_messages() {
        let db = Database::new_in_memory().unwrap();
        let conv = db
            .create_conversation("Msg test", None, "manual")
            .unwrap();
        let agent = db
            .create_agent("Bot", "claude", "#000000", "global", "", None)
            .unwrap();

        let user_msg = db
            .create_message(&conv.id, None, "user", "Hello!", None)
            .unwrap();
        assert_eq!(user_msg.role, "user");
        assert!(user_msg.agent_id.is_none());

        let bot_msg = db
            .create_message(
                &conv.id,
                Some(&agent.id),
                "assistant",
                "Hi there!",
                Some("claude"),
            )
            .unwrap();
        assert_eq!(bot_msg.agent_id.as_deref(), Some(agent.id.as_str()));

        let msgs = db.list_messages(&conv.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "Hello!");
        assert_eq!(msgs[1].content, "Hi there!");
    }

    #[test]
    fn test_conversation_agents_join() {
        let db = Database::new_in_memory().unwrap();
        let conv = db
            .create_conversation("Join test", None, "panel")
            .unwrap();
        let a1 = db
            .create_agent("Agent A", "claude", "#111111", "global", "", None)
            .unwrap();
        let a2 = db
            .create_agent("Agent B", "gemini", "#222222", "global", "", None)
            .unwrap();

        db.add_agent_to_conversation(&conv.id, &a1.id).unwrap();
        db.add_agent_to_conversation(&conv.id, &a2.id).unwrap();
        // Duplicate should be ignored
        db.add_agent_to_conversation(&conv.id, &a1.id).unwrap();

        let agents = db.get_conversation_agents(&conv.id).unwrap();
        assert_eq!(agents.len(), 2);
    }
}
