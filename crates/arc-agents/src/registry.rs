// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub capabilities: Vec<AgentCapability>,
    pub required_mcp_servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentCapability {
    CodeReview,
    Testing,
    SecurityAudit,
    DatabaseDesign,
    FrontendDesign,
    DevOps,
    Documentation,
}

/// A registry that loads and stores the profiles of available specialized sub-agents.
pub struct AgentRegistry {
    profiles: HashMap<String, AgentProfile>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            profiles: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    pub fn get(&self, id: &str) -> Option<&AgentProfile> {
        self.profiles.get(id)
    }

    pub fn list(&self) -> Vec<&AgentProfile> {
        self.profiles.values().collect()
    }

    pub fn find_by_capability(&self, cap: &AgentCapability) -> Vec<&AgentProfile> {
        self.profiles
            .values()
            .filter(|p| p.capabilities.contains(cap))
            .collect()
    }

    fn register_builtins(&mut self) {
        // 1. Review Agent
        self.profiles.insert(
            "reviewer".to_string(),
            AgentProfile {
                id: "reviewer".to_string(),
                name: "Code Reviewer".to_string(),
                description: "Scans code for logic errors, formatting, and best practices.".to_string(),
                system_prompt: "You are an expert Review Agent. Your job is to read code and provide actionable feedback. You do NOT make changes, you only comment.".to_string(),
                capabilities: vec![AgentCapability::CodeReview],
                required_mcp_servers: vec![],
            },
        );

        // 2. Test Engineer
        self.profiles.insert(
            "tester".to_string(),
            AgentProfile {
                id: "tester".to_string(),
                name: "Test Engineer".to_string(),
                description: "Generates high quality unit and integration tests.".to_string(),
                system_prompt: "You are the Test Engineer. You analyze functions and generate comprehensive Rust unit tests covering edge cases.".to_string(),
                capabilities: vec![AgentCapability::Testing],
                required_mcp_servers: vec![],
            },
        );

        // 3. Security Auditor
        self.profiles.insert(
            "security".to_string(),
            AgentProfile {
                id: "security".to_string(),
                name: "Security Auditor".to_string(),
                description: "Analyzes code and dependencies for security vulnerabilities (e.g. unsafe blocks, OWASP top 10).".to_string(),
                system_prompt: "You are a Security Auditor. Scan for unsafe code, injection vectors, and logical flaws.".to_string(),
                capabilities: vec![AgentCapability::SecurityAudit],
                required_mcp_servers: vec![],
            },
        );
    }
}
