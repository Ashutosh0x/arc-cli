//! API Key authentication — prompts user and stores in keyring.

use crate::credentials::{self, CredentialKind, Provider};
use crate::error::ArcResult;
use dialoguer::{Password, theme::ColorfulTheme};
use tracing::info;
use zeroize::Zeroizing;

/// Prompt the user for an API key and store it in the OS keyring.
pub fn authenticate_with_api_key(provider: Provider) -> ArcResult<()> {
    let prompt = format!("Enter your {} API key", provider);

    let key_input = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(&prompt)
        .interact()
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    let key = Zeroizing::new(key_input);

    // Basic validation
    if key.trim().is_empty() {
        return Err(crate::error::ArcError::Auth(format!(
            "Invalid API key for {}",
            provider.to_string()
        ))
        .into());
    }

    // Provider-specific prefix validation
    match provider {
        Provider::Anthropic => {
            if !key.starts_with("sk-ant-") {
                tracing::warn!("Anthropic key doesn't start with 'sk-ant-' — it may be invalid");
            }
        },
        Provider::OpenAI => {
            if !key.starts_with("sk-") {
                tracing::warn!("OpenAI key doesn't start with 'sk-' — it may be invalid");
            }
        },
        _ => {},
    }

    credentials::store_credential(provider, CredentialKind::ApiKey, &key)?;
    info!("API key stored successfully for {provider}");

    Ok(())
}
