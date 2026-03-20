use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A complete execution plan produced by the planning subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub title: String,
    pub description: String,
    pub phases: Vec<PlanPhase>,
    pub dependency_graph: DependencyGraph,
    pub estimated_tokens: u64,
    pub estimated_cost_usd: f64,
    pub risk_assessment: RiskAssessment,
    pub files_to_modify: Vec<FileModification>,
    pub files_to_create: Vec<String>,
    pub files_to_delete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPhase {
    pub id: Uuid,
    pub name: String,
    pub steps: Vec<PlanStep>,
    pub can_parallelize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: Uuid,
    pub phase_id: Uuid,
    pub description: String,
    pub file_path: Option<String>,
    pub action: StepAction,
    pub status: StepStatus,
    pub dependencies: Vec<Uuid>,
    pub estimated_tokens: u32,
    pub risk_level: RiskLevel,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepAction {
    ReadAnalyze {
        paths: Vec<String>,
    },
    Modify {
        path: String,
        description: String,
    },
    Create {
        path: String,
        template: Option<String>,
    },
    Delete {
        path: String,
        reason: String,
    },
    RunCommand {
        command: String,
        safe: bool,
    },
    RunTests {
        test_pattern: Option<String>,
    },
    Refactor {
        scope: String,
        pattern: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<DependencyNode>,
    pub edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub file_path: String,
    pub node_type: NodeType,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub loc: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Module,
    Struct,
    Trait,
    Function,
    Test,
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Import,
    Implements,
    Calls,
    Tests,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: RiskLevel,
    pub breaking_changes: Vec<String>,
    pub test_coverage_gaps: Vec<String>,
    pub security_concerns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModification {
    pub path: String,
    pub change_type: ChangeType,
    pub description: String,
    pub estimated_lines_changed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Add,
    Modify,
    Delete,
    Rename { new_path: String },
}

impl Plan {
    pub fn total_steps(&self) -> usize {
        self.phases.iter().map(|p| p.steps.len()).sum()
    }

    pub fn completed_steps(&self) -> usize {
        self.phases
            .iter()
            .flat_map(|p| &p.steps)
            .filter(|s| s.status == StepStatus::Completed)
            .count()
    }

    pub fn next_actionable_steps(&self) -> Vec<&PlanStep> {
        self.phases
            .iter()
            .flat_map(|p| &p.steps)
            .filter(|step| {
                step.status == StepStatus::Pending
                    && step.dependencies.iter().all(|dep_id| {
                        self.phases
                            .iter()
                            .flat_map(|p| &p.steps)
                            .find(|s| s.id == *dep_id)
                            .map(|s| s.status == StepStatus::Completed)
                            .unwrap_or(true)
                    })
            })
            .collect()
    }

    /// Topological sort of all steps respecting dependency edges.
    pub fn execution_order(&self) -> Vec<&PlanStep> {
        let all_steps: Vec<&PlanStep> = self.phases.iter().flat_map(|p| &p.steps).collect();

        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

        for step in &all_steps {
            in_degree.entry(step.id).or_insert(0);
            adj.entry(step.id).or_default();
            for dep in &step.dependencies {
                adj.entry(*dep).or_default().push(step.id);
                *in_degree.entry(step.id).or_insert(0) += 1;
            }
        }

        let mut queue: std::collections::VecDeque<Uuid> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut sorted_ids = Vec::with_capacity(all_steps.len());

        while let Some(id) = queue.pop_front() {
            sorted_ids.push(id);
            if let Some(neighbors) = adj.get(&id) {
                for &next in neighbors {
                    let deg = in_degree.get_mut(&next).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(next);
                    }
                }
            }
        }

        let step_map: HashMap<Uuid, &PlanStep> = all_steps.into_iter().map(|s| (s.id, s)).collect();

        sorted_ids
            .into_iter()
            .filter_map(|id| step_map.get(&id).copied())
            .collect()
    }
}
