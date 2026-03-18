use std::path::{Path, PathBuf};
use git2::{Repository, Signature, IndexAddOption};

use crate::error::{ArcError, Result};

pub struct CheckpointSystem {
    repo_path: PathBuf,
}

impl CheckpointSystem {
    /// Initialize checkpoint system for a directory.
    /// In MVP, we just use the existing git repository.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let path = path.as_ref().to_path_buf();
        // Check if it's a git repo
        if Repository::discover(&path).is_ok() {
            Ok(Some(Self { repo_path: path }))
        } else {
            // Not a git repo, checkpoints disabled for now or we could `git init` automatically.
            Ok(None)
        }
    }

    /// Create a manual or automatic checkpoint by staging changes and creating a commit
    pub fn create_checkpoint(&self, message: &str) -> Result<String> {
        let repo = Repository::discover(&self.repo_path)?;
        
        // Use a generic ARC signature
        let sig = Signature::now("ARC Agent", "arc@arc-code.dev")?;
        
        let mut index = repo.index()?;
        
        // Add all changed files to index (excluding tracked ignores)
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(|e| ArcError::Checkpoint(format!("Failed to add files to index: {e}")))?;
        
        index.write()?;
        let oid = index.write_tree()?;
        let tree = repo.find_tree(oid)?;

        let parent = match repo.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(_) => None,
        };

        let mut parents = Vec::new();
        if let Some(ref p) = parent {
            parents.push(p);
        }

        let commit_id = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        )?;

        Ok(commit_id.to_string())
    }

    /// Rewind to a specific checkpoint (commit hash)
    pub fn rewind(&self, commit_hash: &str) -> Result<()> {
        let repo = Repository::discover(&self.repo_path)?;
        let oid = git2::Oid::from_str(commit_hash)
            .map_err(|e| ArcError::Checkpoint(format!("Invalid commit hash: {e}")))?;
        
        let commit = repo.find_commit(oid)
            .map_err(|e| ArcError::Checkpoint(format!("Commit not found: {e}")))?;
        
        let tree = commit.tree()?;
        
        repo.checkout_tree(tree.as_object(), None)
            .map_err(|e| ArcError::Checkpoint(format!("Failed to checkout tree: {e}")))?;
            
        // Move HEAD to the checkpoint commit
        repo.set_head_detached(oid)
            .map_err(|e| ArcError::Checkpoint(format!("Failed to detach HEAD: {e}")))?;

        Ok(())
    }
}
