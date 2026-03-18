//! Provider trait and shared types.

use arc_core::error::ArcResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Chat message roles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// Response from a provider.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub finish_reason: Option<String>,
}

/// Provider capability declarations.
#[derive(Debug, Clone)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub max_context_window: u64,
}

/// The core Provider trait — all backends implement this.
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;

    /// Send a chat completion request.
    async fn chat(&self, messages: &[ChatMessage], model: &str) -> ArcResult<ChatResponse>;

    /// Health check — verify the provider is reachable.
    async fn health_check(&self) -> ArcResult<bool>;
}
