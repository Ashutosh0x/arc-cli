// ARC CLI — Persistent Configuration
// Stores selected provider/model in ~/.arc-cli/config.json
// Loaded at startup, updated by CLI commands and TUI model selection.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted configuration for the ARC CLI runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcConfig {
    /// LLM provider type: "ollama" or "openai"
    pub provider: String,
    /// Model identifier (e.g. "gemma3:latest", "gpt-4o")
    pub model: String,
    /// Ollama server base URL
    pub ollama_host: String,
}

impl Default for ArcConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "gemma4:latest".to_string(),
            ollama_host: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        }
    }
}

impl ArcConfig {
    /// Path to the config file: ~/.arc-cli/config.json
    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".arc-cli")
            .join("config.json")
    }

    /// Load config from disk, returning defaults if the file doesn't exist.
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save config to disk, creating directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Update model and provider, then persist.
    pub fn set_model(&mut self, model: &str, provider: &str) -> Result<()> {
        self.model = model.to_string();
        self.provider = provider.to_string();
        self.save()
    }
}
