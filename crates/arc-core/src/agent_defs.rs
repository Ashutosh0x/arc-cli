//! # Agent Definitions — .arc/agents/*.md with Frontmatter
//!
//! User-defined agents with name, description, tools, model, color,
//! effort, background, isolation, memory frontmatter.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub background: bool,
    #[serde(default)]
    pub isolation: Option<String>,
    #[serde(default)]
    pub memory: Option<String>,
    #[serde(skip)]
    pub instructions: String,
    #[serde(skip)]
    pub file_path: PathBuf,
}

pub struct AgentDefinitionRegistry {
    agents: HashMap<String, AgentDefinition>,
}

impl AgentDefinitionRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn discover(&mut self, dir: &Path) -> Result<usize, String> {
        if !dir.exists() {
            return Ok(0);
        }
        let mut count = 0;
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Ok(def) = Self::parse_agent(&path) {
                    self.agents.insert(def.name.clone(), def);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    fn parse_agent(path: &Path) -> Result<AgentDefinition, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err("Missing frontmatter".into());
        }
        let mut def: AgentDefinition =
            serde_yaml::from_str(parts[1].trim()).map_err(|e| e.to_string())?;
        def.instructions = parts[2].trim().to_string();
        def.file_path = path.to_path_buf();
        if def.name.is_empty() {
            def.name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
        }
        Ok(def)
    }

    pub fn get(&self, name: &str) -> Option<&AgentDefinition> {
        self.agents.get(name)
    }
    pub fn list(&self) -> Vec<&AgentDefinition> {
        self.agents.values().collect()
    }
}

impl Default for AgentDefinitionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
