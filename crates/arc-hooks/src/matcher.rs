// SPDX-License-Identifier: MIT
//! Hook matching: determines which hooks fire for a given event.
//! Supports exact event name matching + regex-based tool name filtering.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::events::HookEvent;

/// Defines which events a hook should respond to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    /// The event name to match (e.g., "PreToolUse", "Stop").
    pub event: String,

    /// Optional regex pattern to match against the tool name.
    /// Only applies to tool-related events (PreToolUse, PostToolUse, etc.).
    /// Example: "^(bash|shell)$" matches only bash and shell tools.
    #[serde(default)]
    pub tool_pattern: Option<String>,

    /// Compiled regex (populated at load time, not serialized).
    #[serde(skip)]
    compiled_pattern: Option<Regex>,
}

impl HookMatcher {
    pub fn new(event: impl Into<String>, tool_pattern: Option<String>) -> Self {
        let tool_pattern_str = tool_pattern.clone();
        let compiled = tool_pattern_str
            .as_ref()
            .and_then(|p| Regex::new(p).ok());

        Self {
            event: event.into(),
            tool_pattern,
            compiled_pattern: compiled,
        }
    }

    /// Compile the regex pattern. Must be called after deserialization.
    pub fn compile(&mut self) -> Result<(), regex::Error> {
        if let Some(ref pattern) = self.tool_pattern {
            self.compiled_pattern = Some(Regex::new(pattern)?);
        }
        Ok(())
    }

    /// Check if this matcher applies to the given event.
    pub fn matches(&self, event: &HookEvent) -> bool {
        // First: event name must match
        if self.event != event.event_name() {
            return false;
        }

        // Second: if a tool pattern is specified, the event must have a tool name that matches
        if let Some(ref compiled) = self.compiled_pattern {
            match event.tool_name() {
                Some(tool_name) => compiled.is_match(tool_name),
                None => false, // Tool pattern specified but event has no tool name
            }
        } else {
            true // No tool filter, event name match is sufficient
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::*;
    use uuid::Uuid;

    #[test]
    fn test_exact_event_match() {
        let matcher = HookMatcher::new("SessionStart", None);
        let event = HookEvent::SessionStart(SessionStartPayload {
            session_id: Uuid::new_v4(),
            working_directory: "/tmp".into(),
            provider: "anthropic".into(),
            model: "claude-sonnet".into(),
            timestamp: chrono::Utc::now(),
            project_rules: vec![],
        });
        assert!(matcher.matches(&event));
    }

    #[test]
    fn test_tool_pattern_match() {
        let matcher = HookMatcher::new("PreToolUse", Some("^bash$".into()));
        let event = HookEvent::PreToolUse(PreToolUsePayload {
            session_id: Uuid::new_v4(),
            tool_name: "bash".into(),
            tool_input: serde_json::json!({}),
            target_path: None,
            command: Some("ls -la".into()),
        });
        assert!(matcher.matches(&event));
    }

    #[test]
    fn test_tool_pattern_no_match() {
        let matcher = HookMatcher::new("PreToolUse", Some("^bash$".into()));
        let event = HookEvent::PreToolUse(PreToolUsePayload {
            session_id: Uuid::new_v4(),
            tool_name: "file_write".into(),
            tool_input: serde_json::json!({}),
            target_path: Some("/tmp/foo.rs".into()),
            command: None,
        });
        assert!(!matcher.matches(&event));
    }
}
