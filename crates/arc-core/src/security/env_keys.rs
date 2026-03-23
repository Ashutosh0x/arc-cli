// SPDX-License-Identifier: MIT
//! Environment variable overrides for API keys.
//! Checks env vars first, falls back to keyring.
//! Redacts secrets in all log output.

use crate::credentials::{self, CredentialKind, Provider};
use crate::error::ArcResult;
use tracing::debug;
use zeroize::Zeroizing;

/// Env var names per provider.
fn env_var_name(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "ANTHROPIC_API_KEY",
        Provider::OpenAI => "OPENAI_API_KEY",
        Provider::Gemini => "GEMINI_API_KEY",
        Provider::Ollama => "OLLAMA_HOST",
    }
}

/// Get a credential, preferring env var override, then keyring.
pub fn get_credential_with_env_override(
    provider: Provider,
    kind: CredentialKind,
) -> ArcResult<Zeroizing<String>> {
    let var_name = env_var_name(provider);

    // Check environment first
    if let Ok(val) = std::env::var(var_name) {
        if !val.is_empty() {
            debug!("Using env var {var_name} for {provider} (redacted)");
            return Ok(Zeroizing::new(val));
        }
    }

    // Fall back to keyring
    credentials::get_credential(provider, kind)
}

/// Redact a secret for safe logging: show first 4 chars + "***".
pub fn redact(secret: &str) -> String {
    if secret.len() <= 4 {
        "****".to_string()
    } else {
        format!("{}****", &secret[..4])
    }
}
