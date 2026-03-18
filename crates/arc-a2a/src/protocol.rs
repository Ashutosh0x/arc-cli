use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Defines the type of message being sent between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// A structured request for the other agent to perform a task
    TaskRequest,
    /// The final result of a task
    TaskResult,
    /// Contextual information being shared unprompted
    ContextShare,
    /// Asking the other agent for advice or information
    Inquiry,
}

/// A standard cross-agent payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub message_id: Uuid,
    pub sender_id: String,
    pub target_id: String,
    pub msg_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// A client for communicating with HTTP-based agent endpoints.
pub struct ProtocolClient {
    http_client: Client,
    endpoint_url: String,
    my_id: String,
}

impl ProtocolClient {
    pub fn new(endpoint_url: String, my_agent_id: String) -> Self {
        Self {
            http_client: Client::new(),
            endpoint_url,
            my_id: my_agent_id,
        }
    }

    /// Dispatch a message to another agent endpoint.
    pub async fn send_message(&self, target_id: &str, msg_type: MessageType, data: serde_json::Value) -> Result<()> {
        let msg = A2AMessage {
            message_id: Uuid::new_v4(),
            sender_id: self.my_id.clone(),
            target_id: target_id.to_string(),
            msg_type,
            payload: data,
            timestamp: chrono::Utc::now(),
        };

        let res = self.http_client
            .post(&self.endpoint_url)
            .json(&msg)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("A2A Protocol error {status}: {text}");
        }

        Ok(())
    }
}
