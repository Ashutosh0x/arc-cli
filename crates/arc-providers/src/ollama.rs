//! Ollama local inference provider.

use crate::provider::*;
use arc_core::error::{ArcResult, ArcError};
use async_trait::async_trait;
use serde_json::json;

pub struct OllamaProvider {
    client: reqwest::Client,
    host: String,
}

impl OllamaProvider {
    pub fn new(client: reqwest::Client, host: String) -> Self {
        Self { client, host }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str { "ollama" }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_function_calling: false,
            supports_vision: false,
            max_context_window: 8_192,
        }
    }

    #[tracing::instrument(skip(self, messages), fields(provider = "ollama", model = %model), err)]
    async fn chat(&self, messages: &[ChatMessage], model: &str) -> ArcResult<ChatResponse> {
        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| json!({"role": m.role, "content": m.content}))
            .collect();

        let body = json!({
            "model": model,
            "messages": api_messages,
            "stream": false,
        });

        let url = format!("{}/api/chat", self.host);
        let resp = self.client
            .post(&url)
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|_| ArcError::Provider("Provider Unavailable".to_string()))?;

        let data: serde_json::Value = resp.json().await?;
        let content = data["message"]["content"].as_str().unwrap_or("").to_string();

        Ok(ChatResponse {
            content,
            model: model.to_string(),
            input_tokens: data["prompt_eval_count"].as_u64().map(|v| v as u32),
            output_tokens: data["eval_count"].as_u64().map(|v| v as u32),
            finish_reason: Some("stop".into()),
        })
    }

    async fn health_check(&self) -> ArcResult<bool> {
        let url = format!("{}/api/tags", self.host);
        match self.client.get(&url).timeout(std::time::Duration::from_secs(3)).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
