// SPDX-License-Identifier: MIT
//! Prompt Registry & Snippet System
//!
//! Versioned prompt templates with variable substitution.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub version: u32,
    pub template: String,
    pub description: String,
    pub variables: Vec<String>,
}

impl PromptTemplate {
    pub fn render(&self, vars: &HashMap<String, String>) -> String {
        let mut output = self.template.clone();
        for (key, value) in vars {
            output = output.replace(&format!("{{{key}}}"), value);
        }
        output
    }
}

pub struct PromptRegistry {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            templates: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    pub fn register(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    pub fn render(&self, name: &str, vars: &HashMap<String, String>) -> Option<String> {
        self.get(name).map(|t| t.render(vars))
    }

    pub fn list(&self) -> Vec<&PromptTemplate> {
        self.templates.values().collect()
    }

    fn register_builtins(&mut self) {
        self.register(PromptTemplate {
            name: "system_default".to_string(),
            version: 1,
            template: "You are ARC, an elite AI coding assistant. You are working in {ide} on a {language} project. {context}".to_string(),
            description: "Default system prompt".to_string(),
            variables: vec!["ide".to_string(), "language".to_string(), "context".to_string()],
        });
        self.register(PromptTemplate {
            name: "commit_message".to_string(),
            version: 1,
            template: "Generate a concise git commit message for the following changes:\n\n{diff}\n\nFollow conventional commit format (feat/fix/refactor/docs/chore).".to_string(),
            description: "Git commit message generation".to_string(),
            variables: vec!["diff".to_string()],
        });
        self.register(PromptTemplate {
            name: "code_review".to_string(),
            version: 1,
            template: "Review the following code changes for bugs, security issues, and best practices:\n\n{diff}\n\nFile: {filename}\nLanguage: {language}".to_string(),
            description: "Code review prompt".to_string(),
            variables: vec!["diff".to_string(), "filename".to_string(), "language".to_string()],
        });
        self.register(PromptTemplate {
            name: "session_summary".to_string(),
            version: 1,
            template: "Summarize the user's primary intent in ONE sentence (max 80 chars).\n\nConversation:\n{conversation}\n\nSummary:".to_string(),
            description: "Session summarization".to_string(),
            variables: vec!["conversation".to_string()],
        });
        self.register(PromptTemplate {
            name: "loop_detection".to_string(),
            version: 1,
            template: "Analyze the conversation history to determine if the AI is stuck in a loop.\n\nOriginal request: {user_prompt}\n\nRecent history:\n{history}\n\nRespond with JSON: {{\"unproductive_state_analysis\": \"...\", \"unproductive_state_confidence\": 0.0-1.0}}".to_string(),
            description: "Loop detection LLM check".to_string(),
            variables: vec!["user_prompt".to_string(), "history".to_string()],
        });
        self.register(PromptTemplate {
            name: "conseca_policy".to_string(),
            version: 1,
            template: "Given the user request and available tools, generate a SecurityPolicy.\n\nUser request: {user_prompt}\nAvailable tools: {tools}\n\nOutput JSON with: allowed_tools, denied_patterns, restricted_paths, max_file_write_count".to_string(),
            description: "Conseca security policy generation".to_string(),
            variables: vec!["user_prompt".to_string(), "tools".to_string()],
        });
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
