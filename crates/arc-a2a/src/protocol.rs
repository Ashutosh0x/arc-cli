//! Core A2A protocol types following Google's Agent-to-Agent spec
//! with ARC-specific extensions for speed and security.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Agent Card (Discovery) ─────────────────────────────────────

/// An Agent Card describes an agent's identity, capabilities,
/// authentication requirements, and network endpoint.
/// Served at `/.well-known/agent.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Unique identifier for this agent
    pub agent_id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this agent does
    pub description: String,
    /// Base URL where this agent listens
    pub endpoint: String,
    /// Protocol version this agent speaks
    pub protocol_version: ProtocolVersion,
    /// Skills this agent offers
    pub skills: Vec<Skill>,
    /// Authentication methods this agent accepts
    pub auth_schemes: Vec<AuthScheme>,
    /// Maximum concurrent tasks this agent will accept
    pub max_concurrent_tasks: u32,
    /// Whether this agent supports SSE streaming
    pub supports_streaming: bool,
    /// Whether this agent supports push notifications
    pub supports_push_notifications: bool,
    /// Agent card creation/update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Protocol version for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
}

impl ProtocolVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A skill is a discrete capability an agent advertises.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Machine-readable identifier (e.g., "code_review", "test_generation")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// What this skill does
    pub description: String,
    /// MIME types this skill can consume
    pub input_types: Vec<String>,
    /// MIME types this skill can produce
    pub output_types: Vec<String>,
    /// Optional JSON Schema for input validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Optional tags for discovery/filtering
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Supported authentication schemes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthScheme {
    /// No authentication required
    None,
    /// Bearer token (JWT)
    Bearer {
        /// URL to fetch/validate tokens
        token_url: Option<String>,
    },
    /// HMAC-SHA256 request signing
    HmacSha256 {
        /// Header name where the signature lives
        header_name: String,
    },
    /// API key in header
    ApiKey {
        /// Header name for the API key
        header_name: String,
    },
}

// ── Messages ───────────────────────────────────────────────────

/// The envelope for all A2A communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// Globally unique message identifier
    pub message_id: Uuid,
    /// ID of the sending agent
    pub sender_id: String,
    /// ID of the target agent
    pub target_id: String,
    /// What kind of message this is
    pub msg_type: MessageType,
    /// The actual payload
    pub payload: MessagePayload,
    /// When this message was created
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID to link request/response pairs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<Uuid>,
    /// HMAC-SHA256 signature of the serialized payload (hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Protocol version
    pub version: ProtocolVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request another agent to perform a task
    TaskRequest,
    /// Response containing task result
    TaskResult,
    /// Incremental progress update (via SSE)
    TaskProgress,
    /// Share context/information proactively
    ContextShare,
    /// Ask another agent a question
    Inquiry,
    /// Response to an inquiry
    InquiryResponse,
    /// Task has been canceled
    TaskCanceled,
    /// Agent is reporting an error
    ErrorReport,
    /// Health check ping
    Ping,
    /// Health check response
    Pong,
}

/// Structured payload types — no raw `serde_json::Value` at the top level.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessagePayload {
    Task(TaskPayload),
    Result(ResultPayload),
    Progress(ProgressPayload),
    Context(ContextPayload),
    Inquiry(InquiryPayload),
    Error(ErrorPayload),
    Health(HealthPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPayload {
    pub task_id: Uuid,
    pub skill_id: String,
    pub input: serde_json::Value,
    /// Maximum time in seconds the task should run
    pub timeout_secs: Option<u64>,
    /// Priority (0 = highest)
    pub priority: u8,
    /// Optional webhook URL for push notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultPayload {
    pub task_id: Uuid,
    pub output: serde_json::Value,
    pub execution_time_ms: u64,
    pub tokens_used: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub task_id: Uuid,
    /// 0.0 to 1.0
    pub progress: f64,
    /// Human-readable status message
    pub status_message: String,
    /// Optional partial output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPayload {
    pub context_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InquiryPayload {
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPayload {
    pub uptime_secs: u64,
    pub active_tasks: u32,
    pub load_factor: f64,
}

// ── Builder ────────────────────────────────────────────────────

/// Ergonomic builder for constructing A2A messages.
pub struct MessageBuilder {
    sender_id: String,
    target_id: String,
    correlation_id: Option<Uuid>,
}

impl MessageBuilder {
    pub fn new(sender_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            sender_id: sender_id.into(),
            target_id: target_id.into(),
            correlation_id: None,
        }
    }

    pub fn correlate(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    pub fn task_request(self, skill_id: impl Into<String>, input: serde_json::Value) -> A2AMessage {
        let task_id = Uuid::new_v4();
        self.build(
            MessageType::TaskRequest,
            MessagePayload::Task(TaskPayload {
                task_id,
                skill_id: skill_id.into(),
                input,
                timeout_secs: Some(300),
                priority: 5,
                callback_url: None,
            }),
        )
    }

    pub fn task_result(self, task_id: Uuid, output: serde_json::Value, exec_ms: u64) -> A2AMessage {
        self.build(
            MessageType::TaskResult,
            MessagePayload::Result(ResultPayload {
                task_id,
                output,
                execution_time_ms: exec_ms,
                tokens_used: None,
            }),
        )
    }

    pub fn progress(self, task_id: Uuid, pct: f64, msg: impl Into<String>) -> A2AMessage {
        self.build(
            MessageType::TaskProgress,
            MessagePayload::Progress(ProgressPayload {
                task_id,
                progress: pct.clamp(0.0, 1.0),
                status_message: msg.into(),
                partial_output: None,
            }),
        )
    }

    pub fn ping(self, uptime: u64, active: u32, load: f64) -> A2AMessage {
        self.build(
            MessageType::Ping,
            MessagePayload::Health(HealthPayload {
                uptime_secs: uptime,
                active_tasks: active,
                load_factor: load,
            }),
        )
    }

    pub fn error(self, code: impl Into<String>, message: impl Into<String>) -> A2AMessage {
        self.build(
            MessageType::ErrorReport,
            MessagePayload::Error(ErrorPayload {
                code: code.into(),
                message: message.into(),
                task_id: None,
                details: None,
            }),
        )
    }

    fn build(self, msg_type: MessageType, payload: MessagePayload) -> A2AMessage {
        A2AMessage {
            message_id: Uuid::new_v4(),
            sender_id: self.sender_id,
            target_id: self.target_id,
            msg_type,
            payload,
            timestamp: Utc::now(),
            correlation_id: self.correlation_id,
            signature: None,
            version: ProtocolVersion::CURRENT,
        }
    }
}
