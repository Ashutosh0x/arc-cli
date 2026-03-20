use crate::engine::PolicyViolation;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSeverity {
    /// Action is completely blocked
    Deny,
    /// Action is allowed, but an alert/warning is triggered
    Warn,
}

#[derive(Debug, Clone)]
pub enum PolicyRule {
    /// Prevent the agent from running `git push --force` or `-f`
    NoForcePush,
    /// Prevent reading .env files
    NoEnvFileRead,
    /// General regex matching for commands
    ForbiddenCommandPattern {
        name: String,
        regex_pattern: String,
        severity: RuleSeverity,
    },
    /// Custom path blocker
    ForbiddenPath {
        name: String,
        glob_pattern: String,
        is_write_only: bool,
    },
    /// Custom constraint that changes in `src/core` require adjacent test file edits
    RequireTestsForCorePaths,
}

impl PolicyRule {
    pub fn check_command(&self, command: &str) -> Option<PolicyViolation> {
        match self {
            PolicyRule::NoForcePush => {
                if command.contains("git push")
                    && (command.contains("--force") || command.contains("-f"))
                {
                    Some(PolicyViolation {
                        rule_name: "NoForcePush".to_string(),
                        severity: RuleSeverity::Deny,
                        message: "Agent is not allowed to force push to remote repositories."
                            .to_string(),
                    })
                } else {
                    None
                }
            },
            PolicyRule::ForbiddenCommandPattern {
                name,
                regex_pattern,
                severity,
            } => {
                // In production, instantiate `regex::Regex` properly
                if command.contains(regex_pattern) {
                    Some(PolicyViolation {
                        rule_name: name.clone(),
                        severity: severity.clone(),
                        message: format!("Command matches forbidden pattern: {}", regex_pattern),
                    })
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    pub fn check_file_access(&self, path: &PathBuf, is_write: bool) -> Option<PolicyViolation> {
        let path_str = path.to_string_lossy();

        match self {
            PolicyRule::NoEnvFileRead => {
                if path_str.contains(".env") {
                    Some(PolicyViolation {
                        rule_name: "NoEnvFileRead".to_string(),
                        severity: RuleSeverity::Deny,
                        message: "Agent is forbidden from accessing .env files.".to_string(),
                    })
                } else {
                    None
                }
            },
            PolicyRule::ForbiddenPath {
                name,
                glob_pattern,
                is_write_only,
            } => {
                if *is_write_only && !is_write {
                    return None;
                }

                // Extremely naive glob matching for demo purposes
                let matches = if glob_pattern.ends_with("/*") {
                    let prefix = glob_pattern.trim_end_matches("/*");
                    path_str.starts_with(prefix)
                } else {
                    path_str == *glob_pattern
                };

                if matches {
                    Some(PolicyViolation {
                        rule_name: name.clone(),
                        severity: RuleSeverity::Deny,
                        message: format!(
                            "Agent is forbidden from accessing path matching {}",
                            glob_pattern
                        ),
                    })
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}
