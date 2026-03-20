use crate::registry::AgentRegistry;
use anyhow::Result;
use serde_json::Value;

/// Determines which specialized agent is best suited for a given user prompt.
pub struct AgentRouter {
    registry: AgentRegistry,
    llm_client: std::sync::Arc<dyn arc_providers::streaming::StreamingClient>,
}

impl AgentRouter {
    pub fn new(llm_client: std::sync::Arc<dyn arc_providers::streaming::StreamingClient>) -> Self {
        Self {
            registry: AgentRegistry::new(),
            llm_client,
        }
    }

    /// Ask a lightweight model (e.g. Gemini Flash or Claude Haiku) which
    /// agent should handle the prompt. Returns the Agent ID.
    pub async fn route_prompt(&self, prompt: &str) -> Result<String> {
        let agents = self.registry.list();

        // Build JSON representation of available agents
        let mut options = Vec::new();
        for agent in agents {
            options.push(serde_json::json!({
                "id": agent.id,
                "name": agent.name,
                "description": agent.description,
            }));
        }

        let system_prompt = format!(
            "You are the ARC Router. You must select the single best agent to \
             handle the user's prompt based on these options:\n\n{}\n\n\
             Return ONLY a JSON object: {{\"selected_agent_id\": \"id_here\", \"reason\": \"...\"}}",
            serde_json::to_string_pretty(&options)?
        );

        let mut stream = self
            .llm_client
            .stream_completion(&system_prompt, prompt)
            .await?;

        let mut raw_json = String::new();
        while let Some(event) = stream.recv().await {
            if let arc_providers::streaming::StreamEvent::TextDelta(t) = event {
                raw_json.push_str(&t);
            }
        }

        let parsed: Value =
            serde_json::from_str(&raw_json).unwrap_or_else(|_| serde_json::json!({}));
        let id = parsed.get("selected_agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("generalist") // fallback
            .to_string();

        Ok(id)
    }
}
