// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalMode {
    /// Ask for permission before running tools that modify state (edit, create, shell)
    Ask,
    /// Auto-approve file edits/creates, ask for shell commands
    Auto,
    /// Auto-approve everything, no questions asked (dangerous!)
    Yolo,
    /// Strictly read-only; automatically deny any tool that attempts to modify state
    ReadOnly,
}

impl ApprovalMode {
    /// Returns true if the action requires explicit user prompt
    pub fn requires_prompt(&self, action_is_shell: bool) -> bool {
        match self {
            ApprovalMode::Ask => true,
            ApprovalMode::Auto => action_is_shell,
            ApprovalMode::Yolo => false,
            ApprovalMode::ReadOnly => false, // Will auto-deny instead of prompting
        }
    }

    /// Returns true if the action is outright blocked by this mode
    pub fn is_blocked(&self, action_is_read_only: bool) -> bool {
        if let ApprovalMode::ReadOnly = self {
            !action_is_read_only // Block if the action modifies state
        } else {
            false
        }
    }
}

impl Default for ApprovalMode {
    fn default() -> Self {
        ApprovalMode::Ask
    }
}
