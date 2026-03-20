//! Google OAuth 2.1 + PKCE (S256) authentication flow.
//! Uses a localhost redirect URI to capture the authorization code.
//! Enforces: PKCE S256, CSRF state, exact redirect URI matching, RTR.

use crate::credentials::{self, CredentialKind, Provider};
use crate::error::ArcResult;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope,
    TokenResponse, TokenUrl, basic::BasicClient, reqwest as oauth2_reqwest,
};
use std::io::{BufRead, Write};
use std::net::TcpListener;
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info};
use url::Url;
use zeroize::Zeroizing;

// Google OAuth endpoints
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

// Your registered client ID (public client — no secret needed with PKCE)
const GOOGLE_CLIENT_ID: &str = "YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com";

/// Fixed loopback redirect URI — port is dynamic but host is exact.
const REDIRECT_HOST: &str = "127.0.0.1";

/// Perform the full Google OAuth 2.1 + PKCE S256 flow.
pub async fn authenticate_with_oauth(provider: Provider) -> ArcResult<()> {
    info!("Starting OAuth 2.1 + PKCE flow for {provider}");

    // 1. Bind a random localhost port for the redirect
    let listener = TcpListener::bind(format!("{REDIRECT_HOST}:0")).map_err(|e| {
        crate::error::ArcError::Auth(format!(
            "OAuth failed: {}",
            format!("Failed to bind localhost: {e}")
        ))
    })?;
    let port = listener
        .local_addr()
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?
        .port();

    let redirect_uri = format!("http://{REDIRECT_HOST}:{port}");
    debug!("OAuth redirect URI: {redirect_uri}");

    // 2. Build the OAuth client (public client — no secret)
    let client = BasicClient::new(ClientId::new(GOOGLE_CLIENT_ID.to_string()))
        .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string()))
        })?)
        .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string()))
        })?)
        .set_redirect_uri(RedirectUrl::new(redirect_uri.clone()).map_err(|e| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string()))
        })?);

    // 3. Generate PKCE challenge (S256 ONLY — never plain)
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // 4. Build auth URL with CSRF state + PKCE
    let (auth_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/generative-language".to_string(),
        ))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // 5. Open the browser
    println!("\n🌐 Opening your browser for Google sign-in...");
    println!("   If it doesn't open, visit:\n   {auth_url}\n");

    if let Err(e) = open::that(auth_url.as_str()) {
        tracing::warn!("Could not open browser: {e}");
    }

    // 6. Wait for the callback (with timeout)
    let (code, returned_state) = timeout(Duration::from_secs(120), async {
        tokio::task::spawn_blocking(move || wait_for_callback(listener, &redirect_uri))
            .await
            .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?
    })
    .await
    .map_err(|_| {
        crate::error::ArcError::Auth(format!(
            "OAuth failed: {}",
            "OAuth callback timed out after 120 seconds"
        ))
    })??;

    // 7. Verify CSRF state — MUST match exactly
    if returned_state.secret() != csrf_state.secret() {
        error!("CSRF state mismatch! Possible attack.");
        return Err(crate::error::ArcError::Auth("CSRF Mismatch".into()).into());
    }

    // 8. Exchange code for tokens (with PKCE verifier)
    let http_client = oauth2_reqwest::ClientBuilder::new()
        .redirect(oauth2_reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    let token_result = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| {
            crate::error::ArcError::Auth(format!(
                "OAuth failed: {}",
                format!("Token exchange failed: {e}")
            ))
        })?;

    // 9. Store access token
    let access_token = Zeroizing::new(token_result.access_token().secret().to_string());
    credentials::store_credential(provider, CredentialKind::OAuthAccessToken, &access_token)?;

    // 10. Store refresh token (for Refresh Token Rotation)
    if let Some(refresh) = token_result.refresh_token() {
        let refresh_token = Zeroizing::new(refresh.secret().to_string());
        credentials::store_credential(provider, CredentialKind::OAuthRefreshToken, &refresh_token)?;
    }

    println!("✅ Google OAuth authentication successful!");
    info!("OAuth tokens stored for {provider}");

    Ok(())
}

/// Wait for the OAuth redirect callback on the localhost listener.
fn wait_for_callback(
    listener: TcpListener,
    expected_redirect: &str,
) -> ArcResult<(AuthorizationCode, CsrfToken)> {
    let (mut stream, _) = listener.accept().map_err(|e| {
        crate::error::ArcError::Auth(format!(
            "OAuth failed: {}",
            format!("Failed to accept connection: {e}")
        ))
    })?;

    let mut reader = std::io::BufReader::new(&stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    // Parse the URL from the HTTP request line
    let redirect_url = request_line.split_whitespace().nth(1).ok_or_else(|| {
        crate::error::ArcError::Auth(format!("OAuth failed: {}", "Invalid HTTP request"))
    })?;

    let full_url = format!("{expected_redirect}{redirect_url}");
    let url = Url::parse(&full_url)
        .map_err(|e| crate::error::ArcError::Auth(format!("OAuth failed: {}", e.to_string())))?;

    // Extract code and state from query params
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| AuthorizationCode::new(v.into_owned()))
        .ok_or_else(|| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", "Missing 'code' in callback"))
        })?;

    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| CsrfToken::new(v.into_owned()))
        .ok_or_else(|| {
            crate::error::ArcError::Auth(format!("OAuth failed: {}", "Missing 'state' in callback"))
        })?;

    // Send a nice response to the browser
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h2>✅ ARC authenticated!</h2>\
        <p>You can close this tab and return to your terminal.</p></body></html>";

    let _ = stream.write_all(response.as_bytes());

    Ok((code, state))
}
