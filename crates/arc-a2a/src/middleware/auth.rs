// SPDX-License-Identifier: MIT
//! Mandatory authentication middleware for all A2A endpoints.
//!
//! Fixes: A2A server accepts unauthenticated task submissions/cancellations/streams.
//! Every inbound request MUST carry a valid Bearer JWT or HMAC signature header.
//! Identity is extracted, bound to `AuthenticatedIdentity`, and injected into
//! request extensions so handlers can enforce sender_id matching.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{error, warn};

/// Identity proven by authentication. Inserted into request extensions.
#[derive(Clone, Debug)]
pub struct AuthenticatedIdentity {
    pub subject: String,
    pub roles: Vec<String>,
    pub issued_at: u64,
    pub nonce: Option<String>,
}

/// JWT claims structure.
#[derive(Debug, Serialize, Deserialize)]
pub struct A2AClaims {
    pub sub: String,
    pub roles: Vec<String>,
    pub iat: u64,
    pub exp: u64,
    pub nonce: Option<String>,
}

/// Configuration for the auth middleware.
#[derive(Clone)]
pub struct AuthConfig {
    /// HMAC shared secret (hex-encoded).
    pub hmac_secret: Option<Vec<u8>>,
    /// JWT decoding key (RSA/EC public key or HMAC secret).
    pub jwt_decoding_key: Option<DecodingKey>,
    /// JWT validation parameters.
    pub jwt_validation: Validation,
    /// Maximum clock skew tolerance for timestamps.
    pub max_clock_skew: Duration,
    /// Maximum age of a signed message before rejection.
    pub max_message_age: Duration,
    /// Bounded replay cache for nonce deduplication.
    pub replay_cache: Arc<RwLock<ReplayCache>>,
    /// Route-level authorization rules: method+path -> required roles.
    pub route_policies: HashMap<String, Vec<String>>,
}

/// Bounded LRU-style replay cache to prevent nonce reuse.
#[derive(Debug)]
pub struct ReplayCache {
    entries: HashMap<String, u64>,
    max_entries: usize,
}

impl ReplayCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Returns `true` if the nonce was already seen (replay detected).
    /// Inserts the nonce if fresh.
    pub fn check_and_insert(&mut self, nonce: &str, timestamp: u64) -> bool {
        if self.entries.contains_key(nonce) {
            return true; // replay
        }

        // Evict oldest entries if at capacity
        if self.entries.len() >= self.max_entries {
            let cutoff = timestamp.saturating_sub(300); // 5-minute window
            self.entries.retain(|_, ts| *ts > cutoff);
        }

        // If still at capacity after eviction, reject (safety valve)
        if self.entries.len() >= self.max_entries {
            return true;
        }

        self.entries.insert(nonce.to_string(), timestamp);
        false
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            hmac_secret: None,
            jwt_decoding_key: None,
            jwt_validation: Validation::default(),
            max_clock_skew: Duration::from_secs(30),
            max_message_age: Duration::from_secs(300),
            replay_cache: Arc::new(RwLock::new(ReplayCache::new(10_000))),
            route_policies: HashMap::new(),
        }
    }
}

/// Primary authentication middleware. Must be applied to ALL A2A routes.
///
/// Verifies either:
/// 1. `Authorization: Bearer <JWT>` header, or
/// 2. `X-ARC-Signature: <hex-encoded HMAC-SHA256>` + `X-ARC-Timestamp` + `X-ARC-Nonce` headers.
///
/// On success, injects `AuthenticatedIdentity` into request extensions.
/// On failure, returns 401 Unauthorized or 403 Forbidden.
pub async fn require_auth(
    auth_config: axum::extract::Extension<Arc<AuthConfig>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let headers = request.headers().clone();
    let config = auth_config.0.as_ref();

    // Attempt JWT first, then HMAC
    let identity = if let Some(bearer) = extract_bearer(&headers) {
        validate_jwt(bearer, config).await?
    } else if headers.contains_key("x-arc-signature") {
        validate_hmac(&headers, config).await?
    } else {
        warn!("A2A request missing authentication headers");
        return Err((
            StatusCode::UNAUTHORIZED,
            "Missing authentication: provide Bearer JWT or HMAC signature",
        )
            .into_response());
    };

    // Replay protection: check nonce freshness
    if let Some(ref nonce) = identity.nonce {
        let mut cache = config.replay_cache.write().await;
        if cache.check_and_insert(nonce, identity.issued_at) {
            warn!(
                subject = %identity.subject,
                nonce = %nonce,
                "Replay attack detected: duplicate nonce"
            );
            return Err((StatusCode::UNAUTHORIZED, "Replay detected: duplicate nonce")
                .into_response());
        }
    }

    // Enforce message freshness (timestamp window)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let age = now.abs_diff(identity.issued_at);
    if age > config.max_message_age.as_secs() + config.max_clock_skew.as_secs() {
        warn!(
            subject = %identity.subject,
            age_secs = age,
            "Message too old or clock skew exceeded"
        );
        return Err((StatusCode::UNAUTHORIZED, "Message timestamp expired").into_response());
    }

    // Route-level authorization
    let route_key = format!(
        "{}:{}",
        request.method().as_str(),
        request.uri().path()
    );
    if let Some(required_roles) = config.route_policies.get(&route_key) {
        let has_role = required_roles
            .iter()
            .any(|r| identity.roles.contains(r));
        if !has_role {
            warn!(
                subject = %identity.subject,
                route = %route_key,
                "Insufficient permissions for route"
            );
            return Err((
                StatusCode::FORBIDDEN,
                "Insufficient permissions for this endpoint",
            )
                .into_response());
        }
    }

    // Inject identity into request extensions for handler use
    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

async fn validate_jwt(
    token: &str,
    config: &AuthConfig,
) -> Result<AuthenticatedIdentity, Response> {
    let key = config.jwt_decoding_key.as_ref().ok_or_else(|| {
        error!("JWT decoding key not configured");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server authentication misconfigured",
        )
            .into_response()
    })?;

    let token_data: TokenData<A2AClaims> =
        decode(token, key, &config.jwt_validation).map_err(|e| {
            warn!(error = %e, "JWT validation failed");
            (StatusCode::UNAUTHORIZED, "Invalid or expired JWT token").into_response()
        })?;

    let claims = token_data.claims;

    Ok(AuthenticatedIdentity {
        subject: claims.sub,
        roles: claims.roles,
        issued_at: claims.iat,
        nonce: claims.nonce,
    })
}

async fn validate_hmac(
    headers: &HeaderMap,
    config: &AuthConfig,
) -> Result<AuthenticatedIdentity, Response> {
    let secret = config.hmac_secret.as_ref().ok_or_else(|| {
        error!("HMAC secret not configured");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server authentication misconfigured",
        )
            .into_response()
    })?;

    let signature_hex = headers
        .get("x-arc-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Missing X-ARC-Signature header").into_response()
        })?;

    let timestamp_str = headers
        .get("x-arc-timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Missing X-ARC-Timestamp header").into_response()
        })?;

    let nonce = headers
        .get("x-arc-nonce")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Missing X-ARC-Nonce header").into_response()
        })?;

    let sender = headers
        .get("x-arc-sender")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (StatusCode::UNAUTHORIZED, "Missing X-ARC-Sender header").into_response()
        })?;

    let timestamp: u64 = timestamp_str.parse().map_err(|_| {
        (StatusCode::BAD_REQUEST, "Invalid timestamp format").into_response()
    })?;

    // Reconstruct canonical payload: "timestamp.nonce.sender"
    let canonical = format!("{}.{}.{}", timestamp, nonce, sender);

    let signature_bytes = hex::decode(signature_hex).map_err(|_| {
        (StatusCode::BAD_REQUEST, "Invalid signature hex encoding").into_response()
    })?;

    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(canonical.as_bytes());
    mac.verify_slice(&signature_bytes).map_err(|_| {
        warn!(sender = %sender, "HMAC signature verification failed");
        (StatusCode::UNAUTHORIZED, "Invalid HMAC signature").into_response()
    })?;

    Ok(AuthenticatedIdentity {
        subject: sender.to_string(),
        roles: vec!["agent".to_string()],
        issued_at: timestamp,
        nonce: Some(nonce.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_cache_detects_duplicate() {
        let mut cache = ReplayCache::new(100);
        assert!(!cache.check_and_insert("nonce-1", 1000));
        assert!(cache.check_and_insert("nonce-1", 1001)); // replay
        assert!(!cache.check_and_insert("nonce-2", 1002)); // fresh
    }

    #[test]
    fn replay_cache_evicts_old_entries() {
        let mut cache = ReplayCache::new(2);
        assert!(!cache.check_and_insert("a", 100));
        assert!(!cache.check_and_insert("b", 200));
        // At capacity — insertion of "c" at t=600 evicts entries older than 300
        assert!(!cache.check_and_insert("c", 600));
        // "a" at t=100 should have been evicted
        assert!(!cache.check_and_insert("a", 601)); // treated as new
    }
}
