use crate::session_db::SessionDatabase;
use crate::session_model::{CheckpointId, CheckpointMetadata, SessionState};
use anyhow::{Context, Result};
use chrono::Utc;
use std::time::Instant;
use tracing::{debug, info};
use uuid::Uuid;

/// Configuration for the checkpointing system.
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Save a checkpoint after every N conversation turns
    pub auto_checkpoint_turns: usize,
    /// Keep only the last N checkpoints (0 = unlimited)
    pub max_checkpoints_per_session: usize,
    /// Compress checkpoints before saving
    pub compress_checkpoints: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            auto_checkpoint_turns: 5,
            max_checkpoints_per_session: 20,
            compress_checkpoints: true, // bincode + zstd used under the hood
        }
    }
}

/// Manages creating, saving, and loading checkpoints.
pub struct CheckpointManager {
    db: SessionDatabase,
    config: CheckpointConfig,
}

impl CheckpointManager {
    pub fn new(db: SessionDatabase, config: CheckpointConfig) -> Self {
        Self { db, config }
    }

    /// Open a session by ID. Throws an error if not found.
    pub fn load_session(&self, session_id: Uuid) -> Result<SessionState> {
        let start = Instant::now();
        let session = self
            .db
            .load_session(session_id)?
            .context("Session not found")?;

        debug!("Loaded session {} in {:?}", session_id, start.elapsed());
        Ok(session)
    }

    /// Automatically create a checkpoint if the criteria are met
    /// (e.g. `n` turns have passed since the last checkpoint).
    pub fn conditionally_checkpoint(
        &self,
        state: &mut SessionState,
        force: bool,
        description: impl Into<String>,
    ) -> Result<Option<CheckpointId>> {
        let turns_since_last = state
            .conversation
            .iter()
            .rev()
            .take_while(|t| t.checkpoint_id.is_none())
            .count();

        if force || turns_since_last >= self.config.auto_checkpoint_turns {
            return Ok(Some(self.create_checkpoint(state, description)?));
        }

        Ok(None)
    }

    /// Create an explicit checkpoint of the current state.
    pub fn create_checkpoint(
        &self,
        state: &mut SessionState,
        description: impl Into<String>,
    ) -> Result<CheckpointId> {
        let checkpoint_id = Uuid::new_v4();
        state.updated_at = Utc::now();

        // Mark the last turn with this checkpoint ID
        if let Some(last_turn) = state.conversation.last_mut() {
            last_turn.checkpoint_id = Some(checkpoint_id);
        }

        // Calculate approximate size (in-memory)
        let state_bytes = bincode::serialized_size(&*state).unwrap_or(0);

        let metadata = CheckpointMetadata {
            id: checkpoint_id,
            turn_index: state.conversation.len(),
            created_at: Utc::now(),
            description: description.into(),
            token_count: state.total_input_tokens + state.total_output_tokens,
            size_bytes: state_bytes,
        };

        state.checkpoints.push(metadata);

        // Enforce max checkpoint limit (sliding window)
        if self.config.max_checkpoints_per_session > 0
            && state.checkpoints.len() > self.config.max_checkpoints_per_session
        {
            // Keep the first (initial) and the last N
            let to_remove = state.checkpoints.len() - self.config.max_checkpoints_per_session;
            // Remove items slightly newer than the very first one, shifting the list down
            for _ in 0..to_remove {
                if state.checkpoints.len() > 2 {
                    state.checkpoints.remove(1);
                }
            }
        }

        // Save to DB
        let start = Instant::now();
        self.db.save_session(state)?;
        
        info!(
            "Session '{}' checkpointed. ID: {}, Size: {:.2} MB, Time: {:?}",
            state.session_id,
            checkpoint_id,
            state_bytes as f64 / 1_048_576.0,
            start.elapsed()
        );

        Ok(checkpoint_id)
    }
}
