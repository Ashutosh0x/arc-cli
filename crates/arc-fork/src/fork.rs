//! Session forking: create divergent conversation branches.
//!
//! This is the conversational equivalent of a git branch.
//! /fork creates a snapshot, then starts a new session from that point.
//! /resume lists all forks and lets you switch between them.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use crate::snapshot::SessionSnapshot;

/// A fork point in the conversation tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fork {
    pub id: Uuid,
    /// The snapshot at the fork point.
    pub snapshot_id: Uuid,
    /// Human-readable label.
    pub label: String,
    /// When the fork was created.
    pub created_at: DateTime<Utc>,
    /// The parent fork (None for the root session).
    pub parent_fork_id: Option<Uuid>,
    /// Child sessions that diverged from this fork.
    pub children: Vec<ForkChild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkChild {
    pub session_id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    /// Brief description of what this branch explored.
    pub description: String,
    /// Whether this branch is currently active.
    pub active: bool,
}

/// Manages the fork tree for a session.
pub struct ForkManager {
    /// All forks, keyed by fork ID.
    forks: HashMap<Uuid, Fork>,
    /// All snapshots, keyed by snapshot ID.
    snapshots: HashMap<Uuid, SessionSnapshot>,
    /// The current active session ID.
    current_session_id: Uuid,
    /// Map from session_id to its chain of snapshots.
    session_snapshots: HashMap<Uuid, Vec<Uuid>>,
}

impl ForkManager {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            forks: HashMap::new(),
            snapshots: HashMap::new(),
            current_session_id: session_id,
            session_snapshots: HashMap::new(),
        }
    }

    /// Create a fork at the current conversation state.
    ///
    /// This is the `/fork` command implementation.
    pub fn create_fork(
        &mut self,
        label: &str,
        snapshot: SessionSnapshot,
    ) -> ForkResult {
        let snapshot_id = snapshot.id;
        let fork_id = Uuid::new_v4();

        // Store the snapshot
        self.snapshots.insert(snapshot_id, snapshot);

        // Create the fork point
        let fork = Fork {
            id: fork_id,
            snapshot_id,
            label: label.to_string(),
            created_at: Utc::now(),
            parent_fork_id: None,
            children: vec![],
        };

        self.forks.insert(fork_id, fork);

        info!(
            fork_id = %fork_id,
            label = %label,
            snapshot = %snapshot_id,
            "Created fork point"
        );

        ForkResult {
            fork_id,
            snapshot_id,
        }
    }

    /// Resume from a fork point, creating a new divergent session.
    ///
    /// This is the `/resume` command implementation.
    pub fn resume_from_fork(
        &mut self,
        fork_id: Uuid,
        branch_label: &str,
        branch_description: &str,
    ) -> Result<ResumeResult, ForkError> {
        let fork = self
            .forks
            .get_mut(&fork_id)
            .ok_or(ForkError::ForkNotFound(fork_id))?;

        let snapshot = self
            .snapshots
            .get(&fork.snapshot_id)
            .ok_or(ForkError::SnapshotNotFound(fork.snapshot_id))?
            .clone();

        let new_session_id = Uuid::new_v4();

        // Mark all other children as inactive
        for child in &mut fork.children {
            child.active = false;
        }

        // Add new child session
        fork.children.push(ForkChild {
            session_id: new_session_id,
            label: branch_label.to_string(),
            created_at: Utc::now(),
            description: branch_description.to_string(),
            active: true,
        });

        self.current_session_id = new_session_id;

        info!(
            fork_id = %fork_id,
            new_session = %new_session_id,
            label = %branch_label,
            "Resumed from fork"
        );

        Ok(ResumeResult {
            new_session_id,
            snapshot,
        })
    }

    /// List all fork points with their branches.
    pub fn list_forks(&self) -> Vec<ForkSummary> {
        self.forks
            .values()
            .map(|fork| {
                let snapshot = self.snapshots.get(&fork.snapshot_id);
                ForkSummary {
                    fork_id: fork.id,
                    label: fork.label.clone(),
                    created_at: fork.created_at,
                    turn_number: snapshot.map(|s| s.turn_number).unwrap_or(0),
                    branch_count: fork.children.len(),
                    children: fork.children.clone(),
                }
            })
            .collect()
    }

    /// Get the fork tree as a displayable structure.
    pub fn fork_tree(&self) -> Vec<ForkTreeNode> {
        let mut roots: Vec<_> = self
            .forks
            .values()
            .filter(|f| f.parent_fork_id.is_none())
            .map(|f| self.build_tree_node(f))
            .collect();

        roots.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        roots
    }

    fn build_tree_node(&self, fork: &Fork) -> ForkTreeNode {
        let children: Vec<_> = self
            .forks
            .values()
            .filter(|f| f.parent_fork_id == Some(fork.id))
            .map(|f| self.build_tree_node(f))
            .collect();

        ForkTreeNode {
            fork_id: fork.id,
            label: fork.label.clone(),
            created_at: fork.created_at,
            branches: fork.children.clone(),
            child_forks: children,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForkResult {
    pub fork_id: Uuid,
    pub snapshot_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ResumeResult {
    pub new_session_id: Uuid,
    pub snapshot: SessionSnapshot,
}

#[derive(Debug, Clone)]
pub struct ForkSummary {
    pub fork_id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub turn_number: u32,
    pub branch_count: usize,
    pub children: Vec<ForkChild>,
}

#[derive(Debug, Clone)]
pub struct ForkTreeNode {
    pub fork_id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub branches: Vec<ForkChild>,
    pub child_forks: Vec<ForkTreeNode>,
}

#[derive(Debug, thiserror::Error)]
pub enum ForkError {
    #[error("Fork {0} not found")]
    ForkNotFound(Uuid),

    #[error("Snapshot {0} not found")]
    SnapshotNotFound(Uuid),
}
