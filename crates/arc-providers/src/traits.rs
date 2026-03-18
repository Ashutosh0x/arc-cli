use async_trait::async_trait;
use futures::stream::BoxStream;
use crate::message::{Message, ToolDefinition, StreamEvent};
use std::any::Any;

#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the name of the provider (e.g., "google", "groq")
    fn name(&self) -> &'static str;

    /// Checks if the provider considers itself healthy (e.g. valid API key)
    async fn health_check(&self) -> Result<(), anyhow::Error>;

    /// Returns the models this provider supports
    fn models(&self) -> Vec<String>;

    /// Executes a streaming completion
    async fn stream(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<BoxStream<'static, Result<StreamEvent, anyhow::Error>>, anyhow::Error>;

    /// Downcast to concrete type for specific interactions
    fn as_any(&self) -> &dyn Any;
}
