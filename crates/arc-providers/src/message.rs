use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Events emitted during streaming completion.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A chunk of text from the assistant.
    TextDelta(String),
    /// The assistant wants to call a tool.
    ToolUse {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Stream is complete.
    Done,
}

impl StreamEvent {
    /// Helper for legacy code expecting `.text_delta`.
    pub fn text_delta(&self) -> &str {
        match self {
            StreamEvent::TextDelta(s) => s.as_str(),
            _ => "",
        }
    }
}
