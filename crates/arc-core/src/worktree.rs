// SPDX-License-Identifier: MIT
//! # Worktree Isolation — Git Worktree-Based Session Isolation
//!
//! `--worktree` flag support: sparse-checkout, Enter/ExitWorktree tools,
//! auto-cleanup stale worktrees.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Worktree session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    pub enabled: bool,
    #[serde(default)]
    pub sparse_paths: Vec<String>,
    #[serde(default)]
    pub auto_cleanup: bool,
    #[serde(default)]
    pub branch_prefix: String,
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sparse_paths: Vec::new(),
            auto_cleanup: true,
            branch_prefix: "arc-agent/".into(),
        }
    }
}

/// A managed worktree instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeInstance {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub original_dir: PathBuf,
    pub session_id: String,
    pub created_at: u64,
}

/// Worktree manager for creating and cleaning up worktrees.
pub struct WorktreeManager {
    config: WorktreeConfig,
    repo_root: PathBuf,
    active_worktrees: Vec<WorktreeInstance>,
}

impl WorktreeManager {
    pub fn new(repo_root: PathBuf, config: WorktreeConfig) -> Self {
        Self {
            config,
            repo_root,
            active_worktrees: Vec::new(),
        }
    }

    /// Create a new isolated worktree for an agent session.
    pub fn create_worktree(
        &mut self,
        session_id: &str,
        branch_name: Option<&str>,
    ) -> Result<WorktreeInstance, String> {
        let branch = branch_name.map(|b| b.to_string()).unwrap_or_else(|| {
            format!(
                "{}session-{}",
                self.config.branch_prefix,
                &session_id[..8.min(session_id.len())]
            )
        });
        let wt_dir = self.repo_root.join(".arc-worktrees").join(&branch);

        // Create worktree via git.
        let output = std::process::Command::new("git")
            .args(["worktree", "add", "-b", &branch])
            .arg(&wt_dir)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to create worktree: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Apply sparse-checkout if configured.
        if !self.config.sparse_paths.is_empty() {
            let _ = std::process::Command::new("git")
                .args(["sparse-checkout", "set", "--no-cone"])
                .args(&self.config.sparse_paths)
                .current_dir(&wt_dir)
                .output();
        }

        let instance = WorktreeInstance {
            name: branch.clone(),
            path: wt_dir,
            branch,
            original_dir: self.repo_root.clone(),
            session_id: session_id.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.active_worktrees.push(instance.clone());
        Ok(instance)
    }

    /// Remove a worktree.
    pub fn remove_worktree(&mut self, name: &str) -> Result<(), String> {
        let _ = std::process::Command::new("git")
            .args(["worktree", "remove", "--force", name])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to remove worktree: {e}"))?;

        self.active_worktrees.retain(|w| w.name != name);
        Ok(())
    }

    /// Clean up stale worktrees not associated with active sessions.
    pub fn cleanup_stale(&mut self) -> Result<usize, String> {
        let output = std::process::Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to prune worktrees: {e}"))?;
        // Count removed.
        let removed = String::from_utf8_lossy(&output.stderr).lines().count();
        Ok(removed)
    }

    pub fn active_worktrees(&self) -> &[WorktreeInstance] {
        &self.active_worktrees
    }
}
