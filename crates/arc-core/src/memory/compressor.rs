//! Context Window Compressor
//!
//! Responsible for shrinking the working memory when it nears the context limits.
//!
//! Compression pipeline:
//! 1. Identify oldest N messages eligible for eviction
//! 2. Run summarization (recursive if necessary)
//! 3. Extract core facts to Long-Term Memory (LTM)
//! 4. Update the compressed observation log block

use crate::error::ArcResult;
use crate::memory::working::MemoryMessage;

use tracing::{debug, info};

pub struct Compressor {}

impl Compressor {
    pub fn new() -> Self {
        Self {}
    }

    /// Compress a batch of raw messages into a concise summary string.
    ///
    /// In a production system, this would:
    /// 1. Take a chunk of oldest messages.
    /// 2. Use a "Reflector" LLM agent or a small local model to summarize them.
    /// 3. Output a dense bulleted list.
    ///
    /// Since we don't have the LLM Provider trait fully wired *into* the core memory yet,
    /// we use an algorithmic summarizer or placeholder for now.
    pub async fn compress(&self, raw_messages: &[MemoryMessage]) -> ArcResult<String> {
        let msg_count = raw_messages.len();
        if msg_count == 0 {
            return Ok(String::new());
        }

        debug!("Compressing {} messages...", msg_count);

        // Algorithm:
        // - Strip system prompts/tool calls if irrelevant.
        // - Truncate long content.
        // - Keep first and last sentences of large blocks.

        let mut summary = String::from("### Archived Event Summary\n");

        // Keep bounds
        let start_time = raw_messages
            .first()
            .map(|m| m.timestamp)
            .unwrap_or_default();
        let end_time = raw_messages.last().map(|m| m.timestamp).unwrap_or_default();

        summary.push_str(&format!(
            "- Period: {} to {}\n",
            start_time.format("%H:%M:%S"),
            end_time.format("%H:%M:%S")
        ));
        summary.push_str(&format!("- Exchanged {} messages.\n", msg_count));

        // Note: For an LLM-based compressor, we would invoke the provider here.
        // Example: `let prompt = format!("Summarize this: {raw_messages:?}"); client.chat(prompt).await;`

        // Algorithmic placeholder summary
        for msg in raw_messages
            .iter()
            .filter(|m| m.role == "user" || m.role == "assistant")
        {
            let role_label = if msg.role == "user" { "User" } else { "Agent" };

            // Take just the first 50 chars of each message to show high-level flow
            let truncated: String = msg.content.chars().take(50).collect();
            let suffix = if msg.content.len() > 50 { "..." } else { "" };

            summary.push_str(&format!("  - {}: {}{}\n", role_label, truncated, suffix));
        }

        info!(
            "Compressed {} messages down to {} chars",
            msg_count,
            summary.len()
        );

        Ok(summary)
    }

    /// Iterative hierarchical compression for massive overflow.
    /// If the old observation block + new summary is STILL too large, compress them together.
    pub async fn hierarchical_compress(
        &self,
        _existing_log: &str,
        new_summary: &str,
    ) -> ArcResult<String> {
        debug!("Performing hierarchical compression (merging old logs with new summary)");

        // Placeholder LLM logic
        let mut final_log = String::new();
        final_log.push_str("### Collapsed Historical Context\n");
        final_log.push_str("[... Previous older events ...]\n");
        final_log.push_str(new_summary);

        Ok(final_log)
    }
}
