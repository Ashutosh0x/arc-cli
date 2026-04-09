// ARC CLI — LLM Provider abstraction
// Pluggable provider system with routing and health checks.
// The router is the SINGLE source of truth for provider selection.

pub mod ollama;
pub mod openai;

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::models::LLMUsage;

// =====================================================================
//  LLMProvider trait — each backend implements this
// =====================================================================

#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Human-readable name (e.g. "Ollama", "OpenAI")
    fn name(&self) -> &str;

    /// Check if the provider is reachable
    async fn check_health(&self) -> bool;

    /// Stream a generation. Tokens sent through `tx`.
    /// Returns usage stats on completion.
    async fn generate(
        &self,
        prompt: &str,
        model: &str,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<LLMUsage>;
}

// =====================================================================
//  LLM Router — single source of truth for provider selection
// =====================================================================

pub struct LLMRouter {
    pub ollama: ollama::OllamaProvider,
    pub openai: Option<openai::OpenAIProvider>,
}

impl LLMRouter {
    pub fn new() -> Self {
        Self::with_host(None)
    }

    /// Create router with a specific Ollama host URL.
    pub fn with_host(ollama_host: Option<String>) -> Self {
        let ollama = ollama::OllamaProvider::new(ollama_host);

        // OpenAI provider only if API key is set
        let openai = std::env::var("ARC_OPENAI_KEY").ok().map(|key| {
            let base_url = std::env::var("ARC_OPENAI_BASE")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
            openai::OpenAIProvider::new(key, Some(base_url))
        });

        Self { ollama, openai }
    }

    /// Get an Arc'd provider for the given model tag.
    /// This is the ONLY method that should be used to select a provider.
    ///
    /// Routing rules:
    /// - "Local" / "OSS" → Ollama (always)
    /// - "Premium" / "Fast" → OpenAI (if API key set), otherwise ERROR (no silent fallback)
    /// - Unknown → Ollama
    pub fn get_provider(&self, tag: &str) -> std::result::Result<Arc<dyn LLMProvider>, String> {
        match tag {
            "Premium" | "Fast" => {
                if self.openai.is_some() {
                    // Create a new Arc'd OpenAI provider
                    let key = std::env::var("ARC_OPENAI_KEY").unwrap_or_default();
                    let base = std::env::var("ARC_OPENAI_BASE")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                    Ok(Arc::new(openai::OpenAIProvider::new(key, Some(base))))
                } else {
                    Err(format!(
                        "Selected model requires OpenAI API key. Set ARC_OPENAI_KEY environment variable."
                    ))
                }
            }
            _ => {
                // Local, OSS, or unknown → Ollama
                Ok(Arc::new(ollama::OllamaProvider::new(None)))
            }
        }
    }

    /// Health check all configured providers, return status map.
    pub async fn health_check(&self) -> Vec<(&str, bool)> {
        let mut results = vec![];

        let ollama_ok = self.ollama.check_health().await;
        results.push(("Ollama", ollama_ok));

        if let Some(ref openai) = self.openai {
            let openai_ok = openai.check_health().await;
            results.push(("OpenAI", openai_ok));
        }

        results
    }
}
