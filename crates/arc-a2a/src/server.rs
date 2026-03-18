//! Axum-based A2A server for receiving messages from remote agents.
//! Serves the agent card, accepts task requests, and streams progress via SSE.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use uuid::Uuid;

use crate::protocol::{
    A2AMessage, AgentCard, MessagePayload, MessageType, ProtocolVersion,
};
use crate::streaming;
use crate::task::{TaskRegistry, TaskState};

/// Shared state across all route handlers.
pub struct ServerState {
    pub agent_card: AgentCard,
    pub task_registry: TaskRegistry,
    /// Callback invoked when a task request arrives.
    /// The handler should spawn the actual work asynchronously.
    pub task_handler: Box<dyn Fn(Uuid, String, serde_json::Value) + Send + Sync>,
}

/// Build the Axum router with all A2A routes.
pub fn build_router(state: Arc<ServerState>) -> Router {
    Router::new()
        // Agent card discovery
        .route("/.well-known/agent.json", get(get_agent_card))
        // Message ingress
        .route("/a2a/messages", post(receive_message))
        // Task queries
        .route("/a2a/tasks/:task_id", get(get_task))
        .route("/a2a/tasks/:task_id/cancel", post(cancel_task))
        // SSE streaming
        .route("/a2a/tasks/:task_id/stream", get(stream_task))
        // Health
        .route("/a2a/health", get(health_check))
        .with_state(state)
}

/// Start the A2A server on the given address.
pub async fn start_server(
    bind_addr: &str,
    state: Arc<ServerState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(state);
    let listener = TcpListener::bind(bind_addr).await?;

    info!(addr = %bind_addr, "A2A server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── Route Handlers ─────────────────────────────────────────────

async fn get_agent_card(State(state): State<Arc<ServerState>>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

async fn receive_message(
    State(state): State<Arc<ServerState>>,
    Json(msg): Json<A2AMessage>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    info!(
        message_id = %msg.message_id,
        sender = %msg.sender_id,
        msg_type = ?msg.msg_type,
        "Received A2A message"
    );

    // Version check
    if !msg.version.is_compatible(&ProtocolVersion::CURRENT) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Incompatible protocol version: {} (expected {})",
                msg.version,
                ProtocolVersion::CURRENT
            ),
        ));
    }

    match msg.msg_type {
        MessageType::TaskRequest => handle_task_request(&state, &msg).await,
        MessageType::Ping => handle_ping(&state).await,
        _ => {
            info!(msg_type = ?msg.msg_type, "Accepted non-task message");
            Ok((StatusCode::OK, "Accepted".to_string()))
        }
    }
}

async fn handle_task_request(
    state: &ServerState,
    msg: &A2AMessage,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let payload = match &msg.payload {
        MessagePayload::Task(t) => t,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "TaskRequest must have Task payload".into(),
            ))
        }
    };

    // Verify we support the requested skill
    let skill_supported = state
        .agent_card
        .skills
        .iter()
        .any(|s| s.id == payload.skill_id);

    if !skill_supported {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Skill '{}' not supported", payload.skill_id),
        ));
    }

    // Check capacity
    let active = state.task_registry.active_count();
    if active >= state.agent_card.max_concurrent_tasks {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "At capacity: {active}/{} tasks",
                state.agent_card.max_concurrent_tasks
            ),
        ));
    }

    // Register and start
    let _rx = state.task_registry.register(
        payload.task_id,
        payload.skill_id.clone(),
        msg.sender_id.clone(),
        payload.input.clone(),
    );

    // Transition to working
    state
        .task_registry
        .transition(payload.task_id, TaskState::Working)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Invoke the handler (non-blocking)
    (state.task_handler)(
        payload.task_id,
        payload.skill_id.clone(),
        payload.input.clone(),
    );

    let response = serde_json::json!({
        "task_id": payload.task_id,
        "status": "working"
    });

    Ok((StatusCode::ACCEPTED, response.to_string()))
}

async fn handle_ping(
    state: &ServerState,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let active = state.task_registry.active_count();
    let response = serde_json::json!({
        "status": "healthy",
        "active_tasks": active,
        "agent_id": state.agent_card.agent_id,
    });
    Ok((StatusCode::OK, response.to_string()))
}

async fn get_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match state.task_registry.get(&task_id) {
        Some(task) => Ok(Json(task)),
        None => Err((StatusCode::NOT_FOUND, "Task not found".into())),
    }
}

async fn cancel_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    state
        .task_registry
        .transition(task_id, TaskState::Canceled)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok((StatusCode::OK, "Canceled"))
}

async fn stream_task(
    State(state): State<Arc<ServerState>>,
    Path(task_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let rx = state.task_registry.subscribe(&task_id)
        .ok_or((StatusCode::NOT_FOUND, "Task not found or no watcher".into()))?;

    Ok(streaming::task_state_stream(task_id, rx))
}

async fn health_check(
    State(state): State<Arc<ServerState>>,
) -> Json<serde_json::Value> {
    let active = state.task_registry.active_count();
    Json(serde_json::json!({
        "status": "healthy",
        "agent_id": state.agent_card.agent_id,
        "active_tasks": active,
        "protocol_version": format!("{}", ProtocolVersion::CURRENT),
    }))
}
