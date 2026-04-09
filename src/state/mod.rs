// ARC CLI — File-backed state persistence
// Saves session data to ~/.arc-cli/state.json

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::models::{AgentLog, DiffResult, LLMUsage, Task};

// =====================================================================
//  Session — the root persistence object
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub tasks: Vec<Task>,
    pub logs: Vec<AgentLog>,
    pub diffs: Vec<DiffResult>,
    pub llm_usage: Vec<LLMUsage>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            tasks: Vec::new(),
            logs: Vec::new(),
            diffs: Vec::new(),
            llm_usage: Vec::new(),
        }
    }
}

// =====================================================================
//  StateStore — read / write session to disk
// =====================================================================

pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new() -> Self {
        let base = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".arc-cli");
        Self { path: base }
    }

    fn state_file(&self) -> PathBuf {
        self.path.join("state.json")
    }

    fn history_dir(&self) -> PathBuf {
        self.path.join("history")
    }

    /// Ensure the state directory exists.
    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.path)?;
        std::fs::create_dir_all(self.history_dir())?;
        Ok(())
    }

    /// Save current session to disk.
    pub fn save(&self, session: &Session) -> Result<()> {
        self.init()?;
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(self.state_file(), json)?;
        Ok(())
    }

    /// Load session from disk, or None if no file exists.
    pub fn load(&self) -> Result<Option<Session>> {
        let file = self.state_file();
        if !file.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(file)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    /// Archive current session to history and start fresh.
    pub fn archive(&self, session: &Session) -> Result<()> {
        self.init()?;
        let filename = format!("session_{}.json", session.id);
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(self.history_dir().join(filename), json)?;
        Ok(())
    }
}
