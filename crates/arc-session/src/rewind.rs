use crate::session_db::SessionDatabase;
use crate::session_model::{CheckpointId, SessionState};
use anyhow::{Context, Result};
use std::collections::HashSet;
use tokio::fs;

/// Manages reversing the state of a session back to a previous checkpoint.
pub struct RewindManager {
    db: SessionDatabase,
}

impl RewindManager {
    pub fn new(db: SessionDatabase) -> Self {
        Self { db }
    }

    /// Rewind the session to a specific checkpoint ID.
    /// This restores the conversation history and memory precisely to that point.
    /// It optionally reverts file modifications made AFTER the target checkpoint.
    pub async fn rewind(
        &self,
        session_id: uuid::Uuid,
        target_checkpoint_id: CheckpointId,
        revert_files: bool,
    ) -> Result<SessionState> {
        let mut session = self
            .db
            .load_session(session_id)?
            .context("Session not found")?;

        // 1. Find the target checkpoint
        let target_idx = session
            .checkpoints
            .iter()
            .position(|c| c.id == target_checkpoint_id)
            .context("Checkpoint not found in this session")?;

        let target = &session.checkpoints[target_idx];
        let target_turn_index = target.turn_index;
        let target_timestamp = target.created_at;

        // 2. Identify files modified *after* this checkpoint
        if revert_files {
            let files_to_revert: Vec<_> = session
                .modified_files
                .iter()
                .filter(|m| m.modified_at > target_timestamp)
                .cloned()
                .collect();

            self.revert_filesystem_changes(&files_to_revert).await?;

            // Remove reverted files from the record
            session
                .modified_files
                .retain(|m| m.modified_at <= target_timestamp);
        }

        // 3. Truncate conversation history
        if session.conversation.len() > target_turn_index {
            session.conversation.truncate(target_turn_index);
        }

        // 4. Truncate checkpoint history
        session.checkpoints.truncate(target_idx + 1);

        // 5. Save the updated (rewound) state back to the DB
        self.db.save_session(&session)?;

        Ok(session)
    }

    /// Revert physical files on disk based on the modification records.
    async fn revert_filesystem_changes(
        &self,
        changes: &[crate::session_model::FileModificationRecord],
    ) -> Result<()> {
        // Since a file might be modified multiple times, we only want to apply
        // the *oldest* original content we have in the "future" slice we're deleting.
        let mut resolved_reverts = std::collections::HashMap::new();

        // Sort chronologically ascending so the first entry we see for a path is the oldest
        let mut sorted_changes = changes.to_vec();
        sorted_changes.sort_by_key(|c| c.modified_at);

        for change in sorted_changes {
            resolved_reverts
                .entry(change.path.clone())
                .or_insert(change);
        }

        for (_, change) in resolved_reverts {
            match change.action {
                crate::session_model::FileAction::Created => {
                    // It was created after the checkpoint, so we delete it
                    if let Err(e) = fs::remove_file(&change.path).await {
                        tracing::warn!(
                            "Rewind: Failed to delete created file {}: {}",
                            change.path,
                            e
                        );
                    } else {
                        tracing::info!("Rewind: Deleted file {}", change.path);
                    }
                },
                crate::session_model::FileAction::Modified
                | crate::session_model::FileAction::Deleted => {
                    // Restore original content
                    if let Some(content) = &change.original_content {
                        if let Err(e) = fs::write(&change.path, content).await {
                            tracing::warn!("Rewind: Failed to restore file {}: {}", change.path, e);
                        } else {
                            tracing::info!("Rewind: Restored file {}", change.path);
                        }
                    } else {
                        tracing::warn!(
                            "Rewind: No original content available to restore {}",
                            change.path
                        );
                    }
                },
            }
        }

        Ok(())
    }

    /// Get a diff of what would change if we rewound to this checkpoint.
    pub fn preview_rewind(
        &self,
        session: &SessionState,
        target_checkpoint_id: CheckpointId,
    ) -> Result<RewindPreview> {
        let target = session
            .checkpoints
            .iter()
            .find(|c| c.id == target_checkpoint_id)
            .context("Checkpoint not found")?;

        let turns_to_lose = session.conversation.len().saturating_sub(target.turn_index);

        let files_to_revert: HashSet<_> = session
            .modified_files
            .iter()
            .filter(|m| m.modified_at > target.created_at)
            .map(|m| m.path.clone())
            .collect();

        Ok(RewindPreview {
            turns_lost: turns_to_lose,
            files_affected: files_to_revert.into_iter().collect(),
        })
    }
}

pub struct RewindPreview {
    pub turns_lost: usize,
    pub files_affected: Vec<String>,
}
