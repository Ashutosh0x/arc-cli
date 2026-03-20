//! # Settings Hierarchy — User → Project → Managed + Hot-Reload
//!
//! Layered settings with enterprise managed override support.
//! macOS plist and Windows Registry integration stubs.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SettingsLayer { User, Project, Managed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredSettings {
    layers: Vec<(SettingsLayer, serde_json::Value)>,
    #[serde(skip)]
    watch_paths: Vec<PathBuf>,
}

impl LayeredSettings {
    pub fn new() -> Self { Self { layers: Vec::new(), watch_paths: Vec::new() } }

    pub fn load_layer(&mut self, layer: SettingsLayer, path: &Path) -> Result<(), String> {
        if !path.exists() { return Ok(()); }
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        self.layers.retain(|(l, _)| *l != layer);
        self.layers.push((layer, value));
        self.layers.sort_by_key(|(l, _)| *l);
        self.watch_paths.push(path.to_path_buf());
        Ok(())
    }

    /// Get a value with layer precedence (Managed > Project > User).
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        for (_, val) in self.layers.iter().rev() {
            if let Some(v) = val.get(key) { return Some(v); }
        }
        None
    }

    pub fn get_str(&self, key: &str) -> Option<&str> { self.get(key)?.as_str() }
    pub fn get_bool(&self, key: &str) -> Option<bool> { self.get(key)?.as_bool() }

    /// Load platform-specific managed settings.
    pub fn load_managed_platform(&mut self) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        { self.load_windows_registry()?; }
        #[cfg(target_os = "macos")]
        { self.load_macos_plist()?; }
        // Linux: /etc/arc-cli/managed-settings.json
        #[cfg(target_os = "linux")]
        {
            let path = Path::new("/etc/arc-cli/managed-settings.json");
            if path.exists() { self.load_layer(SettingsLayer::Managed, path)?; }
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn load_windows_registry(&mut self) -> Result<(), String> {
        // Read from HKLM\SOFTWARE\ArcCli\Settings.
        // Stub: would use winreg crate in production.
        let managed_path = PathBuf::from(r"C:\Program Files\ArcCli\managed-settings.json");
        if managed_path.exists() { self.load_layer(SettingsLayer::Managed, &managed_path)?; }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn load_macos_plist(&mut self) -> Result<(), String> {
        // Read from /Library/Managed Preferences/com.arc-cli.plist.
        // Stub: would use plist crate in production.
        let managed_path = PathBuf::from("/Library/Application Support/ArcCli/managed-settings.json");
        if managed_path.exists() { self.load_layer(SettingsLayer::Managed, &managed_path)?; }
        Ok(())
    }
}

impl Default for LayeredSettings { fn default() -> Self { Self::new() } }

/// Enterprise managed settings keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedSettingsConfig {
    #[serde(default)]
    pub allow_managed_permission_rules_only: bool,
    #[serde(default)]
    pub allow_managed_hooks_only: bool,
    #[serde(default)]
    pub strict_known_marketplaces: Vec<String>,
    #[serde(default)]
    pub disable_bypass_permissions_mode: Option<String>,
    #[serde(default)]
    pub plugin_trust_message: Option<String>,
    #[serde(default)]
    pub feedback_survey_rate: Option<f64>,
}
