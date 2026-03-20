//! # Auto-Memory — /memory Command with Auto-Save + Timestamps
//!
//! Persists useful context across sessions. Auto-detects important facts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub content: String,
    pub source: MemorySource,
    pub created_at: u64,
    pub last_modified: u64,
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemorySource {
    Auto,
    User,
    Session,
}

pub struct MemoryStore {
    entries: HashMap<String, MemoryEntry>,
    directory: PathBuf,
}

impl MemoryStore {
    pub fn new(dir: PathBuf) -> Self {
        let mut store = Self {
            entries: HashMap::new(),
            directory: dir,
        };
        let _ = store.load();
        store
    }

    pub fn save_entry(&mut self, key: &str, content: &str, source: MemorySource) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = self
            .entries
            .entry(key.to_string())
            .or_insert_with(|| MemoryEntry {
                key: key.to_string(),
                content: String::new(),
                source: source.clone(),
                created_at: now,
                last_modified: now,
                access_count: 0,
            });
        entry.content = content.to_string();
        entry.last_modified = now;
        entry.access_count += 1;
        let _ = self.persist();
    }

    pub fn get(&self, key: &str) -> Option<&MemoryEntry> {
        self.entries.get(key)
    }

    pub fn all(&self) -> Vec<&MemoryEntry> {
        let mut entries: Vec<_> = self.entries.values().collect();
        entries.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        entries
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            let _ = self.persist();
        }
        removed
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = self.persist();
    }

    fn load(&mut self) -> Result<(), String> {
        let file = self.directory.join("memory.json");
        if !file.exists() {
            return Ok(());
        }
        let data = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        self.entries = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn persist(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        let data = serde_json::to_string_pretty(&self.entries).map_err(|e| e.to_string())?;
        std::fs::write(self.directory.join("memory.json"), data).map_err(|e| e.to_string())
    }
}
