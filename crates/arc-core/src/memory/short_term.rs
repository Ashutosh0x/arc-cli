//! Short-Term Memory — in-memory session history with ring-buffer eviction.
//! Holds the complete raw conversation history for the current session,
//! independent of what's in the working memory context window.

use crate::memory::working::MemoryMessage;
use std::collections::VecDeque;
use tracing::debug;

/// Ring-buffer session history with configurable capacity.
pub struct ShortTermMemory {
    /// Full conversation history for this session
    history: VecDeque<MemoryMessage>,
    /// Maximum messages before oldest are evicted
    capacity: usize,
    /// Total messages ever added (for session stats)
    total_added: u64,
}

impl ShortTermMemory {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(capacity.min(1024)),
            capacity,
            total_added: 0,
        }
    }

    /// Record a message into session history.
    pub fn record(&mut self, msg: MemoryMessage) {
        if self.history.len() >= self.capacity {
            self.history.pop_front();
            debug!(
                "Short-term memory evicted oldest message (cap: {})",
                self.capacity
            );
        }
        self.history.push_back(msg);
        self.total_added += 1;
    }

    /// Get the last N messages from history.
    pub fn recent(&self, n: usize) -> Vec<&MemoryMessage> {
        self.history.iter().rev().take(n).rev().collect()
    }

    /// Get all messages in history.
    pub fn all(&self) -> &VecDeque<MemoryMessage> {
        &self.history
    }

    /// Search history for messages containing a keyword.
    pub fn search(&self, keyword: &str) -> Vec<&MemoryMessage> {
        let keyword_lower = keyword.to_lowercase();
        self.history
            .iter()
            .filter(|m| m.content.to_lowercase().contains(&keyword_lower))
            .collect()
    }

    /// Total token count across all stored messages.
    pub fn total_tokens(&self) -> u32 {
        self.history.iter().map(|m| m.token_count).sum()
    }

    pub fn len(&self) -> usize {
        self.history.len()
    }

    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    pub fn total_messages_processed(&self) -> u64 {
        self.total_added
    }

    /// Export the full history for session persistence.
    pub fn export(&self) -> Vec<MemoryMessage> {
        self.history.iter().cloned().collect()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.history.clear();
    }
}
