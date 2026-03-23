// SPDX-License-Identifier: MIT
use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
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

    fn construct_payload(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Value {
        // Extract system prompt — Anthropic demands it top-level
        let system_msg = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone())
            .unwrap_or_default();

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

        // Build Anthropic-format messages
        let anthropic_msgs: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                match m.role {
                    Role::Tool => {
                        // Tool results go as user messages with tool_result content blocks
                        json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": m.tool_call_id.as_deref().unwrap_or(""),
                                "content": m.content,
                            }]
                        })
                    },
                    Role::Assistant if !m.tool_calls.is_empty() => {
                        // Assistant messages that generated tool calls
                        let mut content_blocks: Vec<Value> = Vec::new();
                        if !m.content.is_empty() {
                            content_blocks.push(json!({
                                "type": "text",
                                "text": m.content,
                            }));
                        }
                        for tc in &m.tool_calls {
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments,
                            }));
                        }
                        json!({
                            "role": "assistant",
                            "content": content_blocks,
                        })
                    },
                    _ => {
                        let role_str = match m.role {
                            Role::User => "user",
                            Role::Assistant => "assistant",
                            _ => "user",
                        };
                        json!({
                            "role": role_str,
                            "content": m.content,
                        })
                    },
                }
            })
            .collect();

        let mut payload = json!({
            "model": model,
            "max_tokens": 8192,
            "system": system_payload,
            "messages": anthropic_msgs,
            "stream": true,
        });

        // Include tool definitions if any
        if !tools.is_empty() {
            let tools_payload: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            payload["tools"] = json!(tools_payload);
        }

        payload
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
        vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
        ]
    }

    async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<
        futures::stream::BoxStream<'static, Result<StreamEvent, anyhow::Error>>,
        anyhow::Error,
    > {
        let payload = self.construct_payload(model, messages, tools);
        let endpoint = "https://api.anthropic.com/v1/messages";

        debug!(
            "Connecting to Anthropic SSE endpoint with {} tools",
            tools.len()
        );

        let response = self
            .http_client
            .post(endpoint)
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

        // Track current tool_use block being built across SSE chunks
        let sse_stream = SseStream::new(response).filter_map(move |json_res| {
            // Use thread-local state for tool_use accumulation
            // We use a simple approach: accumulate JSON input fragments
            async move {
                match json_res {
                    Ok(json_str) => {
                        if json_str == "[DONE]" {
                            return Some(Ok(StreamEvent::Done));
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                            let event_type = parsed.get("type").and_then(|v| v.as_str());

                            match event_type {
                                // Text deltas
                                Some("content_block_delta") => {
                                    if let Some(delta) = parsed.get("delta") {
                                        let delta_type = delta.get("type").and_then(|v| v.as_str());
                                        match delta_type {
                                            Some("text_delta") => {
                                                if let Some(text) = delta.get("text") {
                                                    return Some(Ok(StreamEvent::TextDelta(
                                                        text.as_str().unwrap_or("").to_string(),
                                                    )));
                                                }
                                            },
                                            // input_json_delta is handled at content_block_stop
                                            _ => {},
                                        }
                                    }
                                },
                                // Tool use — Anthropic sends the full tool_use in content_block_stop
                                // but we can also detect it from content_block_start
                                Some("content_block_start") => {
                                    if let Some(content_block) = parsed.get("content_block") {
                                        if content_block.get("type").and_then(|v| v.as_str())
                                            == Some("tool_use")
                                        {
                                            let id = content_block
                                                .get("id")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            let name = content_block
                                                .get("name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            // Input comes in subsequent deltas, but for now
                                            // we'll accumulate and emit at message_delta/stop
                                            debug!("Tool use block started: {} ({})", name, id);
                                        }
                                    }
                                },
                                // message_delta with stop_reason=tool_use means we need to
                                // re-fetch the full message to get complete tool inputs
                                Some("message_delta") => {
                                    if let Some(delta) = parsed.get("delta") {
                                        if delta.get("stop_reason").and_then(|v| v.as_str())
                                            == Some("tool_use")
                                        {
                                            return Some(Ok(StreamEvent::Done));
                                        }
                                    }
                                },
                                Some("message_stop") => {
                                    return Some(Ok(StreamEvent::Done));
                                },
                                _ => {},
                            }
                        }
                        None
                    },
                    Err(e) => Some(Err(e)),
                }
            }
        });

        Ok(Box::pin(sse_stream))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn generate_text(
        &self,
        model: &str,
        messages: &[Message],
    ) -> Result<String, anyhow::Error> {
        let payload = self.construct_payload(model, messages, &[]);

        // Non-streaming endpoint
        let mut non_stream_payload = payload.clone();
        non_stream_payload["stream"] = json!(false);

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&non_stream_payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error: {}", body);
        }

        let body: Value = response.json().await?;
        let text = body
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

/// Make a non-streaming call to get complete tool_use blocks with full input JSON.
/// This is needed because streaming only gives us partial JSON deltas for tool inputs.
pub async fn fetch_complete_response(
    http_client: &Client,
    api_key: &str,
    model: &str,
    messages: &[Message],
    tools: &[ToolDefinition],
) -> Result<Value> {
    let provider = AnthropicProvider::new(http_client.clone(), api_key.to_string());
    let mut payload = provider.construct_payload(model, messages, tools);
    payload["stream"] = json!(false);

    let response = http_client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic API error: {}", body);
    }

    let body: Value = response.json().await?;
    Ok(body)
}
