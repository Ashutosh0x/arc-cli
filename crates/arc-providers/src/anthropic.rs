use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::any::Any;
use tracing::{debug, error};

use crate::message::{Message, Role, StreamEvent, ToolDefinition};
use crate::stream::SseStream;
use crate::traits::Provider;

pub struct AnthropicProvider {
    http_client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(http_client: Client, api_key: String) -> Self {
        Self {
            http_client,
            api_key,
        }
    }

    fn construct_payload(&self, model: &str, messages: &[Message]) -> Value {
        // Extract system prompt out of messages map, as Anthropic demands it top-level
        let system_msg = messages.iter().find(|m| m.role == Role::System).map(|m| m.content.clone()).unwrap_or_default();
        
        // Use prompt caching on the system message
        let system_payload = if system_msg.is_empty() {
            json!([])
        } else {
            json!([
                {
                    "type": "text",
                    "text": system_msg,
                    "cache_control": {"type": "ephemeral"}
                }
            ])
        };
        
        let anthropic_msgs: Vec<Value> = messages.iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role_str = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "user", // Anthropic tool results are modeled differently, fallback to user for now
                    Role::System => "system",
                };
                json!({
                    "role": role_str,
                    "content": m.content,
                })
            }).collect();

        json!({
            "model": model,
            "max_tokens": 8192,
            "system": system_payload,
            "messages": anthropic_msgs,
            "stream": true,
        })
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    fn models(&self) -> Vec<String> {
        vec!["claude-3-5-sonnet-20241022".to_string(), "claude-3-opus-20240229".to_string()]
    }

    async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<futures::stream::BoxStream<'static, Result<StreamEvent, anyhow::Error>>, anyhow::Error> {
        let payload = self.construct_payload(model, messages);
        let endpoint = "https://api.anthropic.com/v1/messages";

        debug!("Connecting to Anthropic SSE endpoint");

        let response = self.http_client.post(endpoint)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "prompt-caching-2024-07-31")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Anthropic SSE Reject [{}]: {}", status, body);
            anyhow::bail!("Anthropic Reject [{}]: {}", status, body);
        }

        let sse_stream = SseStream::new(response)
            .filter_map(|json_res| async move {
                match json_res {
                    Ok(json_str) => {
                        if json_str == "[DONE]" {
                            return None;
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                            if let Some(delta) = parsed.get("delta") {
                                if let Some(text) = delta.get("text") {
                                    return Some(Ok(StreamEvent {
                                        text_delta: text.as_str().unwrap_or("").to_string(),
                                    }));
                                }
                            }
                        }
                        None
                    }
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(sse_stream))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
