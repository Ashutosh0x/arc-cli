//! Central scheduler for long-running sessions.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};
use uuid::Uuid;

use crate::cron::CronTask;
use crate::watcher::FileWatcher;

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub next_run: Option<DateTime<Utc>>,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Active,
    Paused,
    Completed,
    Failed(String),
}

pub struct LoopScheduler {
    tasks: Arc<RwLock<HashMap<Uuid, ScheduledTask>>>,
    cron_tasks: Arc<RwLock<Vec<CronTask>>>,
    file_watcher: Arc<RwLock<Option<FileWatcher>>>,
    tx_events: mpsc::Sender<LoopEvent>,
}

#[derive(Debug)]
pub enum LoopEvent {
    CronTriggered(CronTask),
    FileChanged(Vec<std::path::PathBuf>),
    TaskCompleted(Uuid),
}

impl LoopScheduler {
    pub fn new(tx_events: mpsc::Sender<LoopEvent>) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            cron_tasks: Arc::new(RwLock::new(Vec::new())),
            file_watcher: Arc::new(RwLock::new(None)),
            tx_events,
        }
    }

    /// Add a cron task to the scheduler.
    pub async fn add_cron_task(&self, task: CronTask) -> Uuid {
        let task_id = task.id;
        let mut tasks = self.tasks.write().await;
        let mut cron_tasks = self.cron_tasks.write().await;

        let scheduled_task = ScheduledTask {
            id: task_id,
            name: task.name.clone(),
            description: format!("Cron schedule: {}", task.schedule_expr),
            next_run: task.next_run(),
            status: TaskStatus::Active,
        };

        tasks.insert(task_id, scheduled_task);
        cron_tasks.push(task);

        info!("Added cron task {} ({})", task_id, task_id);
        task_id
    }

    /// Start the main scheduler loop.
    pub async fn run(self: Arc<Self>) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;
            self.check_cron_tasks().await;
        }
    }

    async fn check_cron_tasks(&self) {
        let now = Utc::now();
        let mut cron_tasks = self.cron_tasks.write().await;
        let mut tasks = self.tasks.write().await;

        for cron_task in cron_tasks.iter_mut() {
            if let Some(next_run) = cron_task.next_run() {
                if now >= next_run {
                    // Trigger the task
                    info!("Triggering cron task {}", cron_task.name);
                    let _ = self.tx_events.send(LoopEvent::CronTriggered(cron_task.clone())).await;

                    // Update next run time
                    cron_task.advance_schedule();

                    // Update UI state
                    if let Some(t) = tasks.get_mut(&cron_task.id) {
                        t.next_run = cron_task.next_run();
                    }
                }
            }
        }
    }
}
