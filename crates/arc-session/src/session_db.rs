use crate::session_model::{SessionMetadata, SessionState};
use anyhow::Result;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use uuid::Uuid;

const SESSIONS_TABLE: TableDefinition<[u8; 16], &[u8]> = TableDefinition::new("sessions");
const METADATA_TABLE: TableDefinition<[u8; 16], &[u8]> = TableDefinition::new("session_metadata");

/// A fast, embedded database for storing and retrieving session state using redb.
pub struct SessionDatabase {
    db: Database,
    db_path: PathBuf,
}

impl SessionDatabase {
    /// Open or create the session database at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(&db_path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(SESSIONS_TABLE)?;
            write_txn.open_table(METADATA_TABLE)?;
        }
        write_txn.commit()?;

        debug!("Session database opened at {}", db_path.display());

        Ok(Self { db, db_path })
    }

    /// Save a complete session state to the database.
    pub fn save_session(&self, state: &SessionState) -> Result<()> {
        let session_bytes = bincode::serialize(state)?;
        let metadata_bytes = bincode::serialize(&state.metadata())?;

        let session_id_bytes = state.session_id.into_bytes();

        let write_txn = self.db.begin_write()?;
        {
            let mut sessions = write_txn.open_table(SESSIONS_TABLE)?;
            sessions.insert(session_id_bytes, session_bytes.as_slice())?;

            let mut metadata = write_txn.open_table(METADATA_TABLE)?;
            metadata.insert(session_id_bytes, metadata_bytes.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Load a complete session state by ID.
    pub fn load_session(&self, session_id: Uuid) -> Result<Option<SessionState>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(SESSIONS_TABLE)?;

        let id_bytes = session_id.into_bytes();
        let value = table.get(id_bytes)?;

        if let Some(guard) = value {
            let state: SessionState = bincode::deserialize(guard.value())?;
            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Delete a session and its metadata.
    pub fn delete_session(&self, session_id: Uuid) -> Result<bool> {
        let id_bytes = session_id.into_bytes();
        let write_txn = self.db.begin_write()?;
        let mut was_deleted = false;

        {
            let mut sessions = write_txn.open_table(SESSIONS_TABLE)?;
            if sessions.remove(id_bytes)?.is_some() {
                was_deleted = true;
            }

            let mut metadata = write_txn.open_table(METADATA_TABLE)?;
            metadata.remove(id_bytes)?;
        }
        write_txn.commit()?;

        Ok(was_deleted)
    }

    /// List all sessions, ordering by most recently updated.
    pub fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(METADATA_TABLE)?;

        let mut results = Vec::new();

        for result in table.iter()? {
            let (_, value) = result?;
            let metadata: SessionMetadata = bincode::deserialize(value.value())?;
            results.push(metadata);
        }

        // Sort descending by updated_at
        results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(results)
    }

    /// Returns the database file size in bytes.
    pub fn database_size(&self) -> std::io::Result<u64> {
        let metadata = std::fs::metadata(&self.db_path)?;
        Ok(metadata.len())
    }

    /// Perform maintenance (compaction) to reduce file size.
    pub fn compact(&self) -> Result<()> {
        info!("Compacting session database to reclaim space...");
        self.db.compact()?;
        Ok(())
    }
}
