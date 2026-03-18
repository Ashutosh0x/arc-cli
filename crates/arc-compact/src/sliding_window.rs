use crate::tokenizer::Tokenizer;
use arc_providers::message::Message;
use std::collections::VecDeque;
use tracing::debug;

pub struct WindowConfig {
    pub max_tokens: usize,
    pub preserve_system: bool,
    pub preservation_ratio: f64, // percentage of tokens allocated to oldest vs newest
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8192,
            preserve_system: true,
            preservation_ratio: 0.5,
        }
    }
}

pub struct SlidingWindow {
    config: WindowConfig,
    tokenizer: Tokenizer,
}

impl SlidingWindow {
    pub fn new(config: WindowConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            tokenizer: Tokenizer::new()?,
        })
    }

    /// Compress the message history by dropping the oldest non-system messages
    /// when the token count exceeds the maximum limit.
    pub fn compress(&self, messages: &[Message]) -> Vec<Message> {
        let mut total_tokens = 0;
        let mut system_messages = Vec::new();
        let mut evictable = VecDeque::new();

        for msg in messages {
            let tokens = self.tokenizer.count_tokens(&msg.content);
            if self.config.preserve_system && matches!(msg.role, arc_providers::message::Role::System) {
                total_tokens += tokens;
                system_messages.push(msg.clone());
            } else {
                evictable.push_back((msg.clone(), tokens));
            }
        }

        if total_tokens >= self.config.max_tokens {
            // Already full with just system messages, return only what fits
            return system_messages;
        }

        let mut available_slots = self.config.max_tokens - total_tokens;
        let mut output_evictable = VecDeque::new();

        // Process evictable messages from newest to oldest
        while let Some((msg, tokens)) = evictable.pop_back() {
            if available_slots >= tokens {
                output_evictable.push_front(msg);
                available_slots -= tokens;
            } else {
                // Cannot fit the whole message, we could truncate, but usually dropping is safer for multi-turn.
                debug!("SlidingWindow: Evicted message due to token limit.");
                break;
            }
        }

        let mut final_messages = system_messages;
        final_messages.extend(output_evictable);
        final_messages
    }
}
