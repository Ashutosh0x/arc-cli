//! Agent card discovery with caching and validation.
//! Fetches agent cards from `<endpoint>/.well-known/agent.json`.

use chrono::{Duration, Utc};
use dashmap::DashMap;
use reqwest::Client;
use tracing::{debug, info, warn};
use url::Url;

use crate::error::{A2AError, A2AResult};
use crate::protocol::{AgentCard, ProtocolVersion};

/// Cached agent card entry with TTL.
struct CachedCard {
    card: AgentCard,
    fetched_at: chrono::DateTime<Utc>,
}

/// Discovery service that fetches, validates, and caches agent cards.
pub struct DiscoveryService {
    http_client: Client,
    cache: DashMap<String, CachedCard>,
    /// How long to cache agent cards before re-fetching
    cache_ttl: Duration,
}

impl DiscoveryService {
    pub fn new(http_client: Client, cache_ttl_secs: i64) -> Self {
        Self {
            http_client,
            cache: DashMap::new(),
            cache_ttl: Duration::try_seconds(cache_ttl_secs).unwrap_or(Duration::zero()),
        }
    }

    /// Discover an agent by its base endpoint URL.
    /// Returns a validated, cached agent card.
    pub async fn discover(&self, endpoint: &str) -> A2AResult<AgentCard> {
        // Check cache first
        if let Some(cached) = self.cache.get(endpoint) {
            let age = Utc::now() - cached.fetched_at;
            if age < self.cache_ttl {
                debug!(endpoint, "Agent card cache hit");
                return Ok(cached.card.clone());
            }
            debug!(endpoint, "Agent card cache expired, re-fetching");
        }

        // Build discovery URL
        let card_url = self.build_discovery_url(endpoint)?;
        info!(url = %card_url, "Fetching agent card");

        // Fetch
        let response = self
            .http_client
            .get(card_url.as_str())
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| A2AError::DiscoveryFailed {
                url: endpoint.to_string(),
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(A2AError::DiscoveryFailed {
                url: endpoint.to_string(),
                reason: format!("HTTP {}", response.status()),
            });
        }

        let card: AgentCard = response
            .json()
            .await
            .map_err(|e| A2AError::DiscoveryFailed {
                url: endpoint.to_string(),
                reason: format!("Invalid agent card JSON: {e}"),
            })?;

        // Validate
        self.validate_card(&card)?;

        // Cache
        self.cache.insert(
            endpoint.to_string(),
            CachedCard {
                card: card.clone(),
                fetched_at: Utc::now(),
            },
        );

        info!(
            agent_id = %card.agent_id,
            name = %card.name,
            skills = card.skills.len(),
            "Agent discovered successfully"
        );

        Ok(card)
    }

    /// Check if a remote agent supports a specific skill.
    pub async fn supports_skill(&self, endpoint: &str, skill_id: &str) -> A2AResult<bool> {
        let card = self.discover(endpoint).await?;
        Ok(card.skills.iter().any(|s| s.id == skill_id))
    }

    /// Find all skills matching given tags on a remote agent.
    pub async fn find_skills_by_tag(&self, endpoint: &str, tag: &str) -> A2AResult<Vec<String>> {
        let card = self.discover(endpoint).await?;
        Ok(card
            .skills
            .iter()
            .filter(|s| s.tags.iter().any(|t| t == tag))
            .map(|s| s.id.clone())
            .collect())
    }

    /// Invalidate cached card for an endpoint.
    pub fn invalidate(&self, endpoint: &str) {
        self.cache.remove(endpoint);
    }

    /// Clear entire cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    fn build_discovery_url(&self, endpoint: &str) -> A2AResult<Url> {
        let base = Url::parse(endpoint).map_err(|e| A2AError::DiscoveryFailed {
            url: endpoint.to_string(),
            reason: format!("Invalid URL: {e}"),
        })?;

        base.join("/.well-known/agent.json")
            .map_err(|e| A2AError::DiscoveryFailed {
                url: endpoint.to_string(),
                reason: format!("Cannot construct discovery URL: {e}"),
            })
    }

    fn validate_card(&self, card: &AgentCard) -> A2AResult<()> {
        if card.agent_id.is_empty() {
            return Err(A2AError::InvalidAgentCard("agent_id is empty".into()));
        }

        if card.endpoint.is_empty() {
            return Err(A2AError::InvalidAgentCard("endpoint is empty".into()));
        }

        if !card
            .protocol_version
            .is_compatible(&ProtocolVersion::CURRENT)
        {
            return Err(A2AError::InvalidAgentCard(format!(
                "Incompatible protocol version: {} (we speak {})",
                card.protocol_version,
                ProtocolVersion::CURRENT
            )));
        }

        if card.skills.is_empty() {
            warn!(agent_id = %card.agent_id, "Agent advertises zero skills");
        }

        // Validate endpoint URL is parseable
        Url::parse(&card.endpoint)
            .map_err(|e| A2AError::InvalidAgentCard(format!("Invalid endpoint URL: {e}")))?;

        Ok(())
    }
}
