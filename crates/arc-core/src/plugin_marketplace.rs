//! # Plugin Marketplace — Git/Local Install, Manifests, Trust
//!
//! Install from git URLs, local dirs. `plugin.json` manifests.
//! Managed trust model with version pinning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
    #[serde(default)]
    pub min_arc_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginSource {
    Git,
    Local,
    Registry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    Untrusted,
    UserTrusted,
    ManagedTrusted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub source: PluginSource,
    pub install_path: PathBuf,
    pub origin_url: Option<String>,
    pub pinned_version: Option<String>,
    pub trust: TrustLevel,
    pub enabled: bool,
    pub installed_at: u64,
}

pub struct PluginMarketplace {
    plugins: HashMap<String, InstalledPlugin>,
    plugins_dir: PathBuf,
    known_marketplaces: Vec<String>,
}

impl PluginMarketplace {
    pub fn new(plugins_dir: PathBuf) -> Self {
        let mut mp = Self {
            plugins: HashMap::new(),
            plugins_dir,
            known_marketplaces: vec!["https://github.com/arc-cli-plugins".into()],
        };
        let _ = mp.load_installed();
        mp
    }

    /// Install plugin from a git URL.
    pub fn install_git(&mut self, url: &str, pin: Option<&str>) -> Result<String, String> {
        let name = url
            .rsplit('/')
            .next()
            .unwrap_or("plugin")
            .trim_end_matches(".git")
            .to_string();
        let dest = self.plugins_dir.join(&name);
        std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

        let mut args = vec!["clone", "--depth", "1"];
        if let Some(tag) = pin {
            args.extend(["--branch", tag]);
        }
        args.push(url);
        args.push(dest.to_str().unwrap_or_default());

        let output = std::process::Command::new("git")
            .args(&args)
            .output()
            .map_err(|e| format!("git clone failed: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "git clone error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let manifest = Self::load_manifest(&dest)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.plugins.insert(
            name.clone(),
            InstalledPlugin {
                manifest,
                source: PluginSource::Git,
                install_path: dest,
                origin_url: Some(url.to_string()),
                pinned_version: pin.map(|s| s.to_string()),
                trust: TrustLevel::Untrusted,
                enabled: true,
                installed_at: now,
            },
        );
        self.save_installed()?;
        Ok(name)
    }

    /// Install plugin from a local directory (symlink).
    pub fn install_local(&mut self, path: &Path) -> Result<String, String> {
        let manifest = Self::load_manifest(path)?;
        let name = manifest.name.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.plugins.insert(
            name.clone(),
            InstalledPlugin {
                manifest,
                source: PluginSource::Local,
                install_path: path.to_path_buf(),
                origin_url: None,
                pinned_version: None,
                trust: TrustLevel::UserTrusted,
                enabled: true,
                installed_at: now,
            },
        );
        self.save_installed()?;
        Ok(name)
    }

    pub fn uninstall(&mut self, name: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.remove(name) {
            if plugin.source == PluginSource::Git {
                let _ = std::fs::remove_dir_all(&plugin.install_path);
            }
            self.save_installed()?;
        }
        Ok(())
    }

    pub fn enable(&mut self, name: &str) -> bool {
        if let Some(p) = self.plugins.get_mut(name) {
            p.enabled = true;
            let _ = self.save_installed();
            true
        } else {
            false
        }
    }

    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(p) = self.plugins.get_mut(name) {
            p.enabled = false;
            let _ = self.save_installed();
            true
        } else {
            false
        }
    }

    pub fn set_trust(&mut self, name: &str, trust: TrustLevel) {
        if let Some(p) = self.plugins.get_mut(name) {
            p.trust = trust;
            let _ = self.save_installed();
        }
    }

    pub fn validate(&self, name: &str) -> Result<Vec<String>, String> {
        let plugin = self.plugins.get(name).ok_or("Plugin not found")?;
        let mut issues = Vec::new();
        if plugin.manifest.name.is_empty() {
            issues.push("Missing name in manifest".into());
        }
        if plugin.manifest.version.is_empty() {
            issues.push("Missing version".into());
        }
        if !plugin.install_path.exists() {
            issues.push("Install path missing".into());
        }
        Ok(issues)
    }

    /// Update a git-based plugin by pulling latest.
    pub fn update(&mut self, name: &str) -> Result<(), String> {
        let plugin = self.plugins.get(name).ok_or("Plugin not found")?;
        if plugin.source != PluginSource::Git {
            return Err("Can only update git plugins".into());
        }
        let output = std::process::Command::new("git")
            .args(["pull", "--rebase"])
            .current_dir(&plugin.install_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        // Reload manifest.
        if let Ok(m) = Self::load_manifest(&plugin.install_path) {
            if let Some(p) = self.plugins.get_mut(name) {
                p.manifest = m;
            }
        }
        self.save_installed()
    }

    pub fn list(&self) -> Vec<&InstalledPlugin> {
        self.plugins.values().collect()
    }
    pub fn get(&self, name: &str) -> Option<&InstalledPlugin> {
        self.plugins.get(name)
    }

    fn load_manifest(dir: &Path) -> Result<PluginManifest, String> {
        let manifest_path = dir.join("plugin.json");
        if !manifest_path.exists() {
            return Err("No plugin.json found".into());
        }
        let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| format!("Invalid plugin.json: {e}"))
    }

    fn load_installed(&mut self) -> Result<(), String> {
        let state_file = self.plugins_dir.join("plugins-state.json");
        if !state_file.exists() {
            return Ok(());
        }
        let data = std::fs::read_to_string(&state_file).map_err(|e| e.to_string())?;
        self.plugins = serde_json::from_str(&data).unwrap_or_default();
        Ok(())
    }

    fn save_installed(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.plugins_dir).map_err(|e| e.to_string())?;
        let data = serde_json::to_string_pretty(&self.plugins).map_err(|e| e.to_string())?;
        std::fs::write(self.plugins_dir.join("plugins-state.json"), data).map_err(|e| e.to_string())
    }
}
