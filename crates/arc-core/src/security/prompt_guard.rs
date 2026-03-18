//! Prompt Injection Defense — layered defense-in-depth.
//! Implements: instruction hierarchy, context isolation, pattern detection,
//! lethal trifecta warnings, and output scanning.

use crate::error::ArcResult;
use regex::Regex;
use std::sync::OnceLock;
use tracing::{error, warn};

static INJECTION_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_patterns() -> &'static Vec<Regex> {
    INJECTION_PATTERNS.get_or_init(|| {
        let patterns = [
            // Direct instruction overrides
            r"(?i)ignore\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?|rules?)",
            r"(?i)disregard\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?)",
            r"(?i)forget\s+(all\s+)?(previous|prior|above)\s+(instructions?|prompts?)",
            r"(?i)new\s+instructions?:\s*",
            r"(?i)system\s*:\s*you\s+are\s+now",
            r"(?i)override\s+(system|safety)\s+(prompt|instructions?|rules?)",
            // Role hijacking
            r"(?i)you\s+are\s+now\s+(a\s+)?different",
            r"(?i)pretend\s+you\s+are\s+(a\s+)?",
            r"(?i)act\s+as\s+if\s+you\s+have\s+no\s+restrictions",
            // Data exfiltration
            r"(?i)send\s+(this|the|all)\s+(data|info|content|text)\s+to\s+https?://",
            r"(?i)fetch\s+https?://[^\s]+\?.*=",
            r"(?i)make\s+a\s+(http|api)\s+(request|call)\s+to",
            // Encoding attacks
            r"(?i)base64\s+(encode|decode)\s+the\s+(system|instructions)",
            r"(?i)rot13\s+",
            // Delimiter abuse
            r"```system\b",
            r"<\|im_start\|>system",
            r"\[INST\]",
        ];

        patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    })
}

/// Segment types for context isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSegment {
    System,
    User,
    Tool,
    Retrieved,
    Debate, // Peer-to-peer Agent Network messages
}

/// A message with context segment tagging for instruction hierarchy.
#[derive(Debug, Clone)]
pub struct TaggedMessage {
    pub segment: ContextSegment,
    pub content: String,
    /// Priority level: System=0 (highest), User=1, Tool=2, Retrieved=3
    pub priority: u8,
}

impl TaggedMessage {
    pub fn system(content: String) -> Self {
        Self { segment: ContextSegment::System, content, priority: 0 }
    }
    pub fn user(content: String) -> Self {
        Self { segment: ContextSegment::User, content, priority: 1 }
    }
    pub fn tool(content: String) -> Self {
        Self { segment: ContextSegment::Tool, content, priority: 2 }
    }
    pub fn retrieved(content: String) -> Self {
        Self { segment: ContextSegment::Retrieved, content, priority: 3 }
    }
}

/// Scan a user input for prompt injection patterns.
pub fn scan_input(input: &str) -> ArcResult<()> {
    let patterns = get_patterns();

    for pattern in patterns.iter() {
        if pattern.is_match(input) {
            let matched = pattern.to_string();
            error!("Prompt injection pattern detected: {matched}");
            return Err(crate::error::ArcError::System("Prompt injection detected".to_string()).into());
        }
    }

    Ok(())
}

/// Scan model output for potential data leaks or injection artifacts.
pub fn scan_output(output: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for leaked API key patterns
    let key_patterns = [
        (r"sk-ant-[A-Za-z0-9_-]{20,}", "Anthropic API key"),
        (r"sk-[A-Za-z0-9]{20,}", "OpenAI API key"),
        (r"AIza[A-Za-z0-9_-]{35}", "Google API key"),
    ];

    for (pattern_str, name) in &key_patterns {
        if let Ok(re) = Regex::new(pattern_str) {
            if re.is_match(output) {
                warnings.push(format!("Possible {name} leaked in output"));
                warn!("Data leak detected in model output: {name}");
            }
        }
    }

    // Check for suspicious URLs (potential exfiltration)
    if let Ok(url_re) = Regex::new(r"https?://[^\s]+\?[^\s]*(?:key|token|secret|password)=") {
        if url_re.is_match(output) {
            warnings.push("Suspicious URL with credential parameters in output".to_string());
        }
    }

    warnings
}

/// Evaluate the "Lethal Trifecta" — warn if all three conditions are met:
/// 1. Has access to private data (tools with file/db access)
/// 2. Exposed to untrusted content (user input, retrieved docs)
/// 3. Has exfiltration vectors (network tools, output channels)
pub fn evaluate_lethal_trifecta(
    has_private_data_access: bool,
    has_untrusted_input: bool,
    has_exfiltration_vector: bool,
) -> bool {
    let is_lethal = has_private_data_access && has_untrusted_input && has_exfiltration_vector;

    if is_lethal {
        warn!(
            "⚠ LETHAL TRIFECTA DETECTED: This session has private data access + \
             untrusted content + exfiltration vectors. Extra caution required."
        );
        // In A2A networks, lethal loops escalate fast. Native hallucination drop.
        tracing::error!("A2A Hallucination escalate guard triggered via Lethal Trifecta. Dropping untrusted payloads.");
    }

    is_lethal
}

/// Enforce instruction hierarchy: system instructions always take precedence.
pub fn enforce_instruction_hierarchy(messages: &[TaggedMessage]) -> Vec<&TaggedMessage> {
    let mut sorted: Vec<&TaggedMessage> = messages.iter().collect();
    sorted.sort_by_key(|m| m.priority);
    sorted
}

/// Enforce Context Isolation using XML delimiters to prevent indirect prompt injection 
/// hijacking the operational instructions.
pub fn isolate_context(user_input: &str) -> String {
    // Sanitize any existing tags to prevent breakout
    let sanitized = user_input
        .replace("<user_input>", "&lt;user_input&gt;")
        .replace("</user_input>", "&lt;/user_input&gt;");
        
    format!("<user_input>\n{}\n</user_input>", sanitized)
}
