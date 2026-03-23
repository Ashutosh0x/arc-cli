// SPDX-License-Identifier: MIT
//! Local plugin registry: tracks installed plugins, their state, and configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::LoadedPlugin;

/// Registry of all installed plugins, persisted to .arc/plugins.toml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginRegistry {
    pub plugins: HashMap<String, PluginEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Plugin name.
    pub name: String,

    /// Installed version.
    pub version: String,

    /// Where the plugin is installed on disk.
    pub install_path: String,

    /// Source from where it was installed.
    pub source: PluginSource,

    /// SHA-256 hash of the plugin directory at install time.
    pub integrity_hash: String,

    /// Whether this plugin is currently enabled.
    pub enabled: bool,

    /// When it was installed.
    pub installed_at: DateTime<Utc>,

    /// When it was last updated.
    pub updated_at: DateTime<Utc>,

    /// User-provided configuration overrides.
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginSource {
    /// Installed from the official marketplace.
    #[serde(rename = "marketplace")]
    Marketplace { registry_url: String },

    /// Installed from a git repository.
    #[serde(rename = "git")]
    Git { url: String, branch: Option<String> },

    /// Installed from a local directory.
    #[serde(rename = "local")]
    Local { path: String },
}

impl PluginRegistry {
    /// Load the registry from disk.
    pub fn load(project_root: &Path) -> Self {
        let path = Self::registry_path(project_root);
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save the registry to disk.
    pub fn save(&self, project_root: &Path) -> Result<(), std::io::Error> {
        let path = Self::registry_path(project_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(path, content)
    }

    fn registry_path(project_root: &Path) -> PathBuf {
        project_root.join(".arc").join("plugins.toml")
    }

    /// Register a newly installed plugin.
    pub fn register(&mut self, plugin: &LoadedPlugin, source: PluginSource) {
        let entry = PluginEntry {
            name: plugin.manifest.plugin.name.clone(),
            version: plugin.manifest.plugin.version.clone(),
            install_path: plugin.install_path.display().to_string(),
            source,
            integrity_hash: plugin.integrity_hash.clone(),
            enabled: true,
            installed_at: Utc::now(),
            updated_at: Utc::now(),
            config: HashMap::new(),
        };
        self.plugins.insert(entry.name.clone(), entry);
    }

    /// Uninstall a plugin by name.
    pub fn uninstall(&mut self, name: &str) -> Option<PluginEntry> {
        self.plugins.remove(name)
    }

    /// Enable/disable a plugin.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(entry) = self.plugins.get_mut(name) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// List all installed plugins.
    pub fn list(&self) -> Vec<&PluginEntry> {
        let mut entries: Vec<_> = self.plugins.values().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }

    /// Check if a plugin is installed and enabled.
    pub fn is_active(&self, name: &str) -> bool {
        self.plugins
            .get(name)
            .map_or(false, |e| e.enabled)
    }
}
