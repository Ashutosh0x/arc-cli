//! # Tool Search — Deferred Tool Loading via ToolSearch
//!
//! Reduces initial context by loading tool schemas on demand.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether this tool is loaded eagerly or deferred.
    pub deferred: bool,
}

pub struct ToolSearchEngine {
    /// All registered tools.
    registry: HashMap<String, ToolSchema>,
    /// Tools currently loaded in context.
    loaded: Vec<String>,
    /// Maximum tools to include in initial context.
    max_initial: usize,
}

impl ToolSearchEngine {
    pub fn new(max_initial: usize) -> Self {
        Self { registry: HashMap::new(), loaded: Vec::new(), max_initial }
    }

    pub fn register(&mut self, tool: ToolSchema) {
        let name = tool.name.clone();
        self.registry.insert(name.clone(), tool);
    }

    /// Get tools for initial context (non-deferred only, up to max).
    pub fn initial_tools(&mut self) -> Vec<&ToolSchema> {
        self.loaded.clear();
        let tools: Vec<&ToolSchema> = self.registry.values()
            .filter(|t| !t.deferred)
            .take(self.max_initial)
            .collect();
        for t in &tools { self.loaded.push(t.name.clone()); }
        tools
    }

    /// Search for a tool by name or description (deferred loading).
    pub fn search(&self, query: &str) -> Vec<&ToolSchema> {
        let query_lower = query.to_lowercase();
        self.registry.values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Load a specific tool into the active context.
    pub fn load_tool(&mut self, name: &str) -> Option<&ToolSchema> {
        if self.registry.contains_key(name) && !self.loaded.contains(&name.to_string()) {
            self.loaded.push(name.to_string());
        }
        self.registry.get(name)
    }

    /// Get currently loaded tool schemas for prompt tail.
    pub fn loaded_schemas(&self) -> Vec<&ToolSchema> {
        self.loaded.iter().filter_map(|n| self.registry.get(n)).collect()
    }

    pub fn all(&self) -> Vec<&ToolSchema> { self.registry.values().collect() }
    pub fn loaded_count(&self) -> usize { self.loaded.len() }
    pub fn total_count(&self) -> usize { self.registry.len() }
}

impl Default for ToolSearchEngine { fn default() -> Self { Self::new(20) } }
