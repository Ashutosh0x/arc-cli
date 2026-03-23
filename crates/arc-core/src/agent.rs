// SPDX-License-Identifier: MIT
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};
use crate::session::Session;
use arc_router::Router;
use arc_providers::message::{Message, StreamEvent};

use crate::approval::ApprovalMode;
use crate::checkpoint::CheckpointSystem;
use crate::planner::Planner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub index: usize,
    pub description: String,
    pub status: PlanStepStatus,
}

pub enum AgentEvent {
    Thinking,
    TextChunk(String),
    TurnComplete,
    Error(String),
}

pub struct Agent {
    router: Arc<Router>,
    session: Arc<RwLock<Session>>,
    pub approval_mode: ApprovalMode,
    pub checkpoint: Option<CheckpointSystem>,
    pub planner: Planner,
}

impl Agent {
    pub fn new(router: Arc<Router>, session: Session) -> Self {
        Self {
            router,
            session: Arc::new(RwLock::new(session)),
            approval_mode: ApprovalMode::default(),
            checkpoint: None,
            planner: Planner::new(),
        }
    }

    pub fn with_approval_mode(mut self, mode: ApprovalMode) -> Self {
        self.approval_mode = mode;
        self
    }

    pub fn with_checkpoint(mut self, checkpoint: Option<CheckpointSystem>) -> Self {
        self.checkpoint = checkpoint;
        self
    }

    pub async fn process_message(
        &self,
        prompt: &str,
        event_tx: mpsc::UnboundedSender<AgentEvent>
    ) -> Result<(), anyhow::Error> {
        // 1. Add user message
        {
            let mut sess = self.session.write().await;
            sess.add_message(Message {
                role: arc_providers::message::Role::User,
                content: prompt.to_string(),
                tool_calls: vec![],
                tool_call_id: None,
            });
        }

        // Optional: auto-checkpoint if configured
        if let Some(cp) = &self.checkpoint {
            // we could generate a dynamic message
            let _ = cp.create_checkpoint("Pre-action checkpoint");
        }

        // 2. Route request to best free provider
        let provider = self.router.route(prompt).await?;
        event_tx.send(AgentEvent::Thinking).ok();

        // 3. Get context messages
        let session_messages = self.session.read().await.messages.clone();

        // 4. Stream response
        let stream_result = provider.stream("auto", &session_messages, &[]).await;

        match stream_result {
            Ok(mut stream) => {
                use futures::StreamExt;
                let mut full_text = String::new();
                
                while let Some(chunk_res) = stream.next().await {
                    match chunk_res {
                        Ok(event) => {
                            full_text.push_str(&event.text_delta);
                            event_tx.send(AgentEvent::TextChunk(event.text_delta)).ok();
                        }
                        Err(e) => {
                            event_tx.send(AgentEvent::Error(e.to_string())).ok();
                            return Err(e);
                        }
                    }
                }

                // Add assistant response to session
                let mut sess = self.session.write().await;
                sess.add_message(Message {
                    role: arc_providers::message::Role::Assistant,
                    content: full_text,
                    tool_calls: vec![],
                    tool_call_id: None,
                });
                
                event_tx.send(AgentEvent::TurnComplete).ok();
            }
            Err(e) => {
                event_tx.send(AgentEvent::Error(e.to_string())).ok();
                return Err(e);
            }
        }

        Ok(())
    }
}
