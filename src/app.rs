// ARC CLI — Application state
// Expanded with real agent tracking, task lifecycle, logs, diff, LLM usage.
// 4 screens: Prompt, AgentView, DiffView, Output.
// Dual-mode: Chat (direct LLM) vs Agent (full pipeline).

use std::time::Instant;

use crate::models::{AgentLog, DiffResult, LLMUsage, Task};

/// Determines how the prompt is routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    /// Simple chat — direct LLM call, streamed to Output screen.
    Chat,
    /// Simple code — direct Coder (skip Architect), streamed to Output.
    FastCode,
    /// Complex task — full RepoMap → Architect → Coder pipeline.
    Agent,
}

impl PromptMode {
    pub fn label(&self) -> &'static str {
        match self {
            PromptMode::Chat => "CHAT",
            PromptMode::FastCode => "FAST",
            PromptMode::Agent => "AGENT",
        }
    }
}

/// Classify a prompt into Chat (simple), FastCode (code but simple), or Agent (complex build task).
pub fn classify_prompt(prompt: &str) -> PromptMode {
    let lower = prompt.to_lowercase();

    // Agent keywords — complex multi-step tasks needing architecture
    let agent_keywords = [
        "build full", "scaffold", "setup project", "refactor entire",
        "migrate", "full backend", "full frontend", "full stack",
        "microservice", "deploy", "database schema", "multi-file",
        "project structure", "directory structure",
    ];
    if agent_keywords.iter().any(|k| lower.contains(k)) {
        return PromptMode::Agent;
    }

    // FastCode keywords — simple code generation, skip Architect
    let code_keywords = [
        "write", "create", "generate", "implement", "code",
        "function", "program", "script", "server", "api",
        "fix", "debug", "test", "struct", "class", "module",
        "sort", "search", "parse", "handler", "endpoint",
    ];
    if code_keywords.iter().any(|k| lower.contains(k)) {
        return PromptMode::FastCode;
    }

    PromptMode::Chat
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Prompt,
    AgentView,
    DiffView,
    Output,
}

impl Screen {
    pub fn label(&self) -> &'static str {
        match self {
            Screen::Prompt => "Prompt",
            Screen::AgentView => "Agents",
            Screen::DiffView => "Diff",
            Screen::Output => "Output",
        }
    }

    pub const ALL: [Screen; 4] = [
        Screen::Prompt,
        Screen::AgentView,
        Screen::DiffView,
        Screen::Output,
    ];
}

#[derive(Debug, Clone)]
pub struct ModelEntry {
    pub name: &'static str,
    pub provider: &'static str,
    pub tag: &'static str,
    pub ollama_model: &'static str,
}

pub const MODELS: &[ModelEntry] = &[
    ModelEntry { name: "Claude v3.5 Sonnet", provider: "Anthropic",    tag: "Fast",    ollama_model: "claude-3.5-sonnet" },
    ModelEntry { name: "Gemma 4 (Ollama)",   provider: "Google/Ollama", tag: "Local",   ollama_model: "gemma4:latest" },
    ModelEntry { name: "Llama 3 8B",         provider: "Meta/Groq",    tag: "OSS",     ollama_model: "llama3:8b" },
    ModelEntry { name: "Grok 4.1 Fast",      provider: "xAI",          tag: "Fast",    ollama_model: "grok-4.1" },
    ModelEntry { name: "GPT-4o",             provider: "OpenAI",       tag: "Premium", ollama_model: "gpt-4o" },
];

pub struct App {
    // ── Navigation ──
    pub screen: Screen,
    pub selected_model: usize,
    pub editing: bool,
    pub running: bool,
    pub tick: u64,

    // ── Prompt ──
    pub prompt_text: String,

    // ── Prompt mode ──
    pub mode: PromptMode,

    // ── Response streaming ──
    pub response_text: String,
    pub streaming: bool,
    pub scroll_offset: u16,

    // ── Agent orchestration (REAL) ──
    pub agent_logs: Vec<AgentLog>,
    pub tasks: Vec<Task>,
    pub pipeline_running: bool,
    pub pipeline_complete: bool,
    pub pipeline_failed: bool,
    pub pipeline_error: Option<String>,
    pub agent_log_scroll: u16,

    // ── Diff view (REAL) ──
    pub current_diff: Option<DiffResult>,
    pub diff_scroll: u16,

    // ── LLM usage stats (REAL) ──
    pub llm_usage: Vec<LLMUsage>,

    // ── Session timing ──
    pub session_start: Instant,

    // ── Provider health ──
    pub ollama_healthy: Option<bool>,
    pub openai_healthy: Option<bool>,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Prompt,
            selected_model: 1, // Gemma 4 selected by default
            prompt_text: String::new(),
            editing: false,
            running: true,
            tick: 0,

            mode: PromptMode::Chat,

            response_text: String::new(),
            streaming: false,
            scroll_offset: 0,

            agent_logs: Vec::new(),
            tasks: Vec::new(),
            pipeline_running: false,
            pipeline_complete: false,
            pipeline_failed: false,
            pipeline_error: None,
            agent_log_scroll: 0,

            current_diff: None,
            diff_scroll: 0,

            llm_usage: Vec::new(),
            session_start: Instant::now(),

            ollama_healthy: None,
            openai_healthy: None,
        }
    }

    pub fn next_model(&mut self) {
        self.selected_model = (self.selected_model + 1) % MODELS.len();
    }

    pub fn prev_model(&mut self) {
        if self.selected_model == 0 {
            self.selected_model = MODELS.len() - 1;
        } else {
            self.selected_model -= 1;
        }
    }

    pub fn next_screen(&mut self) {
        let idx = Screen::ALL.iter().position(|s| *s == self.screen).unwrap_or(0);
        self.screen = Screen::ALL[(idx + 1) % Screen::ALL.len()];
    }

    pub fn prev_screen(&mut self) {
        let idx = Screen::ALL.iter().position(|s| *s == self.screen).unwrap_or(0);
        self.screen = if idx == 0 {
            Screen::ALL[Screen::ALL.len() - 1]
        } else {
            Screen::ALL[idx - 1]
        };
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Total LLM latency across all agent calls in this session.
    pub fn total_latency_ms(&self) -> u64 {
        self.llm_usage.iter().map(|u| u.latency_ms).sum()
    }

    /// Total tokens used in this session.
    pub fn total_tokens(&self) -> u64 {
        self.llm_usage.iter().map(|u| u.total_tokens).sum()
    }

    /// Session uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.session_start.elapsed().as_secs()
    }
}
