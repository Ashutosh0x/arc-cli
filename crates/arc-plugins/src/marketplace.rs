//! Marketplace client: search, browse, and fetch plugin metadata from remote registries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub stars: u32,
    pub repository_url: String,
}

pub struct MarketplaceClient {
    registry_url: String,
    http_client: reqwest::Client,
}

impl MarketplaceClient {
    pub fn official() -> Self {
        Self {
            registry_url: "https://raw.githubusercontent.com/arc-cli/arc-plugins-official/main"
                .into(),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn custom(registry_url: String) -> Self {
        Self {
            registry_url,
            http_client: reqwest::Client::new(),
        }
    }

    /// Fetch the full plugin index from the marketplace.
    pub async fn list_plugins(&self) -> Result<Vec<MarketplaceEntry>, MarketplaceError> {
        let url = format!("{}/index.json", self.registry_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| MarketplaceError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(MarketplaceError::NotFound(url));
        }

        let entries: Vec<MarketplaceEntry> = response
            .json()
            .await
            .map_err(|e| MarketplaceError::Parse(e.to_string()))?;

        Ok(entries)
    }

    /// Search plugins by keyword.
    pub async fn search(&self, query: &str) -> Result<Vec<MarketplaceEntry>, MarketplaceError> {
        let all = self.list_plugins().await?;
        let query_lower = query.to_lowercase();

        let matches: Vec<_> = all
            .into_iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
                    || entry
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(matches)
    }

    /// Get details for a specific plugin.
    pub async fn get_plugin(
        &self,
        name: &str,
    ) -> Result<Option<MarketplaceEntry>, MarketplaceError> {
        let all = self.list_plugins().await?;
        Ok(all.into_iter().find(|e| e.name == name))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),
}
