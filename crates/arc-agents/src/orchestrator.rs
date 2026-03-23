// SPDX-License-Identifier: MIT
use crate::registry::AgentRegistry;
use crate::sub_agent::{SubAgent, SubAgentResult};
use anyhow::Result;
use arc_providers::streaming::StreamingClient;
use std::sync::Arc;

use crate::contracts::{
    ArchitectOutput, CoderOutput, EscalationDecision, PlanOutput, should_escalate, validate_plan,
};
use tracing::{info, instrument};

pub struct OrchestratorConfig {
    pub max_concurrent_agents: usize,
    pub enforce_sandbox: bool,
}

/// A request from the main engine to delegate a task to one or more subagents.
#[derive(Debug)]
pub struct TaskDelegation {
    pub target_agent_id: String,
    pub task_description: String,
    pub contextual_files: Vec<String>,
}

/// The central Orchestrator that creates and manages sub-agents.
pub struct Orchestrator {
    registry: AgentRegistry,
    client: Arc<dyn StreamingClient>,
    #[allow(dead_code)]
    config: OrchestratorConfig,
}

impl Orchestrator {
    pub fn new(client: Arc<dyn StreamingClient>, config: OrchestratorConfig) -> Self {
        Self {
            registry: AgentRegistry::new(),
            client,
            config,
        }
    }

    /// Delegate a single task to a specific specialized agent.
    #[instrument(skip(self))]
    pub async fn delegate(&self, task: TaskDelegation) -> Result<SubAgentResult> {
        let profile = self
            .registry
            .get(&task.target_agent_id)
            .ok_or_else(|| anyhow::anyhow!("Agent profile '{}' not found", task.target_agent_id))?;

        info!("Delegating task to sub-agent: {}", profile.name);

        let mut sub_agent = SubAgent::new(profile.clone(), Arc::clone(&self.client));

        // Pass context files over
        for file in task.contextual_files {
            sub_agent.add_context_file(file);
        }

        // Inject strict LLM heuristics
        // Currently achieved via raw task prepending; in production arc-compact manages this memory explicitly.
        let polished_task = format!(
"{}

<arc_heuristics>
1. If your task requires modifying >2 files, you MUST use the `arc-repomap` AST indexer to avoid blowing context ceilings. 
2. Maintain strict surgical editing. DO NOT output full files. Use structured line replacements.
3. If you are debating with another agent natively over A2A, limit your response to 200 words prioritizing verifiable facts over politeness.
</arc_heuristics>", task.task_description);

        // Execute
        sub_agent.execute_task(&polished_task).await
    }

    /// Run multiple sub-agents in parallel on independent tasks.
    pub async fn map_parallel(&self, tasks: Vec<TaskDelegation>) -> Result<Vec<SubAgentResult>> {
        let mut handles = Vec::new();

        for task in tasks {
            let profile = self
                .registry
                .get(&task.target_agent_id)
                .ok_or_else(|| anyhow::anyhow!("Profile not found"))?
                .clone();

            let client = Arc::clone(&self.client);

            let handle = tokio::spawn(async move {
                let mut agent = SubAgent::new(profile, client);
                for f in task.contextual_files {
                    agent.add_context_file(f);
                }

                let polished = format!(
                    "{}\n\n<arc_heuristics>\nStrict Token Budget: Use arc-repomap if evaluating structure. Do not hallucinate outputs.\n</arc_heuristics>",
                    task.task_description
                );
                agent.execute_task(&polished).await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }

        Ok(results)
    }

    /// High-level deterministic execution loop validating strict JSON contracts
    /// (Plan -> Architect -> Code)
    pub async fn execute_pipeline(
        &self,
        task_description: &str,
        context_files: Vec<String>,
    ) -> Result<()> {
        info!("Step 1: Planning Phase");
        let plan_result = self
            .delegate(TaskDelegation {
                target_agent_id: "planner".into(),
                task_description: format!(
                    "Respond ONLY with a valid JSON matching the PlanOutput schema.\nTask: {}",
                    task_description
                ),
                contextual_files: context_files.clone(),
            })
            .await?;

        // Extract JSON block if surrounded by markdown fences
        let content = plan_result
            .output
            .trim()
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .trim();
        let plan: PlanOutput = match serde_json::from_str(content) {
            Ok(p) => p,
            Err(e) => anyhow::bail!("Planner returned malformed JSON: {} | Raw: {}", e, content),
        };

        validate_plan(&plan).map_err(|e| anyhow::anyhow!("Plan validation failed: {:?}", e))?;

        match should_escalate(&plan) {
            EscalationDecision::Halt { reason } => anyhow::bail!("Agent halted: {}", reason),
            EscalationDecision::AskUser { reason } => {
                tracing::warn!("Escalation required: {}", reason)
            },
            EscalationDecision::AutoProceed => info!("Plan confidence high, auto-proceeding."),
        }

        let mut arch_outputs = Vec::new();
        for step in &plan.steps {
            if step.agent == crate::contracts::AgentRole::Architect {
                info!("Step 2: Architect Phase for step {}", step.id);
                let arch_result = self
                    .delegate(TaskDelegation {
                        target_agent_id: "architect".into(),
                        task_description: format!(
                            "Respond ONLY with JSON matching ArchitectOutput.\nDesign: {}",
                            step.description
                        ),
                        contextual_files: context_files.clone(),
                    })
                    .await?;

                let content = arch_result
                    .output
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim();
                let arch: ArchitectOutput = serde_json::from_str(content)?;
                arch_outputs.push(arch);
            }
        }

        let mut all_edits = Vec::new();
        for arch in &arch_outputs {
            for file_spec in &arch.file_specs {
                info!("Step 3: Coder Phase for {:?}", file_spec.path);
                let coder_prompt = format!(
                    "Respond ONLY with JSON matching CoderOutput schemas.\nImplement changes for {}:\n{}",
                    file_spec.path, file_spec.changes_description
                );

                let coded_result = self
                    .delegate(TaskDelegation {
                        target_agent_id: "coder".into(),
                        task_description: coder_prompt,
                        contextual_files: context_files.clone(),
                    })
                    .await?;

                let content = coded_result
                    .output
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim();
                let coded: CoderOutput = serde_json::from_str(content)?;
                all_edits.extend(coded.file_edits);
            }
        }

        info!(
            "Pipeline completed successfully with {} total edits ready for diff review.",
            all_edits.len()
        );
        // Return edits or pass to arc-diff in the CLI layer...
        Ok(())
    }
}
