// ARC CLI — Architect Agent
// Takes user prompt + repo context → generates an implementation plan via LLM.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::llm::LLMProvider;
use crate::models::{AgentKind, AgentLog, OrchestratorEvent, Task};

/// Run the Architect agent: generate an implementation plan from the LLM.
pub async fn run(
    mut task: Task,
    user_prompt: String,
    repo_context: String,
    llm: Arc<dyn LLMProvider>,
    model_name: String,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    task.start();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task.clone()));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Architect, format!("Generating plan for: {}", &user_prompt)),
    ));

    // Build a tight, minimal prompt — no essays
    let full_prompt = format!(
        "You are a software architect. Given the user request and project context below, \
         output ONLY a minimal file plan.\n\n\
         ## User Request\n{}\n\n\
         ## Project Context (top files)\n{}\n\n\
         ## Output Format (STRICT)\n\
         - List files to create/modify, one per line: `FILE: path/to/file.ext`\n\
         - Under each file, 1-2 bullet points max describing what it does\n\
         - NO explanations, NO prose, NO step-by-step guides\n\
         - Keep total output under 300 words",
        user_prompt,
        // Trim repo context to first 50 lines max
        repo_context.lines().take(50).collect::<Vec<_>>().join("\n")
    );

    // Stream from LLM
    let (token_tx, mut token_rx) = mpsc::unbounded_channel::<String>();

    let llm_clone = llm.clone();
    let model_clone = model_name.clone();
    let prompt_clone = full_prompt.clone();

    let llm_handle = tokio::spawn(async move {
        llm_clone.generate(&prompt_clone, &model_clone, token_tx).await
    });

    // Forward tokens to orchestrator
    let _ = event_tx.send(OrchestratorEvent::Token(
        "\n=== Architecture Plan ===\n".to_string(),
    ));

    while let Some(token) = token_rx.recv().await {
        if token == "[DONE]" {
            break;
        }
        let _ = event_tx.send(OrchestratorEvent::Token(token));
    }

    // Get usage stats
    match llm_handle.await {
        Ok(Ok(usage)) => {
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::info(
                    AgentKind::Architect,
                    format!(
                        "Plan generated ({}ms, {} tokens)",
                        usage.latency_ms, usage.total_tokens
                    ),
                ),
            ));
            let _ = event_tx.send(OrchestratorEvent::Usage(usage));
        }
        Ok(Err(e)) => {
            let msg = format!("LLM error: {}", e);
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::error(AgentKind::Architect, &msg),
            ));
            task.fail(msg);
            let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
            return;
        }
        Err(e) => {
            let msg = format!("Task join error: {}", e);
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::error(AgentKind::Architect, &msg),
            ));
            task.fail(msg);
            let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
            return;
        }
    }

    task.complete();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Architect, "Architect agent finished"),
    ));
}
