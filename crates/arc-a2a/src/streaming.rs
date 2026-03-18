//! Server-Sent Events (SSE) streaming for real-time task updates.
//! Enables agents to push progress, partial results, and state changes
//! to the requesting agent without polling.

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Serialize;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tokio_stream::StreamExt;
use tracing::debug;
use uuid::Uuid;

use crate::task::TaskState;

/// SSE event types pushed to subscribers.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseUpdate {
    StateChanged {
        task_id: Uuid,
        old_state: TaskState,
        new_state: TaskState,
    },
    Progress {
        task_id: Uuid,
        progress: f64,
        message: String,
    },
    Completed {
        task_id: Uuid,
        output: serde_json::Value,
    },
    Failed {
        task_id: Uuid,
        error: String,
    },
    Heartbeat {
        active_tasks: u32,
    },
}

/// Create an SSE stream that pushes task state changes in real time.
pub fn task_state_stream(
    task_id: Uuid,
    rx: watch::Receiver<TaskState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send + 'static> {
    let initial_state = *rx.borrow();

    let stream = WatchStream::new(rx)
        .map(move |new_state| {
            let update = match new_state {
                TaskState::Completed => SseUpdate::Completed {
                    task_id,
                    output: serde_json::Value::Null,
                },
                TaskState::Failed => SseUpdate::Failed {
                    task_id,
                    error: "Task failed".into(),
                },
                other => SseUpdate::StateChanged {
                    task_id,
                    old_state: initial_state,
                    new_state: other,
                },
            };

            let data = serde_json::to_string(&update).unwrap_or_default();
            debug!(task_id = %task_id, state = ?new_state, "SSE push");

            Ok(Event::default()
                .event("task_update")
                .data(data)
                .id(Uuid::new_v4().to_string()))
        })
        // Stop streaming once terminal
        .take_while(move |_| !initial_state.is_terminal());

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

/// Create a heartbeat-only SSE stream for health monitoring.
pub fn heartbeat_stream(
    active: u32,
    interval: Duration,
) -> Sse<impl Stream<Item = Result<Event, Infallible>> + Send + 'static> {

    let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(interval))
        .map(move |_| {
            let update = SseUpdate::Heartbeat {
                active_tasks: active,
            };
            let data = serde_json::to_string(&update).unwrap_or_default();
            Ok(Event::default().event("heartbeat").data(data))
        });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}
