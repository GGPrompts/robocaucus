use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux not installed")]
    NotInstalled,
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TmuxError>;

pub struct TmuxSession {
    pub name: String,
    pub created: bool,
}

pub struct TmuxManager {
    prefix: String,
}

impl TmuxManager {
    pub fn new() -> Self {
        Self {
            prefix: "rc-".to_string(),
        }
    }

    /// Build the full session name from a bare id.
    fn session_name(&self, id: &str) -> String {
        format!("{}{}", self.prefix, id)
    }

    /// Create a new detached tmux session with the given dimensions.
    pub async fn create_session(
        &self,
        id: &str,
        width: u16,
        height: u16,
    ) -> Result<TmuxSession> {
        let name = self.session_name(id);
        let output = Command::new("tmux")
            .args([
                "new-session",
                "-d",
                "-s",
                &name,
                "-x",
                &width.to_string(),
                "-y",
                &height.to_string(),
            ])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    TmuxError::NotInstalled
                } else {
                    TmuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TmuxError::CommandFailed(stderr.into_owned()));
        }

        Ok(TmuxSession {
            name,
            created: true,
        })
    }

    /// Send a command string to a tmux session followed by Enter.
    pub async fn send_command(&self, id: &str, command: &str) -> Result<()> {
        let name = self.session_name(id);
        let output = Command::new("tmux")
            .args(["send-keys", "-t", &name, command, "Enter"])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    TmuxError::NotInstalled
                } else {
                    TmuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("can't find")
                || stderr.contains("no such session")
                || stderr.contains("session not found")
            {
                return Err(TmuxError::SessionNotFound(name));
            }
            return Err(TmuxError::CommandFailed(stderr.into_owned()));
        }

        Ok(())
    }

    /// Capture the current pane content of a session.
    pub async fn capture_output(&self, id: &str) -> Result<String> {
        let name = self.session_name(id);
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", &name, "-p"])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    TmuxError::NotInstalled
                } else {
                    TmuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("can't find")
                || stderr.contains("no such session")
                || stderr.contains("session not found")
            {
                return Err(TmuxError::SessionNotFound(name));
            }
            return Err(TmuxError::CommandFailed(stderr.into_owned()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// List all tmux sessions that belong to RoboCaucus (rc- prefix).
    pub async fn list_sessions(&self) -> Result<Vec<String>> {
        let output = Command::new("tmux")
            .args(["ls", "-F", "#{session_name}"])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    TmuxError::NotInstalled
                } else {
                    TmuxError::Io(e)
                }
            })?;

        // tmux ls exits with error when no sessions exist — treat as empty list.
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no server running") || stderr.contains("no sessions") {
                return Ok(Vec::new());
            }
            return Err(TmuxError::CommandFailed(stderr.into_owned()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sessions = stdout
            .lines()
            .filter(|line| line.starts_with(&self.prefix))
            .map(|line| line.to_string())
            .collect();

        Ok(sessions)
    }

    /// Kill a specific tmux session.
    pub async fn kill_session(&self, id: &str) -> Result<()> {
        let name = self.session_name(id);
        let output = Command::new("tmux")
            .args(["kill-session", "-t", &name])
            .output()
            .await
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    TmuxError::NotInstalled
                } else {
                    TmuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("can't find")
                || stderr.contains("no such session")
                || stderr.contains("session not found")
            {
                return Err(TmuxError::SessionNotFound(name));
            }
            return Err(TmuxError::CommandFailed(stderr.into_owned()));
        }

        Ok(())
    }

    /// Check whether a specific session exists.
    pub async fn session_exists(&self, id: &str) -> bool {
        let name = self.session_name(id);
        let output = Command::new("tmux")
            .args(["has-session", "-t", &name])
            .output()
            .await;

        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }

    /// Kill any rc- sessions that are not in the known_ids list.
    /// Returns the names of sessions that were killed.
    pub async fn cleanup_orphans(&self, known_ids: &[String]) -> Result<Vec<String>> {
        let all_sessions = self.list_sessions().await?;

        let known_names: Vec<String> = known_ids
            .iter()
            .map(|id| self.session_name(id))
            .collect();

        let mut killed = Vec::new();
        for session in all_sessions {
            if !known_names.contains(&session) {
                // Kill directly by full name (session already has prefix).
                let output = Command::new("tmux")
                    .args(["kill-session", "-t", &session])
                    .output()
                    .await
                    .map_err(|e| {
                        if e.kind() == std::io::ErrorKind::NotFound {
                            TmuxError::NotInstalled
                        } else {
                            TmuxError::Io(e)
                        }
                    })?;

                if output.status.success() {
                    killed.push(session);
                }
            }
        }

        Ok(killed)
    }
}
