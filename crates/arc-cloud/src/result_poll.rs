use crate::delegation::CloudDelegator;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

pub struct ResultPoller {
    delegator: Arc<dyn CloudDelegator>,
    poll_interval: Duration,
}

impl ResultPoller {
    pub fn new(delegator: Arc<dyn CloudDelegator>, poll_interval_secs: u64) -> Self {
        Self {
            delegator,
            poll_interval: Duration::from_secs(poll_interval_secs),
        }
    }

    pub async fn wait_for_completion(&self, task_id: &str) -> Result<String> {
        info!("Polling for cloud task {} completion", task_id);
        loop {
            match self.delegator.get_status(task_id).await {
                Ok(status) => {
                    if status == "COMPLETED" || status == "FAILED" {
                        info!("Cloud task {} terminal status reached: {}", task_id, status);
                        return Ok(status);
                    }
                }
                Err(e) => {
                    warn!("Error getting status for task {}: {}", task_id, e);
                }
            }
            sleep(self.poll_interval).await;
        }
    }
}
