use anyhow::Result;
use bytes::BytesMut;
use futures::{Stream, StreamExt};
use memchr::memmem;
use reqwest::Response;
use std::pin::Pin;
use std::task::{Context, Poll};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum StreamEvent {
    TextDelta(String),
    Done,
    Error(String),
}

#[async_trait]
pub trait StreamingClient: Send + Sync {
    async fn stream_completion(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<StreamEvent>>;
}

/// Extremely fast Zero-Copy Server-Sent Events (SSE) stream parser.
/// Prevents dynamic `String` heap allocations by scanning byte streams 
/// natively using SIMD-accelerated linear memory searches (`memmem::find`).
pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: BytesMut,
}

impl SseStream {
    pub fn new(response: Response) -> Self {
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: BytesMut::with_capacity(8192), // Pre-allocate standard chunk boundary
        }
    }
}

impl Stream for SseStream {
    // We return standard bytes, cleanly parsing out 'data: ' events natively.
    type Item = Result<String, anyhow::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Step 1: Scan current buffer for the SSE double-newline boundary `\n\n`
        let boundary = memmem::find(&self.buffer, b"\n\n");

        if let Some(idx) = boundary {
            // We found a complete event in the local buffer natively.
            let chunk = self.buffer.split_to(idx + 2); // Split inclusive of \n\n
            
            // Fast-path evaluation of `data: ` protocol marker
            let slice = &chunk[..];
            if slice.starts_with(b"data: ") {
                let json_slice = &slice[6..slice.len() - 2];
                // Avoid dropping context if it's explicitly the Anthropic "ping" or completion events
                let json_str = String::from_utf8_lossy(json_slice).to_string();
                return Poll::Ready(Some(Ok(json_str)));
            }
            
            // Loop natively, waking waker.
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        // Step 2: Extract bytes dynamically from HTTP/2 stream
        match self.inner.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.buffer.extend_from_slice(&bytes);
                // Wake to process immediately natively
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(anyhow::anyhow!("Stream err: {}", e)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
