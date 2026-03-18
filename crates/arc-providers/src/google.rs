use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use std::any::Any;

use crate::message::{Message, StreamEvent, ToolDefinition};
use crate::traits::Provider;

pub struct GoogleProvider {
    api_key: String,
    models: Vec<String>,
    _client: Client,
}

impl GoogleProvider {
    pub fn new(api_key: String, models: Vec<String>) -> Self {
        Self {
            api_key,
            models,
            _client: Client::new(),
        }
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    async fn health_check(&self) -> Result<(), anyhow::Error> {
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("Google API key is missing"));
        }
        // Could also do a lightweight models list check here
        Ok(())
    }

    fn models(&self) -> Vec<String> {
        self.models.clone()
    }

    async fn stream(
        &self,
        _model: &str,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<BoxStream<'static, Result<StreamEvent, anyhow::Error>>, anyhow::Error> {
        // Real implementation would connect to Google AI Studio SSE
        Err(anyhow::anyhow!("Google stream not implemented yet"))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
