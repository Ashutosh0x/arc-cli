// SPDX-License-Identifier: MIT
//! # arc-agents — Multi-Agent Orchestration Subsystem
//!
//! Provides the Coordinator-Worker model where a main orchestration agent
//! delegates specialized tasks, context, and MCP tools to distinct sub-agents.
//! Includes the Agent Registry for discovering and launching specialized personas.

pub mod contracts;
pub mod orchestrator;
pub mod registry;
pub mod routing;
pub mod sub_agent;

pub use orchestrator::{Orchestrator, OrchestratorConfig, TaskDelegation};
pub use registry::{AgentCapability, AgentProfile, AgentRegistry};
pub use sub_agent::{SubAgent, SubAgentResult};

pub mod extensions;
pub use extensions::*;

// Expose common async traits
#[async_trait::async_trait]
pub trait AgentTask {
    async fn execute(&self, agent: &SubAgent) -> anyhow::Result<SubAgentResult>;
}
