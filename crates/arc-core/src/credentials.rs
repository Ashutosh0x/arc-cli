//! Secure credential management via OS keyring.
//! API keys are NEVER written to config files — only to the OS secret service.
//! All secrets are wrapped in `Zeroizing<String>` to ensure memory cleanup.

use crate::error::ArcResult;
use tracing::{debug, info};
use zeroize::Zeroizing;

const SERVICE_PREFIX: &str = "arc-cli";

/// Supported provider identifiers for credential storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Anthropic,
    OpenAI,
    Gemini,
    Ollama,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
        }
    }

    pub fn all() -> &'static [Provider] {
        &[Self::Anthropic, Self::OpenAI, Self::Gemini, Self::Ollama]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAI),
            "gemini" | "google" => Some(Self::Gemini),
            "ollama" => Some(Self::Ollama),
            _ => None,
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Credential types stored in the keyring.
#[derive(Debug, Clone, Copy)]
pub enum CredentialKind {
    ApiKey,
    OAuthAccessToken,
    OAuthRefreshToken,
}

impl CredentialKind {
    fn suffix(&self) -> &'static str {
        match self {
            Self::ApiKey => "api-key",
            Self::OAuthAccessToken => "oauth-access",
            Self::OAuthRefreshToken => "oauth-refresh",
        }
    }
}

/// Build the keyring service name for a provider + kind.
fn keyring_service(provider: Provider, kind: CredentialKind) -> String {
    format!("{SERVICE_PREFIX}:{}-{}", provider.as_str(), kind.suffix())
}

/// Store a credential securely in the OS keyring.
pub fn store_credential(
    provider: Provider,
    kind: CredentialKind,
    secret: &Zeroizing<String>,
) -> ArcResult<()> {
    let service = keyring_service(provider, kind);
    let username = whoami_username();

    debug!("Storing credential: service={service}");

    let entry = keyring::Entry::new(&service, &username).map_err(|e: keyring::Error| {
        crate::error::ArcError::Credential(format!("Failed to create keyring entry: {e}"))
    })?;

    entry
        .set_password(secret.as_str())
        .map_err(|e| crate::error::ArcError::Credential(format!("Keyring store failed: {e}")))?;

    info!("Credential stored for {provider} ({:?})", kind);
    Ok(())
}

/// Retrieve a credential from the OS keyring, wrapped in Zeroizing.
pub fn get_credential(
    provider: Provider,
    kind: CredentialKind,
) -> ArcResult<Zeroizing<String>> {
    let service = keyring_service(provider, kind);
    let username = whoami_username();

    let entry = keyring::Entry::new(&service, &username).map_err(|_e: keyring::Error| {
        crate::error::ArcError::Credential("Keyring access failed".to_string())
    })?;

    let password = entry.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => crate::error::ArcError::Credential(provider.to_string()),
        _other => crate::error::ArcError::Credential("Keyring access failed".to_string()),
    })?;

    Ok(Zeroizing::new(password))
}

/// Check if a credential exists for a given provider and kind.
pub fn has_credential(provider: Provider, kind: CredentialKind) -> bool {
    get_credential(provider, kind).is_ok()
}

/// Delete a specific credential from the keyring.
pub fn delete_credential(provider: Provider, kind: CredentialKind) -> ArcResult<()> {
    let service = keyring_service(provider, kind);
    let username = whoami_username();

    let entry = keyring::Entry::new(&service, &username).map_err(|_e: keyring::Error| {
        crate::error::ArcError::Credential("Keyring access failed".to_string())
    })?;

    entry.delete_credential().map_err(|_e: keyring::Error| {
        crate::error::ArcError::Credential("Keyring access failed".to_string())
    })?;

    info!("Credential deleted for {provider} ({kind:?})");
    Ok(())
}

/// Delete ALL credentials for a provider (API key + OAuth tokens).
pub fn logout_provider(provider: Provider) -> ArcResult<()> {
    let kinds = [
        CredentialKind::ApiKey,
        CredentialKind::OAuthAccessToken,
        CredentialKind::OAuthRefreshToken,
    ];

    for kind in &kinds {
        // Ignore "not found" errors — just clean up what exists
        let _ = delete_credential(provider, *kind);
    }

    info!("All credentials purged for {provider}");
    Ok(())
}

/// Delete credentials for ALL providers.
pub fn logout_all() -> ArcResult<()> {
    for provider in Provider::all() {
        logout_provider(*provider)?;
    }
    info!("All credentials purged from OS keyring");
    Ok(())
}

/// Get the current system username for keyring entries.
fn whoami_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "arc-user".to_string())
}

/// Summary of auth status per provider.
#[derive(Debug)]
pub struct AuthStatus {
    pub provider: Provider,
    pub has_api_key: bool,
    pub has_oauth_access: bool,
    pub has_oauth_refresh: bool,
}

/// Get auth status for all providers.
pub fn auth_status_all() -> Vec<AuthStatus> {
    Provider::all()
        .iter()
        .map(|&p| AuthStatus {
            provider: p,
            has_api_key: has_credential(p, CredentialKind::ApiKey),
            has_oauth_access: has_credential(p, CredentialKind::OAuthAccessToken),
            has_oauth_refresh: has_credential(p, CredentialKind::OAuthRefreshToken),
        })
        .collect()
}
