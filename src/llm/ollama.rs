// ARC CLI — Ollama LLM Provider (real streaming with usage tracking)
// Optimized: connection pooling, keep_alive, token limits, fast timeouts.

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::models::LLMUsage;
use super::LLMProvider;

pub struct OllamaProvider {
    base_url: String,
    client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let base = base_url
            .or_else(|| std::env::var("OLLAMA_HOST").ok())
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        // Pre-configured client with connection pooling and fast timeouts
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .pool_idle_timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(4)
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url: base.trim_end_matches('/').to_string(),
            client,
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &str {
        "Ollama"
    }

    async fn check_health(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn generate(
        &self,
        prompt: &str,
        model: &str,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<LLMUsage> {
        let url = format!("{}/api/generate", self.base_url);
        let start = std::time::Instant::now();

        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": true,
            "keep_alive": "10m",
            "options": {
                "num_predict": 1024,
                "num_ctx": 4096,
                "temperature": 0.3
            }
        });

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama returned {}: {}", status, body_text);
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut total_tokens: u64 = 0;
        let mut prompt_tokens: u64 = 0;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    // Process complete JSON lines
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if line.trim().is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(token) = json.get("response").and_then(|v| v.as_str()) {
                                total_tokens += 1;
                                let _ = tx.send(token.to_string());
                            }

                            // Extract usage from final response
                            if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                                if let Some(pt) = json.get("prompt_eval_count").and_then(|v| v.as_u64()) {
                                    prompt_tokens = pt;
                                }
                                if let Some(et) = json.get("eval_count").and_then(|v| v.as_u64()) {
                                    total_tokens = et;
                                }
                                let _ = tx.send("[DONE]".to_string());

                                let latency = start.elapsed().as_millis() as u64;
                                return Ok(LLMUsage {
                                    model: model.to_string(),
                                    provider: "Ollama".to_string(),
                                    prompt_tokens,
                                    completion_tokens: total_tokens,
                                    total_tokens: prompt_tokens + total_tokens,
                                    latency_ms: latency,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(format!("\n[STREAM ERROR] {}\n", e));
                    break;
                }
            }
        }

        let _ = tx.send("[DONE]".to_string());
        let latency = start.elapsed().as_millis() as u64;

        Ok(LLMUsage {
            model: model.to_string(),
            provider: "Ollama".to_string(),
            prompt_tokens,
            completion_tokens: total_tokens,
            total_tokens: prompt_tokens + total_tokens,
            latency_ms: latency,
        })
    }
}
