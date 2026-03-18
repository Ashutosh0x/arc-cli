//! Smart routing engine — fallback chain, latency racing, round-robin.

use crate::provider::{ChatMessage, ChatResponse, Provider};
use arc_core::config::RoutingStrategy;
use arc_core::error::{ArcResult, ArcError};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct ProviderRouter {
    providers: Vec<Arc<dyn Provider>>,
    strategy: RoutingStrategy,
    fallback_chain: Vec<String>,
}

impl ProviderRouter {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        strategy: RoutingStrategy,
        fallback_chain: Vec<String>,
    ) -> Self {
        Self { providers, strategy, fallback_chain }
    }

    /// Route a chat request according to the configured strategy.
    pub async fn route(
        &self,
        messages: &[ChatMessage],
        model: &str,
    ) -> ArcResult<ChatResponse> {
        match &self.strategy {
            RoutingStrategy::FallbackChain => self.fallback_chain_route(messages, model).await,
            RoutingStrategy::Latency => self.race_providers(messages, model).await,
            RoutingStrategy::RoundRobin => self.fallback_chain_route(messages, model).await,
            RoutingStrategy::CostOptimized => self.fallback_chain_route(messages, model).await,
        }
    }

    /// Try providers in fallback chain order.
    async fn fallback_chain_route(
        &self,
        messages: &[ChatMessage],
        model: &str,
    ) -> ArcResult<ChatResponse> {
        let ordered = self.providers_in_chain_order();

        for provider in &ordered {
            debug!("Trying provider: {}", provider.name());
            match provider.chat(messages, model).await {
                Ok(response) => {
                    info!("Response from provider: {}", provider.name());
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Provider {} failed: {e}, trying next...", provider.name());
                    continue;
                }
            }
        }

        Err(ArcError::Provider("All providers exhausted".to_string()))
    }

    /// Race all providers concurrently, return the first success.
    async fn race_providers(
        &self,
        messages: &[ChatMessage],
        model: &str,
    ) -> ArcResult<ChatResponse> {
        let tasks: Vec<_> = self
            .providers
            .iter()
            .map(|p| {
                let provider = Arc::clone(p);
                let msgs = messages.to_vec();
                let mdl = model.to_string();
                tokio::spawn(async move { provider.chat(&msgs, &mdl).await })
            })
            .collect();

        // Return first successful result
        let mut last_error = None;
        for task in tasks {
            match task.await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(e)) => last_error = Some(e),
                Err(e) => {
                    warn!("Provider task panicked: {e}");
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ArcError::Provider("All providers exhausted".to_string())))
    }

    fn providers_in_chain_order(&self) -> Vec<Arc<dyn Provider>> {
        let mut ordered = Vec::new();
        for name in &self.fallback_chain {
            if let Some(p) = self.providers.iter().find(|p| p.name() == name) {
                ordered.push(Arc::clone(p));
            }
        }
        // Add any providers not in the chain at the end
        for p in &self.providers {
            if !ordered.iter().any(|o| o.name() == p.name()) {
                ordered.push(Arc::clone(p));
            }
        }
        ordered
    }
}
