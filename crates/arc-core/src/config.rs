//! Configuration management for ARC.
//! Stores non-sensitive preferences in `~/.arc/config.toml`.
//! Uses `OnceLock` for singleton lazy initialization.

use crate::error::ArcResult;
use crate::memory::MemoryConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use once_cell::sync::OnceCell;
use tracing::{debug, info};

/// Global config singleton, initialized once via `OnceLock`.
static GLOBAL_CONFIG: OnceCell<ArcConfig> = OnceCell::new();

/// Primary configuration structure, persisted to `~/.arc/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub providers: ProvidersConfig,

    #[serde(default)]
    pub routing: RoutingConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub memory: MemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Default model identifier, e.g., "claude-3-5-sonnet-20241022"
    #[serde(default)]
    pub default_model: Option<String>,

    /// Default provider, e.g., "anthropic"
    #[serde(default)]
    pub default_provider: Option<String>,

    /// Maximum context tokens to send
    #[serde(default = "default_max_context")]
    pub max_context_tokens: u32,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            default_provider: None,
            max_context_tokens: default_max_context(),
        }
    }
}

fn default_max_context() -> u32 {
    128_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub anthropic: ProviderEntry,
    #[serde(default)]
    pub openai: ProviderEntry,
    #[serde(default)]
    pub gemini: ProviderEntry,
    #[serde(default)]
    pub ollama: OllamaEntry,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            anthropic: ProviderEntry::default(),
            openai: ProviderEntry::default(),
            gemini: ProviderEntry::default(),
            ollama: OllamaEntry::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderEntry {
    pub enabled: bool,
    /// Auth method: "api_key" or "oauth"
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
}

fn default_auth_method() -> String {
    "api_key".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaEntry {
    pub enabled: bool,
    #[serde(default = "default_ollama_host")]
    pub host: String,
}

impl Default for OllamaEntry {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_ollama_host(),
        }
    }
}

fn default_ollama_host() -> String {
    "http://localhost:11434".to_string()
}

/// Routing strategy for multi-provider fallback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default = "default_strategy")]
    pub strategy: RoutingStrategy,

    /// Ordered fallback chain of provider names.
    #[serde(default)]
    pub fallback_chain: Vec<String>,

    /// Timeout per-provider request in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            strategy: default_strategy(),
            fallback_chain: vec![
                "anthropic".into(),
                "openai".into(),
                "gemini".into(),
                "ollama".into(),
            ],
            timeout_secs: default_timeout(),
        }
    }
}

fn default_strategy() -> RoutingStrategy {
    RoutingStrategy::FallbackChain
}

fn default_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingStrategy {
    Latency,
    CostOptimized,
    FallbackChain,
    RoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub prompt_guard_enabled: bool,

    #[serde(default = "default_true")]
    pub data_guard_enabled: bool,

    #[serde(default = "default_true")]
    pub audit_logging_enabled: bool,

    #[serde(default = "default_true")]
    pub config_permissions_check: bool,

    /// Maximum requests per minute per provider
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,

    /// Maximum spend per session in USD (approximate)
    #[serde(default = "default_cost_limit")]
    pub cost_limit_usd: f64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            prompt_guard_enabled: true,
            data_guard_enabled: true,
            audit_logging_enabled: true,
            config_permissions_check: true,
            rate_limit_rpm: default_rate_limit(),
            cost_limit_usd: default_cost_limit(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_rate_limit() -> u32 {
    60
}

fn default_cost_limit() -> f64 {
    10.0
}

impl Default for ArcConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            providers: ProvidersConfig::default(),
            routing: RoutingConfig::default(),
            security: SecurityConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl ArcConfig {
    /// Standard config directory: `~/.arc/`
    pub fn dir() -> ArcResult<PathBuf> {
        let home = dirs::home_dir().ok_or(crate::error::ArcError::Config("Config directory not found".to_string()))?;
        Ok(home.join(".arc"))
    }

    /// Standard config file path: `~/.arc/config.toml`
    pub fn path() -> ArcResult<PathBuf> {
        Ok(Self::dir()?.join("config.toml"))
    }

    /// Load config from disk, or return defaults if not found.
    pub fn load() -> ArcResult<Self> {
        let path = Self::path()?;
        if !path.exists() {
            info!("No config found at {}, using defaults", path.display());
            return Ok(Self::default());
        }

        debug!("Loading config from {}", path.display());
        let contents = std::fs::read_to_string(&path).map_err(|e| crate::error::ArcError::Config(e.to_string()))?;
        let config: ArcConfig =
            toml::from_str(&contents).map_err(|e| crate::error::ArcError::Config(e.to_string()))?;
        Ok(config)
    }

    /// Save config to disk, creating the directory if necessary.
    pub fn save(&self) -> ArcResult<()> {
        let dir = Self::dir()?;
        std::fs::create_dir_all(&dir)?;

        let path = Self::path()?;
        let contents =
            toml::to_string_pretty(self).map_err(|e| crate::error::ArcError::Config(e.to_string()))?;
        std::fs::write(&path, contents)?;

        // Restrict permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&path, perms)?;
        }

        info!("Config saved to {}", path.display());
        Ok(())
    }

    /// Get or initialize the global config singleton.
    pub fn global() -> &'static ArcConfig {
        GLOBAL_CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                tracing::warn!("Failed to load config, using defaults: {e}");
                Self::default()
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serialization_roundtrip() {
        let config = ArcConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ArcConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.routing.strategy, RoutingStrategy::FallbackChain);
    }

    #[test]
    fn test_default_values() {
        let config = ArcConfig::default();
        assert!(config.security.prompt_guard_enabled);
        assert_eq!(config.routing.timeout_secs, 30);
        assert_eq!(config.general.max_context_tokens, 128_000);
    }
}
