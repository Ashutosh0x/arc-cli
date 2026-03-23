// SPDX-License-Identifier: MIT
//! Dynamic Model Discovery Engine.
//! Fetches model lists from provider APIs in real-time.

use crate::credentials::{self, CredentialKind, Provider};
use crate::error::ArcResult;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// A discovered model from a provider API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub context_window: u64,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    /// Approximate cost per million input tokens (USD)
    pub input_cost_per_mtok: Option<f64>,
    /// Approximate cost per million output tokens (USD)
    pub output_cost_per_mtok: Option<f64>,
}

/// Result of a model discovery scan across all providers.
#[derive(Debug, Default)]
pub struct ModelRegistry {
    pub models: Vec<ModelInfo>,
}

impl ModelRegistry {
    /// Fetch models from all enabled providers concurrently.
    pub async fn discover_all(client: &reqwest::Client) -> Self {
        let mut registry = Self::default();

        let mut set = tokio::task::JoinSet::new();

        let c1 = client.clone();
        set.spawn(async move { fetch_anthropic_models(&c1).await });

        let c2 = client.clone();
        set.spawn(async move { fetch_openai_models(&c2).await });

        let c3 = client.clone();
        set.spawn(async move { fetch_gemini_models(&c3).await });

        let c4 = client.clone();
        set.spawn(async move { fetch_ollama_models(&c4).await });

        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(models)) => registry.models.extend(models),
                Ok(Err(e)) => warn!("Model discovery partial failure: {e}"),
                Err(e) => warn!("Model discovery task panicked: {e}"),
            }
        }

        // Sort by provider, then context_window descending
        registry.models.sort_by(|a, b| {
            a.provider
                .cmp(&b.provider)
                .then(b.context_window.cmp(&a.context_window))
        });

        info!("Discovered {} models total", registry.models.len());
        registry
    }

    /// Find a model by ID.
    pub fn find(&self, model_id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == model_id)
    }

    /// Filter models by provider.
    pub fn by_provider(&self, provider: &str) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }
}

/// Fetch models from Anthropic's API.
async fn fetch_anthropic_models(client: &reqwest::Client) -> ArcResult<Vec<ModelInfo>> {
    let key: String = match credentials::get_credential(Provider::Anthropic, CredentialKind::ApiKey)
    {
        Ok(k) => k.to_string(),
        Err(_) => return Ok(vec![]),
    };

    let resp = client
        .get("https://api.anthropic.com/v1/models")
        .header("x-api-key", key.as_str())
        .header("anthropic-version", "2024-01-01")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        debug!("Anthropic models API returned {}", resp.status());
        return Ok(default_anthropic_models());
    }

    #[derive(Deserialize)]
    struct AnthropicModelsResponse {
        data: Vec<AnthropicModel>,
    }
    #[derive(Deserialize)]
    struct AnthropicModel {
        id: String,
        display_name: Option<String>,
    }

    match resp.json::<AnthropicModelsResponse>().await {
        Ok(data) => Ok(data
            .data
            .into_iter()
            .map(|m| ModelInfo {
                display_name: m.display_name.clone().unwrap_or_else(|| m.id.clone()),
                id: m.id,
                provider: "anthropic".into(),
                context_window: 200_000,
                supports_streaming: true,
                supports_function_calling: true,
                supports_vision: true,
                input_cost_per_mtok: Some(3.0),
                output_cost_per_mtok: Some(15.0),
            })
            .collect()),
        Err(_) => Ok(default_anthropic_models()),
    }
}

/// Fetch models from OpenAI's API.
async fn fetch_openai_models(client: &reqwest::Client) -> ArcResult<Vec<ModelInfo>> {
    let key: String = match credentials::get_credential(Provider::OpenAI, CredentialKind::ApiKey) {
        Ok(k) => k.to_string(),
        Err(_) => return Ok(vec![]),
    };

    let resp = client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key.as_str())
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct OpenAIModelsResponse {
        data: Vec<OpenAIModel>,
    }
    #[derive(Deserialize)]
    struct OpenAIModel {
        id: String,
    }

    match resp.json::<OpenAIModelsResponse>().await {
        Ok(data) => Ok(data
            .data
            .into_iter()
            .filter(|m| {
                m.id.starts_with("gpt-")
                    || m.id.starts_with("o1")
                    || m.id.starts_with("o3")
                    || m.id.starts_with("o4")
            })
            .map(|m| {
                let ctx = if m.id.contains("gpt-4o") {
                    128_000
                } else {
                    128_000
                };
                ModelInfo {
                    display_name: m.id.clone(),
                    id: m.id,
                    provider: "openai".into(),
                    context_window: ctx,
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_vision: true,
                    input_cost_per_mtok: Some(2.5),
                    output_cost_per_mtok: Some(10.0),
                }
            })
            .collect()),
        Err(_) => Ok(vec![]),
    }
}

/// Fetch models from Google Gemini API.
async fn fetch_gemini_models(client: &reqwest::Client) -> ArcResult<Vec<ModelInfo>> {
    let key: String = match credentials::get_credential(Provider::Gemini, CredentialKind::ApiKey)
        .or_else(|_| {
            credentials::get_credential(Provider::Gemini, CredentialKind::OAuthAccessToken)
        }) {
        Ok(k) => k.to_string(),
        Err(_) => return Ok(vec![]),
    };

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        key.as_str()
    );

    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    #[derive(Deserialize)]
    struct GeminiModelsResponse {
        models: Option<Vec<GeminiModel>>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GeminiModel {
        name: String,
        display_name: Option<String>,
        input_token_limit: Option<u64>,
    }

    match resp.json::<GeminiModelsResponse>().await {
        Ok(data) => Ok(data
            .models
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.name.contains("gemini"))
            .map(|m| {
                let id = m.name.replace("models/", "");
                ModelInfo {
                    display_name: m.display_name.unwrap_or_else(|| id.clone()),
                    id,
                    provider: "gemini".into(),
                    context_window: m.input_token_limit.unwrap_or(1_000_000),
                    supports_streaming: true,
                    supports_function_calling: true,
                    supports_vision: true,
                    input_cost_per_mtok: Some(0.075),
                    output_cost_per_mtok: Some(0.30),
                }
            })
            .collect()),
        Err(_) => Ok(vec![]),
    }
}

/// Fetch models from local Ollama instance.
async fn fetch_ollama_models(client: &reqwest::Client) -> ArcResult<Vec<ModelInfo>> {
    let resp = client
        .get("http://localhost:11434/api/tags")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(_) => return Ok(vec![]),
    };

    #[derive(Deserialize)]
    struct OllamaTagsResponse {
        models: Option<Vec<OllamaModel>>,
    }
    #[derive(Deserialize)]
    struct OllamaModel {
        name: String,
    }

    match resp.json::<OllamaTagsResponse>().await {
        Ok(data) => Ok(data
            .models
            .unwrap_or_default()
            .into_iter()
            .map(|m| ModelInfo {
                display_name: m.name.clone(),
                id: m.name,
                provider: "ollama".into(),
                context_window: 8_192,
                supports_streaming: true,
                supports_function_calling: false,
                supports_vision: false,
                input_cost_per_mtok: Some(0.0),
                output_cost_per_mtok: Some(0.0),
            })
            .collect()),
        Err(_) => Ok(vec![]),
    }
}

/// Fallback list if Anthropic API is unreachable.
fn default_anthropic_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-sonnet-4-20250514".into(),
            provider: "anthropic".into(),
            display_name: "Claude Sonnet 4".into(),
            context_window: 200_000,
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            input_cost_per_mtok: Some(3.0),
            output_cost_per_mtok: Some(15.0),
        },
        ModelInfo {
            id: "claude-3-5-haiku-20241022".into(),
            provider: "anthropic".into(),
            display_name: "Claude 3.5 Haiku".into(),
            context_window: 200_000,
            supports_streaming: true,
            supports_function_calling: true,
            supports_vision: true,
            input_cost_per_mtok: Some(0.80),
            output_cost_per_mtok: Some(4.0),
        },
    ]
}
