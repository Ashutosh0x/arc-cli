use crate::classifier::{TaskClassifier, TaskType};
use crate::tracker::UsageTracker;
use arc_providers::traits::Provider;
use std::sync::Arc;

pub struct Router {
    providers: Vec<Arc<dyn Provider>>,
    usage: Arc<UsageTracker>,
}

impl Router {
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self {
            providers,
            usage: Arc::new(UsageTracker::new()),
        }
    }

    pub async fn route(&self, prompt: &str) -> Result<Arc<dyn Provider>, anyhow::Error> {
        let task_type = TaskClassifier::classify(prompt);

        // Simple ranking logic for Phase 1 MVP
        // In real arc-router, this respects quotas and rate limits

        let ranked: Vec<&Arc<dyn Provider>> = match task_type {
            TaskType::Coding => self
                .providers
                .iter()
                .filter(|p| p.name() == "google" || p.name() == "ollama")
                .collect(),
            TaskType::QuickFix => self
                .providers
                .iter()
                .filter(|p| p.name() == "groq" || p.name() == "ollama")
                .collect(),
            _ => self.providers.iter().collect(),
        };

        for provider in ranked {
            if provider.health_check().await.is_ok() {
                self.usage.record_usage(provider.name());
                return Ok(provider.clone());
            }
        }

        // Final local fallback
        self.providers
            .iter()
            .find(|p| p.name() == "ollama")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("All providers exhausted and Ollama fallback not found"))
    }
}
