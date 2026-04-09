// ARC CLI — Orchestrator
// Event-driven pipeline: RepoMap → Architect → Coder
// FAIL-FAST: stops pipeline if any stage fails.
// Reports PipelineFailed on error, PipelineComplete on success.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::agents::{architect, coder, repo_map};
use crate::llm::LLMProvider;
use crate::models::{AgentKind, AgentLog, OrchestratorEvent, Task, TaskStatus};

/// Check if any task in the received events was failed.
fn check_for_failures(events: &[OrchestratorEvent]) -> Option<String> {
    for event in events {
        if let OrchestratorEvent::TaskUpdate(task) = event {
            if let TaskStatus::Failed(reason) = &task.status {
                return Some(format!("{} failed: {}", task.agent, reason));
            }
        }
    }
    None
}

/// Run the full agent pipeline for a given user prompt.
/// All events (task updates, logs, tokens, diffs) are sent through `event_tx`.
/// Pipeline STOPS if any stage fails (fail-fast).
pub async fn run_pipeline(
    user_prompt: String,
    model_name: String,
    llm: Arc<dyn LLMProvider>,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, "[ORCHESTRATOR] Pipeline started"),
    ));

    // ── Pre-flight: LLM health check ─────────────────────────
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, "[ORCHESTRATOR] Checking LLM connectivity..."),
    ));

    if !llm.check_health().await {
        let msg = format!(
            "LLM provider '{}' is not reachable. Check if Ollama is running or API key is configured.",
            llm.name()
        );
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::error(AgentKind::RepoMap, &msg),
        ));
        let _ = event_tx.send(OrchestratorEvent::PipelineFailed(msg));
        return;
    }

    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, "[ORCHESTRATOR] LLM provider healthy"),
    ));

    // ── Stage 1: RepoMap ─────────────────────────────────────
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::RepoMap, "[ORCHESTRATOR] Dispatching RepoMap Agent..."),
    ));

    let repo_task = Task::new(
        "Scan project structure".to_string(),
        AgentKind::RepoMap,
    );

    let project_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Collect repo context by capturing tokens
    let (repo_event_tx, mut repo_event_rx) = mpsc::unbounded_channel();
    repo_map::run(repo_task, project_dir, repo_event_tx).await;

    let mut repo_context = String::new();
    let mut repo_events = Vec::new();
    while let Ok(event) = repo_event_rx.try_recv() {
        if let OrchestratorEvent::Token(ref t) = event {
            repo_context.push_str(t);
        }
        repo_events.push(event.clone());
        let _ = event_tx.send(event);
    }

    // Check RepoMap success
    if let Some(failure) = check_for_failures(&repo_events) {
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::error(AgentKind::RepoMap, format!("[ORCHESTRATOR] {}", failure)),
        ));
        let _ = event_tx.send(OrchestratorEvent::PipelineFailed(failure));
        return;
    }

    // ── Stage 2: Architect ───────────────────────────────────
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Architect, "[ORCHESTRATOR] Dispatching Architect Agent..."),
    ));

    let arch_task = Task::new(
        format!("Generate architecture plan for: {}", &user_prompt),
        AgentKind::Architect,
    );

    let (arch_event_tx, mut arch_event_rx) = mpsc::unbounded_channel();
    architect::run(
        arch_task,
        user_prompt.clone(),
        repo_context.clone(),
        llm.clone(),
        model_name.clone(),
        arch_event_tx,
    )
    .await;

    let mut plan_context = String::new();
    let mut arch_events = Vec::new();
    while let Ok(event) = arch_event_rx.try_recv() {
        if let OrchestratorEvent::Token(ref t) = event {
            plan_context.push_str(t);
        }
        arch_events.push(event.clone());
        let _ = event_tx.send(event);
    }

    // FAIL-FAST: If Architect failed, do NOT run Coder
    if let Some(failure) = check_for_failures(&arch_events) {
        let msg = format!("Pipeline stopped: {}", failure);
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::error(AgentKind::Architect, format!("[ORCHESTRATOR] {}", msg)),
        ));
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::warn(AgentKind::Coder, "[ORCHESTRATOR] Coder skipped (Architect failed)"),
        ));
        let _ = event_tx.send(OrchestratorEvent::PipelineFailed(msg));
        return;
    }

    // ── Stage 3: Coder ───────────────────────────────────────
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Coder, "[ORCHESTRATOR] Dispatching Coder Agent..."),
    ));

    let coder_task = Task::new(
        format!("Generate code for: {}", &user_prompt),
        AgentKind::Coder,
    );

    let (coder_event_tx, mut coder_event_rx) = mpsc::unbounded_channel();
    coder::run(
        coder_task,
        user_prompt,
        plan_context,
        llm,
        model_name,
        coder_event_tx,
    )
    .await;

    let mut coder_events = Vec::new();
    while let Ok(event) = coder_event_rx.try_recv() {
        coder_events.push(event.clone());
        let _ = event_tx.send(event);
    }

    // Check Coder success
    if let Some(failure) = check_for_failures(&coder_events) {
        let msg = format!("Pipeline stopped: {}", failure);
        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::error(AgentKind::Coder, format!("[ORCHESTRATOR] {}", msg)),
        ));
        let _ = event_tx.send(OrchestratorEvent::PipelineFailed(msg));
        return;
    }

    // ── Pipeline complete (all stages succeeded) ─────────────
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Coder, "[ORCHESTRATOR] Pipeline complete -- all agents succeeded"),
    ));
    let _ = event_tx.send(OrchestratorEvent::PipelineComplete);
}
