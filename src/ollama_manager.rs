// ARC CLI — Ollama Manager
// Auto-start Ollama, pull models, health checks.
// Single command to get LLM ready — no manual setup.

use std::time::Duration;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::models::{AgentKind, AgentLog, OrchestratorEvent};

const OLLAMA_START_TIMEOUT_SECS: u64 = 15;
const OLLAMA_HEALTH_RETRIES: u32 = 10;

/// Check if Ollama is reachable at the given base URL.
pub async fn is_running(base_url: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    match Client::new().get(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Try to start the `ollama serve` process.
/// Returns Ok(true) if started, Ok(false) if already running, Err if failed.
pub async fn auto_start(base_url: &str) -> anyhow::Result<bool> {
    // Already running?
    if is_running(base_url).await {
        return Ok(false);
    }

    // Try to spawn `ollama serve`
    let spawn_result = std::process::Command::new("ollama")
        .arg("serve")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    match spawn_result {
        Ok(_child) => {
            // Wait for it to become healthy
            for i in 0..OLLAMA_HEALTH_RETRIES {
                tokio::time::sleep(Duration::from_millis(
                    if i == 0 { 500 } else { 1500 },
                ))
                .await;

                if is_running(base_url).await {
                    return Ok(true);
                }
            }
            anyhow::bail!(
                "Ollama started but not healthy after {}s",
                OLLAMA_START_TIMEOUT_SECS
            );
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::bail!(
                    "Ollama not installed. Download from: https://ollama.com/download"
                );
            }
            anyhow::bail!("Failed to start Ollama: {}", e);
        }
    }
}

/// Check if a specific model is available locally.
pub async fn is_model_available(base_url: &str, model: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let resp = match Client::new().get(&url).send().await {
        Ok(r) => r,
        Err(_) => return false,
    };

    if let Ok(json) = resp.json::<serde_json::Value>().await {
        if let Some(models) = json.get("models").and_then(|m| m.as_array()) {
            // Normalize: "gemma3:latest" matches "gemma3" or "gemma3:latest"
            let model_base = model.split(':').next().unwrap_or(model);
            return models.iter().any(|m| {
                if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                    let name_base = name.split(':').next().unwrap_or(name);
                    name == model || name_base == model_base
                } else {
                    false
                }
            });
        }
    }
    false
}

/// Pull a model via the Ollama API (streaming pull with progress).
/// Sends progress events through the event channel.
pub async fn pull_model(
    base_url: &str,
    model: &str,
    event_tx: Option<&mpsc::UnboundedSender<OrchestratorEvent>>,
) -> anyhow::Result<()> {
    let url = format!("{}/api/pull", base_url.trim_end_matches('/'));

    if let Some(tx) = event_tx {
        let _ = tx.send(OrchestratorEvent::Log(AgentLog::info(
            AgentKind::RepoMap,
            format!("[OLLAMA] Pulling model: {}...", model),
        )));
    }

    let body = serde_json::json!({
        "name": model,
        "stream": false,
    });

    let resp = Client::new()
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(600)) // 10 min for large models
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama pull failed ({}): {}", status, body_text);
    }

    if let Some(tx) = event_tx {
        let _ = tx.send(OrchestratorEvent::Log(AgentLog::info(
            AgentKind::RepoMap,
            format!("[OLLAMA] Model {} ready", model),
        )));
    }

    Ok(())
}

/// Full auto-setup: start Ollama if needed, pull model if missing.
/// Returns the base URL to use.
pub async fn ensure_ready(
    model: &str,
    event_tx: Option<&mpsc::UnboundedSender<OrchestratorEvent>>,
) -> anyhow::Result<String> {
    let base_url = std::env::var("OLLAMA_HOST")
        .unwrap_or_else(|_| "http://localhost:11434".to_string());

    // Step 1: Start Ollama if not running
    match auto_start(&base_url).await {
        Ok(true) => {
            if let Some(tx) = event_tx {
                let _ = tx.send(OrchestratorEvent::Log(AgentLog::info(
                    AgentKind::RepoMap,
                    "[OLLAMA] Auto-started Ollama server",
                )));
            }
        }
        Ok(false) => {
            if let Some(tx) = event_tx {
                let _ = tx.send(OrchestratorEvent::Log(AgentLog::info(
                    AgentKind::RepoMap,
                    "[OLLAMA] Server already running",
                )));
            }
        }
        Err(e) => {
            return Err(e);
        }
    }

    // Step 2: Check if model is available, pull if not
    if !is_model_available(&base_url, model).await {
        if let Some(tx) = event_tx {
            let _ = tx.send(OrchestratorEvent::Log(AgentLog::warn(
                AgentKind::RepoMap,
                format!("[OLLAMA] Model '{}' not found locally, pulling...", model),
            )));
        }
        pull_model(&base_url, model, event_tx).await?;
    } else if let Some(tx) = event_tx {
        let _ = tx.send(OrchestratorEvent::Log(AgentLog::info(
            AgentKind::RepoMap,
            format!("[OLLAMA] Model '{}' available", model),
        )));
    }

    Ok(base_url)
}
