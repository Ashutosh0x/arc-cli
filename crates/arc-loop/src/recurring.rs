use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tracing::info;

pub struct RecurringTask {
    interval: Duration,
    task_name: String,
}

impl RecurringTask {
    pub fn new(task_name: &str, interval_secs: u64) -> Self {
        Self {
            interval: Duration::from_secs(interval_secs),
            task_name: task_name.to_string(),
        }
    }

    pub async fn run_loop<F, Fut>(&self, mut action: F) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        info!("Starting recurring task: {}", self.task_name);
        let mut ticker = time::interval(self.interval);

        loop {
            ticker.tick().await;
            info!("Executing task: {}", self.task_name);
            if let Err(e) = action().await {
                tracing::error!("Error executing task {}: {}", self.task_name, e);
            }
        }
    }
}
