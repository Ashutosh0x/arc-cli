// SPDX-License-Identifier: MIT
//! Google Gemini provider implementation.

use crate::provider::*;
use arc_core::credentials::{self, CredentialKind, Provider as CredProvider};
use arc_core::error::{ArcError, ArcResult};
use arc_core::security::env_keys;
use async_trait::async_trait;
use serde_json::json;

pub struct GeminiProvider {
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            max_context_window: 1_000_000,
        }
    }

    #[tracing::instrument(skip(self, messages), fields(provider = "gemini", model = %model), err)]
    async fn chat(&self, messages: &[ChatMessage], model: &str) -> ArcResult<ChatResponse> {
        let key = env_keys::get_credential_with_env_override(
            CredProvider::Gemini,
            CredentialKind::ApiKey,
        )
        .or_else(|_| {
            env_keys::get_credential_with_env_override(
                CredProvider::Gemini,
                CredentialKind::OAuthAccessToken,
            )
        })?;

        let contents: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    _ => "user",
                };
                json!({
                    "role": role,
                    "parts": [{"text": &m.content}]
                })
            })
            .collect();

        let system_instruction = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| json!({"parts": [{"text": &m.content}]}));

        let mut body = json!({"contents": contents});
        if let Some(sys) = system_instruction {
            body["systemInstruction"] = sys;
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}",
            key = key.as_str()
        );

        let resp = self.client.post(&url).json(&body).send().await?;

        let status = resp.status();
        if !status.is_success() {
            let _error_body = resp.text().await.unwrap_or_default();
            return Err(ArcError::Provider("Provider API Error".to_string()).into());
        }

        let data: serde_json::Value = resp.json().await?;

        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            content,
            model: model.to_string(),
            input_tokens: data["usageMetadata"]["promptTokenCount"]
                .as_u64()
                .map(|v| v as u32),
            output_tokens: data["usageMetadata"]["candidatesTokenCount"]
                .as_u64()
                .map(|v| v as u32),
            finish_reason: data["candidates"][0]["finishReason"]
                .as_str()
                .map(String::from),
        })
    }

    async fn health_check(&self) -> ArcResult<bool> {
        Ok(
            env_keys::get_credential_with_env_override(
                CredProvider::Gemini,
                CredentialKind::ApiKey,
            )
            .is_ok()
                || credentials::has_credential(
                    CredProvider::Gemini,
                    CredentialKind::OAuthAccessToken,
                ),
        )
    }
}
