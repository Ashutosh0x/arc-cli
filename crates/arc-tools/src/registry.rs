use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

use crate::traits::Tool;
use crate::file_read::FileReadTool;
use crate::file_edit::FileEditTool;
use crate::shell::ShellTool;
use arc_providers::message::ToolDefinition;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // Register core tools
        registry.register(Arc::new(FileReadTool));
        registry.register(Arc::new(FileEditTool));
        registry.register(Arc::new(ShellTool));
        
        registry
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters(),
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<String, anyhow::Error> {
        if let Some(tool) = self.tools.get(name) {
            tool.execute(args).await
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
