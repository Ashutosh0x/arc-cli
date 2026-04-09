// ARC CLI — Coder Agent
// Takes a plan → generates code, produces diffs.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::diff;
use crate::llm::LLMProvider;
use crate::models::{AgentKind, AgentLog, DiffResult, OrchestratorEvent, Task};

/// Run the Coder agent: generate code from the plan.
pub async fn run(
    mut task: Task,
    user_prompt: String,
    plan_context: String,
    llm: Arc<dyn LLMProvider>,
    model_name: String,
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
) {
    task.start();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task.clone()));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Coder, "Starting code generation"),
    ));

    // Tight prompt — code only, no essays
    let plan_snippet = if plan_context.is_empty() {
        "No plan provided — generate code directly from the request.".to_string()
    } else {
        // Trim plan to first 60 lines to avoid bloating context
        plan_context.lines().take(60).collect::<Vec<_>>().join("\n")
    };

    let full_prompt = format!(
        "You are an expert developer. Write code for the following request.\n\n\
         ## Request\n{}\n\n\
         ## Plan\n{}\n\n\
         ## Rules\n\
         - Output ONLY code inside ```language fenced blocks\n\
         - Mark each file with: `// FILE: path/to/file.ext`\n\
         - Clean, idiomatic code with error handling\n\
         - NO explanations outside code blocks",
        user_prompt, plan_snippet
    );

    // Stream from LLM
    let (token_tx, mut token_rx) = mpsc::unbounded_channel::<String>();

    let llm_clone = llm.clone();
    let model_clone = model_name.clone();
    let prompt_clone = full_prompt.clone();

    let llm_handle = tokio::spawn(async move {
        llm_clone.generate(&prompt_clone, &model_clone, token_tx).await
    });

    let _ = event_tx.send(OrchestratorEvent::Token(
        "\n=== Generated Code ===\n".to_string(),
    ));

    let mut full_response = String::new();

    while let Some(token) = token_rx.recv().await {
        if token == "[DONE]" {
            break;
        }
        full_response.push_str(&token);
        let _ = event_tx.send(OrchestratorEvent::Token(token));
    }

    // Generate a diff from the response (comparing empty to generated code)
    if !full_response.is_empty() {
        let diff_result: DiffResult = diff::compute_diff(
            "generated_code.rs",
            "", // old = empty (new file)
            &full_response,
        );

        let _ = event_tx.send(OrchestratorEvent::Log(
            AgentLog::info(
                AgentKind::Coder,
                format!(
                    "Diff produced: +{} additions, -{} deletions",
                    diff_result.additions, diff_result.deletions
                ),
            ),
        ));

        let _ = event_tx.send(OrchestratorEvent::DiffProduced(diff_result));
    }

    // Get usage stats
    match llm_handle.await {
        Ok(Ok(usage)) => {
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::info(
                    AgentKind::Coder,
                    format!(
                        "Code generated ({}ms, {} tokens)",
                        usage.latency_ms, usage.total_tokens
                    ),
                ),
            ));
            let _ = event_tx.send(OrchestratorEvent::Usage(usage));
        }
        Ok(Err(e)) => {
            let msg = format!("LLM error: {}", e);
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::error(AgentKind::Coder, &msg),
            ));
            task.fail(msg);
            let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
            return;
        }
        Err(e) => {
            let msg = format!("Task join error: {}", e);
            let _ = event_tx.send(OrchestratorEvent::Log(
                AgentLog::error(AgentKind::Coder, &msg),
            ));
            task.fail(msg);
            let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
            return;
        }
    }

    task.complete();
    let _ = event_tx.send(OrchestratorEvent::TaskUpdate(task));
    let _ = event_tx.send(OrchestratorEvent::Log(
        AgentLog::info(AgentKind::Coder, "Coder agent finished"),
    ));
}
