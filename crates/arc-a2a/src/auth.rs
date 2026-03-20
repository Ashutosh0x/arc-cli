//! Authentication and message integrity for A2A communications.
//! Supports JWT bearer tokens and HMAC-SHA256 request signing.

use hmac::{Hmac, Mac};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::{debug, warn};

use crate::error::{A2AError, A2AResult};
use crate::protocol::A2AMessage;

type HmacSha256 = Hmac<Sha256>;

// ── JWT ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentClaims {
    /// Subject — the agent ID
    pub sub: String,
    /// Issued at (unix timestamp)
    pub iat: u64,
    /// Expiration (unix timestamp)
    pub exp: u64,
    /// Issuer
    pub iss: String,
    /// Audience — the target agent ID
    pub aud: String,
    /// Scopes/permissions this token grants
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Generates a JWT for authenticating with a remote agent.
pub fn generate_jwt(
    agent_id: &str,
    target_id: &str,
    secret: &[u8],
    ttl_secs: u64,
) -> A2AResult<String> {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = AgentClaims {
        sub: agent_id.to_string(),
        iat: now,
        exp: now + ttl_secs,
        iss: "arc-cli".to_string(),
        aud: target_id.to_string(),
        scopes: vec!["task:submit".into(), "task:query".into()],
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| A2AError::AuthFailed(format!("JWT generation failed: {e}")))
}

/// Validates an incoming JWT and extracts claims.
pub fn validate_jwt(token: &str, secret: &[u8], my_agent_id: &str) -> A2AResult<AgentClaims> {
    let mut validation = Validation::default();
    validation.set_audience(&[my_agent_id]);
    validation.set_issuer(&["arc-cli"]);

    let token_data = decode::<AgentClaims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => A2AError::TokenExpired {
            agent_id: "unknown".into(),
        },
        _ => A2AError::AuthFailed(format!("JWT validation failed: {e}")),
    })?;

    debug!(agent = %token_data.claims.sub, "JWT validated successfully");
    Ok(token_data.claims)
}

// ── HMAC-SHA256 Message Signing ────────────────────────────────

/// Signs a message by computing HMAC-SHA256 over the serialized payload.
/// Mutates the message in place, setting the `signature` field.
pub fn sign_message(msg: &mut A2AMessage, secret: &[u8]) -> A2AResult<()> {
    // Clear any existing signature before computing
    msg.signature = None;

    let canonical = serde_json::to_vec(msg).map_err(A2AError::Serialization)?;

    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|e| A2AError::Internal(e.to_string()))?;
    mac.update(&canonical);
    let result = mac.finalize();

    msg.signature = Some(hex::encode(result.into_bytes()));
    Ok(())
}

/// Verifies the HMAC-SHA256 signature on an incoming message.
pub fn verify_signature(msg: &A2AMessage, secret: &[u8]) -> A2AResult<()> {
    let sig_hex = msg
        .signature
        .as_ref()
        .ok_or_else(|| A2AError::SignatureInvalid)?;

    let sig_bytes = hex::decode(sig_hex).map_err(|_| A2AError::SignatureInvalid)?;

    // Rebuild the message without signature for verification
    let mut verify_msg = msg.clone();
    verify_msg.signature = None;
    let canonical = serde_json::to_vec(&verify_msg).map_err(A2AError::Serialization)?;

    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|e| A2AError::Internal(e.to_string()))?;
    mac.update(&canonical);

    mac.verify_slice(&sig_bytes).map_err(|_| {
        warn!(
            message_id = %msg.message_id,
            sender = %msg.sender_id,
            "HMAC signature verification failed"
        );
        A2AError::SignatureInvalid
    })
}

// ── Auth Credential Container ──────────────────────────────────

/// Encapsulates credentials for authenticating with a remote agent.
#[derive(Clone)]
pub enum Credential {
    /// No authentication
    None,
    /// JWT bearer token (auto-generated)
    Jwt { secret: Vec<u8>, ttl_secs: u64 },
    /// HMAC signing key
    Hmac {
        secret: Vec<u8>,
        header_name: String,
    },
    /// Static API key
    ApiKey { key: String, header_name: String },
}

impl Credential {
    /// Apply authentication to an outgoing reqwest request.
    pub fn apply_to_request(
        &self,
        req: reqwest::RequestBuilder,
        sender_id: &str,
        target_id: &str,
    ) -> A2AResult<reqwest::RequestBuilder> {
        match self {
            Credential::None => Ok(req),
            Credential::Jwt { secret, ttl_secs } => {
                let token = generate_jwt(sender_id, target_id, secret, *ttl_secs)?;
                Ok(req.bearer_auth(token))
            },
            Credential::ApiKey { key, header_name } => {
                Ok(req.header(header_name.as_str(), key.as_str()))
            },
            Credential::Hmac { .. } => {
                // HMAC is applied after body serialization — handled in client
                Ok(req)
            },
        }
    }
}
