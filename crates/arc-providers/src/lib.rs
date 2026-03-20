#![forbid(unsafe_code)]

pub mod provider;
pub mod anthropic;
pub mod openai;
pub mod gemini;
pub mod ollama;
pub mod router;
pub mod stream;
pub mod security;
pub mod message;
pub mod traits;

pub mod breaker;
pub mod routing;
pub mod mock;

// Phase 28: Model Availability & Fallback
pub mod availability;
pub mod fallback;

pub mod streaming {
    pub use crate::stream::{StreamingClient, StreamEvent};

    /// A parsed SSE event with event type and data fields.
    #[derive(Debug, Clone)]
    pub struct ParsedSseEvent {
        pub event_type: String,
        pub data: String,
    }

    /// Parse a raw SSE byte chunk into individual events.
    /// Splits on double-newline boundaries, extracts `event:` and `data:` fields.
    pub fn parse_sse_chunk(input: &[u8]) -> Vec<ParsedSseEvent> {
        let text = String::from_utf8_lossy(input);
        if text.is_empty() {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut current_event_type = String::from("message");
        let mut current_data_lines: Vec<String> = Vec::new();

        for line in text.lines() {
            let line = line.trim_end_matches('\r');

            if line.is_empty() {
                // Empty line = event boundary
                if !current_data_lines.is_empty() {
                    events.push(ParsedSseEvent {
                        event_type: current_event_type.clone(),
                        data: current_data_lines.join("\n"),
                    });
                    current_data_lines.clear();
                    current_event_type = String::from("message");
                }
                continue;
            }

            if let Some(event_type) = line.strip_prefix("event: ").or_else(|| line.strip_prefix("event:")) {
                current_event_type = event_type.trim().to_string();
            } else if let Some(data) = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")) {
                current_data_lines.push(data.to_string());
            }
            // Ignore comments (lines starting with :), id fields, and other non-standard lines
        }

        // Flush any remaining buffered event (stream cut off without trailing blank line)
        if !current_data_lines.is_empty() {
            events.push(ParsedSseEvent {
                event_type: current_event_type,
                data: current_data_lines.join("\n"),
            });
        }

        events
    }
}

