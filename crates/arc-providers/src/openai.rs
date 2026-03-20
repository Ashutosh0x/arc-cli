//! OpenAI provider implementation.

use crate::provider::*;
use arc_core::credentials::{CredentialKind, Provider as CredProvider};
use arc_core::error::{ArcError, ArcResult};
use arc_core::security::env_keys;
use async_trait::async_trait;
use serde_json::json;

pub struct OpenAIProvider {
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            max_context_window: 128_000,
        }
    }

    #[tracing::instrument(skip(self, messages), fields(provider = "openai", model = %model), err)]
    async fn chat(&self, messages: &[ChatMessage], model: &str) -> ArcResult<ChatResponse> {
        let key = env_keys::get_credential_with_env_override(
            CredProvider::OpenAI,
            CredentialKind::ApiKey,
        )?;

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| json!({"role": m.role, "content": m.content}))
            .collect();

        let body = json!({
            "model": model,
            "messages": api_messages,
            "max_tokens": 8192,
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(key.as_str())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let _error_body = resp.text().await.unwrap_or_default();
            return Err(ArcError::Provider("Provider API Error".to_string()).into());
        }

        let data: serde_json::Value = resp.json().await?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            model: data["model"].as_str().unwrap_or(model).to_string(),
            input_tokens: data["usage"]["prompt_tokens"].as_u64().map(|v| v as u32),
            output_tokens: data["usage"]["completion_tokens"]
                .as_u64()
                .map(|v| v as u32),
            finish_reason: data["choices"][0]["finish_reason"]
                .as_str()
                .map(String::from),
        })
    }

    async fn health_check(&self) -> ArcResult<bool> {
        match env_keys::get_credential_with_env_override(
            CredProvider::OpenAI,
            CredentialKind::ApiKey,
        ) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
