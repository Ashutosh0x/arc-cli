use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpSecurityError {
    #[error("Manifest hash mismatch. Expected {expected}, got {actual}")]
    ManifestMismatch { expected: String, actual: String },
    #[error("Context violation: {0}")]
    ContextViolation(String),
}

/// Validates that a given MCP manifest string matches the pinned SHA256 hash.
pub fn verify_manifest_pin(
    manifest_json: &str,
    pinned_hash: &str,
) -> std::result::Result<(), McpSecurityError> {
    let mut hasher = Sha256::new();
    hasher.update(manifest_json.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    if hash != pinned_hash {
        return Err(McpSecurityError::ManifestMismatch {
            expected: pinned_hash.to_string(),
            actual: hash,
        });
    }
    Ok(())
}

/// Minimizes payload context by forcefully truncating or limiting payloads
/// before sending them to a third-party MCP server.
pub fn minimize_context(
    payload: &str,
    max_bytes: usize,
) -> std::result::Result<String, McpSecurityError> {
    if payload.len() > max_bytes {
        // We log and return an error here to prevent uncontrolled data leakage
        return Err(McpSecurityError::ContextViolation(format!(
            "Payload length {} exceeds absolute maximum of {} bytes for third-party MCP servers.",
            payload.len(),
            max_bytes
        )));
    }

    Ok(payload.to_string())
}
