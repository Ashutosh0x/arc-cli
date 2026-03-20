//! Long-Term Memory — persistent storage across sessions.
//! Uses Redb for fast, safe embedded key-value storage.

use crate::error::{ArcError, ArcResult};
use crate::memory::MemoryConfig;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

// Table Definitions
const AGENT_FACTS: TableDefinition<&str, &str> = TableDefinition::new("agent_facts");
const USER_PREFS: TableDefinition<&str, &str> = TableDefinition::new("user_prefs");
const OBSERVATION_EMBEDDINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("obs_embeddings");

pub struct LongTermMemory {
    db: Arc<Database>,
}

impl LongTermMemory {
    pub fn new(config: &MemoryConfig, profile_dir: PathBuf) -> ArcResult<Self> {
        if !config.persistence_enabled {
            return Err(ArcError::System(
                "Long-term memory persistence is disabled in config".into(),
            ));
        }

        let db_path = profile_dir.join("arc_memory.redb");
        debug!("Initializing Long-Term Memory database at {:?}", db_path);

        let db = Database::create(db_path).map_err(|e| ArcError::Database(e.to_string()))?;

        // Initialize tables
        let write_txn = db
            .begin_write()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let _ = write_txn
                .open_table(AGENT_FACTS)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            let _ = write_txn
                .open_table(USER_PREFS)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            let _ = write_txn
                .open_table(OBSERVATION_EMBEDDINGS)
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| ArcError::Database(e.to_string()))?;

        info!("Long-term memory subsystem initialized");
        Ok(Self { db: Arc::new(db) })
    }

    /// Store an agent fact.
    pub fn store_fact(&self, key: &str, fact: &str) -> ArcResult<()> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(AGENT_FACTS)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            table
                .insert(key, fact)
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        Ok(())
    }

    /// Retrieve an agent fact by exact key.
    pub fn get_fact(&self, key: &str) -> ArcResult<Option<String>> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(AGENT_FACTS)
            .map_err(|e| ArcError::Database(e.to_string()))?;
        let result = table
            .get(key)
            .map_err(|e| ArcError::Database(e.to_string()))?;
        Ok(result.map(|v| v.value().to_string()))
    }

    /// Retrieve all facts for dumping into working memory.
    pub fn get_all_facts(&self) -> ArcResult<Vec<(String, String)>> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        let table = read_txn
            .open_table(AGENT_FACTS)
            .map_err(|e| ArcError::Database(e.to_string()))?;

        let mut facts = Vec::new();
        let iter = table
            .iter()
            .map_err(|e| ArcError::Database(e.to_string()))?;

        for result in iter {
            let (k, v) = result.map_err(|e| ArcError::Database(e.to_string()))?;
            facts.push((k.value().to_string(), v.value().to_string()));
        }

        Ok(facts)
    }

    /// Store a user preference.
    pub fn store_preference(&self, key: &str, pref: &str) -> ArcResult<()> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(USER_PREFS)
                .map_err(|e| ArcError::Database(e.to_string()))?;
            table
                .insert(key, pref)
                .map_err(|e| ArcError::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| ArcError::Database(e.to_string()))?;
        Ok(())
    }
}
