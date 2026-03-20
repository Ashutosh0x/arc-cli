//! Session Summary Service
//!
//! Auto-generates 1-line session titles (≤80 chars) using a sliding window
//! of first N + last N messages.

use compact_str::CompactString;

const DEFAULT_MAX_MESSAGES: usize = 20;
const MAX_MESSAGE_LENGTH: usize = 500;

#[derive(Debug, Clone)]
pub struct SessionMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// Generate a prompt for LLM-based session summarization.
pub fn build_summary_prompt(messages: &[SessionMessage]) -> Option<String> {
    let filtered: Vec<&SessionMessage> = messages
        .iter()
        .filter(|m| m.role != MessageRole::System && !m.content.trim().is_empty())
        .collect();

    if filtered.is_empty() {
        return None;
    }

    // Sliding window: first N/2 + last N/2
    let relevant = if filtered.len() <= DEFAULT_MAX_MESSAGES {
        filtered
    } else {
        let first_size = DEFAULT_MAX_MESSAGES / 2;
        let last_size = DEFAULT_MAX_MESSAGES - first_size;
        let mut selected = Vec::with_capacity(DEFAULT_MAX_MESSAGES);
        selected.extend_from_slice(&filtered[..first_size]);
        selected.extend_from_slice(&filtered[filtered.len() - last_size..]);
        selected
    };

    let conversation_text: String = relevant
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::System => "System",
            };
            let truncated = if m.content.len() > MAX_MESSAGE_LENGTH {
                format!("{}...", &m.content[..MAX_MESSAGE_LENGTH])
            } else {
                m.content.clone()
            };
            format!("{role}: {truncated}")
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    Some(format!(
        r#"Summarize the user's primary intent or goal in this conversation in ONE sentence (max 80 characters).
Focus on what the user was trying to accomplish.

Examples:
- "Add dark mode to the app"
- "Fix authentication bug in login flow"
- "Refactor database connection logic"

Conversation:
{conversation_text}

Summary (max 80 chars):"#
    ))
}

/// Clean an LLM-generated summary.
pub fn clean_summary(raw: &str) -> CompactString {
    let cleaned = raw
        .replace('\n', " ")
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();

    // Collapse multiple spaces
    let collapsed: String = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    // Truncate to 80 chars
    if collapsed.len() > 80 {
        CompactString::new(&collapsed[..80])
    } else {
        CompactString::new(&collapsed)
    }
}
