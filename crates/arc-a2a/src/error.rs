// SPDX-License-Identifier: MIT
//! Typed error hierarchy for the A2A protocol layer.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum A2AError {
    // ── Discovery ──────────────────────────────────────────────
    #[error("Agent discovery failed for {url}: {reason}")]
    DiscoveryFailed { url: String, reason: String },

    #[error("Agent card validation failed: {0}")]
    InvalidAgentCard(String),

    #[error("Agent {agent_id} does not support skill: {skill}")]
    UnsupportedSkill { agent_id: String, skill: String },

    // ── Authentication ─────────────────────────────────────────
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("JWT token expired for agent {agent_id}")]
    TokenExpired { agent_id: String },

    #[error("HMAC signature verification failed")]
    SignatureInvalid,

    // ── Task lifecycle ─────────────────────────────────────────
    #[error("Invalid task state transition: {from:?} → {to:?}")]
    InvalidTransition {
        from: crate::task::TaskState,
        to: crate::task::TaskState,
    },

    #[error("Task {task_id} not found")]
    TaskNotFound { task_id: String },

    #[error("Task {task_id} timed out after {timeout_secs}s")]
    TaskTimeout { task_id: String, timeout_secs: u64 },

    // ── Transport ──────────────────────────────────────────────
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Remote agent returned {status}: {body}")]
    RemoteError { status: u16, body: String },

    #[error("Connection to {endpoint} failed after {attempts} attempts")]
    ConnectionExhausted { endpoint: String, attempts: u32 },

    #[error("SSE stream closed unexpectedly for task {task_id}")]
    StreamClosed { task_id: String },

    // ── Serialization ──────────────────────────────────────────
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // ── Internal ───────────────────────────────────────────────
    #[error("Internal A2A error: {0}")]
    Internal(String),
}

pub type A2AResult<T> = Result<T, A2AError>;
