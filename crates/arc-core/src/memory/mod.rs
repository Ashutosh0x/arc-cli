// SPDX-License-Identifier: MIT
//! ARC Memory Subsystem
//!
//! Implementing a tiered memory architecture for autonomous agents:
//! 1. Working Memory (RAM) — The direct LLM context window, managed via blocks and rings.
//! 2. Short-Term Memory (Disk cache) — The complete unstructured session history.
//! 3. Long-Term Memory (Disk layout) — Persistent facts, vectors, and user profiles across sessions.

pub mod arena;
pub mod compressor;
pub mod long_term;
pub mod observation;
pub mod session_store;
pub mod short_term;
pub mod working;

use crate::error::{ArcError, ArcResult};
use crate::memory::compressor::Compressor;
use crate::memory::long_term::LongTermMemory;
pub use crate::memory::session_store::{SessionMetadata, SessionRecord, SessionStore};
use crate::memory::short_term::ShortTermMemory;
use crate::memory::working::{MemoryMessage, WorkingMemory};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// Configuration for the memory subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Max tokens in the active context window before compression triggers
    pub context_budget: u32,
    /// Threshold (e.g. 0.8 for 80%) at which compression starts
    pub compression_threshold: f32,
    /// Number of most recent messages to ALWAYS keep uncompressed
    pub recent_buffer_size: usize,
    /// Whether long-term memory (Redb) is enabled
    pub persistence_enabled: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            context_budget: 8192,
            compression_threshold: 0.8,
            recent_buffer_size: 10,
            persistence_enabled: true,
        }
    }
}

/// The orchestrator for the entire memory architecture.
pub struct MemoryManager {
    config: MemoryConfig,
    session_id: String,

    working: Arc<RwLock<WorkingMemory>>,
    short_term: Arc<RwLock<ShortTermMemory>>,

    long_term: Option<Arc<LongTermMemory>>,
    session_store: Option<Arc<SessionStore>>,

    compressor: Arc<Compressor>,

    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cost_usd: f64,
}

impl MemoryManager {
    /// Initialize a new memory management unit.
    pub fn new(config: MemoryConfig, profile_dir: Option<PathBuf>) -> ArcResult<Self> {
        let session_id = Uuid::new_v4().to_string();

        let ltm = if config.persistence_enabled {
            if let Some(ref dir) = profile_dir {
                Some(Arc::new(LongTermMemory::new(&config, dir.clone())?))
            } else {
                None
            }
        } else {
            None
        };

        let session_store = if config.persistence_enabled {
            if let Some(ref dir) = profile_dir {
                Some(Arc::new(SessionStore::new(dir.clone())?))
            } else {
                None
            }
        } else {
            None
        };

        let short_term_cap = config.context_budget as usize; // Arbitrary large buffer for full session

        Ok(Self {
            config: config.clone(),
            session_id,
            working: Arc::new(RwLock::new(WorkingMemory::new(config))),
            short_term: Arc::new(RwLock::new(ShortTermMemory::new(short_term_cap))),
            long_term: ltm,
            session_store,
            compressor: Arc::new(Compressor::new()),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost_usd: 0.0,
        })
    }

    /// Load an existing session from disk.
    pub async fn load_session(&mut self, session_id: &str) -> ArcResult<()> {
        if let Some(ref store) = self.session_store {
            if let Some(record) = store.load_session(session_id)? {
                info!(
                    "Resuming session {} ({} messages)",
                    record.id,
                    record.messages.len()
                );
                self.session_id = record.id.clone();
                self.total_input_tokens = record.total_input_tokens;
                self.total_output_tokens = record.total_output_tokens;
                self.total_cost_usd = record.total_cost_usd;

                let mut working = self.working.write().await;
                let mut short = self.short_term.write().await;

                // Load all into short term, but only the `recent_buffer_size` into working.
                // The rest acts as compressed observation log.
                for msg in &record.messages {
                    short.record(msg.clone());
                }

                // Simulate compression of the backlog
                let backlog_count = if record.messages.len() > self.config.recent_buffer_size {
                    record.messages.len() - self.config.recent_buffer_size
                } else {
                    0
                };

                if backlog_count > 0 {
                    let to_compress = &record.messages[0..backlog_count];
                    let summary = self.compressor.compress(to_compress).await?;
                    working.replace_observation_log(summary);
                }

                // Load recents into working memory buffer
                let start_idx = backlog_count;
                for msg in &record.messages[start_idx..] {
                    working.add_message(msg.clone());
                }

                return Ok(());
            }
            return Err(ArcError::System(format!(
                "Session {} not found",
                session_id
            )));
        }
        Err(ArcError::System("Persistence is disabled".into()))
    }

    /// Add a new message to the memory system.
    pub async fn add_message(&self, role: &str, content: String) -> ArcResult<()> {
        let msg = MemoryMessage::new(role, content);

        // 1. Always record in full session history
        self.short_term.write().await.record(msg.clone());

        // 2. Extract facts asynchronously for LTM (fire and forget pattern for now)
        if role == "user" {
            let obs = observation::extract_observations(&msg.content).await;
            if !obs.is_empty() && self.long_term.is_some() {
                let ltm = self.long_term.as_ref().unwrap();
                for o in obs {
                    debug!("Extracted fact [{}]: {}", o.category, o.content);
                    // Append fact string
                    if let Ok(Some(existing)) = ltm.get_fact(o.category) {
                        let merged = format!("{}\n- {}", existing, o.content);
                        let _ = ltm.store_fact(o.category, &merged);
                    } else {
                        let _ = ltm.store_fact(o.category, &format!("- {}", o.content));
                    }
                }
            }
        }

        // 3. Add to context window
        let mut working = self.working.write().await;
        working.add_message(msg);

        // 4. Trigger compression if over budget thresholds
        if working.needs_compression() {
            let to_compress = working.drain_compressible_messages();
            if !to_compress.is_empty() {
                // Drop the working lock while awaiting compression LLM calls (if real LLM used)
                drop(working);

                let summary = self.compressor.compress(&to_compress).await?;

                // Re-acquire and apply
                let mut working = self.working.write().await;
                working.append_observations(&summary);
            }
        }

        Ok(())
    }

    pub fn record_metrics(&mut self, input: u64, output: u64, cost: f64) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
        self.total_cost_usd += cost;
    }

    /// Get the current context window optimized for the LLM.
    pub async fn get_context(&self) -> Vec<MemoryMessage> {
        self.working.read().await.build_context()
    }

    /// Set or update the system prompt.
    pub async fn set_system_prompt(&self, content: String) {
        let mut working = self.working.write().await;
        working.set_system_prompt(content);
    }

    /// Sync the session to disk.
    pub async fn flush(&self) -> ArcResult<()> {
        if let Some(ref store) = self.session_store {
            let short = self.short_term.read().await;
            if !short.is_empty() {
                let record = SessionRecord {
                    id: self.session_id.clone(),
                    created_at: short.all().front().unwrap().timestamp, // Time of first msg
                    updated_at: short.all().back().unwrap().timestamp,  // Time of last msg
                    summary: format!("Session with {} messages", short.len()),
                    messages: short.export(),
                    total_input_tokens: self.total_input_tokens,
                    total_output_tokens: self.total_output_tokens,
                    total_cost_usd: self.total_cost_usd,
                };
                store.save_session(&record)?;
            }
        }
        Ok(())
    }

    /// List all saved sessions metadata.
    pub fn list_sessions(&self) -> ArcResult<Vec<SessionMetadata>> {
        if let Some(ref store) = self.session_store {
            store.list_sessions()
        } else {
            Ok(Vec::new())
        }
    }

    /// Delete a session by ID.
    pub fn delete_session(&self, session_id: &str) -> ArcResult<()> {
        if let Some(ref store) = self.session_store {
            store.delete_session(session_id)
        } else {
            Ok(())
        }
    }
}
