//! High-performance A2A client with automatic discovery, retry,
//! authentication, message signing, and SSE streaming.

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

use crate::auth::{Credential, sign_message};
use crate::discovery::DiscoveryService;
use crate::error::{A2AError, A2AResult};
use crate::protocol::{A2AMessage, AgentCard, MessageBuilder, MessagePayload};
use crate::task::TrackedTask;

/// Configuration for the A2A client.
#[derive(Clone)]
pub struct ClientConfig {
    /// This agent's ID
    pub agent_id: String,
    /// Request timeout
    pub timeout: Duration,
    /// Maximum retry attempts for transient failures
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub retry_base_delay: Duration,
    /// Maximum backoff delay cap
    pub retry_max_delay: Duration,
    /// Credential for authenticating with remote agents
    pub credential: Credential,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            agent_id: format!("arc-{}", &Uuid::new_v4().to_string()[..8]),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_base_delay: Duration::from_millis(250),
            retry_max_delay: Duration::from_secs(10),
            credential: Credential::None,
        }
    }
}

/// The primary A2A client for communicating with remote agents.
pub struct A2AClient {
    http: Client,
    config: ClientConfig,
    discovery: Arc<DiscoveryService>,
}

impl A2AClient {
    pub fn new(config: ClientConfig) -> Self {
        let http = Client::builder()
            .timeout(config.timeout)
            .pool_max_idle_per_host(4)
            .tcp_nodelay(true)
            .build()
            .expect("Failed to build HTTP client");

        let discovery = Arc::new(DiscoveryService::new(http.clone(), 300));

        Self {
            http,
            config,
            discovery,
        }
    }

    /// Discover a remote agent and return its capabilities.
    pub async fn discover(&self, endpoint: &str) -> A2AResult<AgentCard> {
        self.discovery.discover(endpoint).await
    }

    /// Submit a task to a remote agent.
    /// Returns the task ID assigned to this request.
    pub async fn submit_task(
        &self,
        endpoint: &str,
        skill_id: &str,
        input: serde_json::Value,
    ) -> A2AResult<Uuid> {
        // Discover first to validate skill support
        let card = self.discovery.discover(endpoint).await?;

        if !card.skills.iter().any(|s| s.id == skill_id) {
            return Err(A2AError::UnsupportedSkill {
                agent_id: card.agent_id,
                skill: skill_id.to_string(),
            });
        }

        let msg = MessageBuilder::new(&self.config.agent_id, &card.agent_id)
            .task_request(skill_id, input);

        let task_id = match &msg.payload {
            MessagePayload::Task(t) => t.task_id,
            _ => unreachable!(),
        };

        self.send_with_retry(endpoint, msg).await?;

        info!(
            task_id = %task_id,
            skill = skill_id,
            agent = %card.agent_id,
            "Task submitted successfully"
        );

        Ok(task_id)
    }

    /// Query the status of a task on a remote agent.
    pub async fn get_task_status(&self, endpoint: &str, task_id: Uuid) -> A2AResult<TrackedTask> {
        let url = format!("{}/a2a/tasks/{}", endpoint.trim_end_matches('/'), task_id);

        let response = self
            .http
            .get(&url)
            .timeout(self.config.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(A2AError::RemoteError { status, body });
        }

        let task: TrackedTask = response.json().await?;
        Ok(task)
    }

    /// Cancel a task on a remote agent.
    pub async fn cancel_task(&self, endpoint: &str, task_id: Uuid) -> A2AResult<()> {
        let url = format!(
            "{}/a2a/tasks/{}/cancel",
            endpoint.trim_end_matches('/'),
            task_id
        );

        let response = self.http.post(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(A2AError::RemoteError { status, body });
        }

        info!(task_id = %task_id, "Task canceled");
        Ok(())
    }

    /// Ping a remote agent to check health.
    pub async fn ping(&self, endpoint: &str) -> A2AResult<serde_json::Value> {
        let url = format!("{}/a2a/health", endpoint.trim_end_matches('/'));

        let response = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(A2AError::RemoteError {
                status: response.status().as_u16(),
                body: response.text().await.unwrap_or_default(),
            });
        }

        let health: serde_json::Value = response.json().await?;
        Ok(health)
    }

    /// Send a raw A2A message with retry and authentication.
    async fn send_with_retry(
        &self,
        endpoint: &str,
        mut msg: A2AMessage,
    ) -> A2AResult<serde_json::Value> {
        let url = format!("{}/a2a/messages", endpoint.trim_end_matches('/'));

        // Sign if using HMAC
        if let Credential::Hmac { ref secret, .. } = self.config.credential {
            sign_message(&mut msg, secret)?;
        }

        let mut last_err = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.backoff_delay(attempt);
                warn!(
                    attempt,
                    max = self.config.max_retries,
                    delay_ms = delay.as_millis(),
                    "Retrying A2A request"
                );
                sleep(delay).await;
            }

            let req = self.http.post(&url).json(&msg);

            let req = self.config.credential.apply_to_request(
                req,
                &self.config.agent_id,
                &msg.target_id,
            )?;

            match req.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        let body: serde_json::Value =
                            response.json().await.unwrap_or(serde_json::Value::Null);
                        return Ok(body);
                    }

                    let status_code = status.as_u16();
                    let body = response.text().await.unwrap_or_default();

                    // Don't retry client errors (4xx) except 429
                    if status.is_client_error() && status_code != 429 {
                        return Err(A2AError::RemoteError {
                            status: status_code,
                            body,
                        });
                    }

                    last_err = Some(A2AError::RemoteError {
                        status: status_code,
                        body,
                    });
                },
                Err(e) => {
                    if e.is_timeout() || e.is_connect() {
                        last_err = Some(A2AError::HttpError(e));
                    } else {
                        return Err(A2AError::HttpError(e));
                    }
                },
            }
        }

        Err(last_err.unwrap_or(A2AError::ConnectionExhausted {
            endpoint: endpoint.to_string(),
            attempts: self.config.max_retries + 1,
        }))
    }

    /// Calculate exponential backoff with jitter.
    fn backoff_delay(&self, attempt: u32) -> Duration {
        let base_ms = self.config.retry_base_delay.as_millis() as u64;
        let exp_ms = base_ms.saturating_mul(1u64 << attempt.min(8));
        let max_ms = self.config.retry_max_delay.as_millis() as u64;
        let capped_ms = exp_ms.min(max_ms);

        // Add ±25% jitter
        let jitter = (rand::random::<f64>() * 0.5 - 0.25) * capped_ms as f64;
        let final_ms = (capped_ms as f64 + jitter).max(1.0) as u64;

        Duration::from_millis(final_ms)
    }
}
