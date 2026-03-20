use crate::registry::AgentProfile;
use anyhow::Result;
use arc_providers::streaming::{StreamEvent, StreamingClient};
use std::sync::Arc;


/// The result returned by a specialized SubAgent after completing its task.
pub struct SubAgentResult {
    pub agent_id: String,
    pub task: String,
    pub output: String,
    pub execution_time_ms: u64,
}

pub struct SubAgent {
    profile: AgentProfile,
    client: Arc<dyn StreamingClient>,
    context_files: Vec<String>,
}

impl SubAgent {
    pub fn new(profile: AgentProfile, client: Arc<dyn StreamingClient>) -> Self {
        Self {
            profile,
            client,
            context_files: Vec::new(),
        }
    }

    pub fn add_context_file(&mut self, path: String) {
        self.context_files.push(path);
    }

    pub async fn execute_task(&self, task: &str) -> Result<SubAgentResult> {
        let start = std::time::Instant::now();

        // 1. Build context from files
        let mut context_body = String::new();
        for file in &self.context_files {
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                context_body.push_str(&format!("\n--- File: {} ---\n{}\n", file, content));
            }
        }

        // 2. Prepare prompts
        let system_prompt = format!(
            "{}\n\nYou have access to the following context files:\n{}",
            self.profile.system_prompt, context_body
        );

        let user_prompt = format!("Please complete the following task:\n\n{}", task);

        // 3. Execute LLM call
        let mut full_response = String::new();
        let mut stream = self
            .client
            .stream_completion(&system_prompt, &user_prompt)
            .await?;

        while let Some(event) = stream.recv().await {
            match event {
                StreamEvent::TextDelta(text) => {
                    full_response.push_str(&text);
                }
                StreamEvent::Done => break,
                StreamEvent::Error(e) => {
                    anyhow::bail!("SubAgent '{}' failed: {}", self.profile.name, e);
                }

            }
        }

        Ok(SubAgentResult {
            agent_id: self.profile.id.clone(),
            task: task.to_string(),
            output: full_response,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}
