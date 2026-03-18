//! Session Management — persistent storage of full conversation histories.

use crate::error::{ArcError, ArcResult};
use crate::memory::working::MemoryMessage;
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

const SESSIONS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub summary: String,
    pub messages: Vec<MemoryMessage>,
    #[serde(default)]
    pub total_input_tokens: u64,
    #[serde(default)]
    pub total_output_tokens: u64,
    #[serde(default)]
    pub total_cost_usd: f64,
}

pub struct SessionStore {
    db: Arc<Database>,
}

impl SessionStore {
    pub fn new(profile_dir: PathBuf) -> ArcResult<Self> {
        let db_path = profile_dir.join("arc_sessions.redb");
        debug!("Initializing Session Store at {:?}", db_path);

        let db = Database::create(db_path).map_err(|e| ArcError::Database(e.to_string()))?;

        let write_txn = db.begin_write().map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let _ = write_txn
                .open_table(SESSIONS_TABLE)
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn.commit().map_err(|e| ArcError::Database(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Save a session to disk.
    pub fn save_session(&self, record: &SessionRecord) -> ArcResult<()> {
        let bytes = serde_json::to_vec(record).map_err(|e| ArcError::System(format!("Serialization error: {}", e.to_string())))?;

        let write_txn = self.db.begin_write().map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(SESSIONS_TABLE)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            table
                .insert(record.id.as_str(), bytes.as_slice())
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn.commit().map_err(|e| ArcError::Database(e.to_string()))?;

        info!("Saved session {} with {} messages", record.id, record.messages.len());
        Ok(())
    }

    /// Load a session by ID.
    pub fn load_session(&self, id: &str) -> ArcResult<Option<SessionRecord>> {
        let read_txn = self.db.begin_read().map_err(|e| ArcError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(SESSIONS_TABLE)
            .map_err(|e| ArcError::Database(e.to_string()))?;

        if let Some(guard) = table.get(id).map_err(|e| ArcError::Database(e.to_string()))? {
            let record: SessionRecord =
                serde_json::from_slice(guard.value()).map_err(|e| ArcError::System(format!("Serialization error: {}", e.to_string())))?;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// List all saved sessions metadata (omits full message history for speed).
    pub fn list_sessions(&self) -> ArcResult<Vec<SessionMetadata>> {
        let read_txn = self.db.begin_read().map_err(|e| ArcError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(SESSIONS_TABLE)
            .map_err(|e| ArcError::Database(e.to_string()))?;

        let mut sessions = Vec::new();
        let iter = table.iter().map_err(|e| ArcError::Database(e.to_string()))?;

        for result in iter {
            let (_, v) = result.map_err(|e| ArcError::Database(e.to_string()))?;
            // Partially deserialize just to get metadata fields fast
            if let Ok(record) = serde_json::from_slice::<SessionRecord>(v.value()) {
                sessions.push(SessionMetadata {
                    id: record.id,
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                    summary: record.summary,
                    message_count: record.messages.len(),
                    total_cost_usd: record.total_cost_usd,
                });
            }
        }

        // Sort by newest first
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    /// Delete a session.
    pub fn delete_session(&self, id: &str) -> ArcResult<()> {
        let write_txn = self.db.begin_write().map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(SESSIONS_TABLE)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            table.remove(id).map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn.commit().map_err(|e| ArcError::Database(e.to_string()))?;
        Ok(())
    }
}

pub struct SessionMetadata {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub summary: String,
    pub message_count: usize,
    pub total_cost_usd: f64,
}
