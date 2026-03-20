use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SkillContext {
    pub parameters: HashMap<String, Value>,
    pub workspace_dir: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct SkillResult {
    pub output: String,
    pub success: bool,
}

#[async_trait]
pub trait Skill: Send + Sync {
    /// The unique name of the skill (e.g. `github_pr_create`)
    fn name(&self) -> &str;

    /// A human-readable and LLM-readable description of what the skill does
    fn description(&self) -> &str;

    /// JSON schema describing the required and optional parameters
    fn parameters_schema(&self) -> Value;

    /// Execute the skill with the given context
    async fn execute(&self, ctx: SkillContext) -> Result<SkillResult>;
}
