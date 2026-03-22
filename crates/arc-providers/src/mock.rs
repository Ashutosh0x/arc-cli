use crate::message::{Message, StreamEvent, ToolDefinition};
use crate::traits::Provider;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct MockProvider {
    pub default_responses: Vec<String>,
    pub throw_error_on_next: bool,
    pub call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl MockProvider {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            default_responses: responses,
            throw_error_on_next: false,
            call_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn calls(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn health_check(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn models(&self) -> Vec<String> {
        vec!["mock-v1".to_string()]
    }

    async fn generate_text(&self, _model: &str, _messages: &[Message]) -> anyhow::Result<String> {
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if self.throw_error_on_next {
            return Err(anyhow::anyhow!("MockProvider artificial error triggered"));
        }

        if count < self.default_responses.len() {
            Ok(self.default_responses[count].clone())
        } else {
            Ok("Default Mock Response".to_string())
        }
    }

    async fn stream(
        &self,
        _model: &str,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<BoxStream<'static, Result<StreamEvent, anyhow::Error>>, anyhow::Error> {
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let text = if count < self.default_responses.len() {
            self.default_responses[count].clone()
        } else {
            "Default Mock".to_string()
        };

        if self.throw_error_on_next {
            return Err(anyhow::anyhow!("MockProvider artificial error triggered"));
        }

        let stream = futures::stream::iter(vec![Ok(StreamEvent::TextDelta(text))]);
        Ok(Box::pin(stream))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
