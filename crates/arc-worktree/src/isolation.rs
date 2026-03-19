//! Subagent isolation modes using worktrees.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Isolation strategy for a subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SubagentIsolation {
    /// No isolation — subagent works in the main worktree.
    #[serde(rename = "none")]
    None,

    /// Worktree isolation — subagent gets its own git worktree.
    #[serde(rename = "worktree")]
    Worktree {
        /// Optional name for the worktree. Auto-generated if not set.
        #[serde(default)]
        name: Option<String>,

        /// Sparse paths for monorepo optimization.
        #[serde(default)]
        sparse_paths: Vec<String>,

        /// Whether to auto-merge the worktree branch on completion.
        #[serde(default)]
        auto_merge: bool,
    },

    /// Shadow directory isolation (existing ARC feature — hardlinks).
    #[serde(rename = "shadow")]
    Shadow {
        shadow_dir: Option<String>,
    },
}

impl SubagentIsolation {
    /// Generate a worktree name for a subagent.
    pub fn worktree_name_for_agent(agent_name: &str, session_id: Uuid) -> String {
        format!(
            "{}-{}",
            agent_name,
            &session_id.to_string()[..8]
        )
    }
}
