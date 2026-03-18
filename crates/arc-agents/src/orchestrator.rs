use crate::registry::{AgentProfile, AgentRegistry};
use crate::sub_agent::{SubAgent, SubAgentResult};
use anyhow::Result;
use arc_providers::streaming::StreamingClient;
use std::sync::Arc;
use tokio::sync::mpsc;
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
    client: Arc<StreamingClient>,
    #[allow(dead_code)]
    config: OrchestratorConfig,
}

impl Orchestrator {
    pub fn new(client: StreamingClient, config: OrchestratorConfig) -> Self {
        Self {
            registry: AgentRegistry::new(),
            client: Arc::new(client),
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
                
                let polished = format!("{}\n\n<arc_heuristics>\nStrict Token Budget: Use arc-repomap if evaluating structure. Do not hallucinate outputs.\n</arc_heuristics>", task.task_description);
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
}
