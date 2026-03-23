// SPDX-License-Identifier: MIT
//! Zero-copy SSE (Server-Sent Events) line parser for streaming responses.

use bytes::BytesMut;
use tokio_util::codec::Decoder;

/// SSE line decoder — zero-copy parsing of `data: ...` lines.
pub struct SseDecoder;

#[derive(Debug)]
pub enum SseEvent {
    Data(String),
    Done,
}

impl Decoder for SseDecoder {
    type Item = SseEvent;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Find newline
        if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes = buf.split_to(pos + 1);
            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();

            if line.is_empty() {
                return Ok(None); // Empty line, skip
            }

            if line.starts_with("data: [DONE]") {
                return Ok(Some(SseEvent::Done));
            }

            if let Some(data) = line.strip_prefix("data: ") {
                return Ok(Some(SseEvent::Data(data.to_string())));
            }

            // Ignore non-data lines (comments, event types, etc.)
            Ok(None)
        } else {
            Ok(None) // Need more data
        }
    }
}
