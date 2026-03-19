use std::sync::Arc;
use crate::traits::Provider;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// Try providers in exact order. Stop at first success.
    Fallback,
    /// Pick cheapest capable provider (requires pricing metadata)
    CostOptimized,
    /// Pick fastest historical response
    LatencyOptimized,
    /// Evenly distribute
    RoundRobin,
    /// Pick one and stick to it unless it fails
    Sticky(String),
}

/// A sequential or strategized chain of capable providers.
pub struct ProviderChain {
    pub providers: Vec<ProviderEntry>,
    pub strategy: RoutingStrategy,
}

pub struct ProviderEntry {
    pub id: String,
    pub client: Arc<dyn Provider>,
    // Optional: pub breaker: Option<Arc<Mutex<CircuitBreaker>>>
}

impl ProviderChain {
    pub fn new(strategy: RoutingStrategy) -> Self {
        Self {
            providers: Vec::new(),
            strategy,
        }
    }

    pub fn add(&mut self, id: String, client: Arc<dyn Provider>) {
        self.providers.push(ProviderEntry { id, client });
    }

    /// Primary execution flow: Returns the first successful response according to the strategy.
    pub async fn execute<F, Fut, R>(&self, action: F) -> Result<R, arc_core::error::ArcError>
    where
        F: Fn(Arc<dyn Provider>) -> Fut,
        Fut: std::future::Future<Output = Result<R, anyhow::Error>>,
    {
        if self.providers.is_empty() {
            return Err(arc_core::error::ArcError::Config(
                "No providers available in routing chain".into(),
            ));
        }

        match self.strategy {
            RoutingStrategy::Fallback | RoutingStrategy::Sticky(_) => {
                let mut last_error = None;
                for entry in &self.providers {
                    match action(Arc::clone(&entry.client)).await {
                        Ok(res) => return Ok(res),
                        Err(e) => {
                            tracing::warn!("Provider {} failed: {}", entry.id, e);
                            last_error = Some(e);
                        }
                    }
                }
                Err(arc_core::error::ArcError::Network(format!(
                    "All fallback chain providers failed. Last error: {:?}",
                    last_error
                )))
            }
            _ => {
                // Future extension
                unimplemented!("Advanced routing strategies pending metrics integration")
            }
        }
    }
}
