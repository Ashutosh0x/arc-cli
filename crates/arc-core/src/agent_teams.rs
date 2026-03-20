//! # Agent Teams — Leader/Teammate Model with Background Agents
//!
//! Multi-agent orchestration with leader coordination, background workers,
//! and worktree-based isolation per agent.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Leader,
    Teammate,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Working,
    Waiting,
    Completed,
    Failed,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IsolationMode {
    None,
    Worktree,
    Process,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAgent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub status: AgentStatus,
    pub isolation: IsolationMode,
    pub worktree_path: Option<PathBuf>,
    pub model: Option<String>,
    pub current_task: Option<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
}

pub struct AgentTeam {
    agents: HashMap<String, TeamAgent>,
    leader_id: Option<String>,
    messages: Vec<TeamMessage>,
    max_agents: usize,
}

impl AgentTeam {
    pub fn new(max_agents: usize) -> Self {
        Self {
            agents: HashMap::new(),
            leader_id: None,
            messages: Vec::new(),
            max_agents,
        }
    }

    /// Spawn a new agent in the team.
    pub fn spawn_agent(
        &mut self,
        name: &str,
        role: AgentRole,
        isolation: IsolationMode,
    ) -> Result<String, String> {
        if self.agents.len() >= self.max_agents {
            return Err(format!("Max agents ({}) reached", self.max_agents));
        }
        let id = format!("agent-{}-{}", name, self.agents.len());
        let agent = TeamAgent {
            id: id.clone(),
            name: name.to_string(),
            role,
            status: AgentStatus::Idle,
            isolation,
            worktree_path: None,
            model: None,
            current_task: None,
            output: Vec::new(),
        };
        if role == AgentRole::Leader {
            if self.leader_id.is_some() {
                return Err("Team already has a leader".into());
            }
            self.leader_id = Some(id.clone());
        }
        self.agents.insert(id.clone(), agent);
        Ok(id)
    }

    /// Assign a task to an agent.
    pub fn assign_task(&mut self, agent_id: &str, task: &str) -> Result<(), String> {
        let agent = self.agents.get_mut(agent_id).ok_or("Agent not found")?;
        agent.current_task = Some(task.to_string());
        agent.status = AgentStatus::Working;
        Ok(())
    }

    /// Send a message between agents.
    pub fn send_message(&mut self, from: &str, to: &str, content: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.messages.push(TeamMessage {
            from: from.to_string(),
            to: to.to_string(),
            content: content.to_string(),
            timestamp: now,
        });
    }

    /// Kill a background agent.
    pub fn kill_agent(&mut self, agent_id: &str) -> bool {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = AgentStatus::Killed;
            true
        } else {
            false
        }
    }

    /// Complete an agent's current task.
    pub fn complete_task(&mut self, agent_id: &str, output: &str) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = AgentStatus::Completed;
            agent.output.push(output.to_string());
            agent.current_task = None;
        }
    }

    pub fn agents(&self) -> Vec<&TeamAgent> {
        self.agents.values().collect()
    }
    pub fn leader(&self) -> Option<&TeamAgent> {
        self.leader_id.as_ref().and_then(|id| self.agents.get(id))
    }
    pub fn background_agents(&self) -> Vec<&TeamAgent> {
        self.agents
            .values()
            .filter(|a| a.role == AgentRole::Background)
            .collect()
    }
    pub fn messages(&self) -> &[TeamMessage] {
        &self.messages
    }
    pub fn active_count(&self) -> usize {
        self.agents
            .values()
            .filter(|a| a.status == AgentStatus::Working)
            .count()
    }
}

impl Default for AgentTeam {
    fn default() -> Self {
        Self::new(8)
    }
}
