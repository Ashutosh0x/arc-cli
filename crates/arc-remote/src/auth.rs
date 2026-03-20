//! Authentication for the remote control server.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteAuthClaims {
    pub exp: usize,
    pub session_id: String,
}

pub struct SecretKey(pub String);

/// Ensures WebSocket upgrades provide a valid JWT with the CLI's secret key.
pub async fn auth_middleware(
    req: Request,
    next: Next,
    secret: String,
) -> Result<Response, StatusCode> {
    let headers = req.headers();

    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => {
            warn!("Missing or invalid Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        },
    };

    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

    match decode::<RemoteAuthClaims>(token, &decoding_key, &validation) {
        Ok(token_data) => {
            // Bind the claims to the request parts if needed
            let mut req = req;
            req.extensions_mut().insert(token_data.claims);
            Ok(next.run(req).await)
        },
        Err(e) => {
            warn!("Invalid remote JWT: {}", e);
            Err(StatusCode::UNAUTHORIZED)
        },
    }
}
