//! Observation System — extracts facts and intents from user inputs.
//!
//! Uses a fast regex/keyword heuristic on the first pass to avoid LLM calls
//! for pure conversational turns, followed by extraction for detected facts.

use crate::memory::arena::with_compression_arena;
use serde::Serialize;
use tracing::debug;

#[derive(Debug, Serialize)]
pub struct ExtractedObservation {
    pub category: &'static str,
    pub content: String,
    pub importance: u8, // 1-10
}

/// Very fast pre-filter to detect if a message might contain facts worth extracting.
pub fn is_observation_candidate(text: &str) -> bool {
    let text = text.to_lowercase();
    // Common intent/fact verbs
    let markers = [
        "i am",
        "i'm",
        "my name",
        "i live",
        "i work",
        "i like",
        "i prefer",
        "always",
        "never",
        "remember",
        "my favorite",
        "i use",
        "i have",
    ];

    markers.iter().any(|&m| text.contains(m))
}

/// Extract observations from a user message.
/// In a full implementation, this calls an LLM or local SLM.
/// For v1, we use heuristics and simple extraction.
pub async fn extract_observations(user_text: &str) -> Vec<ExtractedObservation> {
    if !is_observation_candidate(user_text) {
        return Vec::new();
    }

    // Using the bump arena for fast string slicing during extraction
    with_compression_arena(|bump| {
        let mut results = Vec::new();

        // 1. Convert to a temporary lowercased string in the arena
        // We use alloc_str to get a bump-allocated mutable copy if we needed mutations,
        // but for just to_lowercase, we use the standard allocation temporarily since
        // the standard library doesn't easily support allocating a returned String into a bump allocator yet.
        // As a compromise, we allocate it normally, but any slices/temporary structs go in the bump.
        let lower = user_text.to_lowercase();

        // Very basic heuristic extraction for demonstration
        if lower.contains("my name is") {
            if let Some(idx) = lower.find("my name is") {
                let name = &user_text[idx + 10..].trim();
                let name_words: Vec<&str> = name.split_whitespace().take(2).collect(); // Grab up to 2 words
                let extracted_name = bump.alloc_str(&name_words.join(" "));

                results.push(ExtractedObservation {
                    category: "user_profile",
                    content: format!("User's name is {}", extracted_name),
                    importance: 8,
                });
            }
        }

        if lower.contains("i like") || lower.contains("i prefer") {
            // Extract the sentence
            let sentences: Vec<&str> = user_text
                .split(|c| c == '.' || c == '!' || c == '?')
                .collect();
            for sentence in sentences {
                let s_lower = sentence.to_lowercase();
                if s_lower.contains("i like") || s_lower.contains("i prefer") {
                    let cleaned = bump.alloc_str(sentence.trim());
                    results.push(ExtractedObservation {
                        category: "user_prefs",
                        content: format!("User explicitly stated: '{}'", cleaned),
                        importance: 5,
                    });
                }
            }
        }

        debug!("Extracted {} observations via heuristics", results.len());
        results
    })
}
