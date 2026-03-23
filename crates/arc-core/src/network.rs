// SPDX-License-Identifier: MIT
use crate::config::ProvidersConfig;
use std::time::Duration;

pub enum ConnectivityState {
    Online,
    Offline,
    Degraded { reachable: Vec<String> }, // A list of Provider IDs that are reachable
}

/// Active probe of configured provider health endpoints to determine
/// early connectivity status without waiting for an LLM query to timeout.
pub async fn probe_connectivity(providers: &ProvidersConfig) -> ConnectivityState {
    let mut targets = Vec::new();

    if providers.anthropic.enabled {
        targets.push((
            "anthropic".to_string(),
            "https://api.anthropic.com".to_string(),
        ));
    }
    if providers.openai.enabled {
        targets.push(("openai".to_string(), "https://api.openai.com".to_string()));
    }
    if providers.gemini.enabled {
        targets.push((
            "gemini".to_string(),
            "https://generativelanguage.googleapis.com".to_string(),
        ));
    }
    if providers.ollama.enabled {
        targets.push(("ollama".to_string(), providers.ollama.host.clone()));
    }

    if targets.is_empty() {
        return ConnectivityState::Offline;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    let mut futures = Vec::new();
    for (id, url) in &targets {
        let id_clone = id.clone();
        let client_clone = client.clone();
        let url_clone = url.clone();

        futures.push(async move {
            let res = client_clone.head(&url_clone).send().await;
            (id_clone, res.is_ok())
        });
    }

    let results: Vec<(String, bool)> = futures::future::join_all(futures).await;

    let reachable: Vec<String> = results
        .into_iter()
        .filter(|(_, is_ok)| *is_ok)
        .map(|(id, _)| id)
        .collect();

    if reachable.is_empty() {
        ConnectivityState::Offline
    } else if reachable.len() == targets.len() {
        ConnectivityState::Online
    } else {
        ConnectivityState::Degraded { reachable }
    }
}
