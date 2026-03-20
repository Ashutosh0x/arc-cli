//! 3-Layer Loop Detection Service
//!
//! Prevents the LLM from getting stuck in unproductive loops:
//! 1. **Tool Call Dedup** — SHA-256 hash of `name:args`, trip after N identical calls
//! 2. **Content Chanting** — Sliding-window chunk hashing with collision verification
//! 3. **LLM Double-Check** — Sends recent history to a fast model for loop confidence scoring

use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ── Thresholds ──────────────────────────────────────────────────────────────
const TOOL_CALL_LOOP_THRESHOLD: u32 = 5;
const CONTENT_LOOP_THRESHOLD: usize = 10;
const CONTENT_CHUNK_SIZE: usize = 50;
const MAX_HISTORY_LENGTH: usize = 5_000;
const LLM_CHECK_AFTER_TURNS: u32 = 30;
const DEFAULT_LLM_CHECK_INTERVAL: u32 = 10;
const MIN_LLM_CHECK_INTERVAL: u32 = 5;
const MAX_LLM_CHECK_INTERVAL: u32 = 15;
const LLM_CONFIDENCE_THRESHOLD: f64 = 0.9;

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopType {
    ConsecutiveIdenticalToolCalls,
    ContentChantingLoop,
    LlmDetectedLoop,
}

#[derive(Debug, Clone)]
pub struct LoopDetectionResult {
    pub count: u32,
    pub loop_type: Option<LoopType>,
    pub detail: Option<String>,
    pub confirmed_by_model: Option<String>,
}

impl LoopDetectionResult {
    pub fn none() -> Self {
        Self { count: 0, loop_type: None, detail: None, confirmed_by_model: None }
    }

    pub fn is_loop(&self) -> bool {
        self.count > 0
    }
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    ToolCallRequest { name: String, args: String },
    Content(String),
}

// ── Service ─────────────────────────────────────────────────────────────────

pub struct LoopDetectionService {
    disabled: bool,

    // Tool call tracking
    last_tool_call_hash: Option<String>,
    tool_call_repetition_count: u32,

    // Content chanting tracking
    stream_content_history: String,
    content_stats: HashMap<String, Vec<usize>>,
    last_content_index: usize,
    in_code_block: bool,

    // LLM-based tracking
    turns_in_current_prompt: u32,
    llm_check_interval: u32,
    last_check_turn: u32,

    // State
    loop_detected: bool,
    detected_count: u32,
    last_loop_detail: Option<String>,
    last_loop_type: Option<LoopType>,
}

impl LoopDetectionService {
    pub fn new() -> Self {
        Self {
            disabled: false,
            last_tool_call_hash: None,
            tool_call_repetition_count: 0,
            stream_content_history: String::new(),
            content_stats: HashMap::new(),
            last_content_index: 0,
            in_code_block: false,
            turns_in_current_prompt: 0,
            llm_check_interval: DEFAULT_LLM_CHECK_INTERVAL,
            last_check_turn: 0,
            loop_detected: false,
            detected_count: 0,
            last_loop_detail: None,
            last_loop_type: None,
        }
    }

    pub fn disable_for_session(&mut self) {
        self.disabled = true;
    }

    /// Process a stream event and check for loop conditions.
    pub fn add_and_check(&mut self, event: &StreamEvent) -> LoopDetectionResult {
        if self.disabled {
            return LoopDetectionResult::none();
        }
        if self.loop_detected {
            return LoopDetectionResult {
                count: self.detected_count,
                loop_type: self.last_loop_type.clone(),
                detail: self.last_loop_detail.clone(),
                confirmed_by_model: None,
            };
        }

        let (is_loop, detail, loop_type) = match event {
            StreamEvent::ToolCallRequest { name, args } => {
                self.reset_content_tracking();
                let is_loop = self.check_tool_call_loop(name, args);
                let detail = if is_loop {
                    Some(format!("Repeated tool call: {name} with args {args}"))
                } else {
                    None
                };
                (is_loop, detail, Some(LoopType::ConsecutiveIdenticalToolCalls))
            }
            StreamEvent::Content(text) => {
                let is_loop = self.check_content_loop(text);
                let detail = if is_loop {
                    let start = self.last_content_index.saturating_sub(20);
                    let end = (self.last_content_index + CONTENT_CHUNK_SIZE)
                        .min(self.stream_content_history.len());
                    Some(format!(
                        "Repeating content detected: \"{}...\"",
                        &self.stream_content_history[start..end]
                    ))
                } else {
                    None
                };
                (is_loop, detail, Some(LoopType::ContentChantingLoop))
            }
        };

        if is_loop {
            self.loop_detected = true;
            self.detected_count += 1;
            self.last_loop_detail = detail.clone();
            self.last_loop_type = loop_type.clone();
        }

        if is_loop {
            LoopDetectionResult {
                count: self.detected_count,
                loop_type,
                detail,
                confirmed_by_model: None,
            }
        } else {
            LoopDetectionResult::none()
        }
    }

    /// Signal the start of a new turn; returns whether an LLM check should be triggered.
    pub fn turn_started(&mut self) -> bool {
        if self.disabled || self.loop_detected {
            return false;
        }
        self.turns_in_current_prompt += 1;
        self.turns_in_current_prompt >= LLM_CHECK_AFTER_TURNS
            && (self.turns_in_current_prompt - self.last_check_turn) >= self.llm_check_interval
    }

    /// Record the result of an LLM-based loop check.
    pub fn record_llm_check(&mut self, confidence: f64, analysis: Option<String>, model: Option<String>) -> LoopDetectionResult {
        self.last_check_turn = self.turns_in_current_prompt;

        if confidence >= LLM_CONFIDENCE_THRESHOLD {
            self.loop_detected = true;
            self.detected_count += 1;
            self.last_loop_detail = analysis.clone();
            self.last_loop_type = Some(LoopType::LlmDetectedLoop);

            LoopDetectionResult {
                count: self.detected_count,
                loop_type: Some(LoopType::LlmDetectedLoop),
                detail: analysis,
                confirmed_by_model: model,
            }
        } else {
            self.update_check_interval(confidence);
            LoopDetectionResult::none()
        }
    }

    /// Reset all loop detection state for a new prompt.
    pub fn reset(&mut self) {
        self.last_tool_call_hash = None;
        self.tool_call_repetition_count = 0;
        self.reset_content_tracking();
        self.turns_in_current_prompt = 0;
        self.llm_check_interval = DEFAULT_LLM_CHECK_INTERVAL;
        self.last_check_turn = 0;
        self.loop_detected = false;
        self.detected_count = 0;
        self.last_loop_detail = None;
        self.last_loop_type = None;
    }

    /// Clear detection flag to allow a recovery turn.
    pub fn clear_detection(&mut self) {
        self.loop_detected = false;
    }

    // ── Private: Tool call dedup ────────────────────────────────────────────

    fn tool_call_key(name: &str, args: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{name}:{args}").as_bytes());
        hex::encode(hasher.finalize())
    }

    fn check_tool_call_loop(&mut self, name: &str, args: &str) -> bool {
        let key = Self::tool_call_key(name, args);
        if self.last_tool_call_hash.as_deref() == Some(&key) {
            self.tool_call_repetition_count += 1;
        } else {
            self.last_tool_call_hash = Some(key);
            self.tool_call_repetition_count = 1;
        }
        self.tool_call_repetition_count >= TOOL_CALL_LOOP_THRESHOLD
    }

    // ── Private: Content chanting ───────────────────────────────────────────

    fn check_content_loop(&mut self, content: &str) -> bool {
        // Reset on structural markdown elements to avoid false positives
        let has_structure = content.contains("```")
            || content.contains("| ")
            || content.contains("- ")
            || content.contains("# ")
            || content.contains("> ");

        if has_structure {
            self.reset_content_tracking();
        }

        // Toggle code block tracking
        let fence_count = content.matches("```").count();
        if fence_count % 2 != 0 {
            self.in_code_block = !self.in_code_block;
        }
        if self.in_code_block {
            return false;
        }

        self.stream_content_history.push_str(content);
        self.truncate_history();
        self.analyze_content_chunks()
    }

    fn truncate_history(&mut self) {
        if self.stream_content_history.len() <= MAX_HISTORY_LENGTH {
            return;
        }
        let truncation = self.stream_content_history.len() - MAX_HISTORY_LENGTH;
        self.stream_content_history = self.stream_content_history[truncation..].to_string();
        self.last_content_index = self.last_content_index.saturating_sub(truncation);

        // Adjust stored indices
        let mut to_remove = Vec::new();
        for (hash, indices) in self.content_stats.iter_mut() {
            indices.retain_mut(|idx| {
                *idx = idx.saturating_sub(truncation);
                *idx > 0
            });
            if indices.is_empty() {
                to_remove.push(hash.clone());
            }
        }
        for key in to_remove {
            self.content_stats.remove(&key);
        }
    }

    fn analyze_content_chunks(&mut self) -> bool {
        while self.last_content_index + CONTENT_CHUNK_SIZE <= self.stream_content_history.len() {
            let chunk = &self.stream_content_history
                [self.last_content_index..self.last_content_index + CONTENT_CHUNK_SIZE];
            let mut hasher = Sha256::new();
            hasher.update(chunk.as_bytes());
            let chunk_hash = hex::encode(hasher.finalize());

            if self.is_loop_for_chunk(chunk, &chunk_hash) {
                return true;
            }
            self.last_content_index += 1;
        }
        false
    }

    fn is_loop_for_chunk(&mut self, chunk: &str, hash: &str) -> bool {
        let indices = self.content_stats.entry(hash.to_string()).or_default();

        if indices.is_empty() {
            indices.push(self.last_content_index);
            return false;
        }

        // Verify actual content match (prevent hash collisions)
        let first_idx = indices[0];
        if first_idx + CONTENT_CHUNK_SIZE <= self.stream_content_history.len() {
            let original = &self.stream_content_history[first_idx..first_idx + CONTENT_CHUNK_SIZE];
            if original != chunk {
                return false;
            }
        }

        indices.push(self.last_content_index);

        if indices.len() < CONTENT_LOOP_THRESHOLD {
            return false;
        }

        // Check clustering of recent occurrences
        let recent: Vec<usize> = indices.iter().rev().take(CONTENT_LOOP_THRESHOLD).copied().collect();
        let total_distance = recent[0] - recent[recent.len() - 1];
        let avg_distance = total_distance / (CONTENT_LOOP_THRESHOLD - 1);
        let max_allowed = CONTENT_CHUNK_SIZE * 5;

        if avg_distance > max_allowed {
            return false;
        }

        // Verify period repetition
        let mut periods = std::collections::HashSet::new();
        for window in recent.windows(2) {
            let start = window[1];
            let end = window[0];
            if end <= self.stream_content_history.len() {
                periods.insert(
                    self.stream_content_history[start..end.min(self.stream_content_history.len())]
                        .to_string(),
                );
            }
        }

        periods.len() <= CONTENT_LOOP_THRESHOLD / 2
    }

    fn reset_content_tracking(&mut self) {
        self.stream_content_history.clear();
        self.content_stats.clear();
        self.last_content_index = 0;
    }

    fn update_check_interval(&mut self, confidence: f64) {
        let range = (MAX_LLM_CHECK_INTERVAL - MIN_LLM_CHECK_INTERVAL) as f64;
        self.llm_check_interval =
            MIN_LLM_CHECK_INTERVAL + (range * (1.0 - confidence)) as u32;
    }
}

impl Default for LoopDetectionService {
    fn default() -> Self {
        Self::new()
    }
}

/// System prompt for the LLM-based loop detection double-check.
pub const LOOP_DETECTION_SYSTEM_PROMPT: &str = r#"You are a diagnostic agent that determines whether a conversational AI assistant is stuck in an unproductive loop. Analyze the conversation history to determine this.

An unproductive state requires BOTH:
1. Repetitive pattern over at least 5 consecutive model actions
2. The repetition produces NO net change or forward progress

What is NOT a loop:
- Cross-file batch operations (same tool, different files)
- Incremental same-file edits (different line ranges)
- Retry with variation (modified arguments)

Respond with JSON: {"unproductive_state_analysis": "...", "unproductive_state_confidence": 0.0-1.0}"#;
