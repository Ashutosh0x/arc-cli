// SPDX-License-Identifier: MIT
//! Working Memory — manages the active context window.
//!
//! Implements the MemGPT-inspired block architecture:
//! - System prompt block (pinned, never evicted)
//! - Core memory block (agent-managed key facts)
//! - Observation block (compressed history)
//! - Recent buffer (verbatim last N messages)

use crate::memory::MemoryConfig;
use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::{debug, warn};

/// Approximate token count for a string (4 chars ≈ 1 token).
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32 + 3) / 4
}

/// A single message in working memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMessage {
    pub role: CompactString,
    pub content: String,
    pub token_count: u32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this message is pinned (never evicted)
    pub pinned: bool,
}

impl MemoryMessage {
    pub fn new(role: &str, content: String) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            role: CompactString::new(role),
            content,
            token_count,
            timestamp: chrono::Utc::now(),
            pinned: false,
        }
    }

    pub fn system(content: String) -> Self {
        let mut msg = Self::new("system", content);
        msg.pinned = true; // System messages are always pinned
        msg
    }
}

/// Named memory blocks that the agent can read/write to directly.
/// These persist across turns within a session and hold key facts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreMemoryBlocks {
    /// Facts about the current user
    pub user_profile: String,
    /// Agent's own persona/instructions
    pub agent_persona: String,
    /// Current task context / objectives
    pub task_context: String,
    /// Scratchpad for intermediate reasoning
    pub scratchpad: String,
}

impl CoreMemoryBlocks {
    pub fn total_tokens(&self) -> u32 {
        estimate_tokens(&self.user_profile)
            + estimate_tokens(&self.agent_persona)
            + estimate_tokens(&self.task_context)
            + estimate_tokens(&self.scratchpad)
    }

    /// Format core memory into a context string for the LLM.
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();
        if !self.user_profile.is_empty() {
            parts.push(format!(
                "<core_memory:user>\n{}\n</core_memory:user>",
                self.user_profile
            ));
        }
        if !self.agent_persona.is_empty() {
            parts.push(format!(
                "<core_memory:persona>\n{}\n</core_memory:persona>",
                self.agent_persona
            ));
        }
        if !self.task_context.is_empty() {
            parts.push(format!(
                "<core_memory:task>\n{}\n</core_memory:task>",
                self.task_context
            ));
        }
        if !self.scratchpad.is_empty() {
            parts.push(format!(
                "<core_memory:scratchpad>\n{}\n</core_memory:scratchpad>",
                self.scratchpad
            ));
        }
        parts.join("\n")
    }
}

/// The active working memory state.
pub struct WorkingMemory {
    config: MemoryConfig,

    /// System prompt — always first, never evicted
    system_prompt: Option<MemoryMessage>,

    /// Agent-managed core memory blocks
    pub core_blocks: CoreMemoryBlocks,

    /// Compressed observation log from prior turns
    observation_log: String,
    observation_tokens: u32,

    /// Recent message buffer (verbatim, most recent turns)
    recent_buffer: VecDeque<MemoryMessage>,

    /// Total tokens currently in the context window
    total_tokens: u32,
}

impl WorkingMemory {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            system_prompt: None,
            core_blocks: CoreMemoryBlocks::default(),
            observation_log: String::new(),
            observation_tokens: 0,
            recent_buffer: VecDeque::new(),
            total_tokens: 0,
        }
    }

    /// Set the system prompt (pinned, always included).
    pub fn set_system_prompt(&mut self, content: String) {
        self.system_prompt = Some(MemoryMessage::system(content));
        self.recalculate_tokens();
    }

    /// Add a message to the recent buffer.
    pub fn add_message(&mut self, msg: MemoryMessage) {
        self.recent_buffer.push_back(msg);
        self.recalculate_tokens();

        // Check if we need compression
        if self.needs_compression() {
            debug!(
                "Working memory at {:.0}% capacity ({}/{} tokens), compression needed",
                self.utilization() * 100.0,
                self.total_tokens,
                self.config.context_budget
            );
        }
    }

    /// Check if context window utilization exceeds the compression threshold.
    pub fn needs_compression(&self) -> bool {
        self.utilization() >= self.config.compression_threshold
    }

    /// Current utilization ratio (0.0 – 1.0).
    pub fn utilization(&self) -> f32 {
        if self.config.context_budget == 0 {
            return 0.0;
        }
        self.total_tokens as f32 / self.config.context_budget as f32
    }

    /// Get messages that should be compressed (oldest ones beyond the recent buffer).
    /// Returns the messages to compress and removes them from the buffer.
    pub fn drain_compressible_messages(&mut self) -> Vec<MemoryMessage> {
        let keep_count = self.config.recent_buffer_size;
        let mut to_compress = Vec::new();

        while self.recent_buffer.len() > keep_count {
            if let Some(msg) = self.recent_buffer.pop_front() {
                if !msg.pinned {
                    to_compress.push(msg);
                } else {
                    // Put pinned messages back (shouldn't happen for non-system)
                    self.recent_buffer.push_front(msg);
                    break;
                }
            }
        }

        self.recalculate_tokens();
        to_compress
    }

    /// Append compressed observations to the observation log.
    pub fn append_observations(&mut self, observations: &str) {
        if !self.observation_log.is_empty() {
            self.observation_log.push('\n');
        }
        self.observation_log.push_str(observations);
        self.observation_tokens = estimate_tokens(&self.observation_log);
        self.recalculate_tokens();
    }

    /// Replace the entire observation log (after reflector-level compression).
    pub fn replace_observation_log(&mut self, new_log: String) {
        self.observation_tokens = estimate_tokens(&new_log);
        self.observation_log = new_log;
        self.recalculate_tokens();
    }

    /// Update a core memory block.
    pub fn update_core_block(&mut self, block: &str, content: String) {
        match block {
            "user_profile" => self.core_blocks.user_profile = content,
            "agent_persona" => self.core_blocks.agent_persona = content,
            "task_context" => self.core_blocks.task_context = content,
            "scratchpad" => self.core_blocks.scratchpad = content,
            _ => warn!("Unknown core memory block: {block}"),
        }
        self.recalculate_tokens();
    }

    /// Build the full context to send to the LLM.
    /// Order: System → Core Blocks → Observation Log → Recent Messages
    pub fn build_context(&self) -> Vec<MemoryMessage> {
        let mut context = Vec::new();

        // 1. System prompt (always first)
        if let Some(ref sys) = self.system_prompt {
            let mut augmented = sys.clone();
            let core_str = self.core_blocks.to_context_string();
            if !core_str.is_empty() {
                augmented.content = format!("{}\n\n{}", augmented.content, core_str);
                augmented.token_count = estimate_tokens(&augmented.content);
            }
            context.push(augmented);
        }

        // 2. Observation log as a system-level context injection
        if !self.observation_log.is_empty() {
            context.push(MemoryMessage {
                role: CompactString::new("system"),
                content: format!(
                    "<observation_log>\n{}\n</observation_log>",
                    self.observation_log
                ),
                token_count: self.observation_tokens + 4,
                timestamp: chrono::Utc::now(),
                pinned: false,
            });
        }

        // 3. Recent messages (verbatim)
        for msg in &self.recent_buffer {
            context.push(msg.clone());
        }

        context
    }

    /// Recalculate total token usage across all blocks.
    fn recalculate_tokens(&mut self) {
        let sys_tokens = self.system_prompt.as_ref().map_or(0, |m| m.token_count);
        let core_tokens = self.core_blocks.total_tokens();
        let recent_tokens: u32 = self.recent_buffer.iter().map(|m| m.token_count).sum();

        self.total_tokens = sys_tokens + core_tokens + self.observation_tokens + recent_tokens;
    }

    pub fn total_tokens(&self) -> u32 {
        self.total_tokens
    }

    pub fn recent_message_count(&self) -> usize {
        self.recent_buffer.len()
    }

    /// Get token usage breakdown for diagnostics.
    pub fn token_breakdown(&self) -> TokenBreakdown {
        TokenBreakdown {
            system: self.system_prompt.as_ref().map_or(0, |m| m.token_count),
            core_blocks: self.core_blocks.total_tokens(),
            observations: self.observation_tokens,
            recent_messages: self.recent_buffer.iter().map(|m| m.token_count).sum(),
            total: self.total_tokens,
            budget: self.config.context_budget,
        }
    }
}

#[derive(Debug)]
pub struct TokenBreakdown {
    pub system: u32,
    pub core_blocks: u32,
    pub observations: u32,
    pub recent_messages: u32,
    pub total: u32,
    pub budget: u32,
}

impl std::fmt::Display for TokenBreakdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tokens: {}/{} ({:.0}%) [sys:{} core:{} obs:{} recent:{}]",
            self.total,
            self.budget,
            (self.total as f64 / self.budget as f64) * 100.0,
            self.system,
            self.core_blocks,
            self.observations,
            self.recent_messages,
        )
    }
}
