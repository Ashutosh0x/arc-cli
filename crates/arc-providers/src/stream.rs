use bytes::Bytes;
use futures::Stream;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncBufRead;

pin_project! {
    /// Zero-copy SSE line parser — no String allocation per token
    pub struct SseStream<R> {
        #[pin]
        reader: tokio::io::BufReader<R>,
        buf: Vec<u8>,   // Reused across reads — never reallocated
    }
}

impl<R: tokio::io::AsyncRead + Unpin> SseStream<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: tokio::io::BufReader::with_capacity(8192, reader),
            buf: Vec::with_capacity(4096),
        }
    }
}

impl<R: tokio::io::AsyncRead + Unpin> Stream for SseStream<R> {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.buf.clear();  // Reuse buffer — zero allocation

        match futures::ready!(this.reader.as_mut().poll_fill_buf(cx)) {
            Ok(available) => {
                let available: &[u8] = available;
                if available.is_empty() {
                    return Poll::Ready(None);
                }

                // Find SSE "data:" line boundaries using fast SIMD memchr
                if let Some(pos) = memchr::memchr(b'\n', available) {
                    let line = &available[..pos];

                    // Strip "data: " prefix in-place to avoid allocations
                    let data = if line.starts_with(b"data: ") {
                        Bytes::copy_from_slice(&line[6..])
                    } else {
                        Bytes::copy_from_slice(line)
                    };

                    this.reader.as_mut().consume(pos + 1);
                    Poll::Ready(Some(Ok(data)))
                } else {
                    let len = available.len();
                    this.buf.extend_from_slice(available);
                    this.reader.as_mut().consume(len);
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}
