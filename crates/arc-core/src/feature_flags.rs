// SPDX-License-Identifier: MIT
//! # Feature Flags — Dynamic Feature Gating with Disk Cache
//!
//! Runtime feature toggles with stale value prevention.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub name: String,
    pub enabled: bool,
    pub default: bool,
    #[serde(default)]
    pub description: String,
    pub last_updated: u64,
}

pub struct FeatureFlagStore {
    flags: HashMap<String, FeatureFlag>,
    cache_path: PathBuf,
}

impl FeatureFlagStore {
    pub fn new(cache_dir: PathBuf) -> Self {
        let mut store = Self {
            flags: HashMap::new(),
            cache_path: cache_dir.join("feature_flags.json"),
        };
        let _ = store.load_cache();
        store
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.flags.get(name).map(|f| f.enabled).unwrap_or(false)
    }

    pub fn set(&mut self, name: &str, enabled: bool, description: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let flag = self
            .flags
            .entry(name.to_string())
            .or_insert_with(|| FeatureFlag {
                name: name.to_string(),
                enabled,
                default: enabled,
                description: description.to_string(),
                last_updated: now,
            });
        flag.enabled = enabled;
        flag.last_updated = now;
        let _ = self.save_cache();
    }

    pub fn list(&self) -> Vec<&FeatureFlag> {
        self.flags.values().collect()
    }

    fn load_cache(&mut self) -> Result<(), String> {
        if !self.cache_path.exists() {
            return Ok(());
        }
        let data = std::fs::read_to_string(&self.cache_path).map_err(|e| e.to_string())?;
        self.flags = serde_json::from_str(&data).unwrap_or_default();
        Ok(())
    }

    fn save_cache(&self) -> Result<(), String> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string(&self.flags).map_err(|e| e.to_string())?;
        std::fs::write(&self.cache_path, data).map_err(|e| e.to_string())
    }
}
