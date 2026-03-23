// SPDX-License-Identifier: MIT
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
}
