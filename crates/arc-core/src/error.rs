//! Centralised error hierarchy for all ARC crates.

use thiserror::Error;

/// Top-level ARC error.
#[derive(Debug, Error)]
pub enum ArcError {
    /// Configuration parsing or validation error.
    #[error("config: {0}")]
    Config(String),

    /// Credential storage / retrieval error.
    #[error("credential: {0}")]
    Credential(String),

    /// AI provider API error.
    #[error("provider: {0}")]
    Provider(String),

    /// Embedded database (redb) error.
    #[error("database: {0}")]
    Database(String),

    /// Generic system / IO error.
    #[error("system: {0}")]
    System(String),

    /// Network / HTTP error.
    #[error("network: {0}")]
    Network(String),

    /// Authentication / OAuth error.
    #[error("auth: {0}")]
    Auth(String),

    /// Rate limit exceeded.
    #[error("rate limited: {provider} — retry after {retry_after_secs}s")]
    RateLimited {
        provider: String,
        retry_after_secs: u64,
    },

    /// User cancelled the operation.
    #[error("operation cancelled")]
    Cancelled,

    /// Multiple instances of ARC running in same directory
    #[error("instance conflict: {0}")]
    InstanceConflict(String),
}

impl From<std::io::Error> for ArcError {
    fn from(e: std::io::Error) -> Self {
        ArcError::System(e.to_string())
    }
}

impl From<reqwest::Error> for ArcError {
    fn from(e: reqwest::Error) -> Self {
        ArcError::Network(e.to_string())
    }
}

impl From<toml::de::Error> for ArcError {
    fn from(e: toml::de::Error) -> Self {
        ArcError::Config(e.to_string())
    }
}

impl From<serde_json::Error> for ArcError {
    fn from(e: serde_json::Error) -> Self {
        ArcError::System(format!("JSON: {e}"))
    }
}

pub type ArcResult<T> = std::result::Result<T, ArcError>;
