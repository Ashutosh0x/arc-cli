//! # Session Fork/Branch — /fork and /branch commands
//!
//! Selective state copying with per-fork plan files.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFork {
    pub id: String,
    pub parent_session_id: String,
    pub branch_name: String,
    pub fork_point_turn: usize,
    pub created_at: u64,
    pub plan_file: Option<PathBuf>,
}

pub struct ForkManager {
    forks: Vec<SessionFork>,
    session_dir: PathBuf,
}

impl ForkManager {
    pub fn new(session_dir: PathBuf) -> Self {
        Self {
            forks: Vec::new(),
            session_dir,
        }
    }

    pub fn create_fork(
        &mut self,
        parent_id: &str,
        turn: usize,
        name: Option<&str>,
    ) -> Result<SessionFork, String> {
        let branch = name
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("fork-{turn}"));
        let fork = SessionFork {
            id: format!("{parent_id}-{branch}"),
            parent_session_id: parent_id.to_string(),
            branch_name: branch,
            fork_point_turn: turn,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            plan_file: None,
        };
        self.forks.push(fork.clone());
        Ok(fork)
    }

    pub fn list_forks(&self, parent_id: &str) -> Vec<&SessionFork> {
        self.forks
            .iter()
            .filter(|f| f.parent_session_id == parent_id)
            .collect()
    }
}
