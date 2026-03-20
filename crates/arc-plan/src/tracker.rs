//! Persistent Task Tracker with DAG dependency validation
//!
//! Disk-backed task management with parent-child hierarchy,
//! circular dependency detection, and close validation.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub parent_id: Option<String>,
    pub dependencies: Vec<String>,
    pub labels: Vec<String>,
}

pub struct TrackerService {
    tasks_dir: PathBuf,
}

impl TrackerService {
    pub fn new(tasks_dir: &Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(tasks_dir)?;
        Ok(Self { tasks_dir: tasks_dir.to_path_buf() })
    }

    fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("{:06x}", ts & 0xFFFFFF)
    }

    pub fn create_task(&self, title: String, description: String, parent_id: Option<String>) -> Result<TrackerTask, String> {
        if let Some(ref pid) = parent_id {
            if self.get_task(pid)?.is_none() {
                return Err(format!("Parent task '{pid}' not found"));
            }
        }
        let task = TrackerTask {
            id: Self::generate_id(),
            title,
            description,
            status: TaskStatus::Open,
            parent_id,
            dependencies: Vec::new(),
            labels: Vec::new(),
        };
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn get_task(&self, id: &str) -> Result<Option<TrackerTask>, String> {
        let path = self.tasks_dir.join(format!("{id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let task: TrackerTask = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(Some(task))
    }

    pub fn list_tasks(&self) -> Result<Vec<TrackerTask>, String> {
        let mut tasks = Vec::new();
        let entries = std::fs::read_dir(&self.tasks_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(task) = serde_json::from_str::<TrackerTask>(&content) {
                        tasks.push(task);
                    }
                }
            }
        }
        Ok(tasks)
    }

    pub fn update_task(&self, id: &str, updates: TaskUpdate) -> Result<TrackerTask, String> {
        let Some(mut task) = self.get_task(id)? else {
            return Err(format!("Task '{id}' not found"));
        };

        if let Some(title) = updates.title { task.title = title; }
        if let Some(desc) = updates.description { task.description = desc; }
        if let Some(status) = updates.status {
            if status == TaskStatus::Closed && task.status != TaskStatus::Closed {
                self.validate_can_close(&task)?;
            }
            task.status = status;
        }
        if let Some(deps) = updates.dependencies {
            task.dependencies = deps;
            self.validate_no_circular_deps(&task)?;
        }
        if let Some(labels) = updates.labels { task.labels = labels; }

        self.save_task(&task)?;
        Ok(task)
    }

    fn save_task(&self, task: &TrackerTask) -> Result<(), String> {
        let path = self.tasks_dir.join(format!("{}.json", task.id));
        let content = serde_json::to_string_pretty(task).map_err(|e| e.to_string())?;
        std::fs::write(path, content).map_err(|e| e.to_string())
    }

    fn validate_can_close(&self, task: &TrackerTask) -> Result<(), String> {
        for dep_id in &task.dependencies {
            let dep = self.get_task(dep_id)?
                .ok_or_else(|| format!("Dependency '{dep_id}' not found"))?;
            if dep.status != TaskStatus::Closed {
                return Err(format!(
                    "Cannot close '{}': dependency '{}' is still {:?}",
                    task.id, dep_id, dep.status
                ));
            }
        }
        Ok(())
    }

    fn validate_no_circular_deps(&self, task: &TrackerTask) -> Result<(), String> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut cache = HashMap::new();
        cache.insert(task.id.clone(), task.clone());
        self.check_cycle(&task.id, &mut visited, &mut stack, &mut cache)
    }

    fn check_cycle(
        &self,
        id: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
        cache: &mut HashMap<String, TrackerTask>,
    ) -> Result<(), String> {
        if stack.contains(id) {
            return Err(format!("Circular dependency detected involving '{id}'"));
        }
        if visited.contains(id) {
            return Ok(());
        }
        visited.insert(id.to_string());
        stack.insert(id.to_string());

        let task = if let Some(t) = cache.get(id) {
            t.clone()
        } else {
            let t = self.get_task(id)?.ok_or(format!("Dependency '{id}' not found"))?;
            cache.insert(id.to_string(), t.clone());
            t
        };

        for dep_id in &task.dependencies {
            self.check_cycle(dep_id, visited, stack, cache)?;
        }

        stack.remove(id);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub dependencies: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
}
