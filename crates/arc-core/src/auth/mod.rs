//! Authentication module — dispatches between API key and OAuth flows.

pub mod api_key;
pub mod oauth_google;

use crate::credentials::Provider;
use crate::error::ArcResult;

/// Authenticate a provider using the appropriate method.
pub async fn authenticate_provider(provider: Provider, method: &str) -> ArcResult<()> {
    match method {
        "api_key" => api_key::authenticate_with_api_key(provider),
        "oauth" => oauth_google::authenticate_with_oauth(provider).await,
        other => {
            tracing::error!("Unknown auth method: {other}");
            Err(crate::error::ArcError::Auth(format!(
                "Unknown auth method: {other}"
            ))
            .into())
        }
    }
}
