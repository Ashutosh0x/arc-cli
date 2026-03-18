//! # arc-worktree
//!
//! Subsystem for managing parallel Git Worktrees natively instead of shadow
//! workspaces, allowing branch-bound multi-agent collaboration.

use anyhow::Result;
use std::process::Command;

pub struct WorktreeManager {
    base_dir: std::path::PathBuf,
}

impl WorktreeManager {
    pub fn new(base: std::path::PathBuf) -> Self {
        Self { base_dir: base }
    }

    pub fn create_worktree(&self, name: &str, branch: &str) -> Result<()> {
        let status = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch)
            .arg(name)
            .current_dir(&self.base_dir)
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to create worktree {}", name);
        }

        Ok(())
    }
}
