use crate::dependency_mapper::DependencyMapper;
use crate::plan_model::*;
use crate::read_only_tools::ReadOnlyToolSet;
use anyhow::Result;
use arc_core::session::Session;
use arc_providers::streaming::{StreamEvent, StreamingClient};
use chrono::Utc;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::instrument;
use uuid::Uuid;

/// The main planning engine. Operates in read-only mode.
/// No file writes, no command execution, no side effects.
pub struct Planner {
    tools: ReadOnlyToolSet,
    streaming_client: StreamingClient,
    project_root: PathBuf,
}

/// Events emitted during planning for TUI rendering.
#[derive(Debug, Clone)]
pub enum PlanEvent {
    /// Planning has started
    Started { task_description: String },
    /// Currently analyzing a file
    Analyzing { file: String },
    /// Dependency graph built
    DependencyGraphBuilt { node_count: usize, edge_count: usize },
    /// A plan step was generated
    StepGenerated { step: PlanStep },
    /// Risk assessment completed
    RiskAssessed { assessment: RiskAssessment },
    /// Streaming text from the model (for reasoning display)
    Reasoning { text: String },
    /// Plan is complete
    Completed { plan: Plan },
    /// An error occurred
    Error { message: String },
}

impl Planner {
    pub fn new(
        project_root: PathBuf,
        streaming_client: StreamingClient,
    ) -> Self {
        Self {
            tools: ReadOnlyToolSet::new(project_root.clone()),
            streaming_client,
            project_root,
        }
    }

    /// Execute the planning phase. Returns a Plan and streams events.
    #[instrument(skip(self, event_tx))]
    pub async fn create_plan(
        &self,
        task_description: &str,
        event_tx: mpsc::UnboundedSender<PlanEvent>,
    ) -> Result<Plan> {
        event_tx.send(PlanEvent::Started {
            task_description: task_description.to_string(),
        })?;

        // Phase 1: Build dependency graph
        let mapper = DependencyMapper::new(&self.tools);
        let dep_graph = mapper.build_graph().await?;
        event_tx.send(PlanEvent::DependencyGraphBuilt {
            node_count: dep_graph.nodes.len(),
            edge_count: dep_graph.edges.len(),
        })?;

        // Phase 2: Read project context file (ARC.md) if it exists
        let project_context = self.load_project_context().await;

        // Phase 3: Analyze the codebase structure
        let tree = self.tools.list_tree(&self.project_root, 4).await?;
        let tree_string = tree
            .iter()
            .map(|e| {
                let indent = "  ".repeat(e.depth);
                let marker = if e.is_dir { "📁" } else { "📄" };
                format!("{indent}{marker} {} ({} bytes)", e.path.display(), e.size)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Phase 4: Send the analysis prompt to the LLM
        let system_prompt = self.build_planning_system_prompt(
            &project_context,
            &tree_string,
            &dep_graph,
        );

        let user_prompt = format!(
            "Create a detailed, phased execution plan for the following task:\n\n\
             <task>\n{task_description}\n</task>\n\n\
             Analyze the codebase structure, identify all files that need modification, \
             map dependencies, assess risks, and produce a step-by-step plan.\n\n\
             Respond with a JSON object matching this schema:\n\
             {}\n",
            serde_json::to_string_pretty(&plan_json_schema())?
        );

        // Stream the response
        let mut full_response = String::new();
        let mut stream = self
            .streaming_client
            .stream_completion(&system_prompt, &user_prompt)
            .await?;

        while let Some(event) = stream.recv().await {
            match event {
                StreamEvent::TextDelta(text) => {
                    full_response.push_str(&text);
                    event_tx.send(PlanEvent::Reasoning {
                        text: text.clone(),
                    })?;
                }
                StreamEvent::Done => break,
                StreamEvent::Error(e) => {
                    event_tx.send(PlanEvent::Error {
                        message: e.to_string(),
                    })?;
                    anyhow::bail!("Streaming error during planning: {e}");
                }
                _ => {}
            }
        }

        // Phase 5: Parse the plan from the LLM response
        let plan = self.parse_plan_response(&full_response, dep_graph)?;

        event_tx.send(PlanEvent::RiskAssessed {
            assessment: plan.risk_assessment.clone(),
        })?;

        for phase in &plan.phases {
            for step in &phase.steps {
                event_tx.send(PlanEvent::StepGenerated {
                    step: step.clone(),
                })?;
            }
        }

        event_tx.send(PlanEvent::Completed { plan: plan.clone() })?;

        Ok(plan)
    }

    /// Validate a plan by checking all referenced files exist and
    /// dependencies are satisfiable.
    pub async fn validate_plan(&self, plan: &Plan) -> Result<Vec<PlanValidationIssue>> {
        let mut issues = Vec::new();

        for modification in &plan.files_to_modify {
            if self.tools.read_file(&modification.path).await.is_err() {
                issues.push(PlanValidationIssue {
                    severity: IssueSeverity::Error,
                    message: format!(
                        "File referenced for modification does not exist: {}",
                        modification.path
                    ),
                    step_id: None,
                });
            }
        }

        // Check for circular dependencies
        let ordered = plan.execution_order();
        if ordered.len() < plan.total_steps() {
            issues.push(PlanValidationIssue {
                severity: IssueSeverity::Error,
                message: "Plan contains circular dependencies".to_string(),
                step_id: None,
            });
        }

        // Check for steps referencing non-existent dependencies
        let all_step_ids: std::collections::HashSet<Uuid> = plan
            .phases
            .iter()
            .flat_map(|p| &p.steps)
            .map(|s| s.id)
            .collect();

        for phase in &plan.phases {
            for step in &phase.steps {
                for dep in &step.dependencies {
                    if !all_step_ids.contains(dep) {
                        issues.push(PlanValidationIssue {
                            severity: IssueSeverity::Warning,
                            message: format!(
                                "Step {} references non-existent dependency {}",
                                step.id, dep
                            ),
                            step_id: Some(step.id),
                        });
                    }
                }
            }
        }

        Ok(issues)
    }

    async fn load_project_context(&self) -> String {
        // Try ARC.md first, then fall back to common alternatives
        let candidates = ["ARC.md", ".arc/ARC.md", "CLAUDE.md", "GEMINI.md"];

        for candidate in candidates {
            if let Ok(content) = self.tools.read_file(candidate).await {
                return content;
            }
        }

        String::from("No project context file found.")
    }

    fn build_planning_system_prompt(
        &self,
        project_context: &str,
        tree_string: &str,
        dep_graph: &DependencyGraph,
    ) -> String {
        let graph_summary = format!(
            "Dependency graph: {} files, {} dependency edges.\n\
             Top files by imports:\n{}",
            dep_graph.nodes.len(),
            dep_graph.edges.len(),
            dep_graph
                .nodes
                .iter()
                .take(20)
                .map(|n| format!(
                    "  {} ({} LOC, {} exports)",
                    n.file_path,
                    n.loc,
                    n.exports.len()
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        format!(
            "You are ARC Plan, a read-only planning agent. You MUST NOT suggest executing \
             any commands or making changes directly. Your role is to analyze the codebase \
             and produce a detailed execution plan.\n\n\
             ## Project Context\n{project_context}\n\n\
             ## Codebase Structure\n{tree_string}\n\n\
             ## Dependency Analysis\n{graph_summary}\n\n\
             ## Instructions\n\
             1. Identify all files that need to be modified, created, or deleted.\n\
             2. Map dependencies between changes (what must happen before what).\n\
             3. Group changes into logical phases.\n\
             4. Assess risk for each step (breaking changes, test coverage gaps).\n\
             5. Estimate token cost for each step.\n\
             6. Identify which steps can be parallelized.\n\
             7. Output a valid JSON plan object."
        )
    }

    fn parse_plan_response(
        &self,
        response: &str,
        dep_graph: DependencyGraph,
    ) -> Result<Plan> {
        // Extract JSON from the response (may be wrapped in markdown code blocks)
        let json_str = extract_json_block(response)?;
        let raw: Value = serde_json::from_str(&json_str)?;

        // Parse phases and steps from the JSON
        let mut phases = Vec::new();

        if let Some(raw_phases) = raw.get("phases").and_then(|v| v.as_array()) {
            for (phase_idx, raw_phase) in raw_phases.iter().enumerate() {
                let phase_id = Uuid::new_v4();
                let mut steps = Vec::new();

                if let Some(raw_steps) = raw_phase.get("steps").and_then(|v| v.as_array()) {
                    for raw_step in raw_steps {
                        let step = PlanStep {
                            id: Uuid::new_v4(),
                            phase_id,
                            description: raw_step
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unnamed step")
                                .to_string(),
                            file_path: raw_step
                                .get("file_path")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            action: parse_step_action(raw_step),
                            status: StepStatus::Pending,
                            dependencies: Vec::new(), // Resolved below
                            estimated_tokens: raw_step
                                .get("estimated_tokens")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(500)
                                as u32,
                            risk_level: parse_risk_level(
                                raw_step
                                    .get("risk")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("low"),
                            ),
                            rationale: raw_step
                                .get("rationale")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        };
                        steps.push(step);
                    }
                }

                phases.push(PlanPhase {
                    id: phase_id,
                    name: raw_phase
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&format!("Phase {}", phase_idx + 1))
                        .to_string(),
                    steps,
                    can_parallelize: raw_phase
                        .get("can_parallelize")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                });
            }
        }

        let risk_assessment = RiskAssessment {
            overall_risk: parse_risk_level(
                raw.get("overall_risk")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium"),
            ),
            breaking_changes: parse_string_array(&raw, "breaking_changes"),
            test_coverage_gaps: parse_string_array(&raw, "test_coverage_gaps"),
            security_concerns: parse_string_array(&raw, "security_concerns"),
        };

        let files_to_modify: Vec<FileModification> = raw
            .get("files_to_modify")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| FileModification {
                        path: v
                            .get("path")
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string(),
                        change_type: ChangeType::Modify,
                        description: v
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        estimated_lines_changed: v
                            .get("lines_changed")
                            .and_then(|l| l.as_u64())
                            .unwrap_or(10)
                            as u32,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let plan = Plan {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            title: raw
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Plan")
                .to_string(),
            description: raw
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            phases,
            dependency_graph: dep_graph,
            estimated_tokens: raw
                .get("estimated_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000),
            estimated_cost_usd: raw
                .get("estimated_cost_usd")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            risk_assessment,
            files_to_modify,
            files_to_create: parse_string_array(&raw, "files_to_create"),
            files_to_delete: parse_string_array(&raw, "files_to_delete"),
        };

        Ok(plan)
    }

    /// Convert a Plan into a set of executable agent instructions.
    pub fn plan_to_instructions(&self, plan: &Plan) -> Vec<AgentInstruction> {
        plan.execution_order()
            .iter()
            .map(|step| AgentInstruction {
                step_id: step.id,
                prompt: format!(
                    "Execute this step:\n\nDescription: {}\nFile: {}\nRationale: {}\n\n\
                     Important: Only modify the specified file(s). Do not make changes \
                     outside the scope of this step.",
                    step.description,
                    step.file_path.as_deref().unwrap_or("N/A"),
                    step.rationale,
                ),
                allowed_tools: match &step.action {
                    StepAction::ReadAnalyze { .. } => vec!["read_file", "grep", "glob"],
                    StepAction::Modify { .. } => {
                        vec!["read_file", "write_file", "grep"]
                    }
                    StepAction::Create { .. } => vec!["write_file", "create_directory"],
                    StepAction::Delete { .. } => vec!["delete_file"],
                    StepAction::RunCommand { .. } => vec!["bash"],
                    StepAction::RunTests { .. } => vec!["bash"],
                    StepAction::Refactor { .. } => {
                        vec!["read_file", "write_file", "grep", "glob"]
                    }
                }
                .into_iter()
                .map(String::from)
                .collect(),
                timeout_seconds: 120,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct AgentInstruction {
    pub step_id: Uuid,
    pub prompt: String,
    pub allowed_tools: Vec<String>,
    pub timeout_seconds: u64,
}

#[derive(Debug)]
pub struct PlanValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub step_id: Option<Uuid>,
}

#[derive(Debug)]
pub enum IssueSeverity {
    Warning,
    Error,
}

fn extract_json_block(response: &str) -> Result<String> {
    // Try to find JSON in ```json ... ``` blocks first
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return Ok(response[json_start..json_start + end].trim().to_string());
        }
    }

    // Try to find raw JSON object
    if let Some(start) = response.find('{') {
        let mut depth = 0;
        let bytes = response.as_bytes();
        for (i, &b) in bytes[start..].iter().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(response[start..start + i + 1].to_string());
                    }
                }
                _ => {}
            }
        }
    }

    anyhow::bail!("No valid JSON found in LLM response")
}

fn parse_step_action(raw: &Value) -> StepAction {
    match raw.get("action").and_then(|v| v.as_str()).unwrap_or("modify") {
        "read" | "analyze" => StepAction::ReadAnalyze {
            paths: parse_string_array(raw, "paths"),
        },
        "create" => StepAction::Create {
            path: raw
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            template: raw.get("template").and_then(|v| v.as_str()).map(String::from),
        },
        "delete" => StepAction::Delete {
            path: raw
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            reason: raw
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        "run_command" => StepAction::RunCommand {
            command: raw
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            safe: raw
                .get("safe")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        },
        "test" => StepAction::RunTests {
            test_pattern: raw.get("test_pattern").and_then(|v| v.as_str()).map(String::from),
        },
        "refactor" => StepAction::Refactor {
            scope: raw
                .get("scope")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            pattern: raw
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        _ => StepAction::Modify {
            path: raw
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: raw
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
    }
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "low" => RiskLevel::Low,
        "medium" | "med" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" | "crit" => RiskLevel::Critical,
        _ => RiskLevel::Medium,
    }
}

fn parse_string_array(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn plan_json_schema() -> Value {
    serde_json::json!({
        "title": "string",
        "description": "string",
        "overall_risk": "low | medium | high | critical",
        "estimated_tokens": 0,
        "estimated_cost_usd": 0.0,
        "phases": [{
            "name": "string",
            "can_parallelize": false,
            "steps": [{
                "description": "string",
                "file_path": "string or null",
                "action": "read | modify | create | delete | run_command | test | refactor",
                "risk": "low | medium | high | critical",
                "estimated_tokens": 0,
                "rationale": "string",
                "paths": ["string"],
                "command": "string",
                "safe": false,
                "test_pattern": "string or null"
            }]
        }],
        "files_to_modify": [{"path": "string", "description": "string", "lines_changed": 0}],
        "files_to_create": ["string"],
        "files_to_delete": ["string"],
        "breaking_changes": ["string"],
        "test_coverage_gaps": ["string"],
        "security_concerns": ["string"]
    })
}
