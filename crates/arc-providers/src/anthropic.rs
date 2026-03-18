//! Anthropic Claude provider implementation.

use crate::provider::*;
use arc_core::credentials::{CredentialKind, Provider as CredProvider};
use arc_core::error::{ArcResult, ArcError};
use arc_core::security::env_keys;
use async_trait::async_trait;
use serde_json::json;


pub struct AnthropicProvider {
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    fn get_api_key(&self) -> ArcResult<zeroize::Zeroizing<String>> {
        env_keys::get_credential_with_env_override(CredProvider::Anthropic, CredentialKind::ApiKey)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            max_context_window: 200_000,
        }
    }

    #[tracing::instrument(skip(self, messages), fields(provider = "anthropic", model = %model), err)]
    async fn chat(&self, messages: &[ChatMessage], model: &str) -> ArcResult<ChatResponse> {
        let key = self.get_api_key()?;

        // Separate system message from others (Anthropic API requires it separately)
        let system_msg: Option<&str> = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.as_str());

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut body = json!({
            "model": model,
            "max_tokens": 8192,
            "messages": api_messages,
        });

        if let Some(sys) = system_msg {
            body["system"] = json!(sys);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key.as_str())
            .header("anthropic-version", "2024-01-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let _error_body = resp.text().await.unwrap_or_default();
            return Err(ArcError::Provider("Provider API Error".to_string())
            .into());
        }

        let data: serde_json::Value = resp.json().await?;

        let content = data["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            model: data["model"].as_str().unwrap_or(model).to_string(),
            input_tokens: data["usage"]["input_tokens"].as_u64().map(|v| v as u32),
            output_tokens: data["usage"]["output_tokens"].as_u64().map(|v| v as u32),
            finish_reason: data["stop_reason"].as_str().map(String::from),
        })
    }

    async fn health_check(&self) -> ArcResult<bool> {
        match self.get_api_key() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
