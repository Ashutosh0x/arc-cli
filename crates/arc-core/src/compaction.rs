//! # Context Compaction — Intelligent Context Window Management
//!
//! Auto-compacts when context limit approached. Circuit breaker after 3 failures.
//! Image stripping, token counting, plan preservation.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Token budget configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub max_context_tokens: usize,
    pub compaction_threshold: f64,
    pub protection_window: usize,
    pub max_output_tokens: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_context_tokens: 200_000,
            compaction_threshold: 0.85,
            protection_window: 50_000,
            max_output_tokens: 16_384,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactMessage {
    pub role: MessageRole,
    pub content: String,
    pub token_estimate: usize,
    #[serde(default)]
    pub has_media: bool,
    #[serde(default)]
    pub is_plan: bool,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    pub turn: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    Summarize,
    StripMedia,
    TruncateToolOutputs,
    DropOldTurns,
}

#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub summary: String,
    pub messages_removed: usize,
    pub tokens_freed: usize,
    pub new_total_tokens: usize,
    pub strategy: CompactionStrategy,
}

struct CircuitBreaker {
    failure_count: u32,
    max_failures: u32,
    last_failure: Option<Instant>,
    cooldown: Duration,
}

impl CircuitBreaker {
    fn new(max_failures: u32, cooldown: Duration) -> Self {
        Self {
            failure_count: 0,
            max_failures,
            last_failure: None,
            cooldown,
        }
    }
    fn is_open(&self) -> bool {
        if self.failure_count >= self.max_failures {
            if let Some(last) = self.last_failure {
                return last.elapsed() < self.cooldown;
            }
        }
        false
    }
    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
    }
    fn record_success(&mut self) {
        self.failure_count = 0;
        self.last_failure = None;
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

pub fn estimate_message_tokens(msg: &CompactMessage) -> usize {
    4 + msg.token_estimate + if msg.has_media { 500 } else { 0 }
}

/// Main compaction engine.
pub struct ContextCompactor {
    budget: TokenBudget,
    circuit_breaker: CircuitBreaker,
    compaction_count: u32,
}

impl ContextCompactor {
    pub fn new() -> Self {
        Self {
            budget: TokenBudget::default(),
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(60)),
            compaction_count: 0,
        }
    }
    pub fn with_budget(budget: TokenBudget) -> Self {
        Self {
            budget,
            circuit_breaker: CircuitBreaker::new(3, Duration::from_secs(60)),
            compaction_count: 0,
        }
    }
    pub fn needs_compaction(&self, total_tokens: usize) -> bool {
        total_tokens
            > (self.budget.max_context_tokens as f64 * self.budget.compaction_threshold) as usize
    }
    pub fn utilization(&self, total_tokens: usize) -> f64 {
        total_tokens as f64 / self.budget.max_context_tokens as f64
    }

    pub fn compact(
        &mut self,
        messages: &mut Vec<CompactMessage>,
    ) -> Result<CompactionResult, String> {
        if self.circuit_breaker.is_open() {
            return Err("Compaction circuit breaker open".into());
        }
        // Phase 1: Strip media
        let media_freed = self.strip_old_media(messages);
        let t1: usize = messages.iter().map(|m| estimate_message_tokens(m)).sum();
        if !self.needs_compaction(t1) {
            self.circuit_breaker.record_success();
            self.compaction_count += 1;
            return Ok(CompactionResult {
                summary: String::new(),
                messages_removed: 0,
                tokens_freed: media_freed,
                new_total_tokens: t1,
                strategy: CompactionStrategy::StripMedia,
            });
        }
        // Phase 2: Truncate tool outputs
        let tool_freed = self.truncate_tool_outputs(messages);
        let t2: usize = messages.iter().map(|m| estimate_message_tokens(m)).sum();
        if !self.needs_compaction(t2) {
            self.circuit_breaker.record_success();
            self.compaction_count += 1;
            return Ok(CompactionResult {
                summary: String::new(),
                messages_removed: 0,
                tokens_freed: media_freed + tool_freed,
                new_total_tokens: t2,
                strategy: CompactionStrategy::TruncateToolOutputs,
            });
        }
        // Phase 3: Summarize old turns
        let boundary = self.protection_boundary(messages);
        if boundary == 0 {
            self.circuit_breaker.record_failure();
            return Err("All messages in protection window".into());
        }
        let summary = self.generate_summary(&messages[..boundary]);
        let old: Vec<CompactMessage> = messages.drain(..boundary).collect();
        let old_tok: usize = old.iter().map(|m| estimate_message_tokens(m)).sum();
        let stok = estimate_tokens(&summary);
        messages.insert(
            0,
            CompactMessage {
                role: MessageRole::System,
                content: format!("[Compacted]\n{summary}"),
                token_estimate: stok,
                has_media: false,
                is_plan: false,
                tool_use_id: None,
                turn: 0,
            },
        );
        let new_total: usize = messages.iter().map(|m| estimate_message_tokens(m)).sum();
        self.circuit_breaker.record_success();
        self.compaction_count += 1;
        Ok(CompactionResult {
            summary,
            messages_removed: old.len(),
            tokens_freed: old_tok.saturating_sub(stok),
            new_total_tokens: new_total,
            strategy: CompactionStrategy::Summarize,
        })
    }

    fn strip_old_media(&self, messages: &mut [CompactMessage]) -> usize {
        let b = self.protection_boundary(messages);
        let mut freed = 0;
        for msg in &mut messages[..b] {
            if msg.has_media {
                msg.has_media = false;
                freed += 500;
            }
        }
        freed
    }

    fn truncate_tool_outputs(&self, messages: &mut [CompactMessage]) -> usize {
        let b = self.protection_boundary(messages);
        let mut freed = 0;
        for msg in &mut messages[..b] {
            if msg.role == MessageRole::Tool && msg.token_estimate > 2000 {
                let old = msg.token_estimate;
                let keep = 2000usize.min(msg.content.len());
                msg.content = format!("{}...[truncated]", &msg.content[..keep]);
                msg.token_estimate = estimate_tokens(&msg.content);
                freed += old.saturating_sub(msg.token_estimate);
            }
        }
        freed
    }

    fn protection_boundary(&self, messages: &[CompactMessage]) -> usize {
        let mut acc = 0;
        for (i, msg) in messages.iter().enumerate().rev() {
            acc += estimate_message_tokens(msg);
            if acc >= self.budget.protection_window {
                return i;
            }
        }
        0
    }

    fn generate_summary(&self, messages: &[CompactMessage]) -> String {
        let uc = messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count();
        let ac = messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();
        let tc = messages
            .iter()
            .filter(|m| m.role == MessageRole::Tool)
            .count();
        format!("Compacted: {uc} user msgs, {ac} assistant msgs, {tc} tool calls.")
    }

    pub fn stats(&self) -> CompactionStats {
        CompactionStats {
            total_compactions: self.compaction_count,
            circuit_breaker_open: self.circuit_breaker.is_open(),
            max_context_tokens: self.budget.max_context_tokens,
            compaction_threshold: self.budget.compaction_threshold,
        }
    }
}

impl Default for ContextCompactor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionStats {
    pub total_compactions: u32,
    pub circuit_breaker_open: bool,
    pub max_context_tokens: usize,
    pub compaction_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextDiagnostic {
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub utilization_percent: f64,
    pub system_prompt_tokens: usize,
    pub user_message_tokens: usize,
    pub assistant_tokens: usize,
    pub tool_result_tokens: usize,
    pub media_tokens: usize,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub fn diagnose_context(messages: &[CompactMessage], budget: &TokenBudget) -> ContextDiagnostic {
    let mut d = ContextDiagnostic {
        total_tokens: 0,
        max_tokens: budget.max_context_tokens,
        utilization_percent: 0.0,
        system_prompt_tokens: 0,
        user_message_tokens: 0,
        assistant_tokens: 0,
        tool_result_tokens: 0,
        media_tokens: 0,
        warnings: Vec::new(),
        suggestions: Vec::new(),
    };
    for msg in messages {
        let t = estimate_message_tokens(msg);
        d.total_tokens += t;
        match msg.role {
            MessageRole::System => d.system_prompt_tokens += t,
            MessageRole::User => d.user_message_tokens += t,
            MessageRole::Assistant => d.assistant_tokens += t,
            MessageRole::Tool => d.tool_result_tokens += t,
        }
        if msg.has_media {
            d.media_tokens += 500;
        }
    }
    d.utilization_percent = (d.total_tokens as f64 / budget.max_context_tokens as f64) * 100.0;
    if d.utilization_percent > 90.0 {
        d.warnings
            .push(format!("Context {:.0}% full", d.utilization_percent));
    }
    if d.tool_result_tokens > d.total_tokens / 2 {
        d.suggestions.push("Use /compact to free space".into());
    }
    d
}
