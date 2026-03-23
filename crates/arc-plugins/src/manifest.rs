// SPDX-License-Identifier: MIT
//! Plugin manifest format.
//!
//! A plugin is a directory with this structure:
//!
//! ```text
//! my-plugin/
//! ├── plugin.toml          # Manifest (required)
//! ├── commands/             # Slash commands
//! │   ├── my-command.toml
//! │   └── my-command.sh
//! ├── agents/               # Agent definitions
//! │   └── code-reviewer.toml
//! ├── skills/               # Auto-invoked skills
//! │   └── frontend-design.md
//! ├── hooks/                # Lifecycle hooks
//! │   └── security-scan.toml
//! └── mcp/                  # MCP server configs
//!     └── my-server.json
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The main plugin manifest file (plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata.
    pub plugin: PluginMeta,

    /// Dependencies on other plugins (optional).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    /// Configuration schema (optional).
    #[serde(default)]
    pub config: HashMap<String, ConfigField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Unique plugin name (e.g., "security-scanner", "pr-review-toolkit").
    pub name: String,

    /// Semantic version.
    pub version: String,

    /// Human-readable description.
    pub description: String,

    /// Author name or organization.
    pub author: String,

    /// License identifier (e.g., "MIT", "Apache-2.0").
    #[serde(default = "default_license")]
    pub license: String,

    /// Minimum ARC CLI version required.
    #[serde(default)]
    pub min_arc_version: Option<String>,

    /// Tags for marketplace search.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Homepage URL.
    #[serde(default)]
    pub homepage: Option<String>,

    /// Repository URL.
    #[serde(default)]
    pub repository: Option<String>,
}

fn default_license() -> String {
    "MIT".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub description: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub default: Option<serde_json::Value>,
    pub required: bool,
}

/// A fully loaded plugin with all its components resolved.
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub install_path: PathBuf,
    pub commands: Vec<PluginCommand>,
    pub agents: Vec<PluginAgent>,
    pub skills: Vec<PluginSkill>,
    pub hooks: Vec<PluginHook>,
    pub mcp_configs: Vec<PluginMcpConfig>,
    pub integrity_hash: String,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
    pub description: String,
    /// The command handler: either a shell script path or inline command.
    pub handler: CommandHandler,
    /// Arguments this command accepts.
    #[serde(default)]
    pub args: Vec<CommandArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandHandler {
    #[serde(rename = "script")]
    Script { path: String },
    #[serde(rename = "inline")]
    Inline { command: String },
    #[serde(rename = "agent")]
    Agent { agent_name: String, prompt_template: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandArg {
    pub name: String,
    pub description: String,
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAgent {
    pub name: String,
    pub description: String,
    pub system_prompt_file: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSkill {
    pub name: String,
    pub description: String,
    /// File patterns that trigger this skill (e.g., "*.tsx", "*.css").
    pub trigger_patterns: Vec<String>,
    /// The skill prompt (markdown file path).
    pub prompt_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHook {
    pub name: String,
    pub hook_config: arc_hooks::config::HookDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMcpConfig {
    pub server_name: String,
    pub config_file: String,
}

impl LoadedPlugin {
    /// Load a plugin from a directory.
    pub fn load_from_dir(dir: &Path) -> Result<Self, PluginLoadError> {
        let manifest_path = dir.join("plugin.toml");
        if !manifest_path.exists() {
            return Err(PluginLoadError::NoManifest(dir.to_path_buf()));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)
            .map_err(|e| PluginLoadError::Io(manifest_path.clone(), e))?;

        let manifest: PluginManifest = toml::from_str(&manifest_content)
            .map_err(|e| PluginLoadError::Parse(manifest_path.clone(), e))?;

        // Load commands
        let commands = Self::load_components::<PluginCommand>(&dir.join("commands"))?;

        // Load agents
        let agents = Self::load_components::<PluginAgent>(&dir.join("agents"))?;

        // Load skills
        let skills = Self::load_components::<PluginSkill>(&dir.join("skills"))?;

        // Load hooks
        let hooks = Self::load_components::<PluginHook>(&dir.join("hooks"))?;

        // Load MCP configs
        let mcp_configs = Self::load_components::<PluginMcpConfig>(&dir.join("mcp"))?;

        // Compute integrity hash
        let integrity_hash = Self::compute_integrity_hash(dir)?;

        Ok(Self {
            manifest,
            install_path: dir.to_path_buf(),
            commands,
            agents,
            skills,
            hooks,
            mcp_configs,
            integrity_hash,
            installed_at: Utc::now(),
        })
    }

    fn load_components<T: serde::de::DeserializeOwned>(dir: &Path) -> Result<Vec<T>, PluginLoadError> {
        if !dir.exists() {
            return Ok(vec![]);
        }

        let mut components = Vec::new();
        for entry in walkdir::WalkDir::new(dir).max_depth(1).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| PluginLoadError::Io(path.to_path_buf(), e))?;
                let component: T = toml::from_str(&content)
                    .map_err(|e| PluginLoadError::Parse(path.to_path_buf(), e))?;
                components.push(component);
            }
        }

        Ok(components)
    }

    fn compute_integrity_hash(dir: &Path) -> Result<String, PluginLoadError> {
        let mut hasher = Sha256::new();

        for entry in walkdir::WalkDir::new(dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let content = std::fs::read(entry.path())
                    .map_err(|e| PluginLoadError::Io(entry.path().to_path_buf(), e))?;
                hasher.update(&content);
            }
        }

        Ok(hex::encode(hasher.finalize()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginLoadError {
    #[error("No plugin.toml found in {0}")]
    NoManifest(PathBuf),

    #[error("I/O error reading {0}: {1}")]
    Io(PathBuf, std::io::Error),

    #[error("Parse error in {0}: {1}")]
    Parse(PathBuf, toml::de::Error),
}
