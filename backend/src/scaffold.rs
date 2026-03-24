use std::fs;
use std::io;
use std::path::Path;

/// Create the agent home directory and write the provider-specific instruction
/// file containing the agent's system prompt.
///
/// Supported providers:
/// - `claude`  -> `{agent_home}/CLAUDE.md`
/// - `codex`   -> `{agent_home}/.codex/instructions.md`
/// - `gemini`  -> `{agent_home}/GEMINI.md`
/// - `copilot` -> `{agent_home}/.copilot-instructions.md`
pub fn scaffold_agent_folder(provider: &str, agent_home: &str, system_prompt: &str) -> io::Result<()> {
    let base = Path::new(agent_home);
    fs::create_dir_all(base)?;

    let (dir, filename) = match provider {
        "claude" => (None, "CLAUDE.md"),
        "codex" => (Some(".codex"), "instructions.md"),
        "gemini" => (None, "GEMINI.md"),
        "copilot" => (None, ".copilot-instructions.md"),
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported provider: {other}"),
            ));
        }
    };

    let file_path = match dir {
        Some(sub) => {
            let sub_dir = base.join(sub);
            fs::create_dir_all(&sub_dir)?;
            sub_dir.join(filename)
        }
        None => base.join(filename),
    };

    fs::write(&file_path, system_prompt)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("robocaucus_test_{name}_{}", std::process::id()));
        // Clean up from any prior run.
        let _ = fs::remove_dir_all(&dir);
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn test_scaffold_claude() {
        let home = temp_dir("claude");
        scaffold_agent_folder("claude", &home, "You are a helpful assistant.").unwrap();
        let content = fs::read_to_string(Path::new(&home).join("CLAUDE.md")).unwrap();
        assert_eq!(content, "You are a helpful assistant.");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_scaffold_codex() {
        let home = temp_dir("codex");
        scaffold_agent_folder("codex", &home, "Codex prompt").unwrap();
        let content = fs::read_to_string(Path::new(&home).join(".codex/instructions.md")).unwrap();
        assert_eq!(content, "Codex prompt");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_scaffold_gemini() {
        let home = temp_dir("gemini");
        scaffold_agent_folder("gemini", &home, "Gemini prompt").unwrap();
        let content = fs::read_to_string(Path::new(&home).join("GEMINI.md")).unwrap();
        assert_eq!(content, "Gemini prompt");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_scaffold_copilot() {
        let home = temp_dir("copilot");
        scaffold_agent_folder("copilot", &home, "Copilot prompt").unwrap();
        let content = fs::read_to_string(Path::new(&home).join(".copilot-instructions.md")).unwrap();
        assert_eq!(content, "Copilot prompt");
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_scaffold_unknown_provider() {
        let home = temp_dir("unknown");
        let result = scaffold_agent_folder("unknown_cli", &home, "prompt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsupported provider"));
        let _ = fs::remove_dir_all(&home);
    }
}
