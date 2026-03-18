use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    /// Name of the tool as exposed to the LLM
    fn name(&self) -> &'static str;

    /// Description of what the tool does
    fn description(&self) -> &'static str;

    /// JSON Schema of the parameters the tool expects
    fn parameters(&self) -> Value;

    /// Execute the tool with the given JSON arguments
    async fn execute(&self, args: Value) -> Result<String, anyhow::Error>;
}
