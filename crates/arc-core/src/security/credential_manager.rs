//! Hardened credential manager wrapping the OS keyring.
//!
//! Features:
//! - Expiry tracking per credential.
//! - Key rotation support.
//! - Secure in-memory zeroization on drop.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::ArcError;

const SERVICE_NAME: &str = "arc-cli";

/// Metadata stored alongside each credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialMeta {
    /// Provider name (e.g. `"anthropic"`, `"openai"`).
    pub provider: String,
    /// Kind: `"api_key"` | `"oauth_refresh"` | `"oauth_access"`.
    pub kind: String,
    /// When this credential was stored (unix secs).
    pub stored_at: u64,
    /// Optional expiry (unix secs).  `None` = never expires.
    pub expires_at: Option<u64>,
    /// Rotation generation counter.
    pub generation: u32,
}

impl CredentialMeta {
    /// Returns `true` if the credential has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            now >= exp
        } else {
            false
        }
    }

    /// Time remaining before expiry.
    pub fn ttl(&self) -> Option<Duration> {
        self.expires_at.and_then(|exp| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            exp.checked_sub(now).map(Duration::from_secs)
        })
    }
}

/// Hardened credential manager.
pub struct CredentialManager;

impl CredentialManager {
    /// Store a credential with metadata.
    pub fn store(
        provider: &str,
        kind: &str,
        secret: &str,
        ttl: Option<Duration>,
    ) -> Result<(), ArcError> {
        let key = format!("{provider}_{kind}");

        let entry = keyring::Entry::new(SERVICE_NAME, &key)
            .map_err(|e| ArcError::Credential(format!("keyring entry error: {e}")))?;

        entry
            .set_password(secret)
            .map_err(|e| ArcError::Credential(format!("keyring store failed: {e}")))?;

        // Store metadata separately.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let meta = CredentialMeta {
            provider: provider.to_owned(),
            kind: kind.to_owned(),
            stored_at: now,
            expires_at: ttl.map(|d| now + d.as_secs()),
            generation: Self::current_generation(provider, kind).unwrap_or(0) + 1,
        };

        let meta_key = format!("{provider}_{kind}_meta");
        let meta_entry = keyring::Entry::new(SERVICE_NAME, &meta_key)
            .map_err(|e| ArcError::Credential(format!("keyring entry error: {e}")))?;

        let meta_json = serde_json::to_string(&meta)
            .map_err(|e| ArcError::System(e.to_string()))?;

        meta_entry
            .set_password(&meta_json)
            .map_err(|e| ArcError::Credential(format!("keyring meta store failed: {e}")))?;

        tracing::info!(
            provider = provider,
            kind = kind,
            generation = meta.generation,
            "credential stored"
        );

        Ok(())
    }

    /// Retrieve a credential.  Returns `None` if not found or expired.
    pub fn get(provider: &str, kind: &str) -> Result<Option<Zeroizing<String>>, ArcError> {
        // Check expiry first.
        if let Some(meta) = Self::get_meta(provider, kind)? {
            if meta.is_expired() {
                tracing::warn!(
                    provider = provider,
                    kind = kind,
                    "credential expired — deleting"
                );
                Self::delete(provider, kind)?;
                return Ok(None);
            }
        }

        let key = format!("{provider}_{kind}");
        let entry = keyring::Entry::new(SERVICE_NAME, &key)
            .map_err(|e| ArcError::Credential(format!("keyring entry error: {e}")))?;

        match entry.get_password() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ArcError::Credential(format!("keyring read failed: {e}"))),
        }
    }

    /// Delete a credential and its metadata.
    pub fn delete(provider: &str, kind: &str) -> Result<(), ArcError> {
        for suffix in ["", "_meta"] {
            let key = format!("{provider}_{kind}{suffix}");
            let entry = keyring::Entry::new(SERVICE_NAME, &key)
                .map_err(|e| ArcError::Credential(format!("keyring entry error: {e}")))?;

            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => {
                    return Err(ArcError::Credential(format!(
                        "keyring delete failed: {e}"
                    )));
                }
            }
        }

        tracing::info!(provider = provider, kind = kind, "credential deleted");
        Ok(())
    }

    /// Delete all ARC credentials for a provider.
    pub fn delete_provider(provider: &str) -> Result<(), ArcError> {
        for kind in ["api_key", "oauth_refresh", "oauth_access"] {
            Self::delete(provider, kind)?;
        }
        Ok(())
    }

    /// Delete every ARC credential (all providers).
    pub fn purge_all() -> Result<(), ArcError> {
        for provider in ["anthropic", "openai", "gemini", "ollama"] {
            Self::delete_provider(provider)?;
        }
        tracing::warn!("all credentials purged");
        Ok(())
    }

    /// Get metadata for a credential.
    pub fn get_meta(provider: &str, kind: &str) -> Result<Option<CredentialMeta>, ArcError> {
        let meta_key = format!("{provider}_{kind}_meta");
        let entry = keyring::Entry::new(SERVICE_NAME, &meta_key)
            .map_err(|e| ArcError::Credential(format!("keyring entry error: {e}")))?;

        match entry.get_password() {
            Ok(json) => {
                let meta: CredentialMeta = serde_json::from_str(&json)
                    .map_err(|e| ArcError::System(format!("corrupt credential meta: {e}")))?;
                Ok(Some(meta))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(ArcError::Credential(format!(
                "keyring meta read failed: {e}"
            ))),
        }
    }

    /// Summarise the status of all known provider credentials.
    pub fn status_all() -> HashMap<String, Vec<CredentialStatus>> {
        let mut result: HashMap<String, Vec<CredentialStatus>> = HashMap::new();

        for provider in ["anthropic", "openai", "gemini", "ollama"] {
            let mut statuses = Vec::new();
            for kind in ["api_key", "oauth_refresh", "oauth_access"] {
                let status = match Self::get_meta(provider, kind) {
                    Ok(Some(meta)) => {
                        if meta.is_expired() {
                            CredentialStatus::Expired
                        } else {
                            CredentialStatus::Active {
                                generation: meta.generation,
                                ttl: meta.ttl(),
                            }
                        }
                    }
                    Ok(None) => CredentialStatus::NotSet,
                    Err(_) => CredentialStatus::Error,
                };
                statuses.push(status);
            }
            result.insert(provider.to_owned(), statuses);
        }

        result
    }

    fn current_generation(provider: &str, kind: &str) -> Option<u32> {
        Self::get_meta(provider, kind)
            .ok()
            .flatten()
            .map(|m| m.generation)
    }
}

/// Status of a single credential slot.
#[derive(Debug, Clone)]
pub enum CredentialStatus {
    /// Credential is set and not expired.
    Active {
        generation: u32,
        ttl: Option<Duration>,
    },
    /// Credential exists but has expired.
    Expired,
    /// No credential stored.
    NotSet,
    /// Keyring error.
    Error,
}
