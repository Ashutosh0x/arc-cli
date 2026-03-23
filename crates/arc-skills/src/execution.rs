// SPDX-License-Identifier: MIT
use crate::registry::SkillRegistry;
use crate::skill::{SkillContext, SkillResult};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tracing::{error, info};

pub struct SkillExecutor {
    registry: Arc<SkillRegistry>,
}

impl SkillExecutor {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self { registry }
    }

    pub async fn execute(&self, name: &str, ctx: SkillContext) -> Result<SkillResult> {
        let skill = self
            .registry
            .get(name)
            .ok_or_else(|| anyhow!("Skill not found: {}", name))?;

        info!("Executing skill: {}", name);
        match skill.execute(ctx).await {
            Ok(result) => {
                info!("Skill {} executed successfully", name);
                Ok(result)
            },
            Err(e) => {
                error!("Skill {} execution failed: {}", name, e);
                Err(e)
            },
        }
    }
}
