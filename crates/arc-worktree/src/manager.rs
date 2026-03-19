//! Worktree manager: create, list, and cleanup git worktrees for isolated sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for worktree behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    /// Default base directory for worktrees (relative to repo root).
    #[serde(default = "default_base_dir")]
    pub base_dir: String,

    /// Paths to include in sparse checkout (for monorepos).
    #[serde(default)]
    pub sparse_paths: Vec<String>,

    /// Whether to auto-cleanup worktrees on session exit.
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup: bool,

    /// Maximum number of concurrent worktrees.
    #[serde(default = "default_max_worktrees")]
    pub max_worktrees: usize,
}

fn default_base_dir() -> String {
    ".arc-worktrees".into()
}
fn default_auto_cleanup() -> bool {
    true
}
fn default_max_worktrees() -> usize {
    10
}

impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            base_dir: default_base_dir(),
            sparse_paths: vec![],
            auto_cleanup: true,
            max_worktrees: 10,
        }
    }
}

/// A managed worktree instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedWorktree {
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub sparse_paths: Vec<String>,
    pub status: WorktreeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorktreeStatus {
    Active,
    Idle,
    MarkedForCleanup,
}

#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("Git command failed: {0}")]
    GitCommand(String),

    #[error("Worktree '{0}' already exists")]
    AlreadyExists(String),

    #[error("Worktree '{0}' not found")]
    NotFound(String),

    #[error("Maximum worktree limit ({0}) reached")]
    LimitReached(usize),

    #[error("Not in a git repository")]
    NotGitRepo,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct WorktreeManager {
    repo_root: PathBuf,
    config: WorktreeConfig,
    worktrees: HashMap<Uuid, ManagedWorktree>,
}

impl WorktreeManager {
    /// Create a new WorktreeManager for the given repository root.
    pub async fn new(repo_root: &Path, config: WorktreeConfig) -> Result<Self, WorktreeError> {
        // Verify we're in a git repo
        let output = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(repo_root)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            return Err(WorktreeError::NotGitRepo);
        }

        let mut manager = Self {
            repo_root: repo_root.to_path_buf(),
            config,
            worktrees: HashMap::new(),
        };

        // Scan for existing worktrees
        manager.refresh_worktree_list().await?;

        Ok(manager)
    }

    /// Create a new isolated worktree.
    ///
    /// This is the equivalent of `claude --worktree <name>`.
    pub async fn create(
        &mut self,
        name: &str,
        session_id: Option<Uuid>,
    ) -> Result<ManagedWorktree, WorktreeError> {
        // Check limits
        let active_count = self
            .worktrees
            .values()
            .filter(|w| w.status == WorktreeStatus::Active)
            .count();

        if active_count >= self.config.max_worktrees {
            return Err(WorktreeError::LimitReached(self.config.max_worktrees));
        }

        let worktree_id = Uuid::new_v4();
        let branch_name = format!("arc-worktree/{name}-{}", &worktree_id.to_string()[..8]);
        let worktree_path = self
            .repo_root
            .join(&self.config.base_dir)
            .join(name);

        if worktree_path.exists() {
            return Err(WorktreeError::AlreadyExists(name.to_string()));
        }

        // Create the base directory
        std::fs::create_dir_all(worktree_path.parent().unwrap())?;

        // Create the worktree with a new branch
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch_name,
                &worktree_path.display().to_string(),
                "HEAD",
            ])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitCommand(stderr.to_string()));
        }

        // Apply sparse checkout if configured
        if !self.config.sparse_paths.is_empty() {
            self.apply_sparse_checkout(&worktree_path, &self.config.sparse_paths)
                .await?;
        }

        let worktree = ManagedWorktree {
            id: worktree_id,
            name: name.to_string(),
            path: worktree_path,
            branch: branch_name,
            session_id,
            created_at: Utc::now(),
            sparse_paths: self.config.sparse_paths.clone(),
            status: WorktreeStatus::Active,
        };

        info!(
            worktree = %name,
            path = %worktree.path.display(),
            branch = %worktree.branch,
            "Created isolated worktree"
        );

        self.worktrees.insert(worktree_id, worktree.clone());

        Ok(worktree)
    }

    /// Create a worktree with custom sparse paths (for monorepo subsets).
    pub async fn create_sparse(
        &mut self,
        name: &str,
        sparse_paths: Vec<String>,
        session_id: Option<Uuid>,
    ) -> Result<ManagedWorktree, WorktreeError> {
        let worktree_id = Uuid::new_v4();
        let branch_name = format!("arc-worktree/{name}-{}", &worktree_id.to_string()[..8]);
        let worktree_path = self
            .repo_root
            .join(&self.config.base_dir)
            .join(name);

        if worktree_path.exists() {
            return Err(WorktreeError::AlreadyExists(name.to_string()));
        }

        std::fs::create_dir_all(worktree_path.parent().unwrap())?;

        // Create worktree with no checkout first
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "--no-checkout",
                "-b",
                &branch_name,
                &worktree_path.display().to_string(),
                "HEAD",
            ])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitCommand(stderr.to_string()));
        }

        // Apply sparse checkout
        self.apply_sparse_checkout(&worktree_path, &sparse_paths)
            .await?;

        // Now checkout
        let output = Command::new("git")
            .args(["checkout", &branch_name])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            debug!(
                stderr = %String::from_utf8_lossy(&output.stderr),
                "Sparse checkout may have warnings"
            );
        }

        let worktree = ManagedWorktree {
            id: worktree_id,
            name: name.to_string(),
            path: worktree_path,
            branch: branch_name,
            session_id,
            created_at: Utc::now(),
            sparse_paths,
            status: WorktreeStatus::Active,
        };

        info!(
            worktree = %name,
            sparse_paths = ?worktree.sparse_paths,
            "Created sparse worktree"
        );

        self.worktrees.insert(worktree_id, worktree.clone());

        Ok(worktree)
    }

    /// Apply git sparse-checkout to a worktree.
    async fn apply_sparse_checkout(
        &self,
        worktree_path: &Path,
        paths: &[String],
    ) -> Result<(), WorktreeError> {
        // Initialize sparse checkout
        let output = Command::new("git")
            .args(["sparse-checkout", "init", "--cone"])
            .current_dir(worktree_path)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "sparse-checkout init warning");
        }

        // Set the sparse checkout paths
        let mut cmd = Command::new("git");
        cmd.arg("sparse-checkout").arg("set");
        for path in paths {
            cmd.arg(path);
        }
        cmd.current_dir(worktree_path);

        let output = cmd
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitCommand(format!(
                "sparse-checkout set failed: {stderr}"
            )));
        }

        info!(
            paths = ?paths,
            worktree = %worktree_path.display(),
            "Applied sparse checkout"
        );

        Ok(())
    }

    /// Remove a worktree and optionally delete its branch.
    pub async fn remove(
        &mut self,
        id: Uuid,
        delete_branch: bool,
    ) -> Result<(), WorktreeError> {
        let worktree = self
            .worktrees
            .get(&id)
            .ok_or(WorktreeError::NotFound(id.to_string()))?
            .clone();

        // Remove the worktree
        let output = Command::new("git")
            .args(["worktree", "remove", "--force", &worktree.path.display().to_string()])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "Worktree removal warning, attempting force cleanup");
            // Force cleanup
            if worktree.path.exists() {
                std::fs::remove_dir_all(&worktree.path)?;
            }
        }

        // Optionally delete the branch
        if delete_branch {
            let _ = Command::new("git")
                .args(["branch", "-D", &worktree.branch])
                .current_dir(&self.repo_root)
                .output()
                .await;
        }

        self.worktrees.remove(&id);

        info!(
            worktree = %worktree.name,
            "Removed worktree"
        );

        Ok(())
    }

    /// Cleanup all worktrees marked for cleanup or associated with ended sessions.
    pub async fn cleanup_stale(&mut self) -> Result<usize, WorktreeError> {
        let stale_ids: Vec<Uuid> = self
            .worktrees
            .iter()
            .filter(|(_, w)| w.status == WorktreeStatus::MarkedForCleanup)
            .map(|(id, _)| *id)
            .collect();

        let count = stale_ids.len();
        for id in stale_ids {
            self.remove(id, true).await?;
        }

        if count > 0 {
            info!(count, "Cleaned up stale worktrees");
        }

        Ok(count)
    }

    /// Mark a worktree for cleanup (called on session end if auto_cleanup is true).
    pub fn mark_for_cleanup(&mut self, id: Uuid) {
        if let Some(worktree) = self.worktrees.get_mut(&id) {
            worktree.status = WorktreeStatus::MarkedForCleanup;
        }
    }

    /// List all managed worktrees.
    pub fn list(&self) -> Vec<&ManagedWorktree> {
        let mut worktrees: Vec<_> = self.worktrees.values().collect();
        worktrees.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        worktrees
    }

    /// Refresh the worktree list by scanning git.
    async fn refresh_worktree_list(&mut self) -> Result<(), WorktreeError> {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.repo_root)
            .output()
            .await
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            debug!(worktree_list = %stdout, "Scanned existing worktrees");
        }

        Ok(())
    }
}
