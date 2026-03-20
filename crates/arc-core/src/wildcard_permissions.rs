//! Wildcard Tool Permissions — glob-style permission rules.
//! Supports patterns like Bash(npm *), Bash(*-h*), Read(src/**).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildcardPermission {
    pub tool: String,
    pub pattern: String,
    pub action: PermissionAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

pub struct WildcardPermissionEngine {
    pub rules: Vec<WildcardPermission>,
}

impl WildcardPermissionEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, tool: &str, pattern: &str, action: PermissionAction) {
        self.rules.push(WildcardPermission {
            tool: tool.to_string(),
            pattern: pattern.to_string(),
            action,
        });
    }

    /// Parse rules from Claude Code format: "Bash(npm *)" -> tool="Bash", pattern="npm *"
    pub fn parse_rule(rule_str: &str, action: PermissionAction) -> Option<WildcardPermission> {
        let open = rule_str.find('(')?;
        let close = rule_str.rfind(')')?;
        if close <= open {
            return None;
        }
        let tool = rule_str[..open].trim().to_string();
        let pattern = rule_str[open + 1..close].trim().to_string();
        Some(WildcardPermission {
            tool,
            pattern,
            action,
        })
    }

    /// Check if a tool invocation matches any permission rule
    pub fn check(&self, tool: &str, argument: &str) -> PermissionAction {
        for rule in &self.rules {
            if rule.tool != tool && rule.tool != "*" {
                continue;
            }
            if glob_match(&rule.pattern, argument) {
                return rule.action.clone();
            }
        }
        PermissionAction::Ask // Default: ask
    }

    /// Load rules from settings
    pub fn load_from_settings(&mut self, settings: &serde_json::Value) {
        if let Some(allow) = settings.get("allow").and_then(|v| v.as_array()) {
            for rule_val in allow {
                if let Some(s) = rule_val.as_str() {
                    if let Some(rule) = Self::parse_rule(s, PermissionAction::Allow) {
                        self.rules.push(rule);
                    }
                }
            }
        }
        if let Some(deny) = settings.get("deny").and_then(|v| v.as_array()) {
            for rule_val in deny {
                if let Some(s) = rule_val.as_str() {
                    if let Some(rule) = Self::parse_rule(s, PermissionAction::Deny) {
                        self.rules.push(rule);
                    }
                }
            }
        }
    }
}

/// Simple glob matching supporting * and ** wildcards
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('*').collect();
    if pattern_parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0;
    for (i, part) in pattern_parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            if i == 0 && found != 0 {
                return false; // Must match from start
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }

    // If pattern doesn't end with *, text must end at pos
    if !pattern.ends_with('*') && pos != text.len() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("npm *", "npm install"));
        assert!(glob_match("npm *", "npm run build"));
        assert!(!glob_match("npm *", "cargo build"));
        assert!(glob_match("*-h*", "cargo-help"));
        assert!(glob_match("src/**", "src/main.rs"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_parse_rule() {
        let rule =
            WildcardPermissionEngine::parse_rule("Bash(npm *)", PermissionAction::Allow).unwrap();
        assert_eq!(rule.tool, "Bash");
        assert_eq!(rule.pattern, "npm *");
    }
}
