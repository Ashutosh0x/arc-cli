// SPDX-License-Identifier: MIT
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudTask {
    pub id: String,
    pub agent_id: String,
    pub payload: String,
    pub constraints: serde_json::Value,
}

#[async_trait::async_trait]
pub trait CloudDelegator: Send + Sync {
    async fn delegate(&self, payload: &str, constraints: serde_json::Value) -> Result<String>;
    async fn get_status(&self, task_id: &str) -> Result<String>;
}
