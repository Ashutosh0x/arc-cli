// SPDX-License-Identifier: MIT
//! Unified OpenAI-compatible streaming provider.
//!
//! Works with any API that follows the OpenAI chat completions format:
//! - OpenAI (api.openai.com)
//! - Groq (api.groq.com)
//! - xAI Grok (api.x.ai)
//! - Any other OpenAI-compatible endpoint

use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};
use std::any::Any;
use tracing::{debug, error};

use crate::message::{Message, Role, StreamEvent, ToolDefinition};
use crate::stream::SseStream;
use crate::traits::Provider;

/// A streaming provider for any OpenAI-compatible API.
pub struct OpenAICompatProvider {
    http_client: Client,
    api_key: String,
    base_url: String,
    provider_name: &'static str,
}

impl OpenAICompatProvider {
    pub fn new(
        http_client: Client,
        api_key: String,
        base_url: String,
        provider_name: &'static str,
    ) -> Self {
        Self {
            http_client,
            api_key,
            base_url,
            provider_name,
        }
    }

    /// Create a Groq provider.
    pub fn groq(http_client: Client, api_key: String) -> Self {
        Self::new(
            http_client,
            api_key,
            "https://api.groq.com/openai/v1".to_string(),
            "groq",
        )
    }

    /// Create an xAI Grok provider.
    pub fn xai(http_client: Client, api_key: String) -> Self {
        Self::new(
            http_client,
            api_key,
            "https://api.x.ai/v1".to_string(),
            "xai",
        )
    }

    /// Create an OpenAI provider.
    pub fn openai(http_client: Client, api_key: String) -> Self {
        Self::new(
            http_client,
            api_key,
            "https://api.openai.com/v1".to_string(),
            "openai",
        )
    }

    fn construct_payload(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Value {
        let api_messages: Vec<Value> = messages
            .iter()
            .map(|m| match m.role {
                Role::System => json!({
                    "role": "system",
                    "content": m.content,
                }),
                Role::Tool => json!({
                    "role": "tool",
                    "tool_call_id": m.tool_call_id.as_deref().unwrap_or(""),
                    "content": m.content,
                }),
                Role::Assistant if !m.tool_calls.is_empty() => {
                    let tool_calls: Vec<Value> = m
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                    let mut msg = json!({
                        "role": "assistant",
                        "tool_calls": tool_calls,
                    });
                    if !m.content.is_empty() {
                        msg["content"] = json!(m.content);
                    }
                    msg
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
            })
            .collect();

        let mut payload = json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
            "max_tokens": 8192,
        });

        if !tools.is_empty() {
            let tools_payload: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            payload["tools"] = json!(tools_payload);
        }

        payload
    }
}

#[async_trait::async_trait]
impl Provider for OpenAICompatProvider {
    fn name(&self) -> &'static str {
        self.provider_name
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    fn models(&self) -> Vec<String> {
        match self.provider_name {
            "groq" => vec![
                "llama-3.3-70b-versatile".to_string(),
                "llama-3.1-8b-instant".to_string(),
                "mixtral-8x7b-32768".to_string(),
            ],
            "xai" => vec![
                "grok-4.20-0309-non-reasoning".to_string(),
                "grok-4-1-fast-non-reasoning".to_string(),
            ],
            "openai" => vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "o3-mini".to_string(),
            ],
            _ => vec![],
        }
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
        let endpoint = format!("{}/chat/completions", self.base_url);

        debug!(
            "[{}] Streaming to {} with {} tools",
            self.provider_name,
            endpoint,
            tools.len()
        );

        let response = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("[{}] API error [{}]: {}", self.provider_name, status, body);
            anyhow::bail!("[{}] API error [{}]: {}", self.provider_name, status, body);
        }

        let sse_stream = SseStream::new(response).filter_map(move |json_res| async move {
            match json_res {
                Ok(json_str) => {
                    if json_str.trim() == "[DONE]" {
                        return Some(Ok(StreamEvent::Done));
                    }
                    if let Ok(parsed) = serde_json::from_str::<Value>(&json_str) {
                        if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.first() {
                                let delta = choice.get("delta");
                                let finish_reason =
                                    choice.get("finish_reason").and_then(|v| v.as_str());

                                // Text delta
                                if let Some(d) = delta {
                                    if let Some(content) = d.get("content").and_then(|v| v.as_str())
                                    {
                                        if !content.is_empty() {
                                            return Some(Ok(StreamEvent::TextDelta(
                                                content.to_string(),
                                            )));
                                        }
                                    }

                                    // Tool call delta (OpenAI sends tool_calls in delta)
                                    if let Some(tool_calls) =
                                        d.get("tool_calls").and_then(|v| v.as_array())
                                    {
                                        for tc in tool_calls {
                                            if let Some(function) = tc.get("function") {
                                                let id = tc
                                                    .get("id")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                let name = function
                                                    .get("name")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string();
                                                if !name.is_empty() && !id.is_empty() {
                                                    // First chunk with ID+name
                                                    debug!("Tool call started: {} ({})", name, id);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Finish reason
                                if finish_reason == Some("stop") {
                                    return Some(Ok(StreamEvent::Done));
                                }
                                if finish_reason == Some("tool_calls") {
                                    return Some(Ok(StreamEvent::Done));
                                }
                            }
                        }
                    }
                    None
                },
                Err(e) => Some(Err(e)),
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
        let mut payload = self.construct_payload(model, messages, &[]);
        payload["stream"] = json!(false);

        let endpoint = format!("{}/chat/completions", self.base_url);

        let response = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("[{}] API error: {}", self.provider_name, body);
        }

        let body: Value = response.json().await?;
        let text = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        Ok(text)
    }
}

/// Make a non-streaming call to get complete tool calls from an OpenAI-compatible API.
pub async fn fetch_complete_response(
    http_client: &Client,
    api_key: &str,
    base_url: &str,
    model: &str,
    messages: &[Message],
    tools: &[ToolDefinition],
) -> Result<Value> {
    let provider = OpenAICompatProvider::new(
        http_client.clone(),
        api_key.to_string(),
        base_url.to_string(),
        "openai_compat",
    );
    let mut payload = provider.construct_payload(model, messages, tools);
    payload["stream"] = json!(false);

    let endpoint = format!("{}/chat/completions", base_url);

    let response = http_client
        .post(&endpoint)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("API error: {}", body);
    }

    let body: Value = response.json().await?;
    Ok(body)
}
