//! # arc-agents — Multi-Agent Orchestration Subsystem
//!
//! Provides the Coordinator-Worker model where a main orchestration agent
//! delegates specialized tasks, context, and MCP tools to distinct sub-agents.
//! Includes the Agent Registry for discovering and launching specialized personas.

pub mod registry;
pub mod orchestrator;
pub mod sub_agent;
pub mod routing;
pub mod contracts;

pub use registry::{AgentRegistry, AgentProfile, AgentCapability};
pub use orchestrator::{Orchestrator, OrchestratorConfig, TaskDelegation};
pub use sub_agent::{SubAgent, SubAgentResult};

pub mod extensions;
pub use extensions::*;

// Expose common async traits
#[async_trait::async_trait]
pub trait AgentTask {
    async fn execute(&self, agent: &SubAgent) -> anyhow::Result<SubAgentResult>;
}
