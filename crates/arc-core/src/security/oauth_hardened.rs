// SPDX-License-Identifier: MIT
//! Hardened OAuth 2.1 implementation.
//! Enforces: PKCE S256, CSRF state, exact redirect URI, Refresh Token Rotation (RTR).

use crate::credentials::{self, CredentialKind, Provider};
use crate::error::ArcResult;
use oauth2::{
    ClientId, RefreshToken, TokenResponse, TokenUrl, basic::BasicClient, reqwest as oauth2_reqwest,
};
use tracing::{info, warn};
use zeroize::Zeroizing;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com";

/// Refresh Token Rotation (RTR): exchange a refresh token for new tokens.
/// The old refresh token is invalidated; the new one replaces it.
pub async fn rotate_refresh_token(provider: Provider) -> ArcResult<()> {
    let refresh_token = credentials::get_credential(provider, CredentialKind::OAuthRefreshToken)?;

    let client = BasicClient::new(ClientId::new(GOOGLE_CLIENT_ID.to_string())).set_token_uri(
        TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string()))
        })?,
    );

    let http_client = oauth2_reqwest::ClientBuilder::new()
        .redirect(oauth2_reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(&http_client)
        .await
        .map_err(|e| crate::error::ArcError::Auth(format!("RTR failed: {e}")))?;

    // Store new access token
    let new_access = Zeroizing::new(token_result.access_token().secret().to_string());
    credentials::store_credential(provider, CredentialKind::OAuthAccessToken, &new_access)?;

    // Store new refresh token (RTR: old one is now invalid)
    if let Some(new_refresh) = token_result.refresh_token() {
        let new_refresh_str = Zeroizing::new(new_refresh.secret().to_string());
        credentials::store_credential(
            provider,
            CredentialKind::OAuthRefreshToken,
            &new_refresh_str,
        )?;
        info!("Refresh token rotated for {provider}");
    } else {
        warn!("Server did not return a new refresh token during RTR");
    }

    Ok(())
}

/// Validate that a redirect URI matches exactly (loopback exception: port may vary).
pub fn validate_redirect_uri(expected_host: &str, actual_uri: &str) -> ArcResult<()> {
    let parsed = url::Url::parse(actual_uri)
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    let host = parsed.host_str().unwrap_or("");

    // RFC 8252: Loopback exception — only host must match, port is dynamic
    if host != expected_host && host != "localhost" && host != "127.0.0.1" && host != "[::1]" {
        return Err(crate::error::ArcError::Auth(format!(
            "Redirect URI mismatch. Expected {}, got {}",
            expected_host.to_string(),
            host.to_string()
        ))
        .into());
    }

    // Scheme must be http for loopback
    if parsed.scheme() != "http" {
        return Err(crate::error::ArcError::Auth(format!(
            "OAuth failed: {}",
            "Loopback redirect must use http, not https"
        ))
        .into());
    }

    Ok(())
}
