// SPDX-License-Identifier: MIT
use anyhow::Result;
use arc_providers::message::{Message, Role};
use arc_providers::traits::Provider;
use std::sync::Arc;
use tracing::info;

pub struct Summarizer {
    provider: Arc<dyn Provider>,
}

impl Summarizer {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    /// Summarize older messages using the LLM provider to maintain context density
    pub async fn summarize_history(&self, messages: &[Message]) -> Result<String> {
        let history_text = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "Summarize the following conversation history. Retain key facts, decision points, and user constraints. Omit pleasantries and redundant thoughts.\n\nHistory:\n{}",
            history_text
        );

        let msg = Message {
            role: Role::User,
            content: prompt,
            tool_calls: vec![],
            tool_call_id: None,
        };
        let summary: String = self.provider.generate_text("default", &[msg]).await?;
        info!(
            "History summarized. Compressed {} messages into {} characters.",
            messages.len(),
            summary.len()
        );
        Ok(summary)
    }
}
