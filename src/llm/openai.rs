// ARC CLI — OpenAI-compatible LLM Provider
// Works with OpenAI, Groq, xAI, and any OpenAI-compatible API.

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::models::LLMUsage;
use super::LLMProvider;

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
                .trim_end_matches('/')
                .to_string(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn check_health(&self) -> bool {
        let url = format!("{}/models", self.base_url);
        match self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
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
        let url = format!("{}/chat/completions", self.base_url);
        let start = std::time::Instant::now();

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "user", "content": prompt}
            ],
            "stream": true,
        });

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to OpenAI-compatible API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API returned {}: {}", status, body_text);
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut completion_tokens: u64 = 0;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));

                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();

                        let line = line.trim();
                        if line.is_empty() || !line.starts_with("data: ") {
                            continue;
                        }

                        let data = &line[6..];
                        if data == "[DONE]" {
                            let _ = tx.send("[DONE]".to_string());
                            let latency = start.elapsed().as_millis() as u64;
                            return Ok(LLMUsage {
                                model: model.to_string(),
                                provider: "OpenAI".to_string(),
                                prompt_tokens: 0, // not available in stream
                                completion_tokens,
                                total_tokens: completion_tokens,
                                latency_ms: latency,
                            });
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(delta) = json
                                .get("choices")
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("delta"))
                                .and_then(|d| d.get("content"))
                                .and_then(|v| v.as_str())
                            {
                                completion_tokens += 1;
                                let _ = tx.send(delta.to_string());
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
            provider: "OpenAI".to_string(),
            prompt_tokens: 0,
            completion_tokens,
            total_tokens: completion_tokens,
            latency_ms: latency,
        })
    }
}
