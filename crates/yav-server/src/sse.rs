//! Server-Sent Events (SSE) primitives for real-time streaming updates.
//!
//! Provides protocol formatting, channel broadcasting, and Axum response conversion
//! compatible with standard EventSource and HTMX `hx-ext="sse"`.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::sync::mpsc;

/// An individual Server-Sent Event message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseMsg {
    /// Optional event name (e.g. `"notification"` or `"message"`).
    pub event: Option<String>,
    /// Payload data string.
    pub data: String,
    /// Optional unique identifier for client reconnection (`Last-Event-ID`).
    pub id: Option<String>,
    /// Optional reconnection interval in milliseconds.
    pub retry: Option<u64>,
}

impl SseMsg {
    /// Create a new message with payload data.
    pub fn new(data: impl Into<String>) -> Self {
        Self {
            event: None,
            data: data.into(),
            id: None,
            retry: None,
        }
    }

    /// Set event type name.
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.event = Some(event.into());
        self
    }

    /// Set event ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set reconnection retry timeout in milliseconds.
    pub fn retry(mut self, millis: u64) -> Self {
        self.retry = Some(millis);
        self
    }

    /// Format message conforming to W3C Server-Sent Events protocol.
    pub fn to_sse_string(&self) -> String {
        let mut out = String::new();

        if let Some(ref ev) = self.event {
            let clean_ev: String = ev.chars().filter(|&c| c != '\r' && c != '\n').collect();
            out.push_str(&format!("event: {clean_ev}\n"));
        }
        if let Some(ref id) = self.id {
            let clean_id: String = id.chars().filter(|&c| c != '\r' && c != '\n').collect();
            out.push_str(&format!("id: {clean_id}\n"));
        }
        if let Some(retry) = self.retry {
            out.push_str(&format!("retry: {retry}\n"));
        }

        // Each line of multiline data must be prefixed by `data: `
        for line in self.data.lines() {
            out.push_str(&format!("data: {line}\n"));
        }
        if self.data.is_empty() {
            out.push_str("data:\n");
        }

        out.push('\n');
        out
    }
}

/// A stream wrapper around an `mpsc::Receiver<SseMsg>` emitting formatted bytes.
pub struct SseStream {
    receiver: mpsc::Receiver<SseMsg>,
}

impl SseStream {
    /// Create a stream wrapper from an mpsc receiver.
    pub fn new(receiver: mpsc::Receiver<SseMsg>) -> Self {
        Self { receiver }
    }
}

// Implement futures_core::Stream manually without external dependency
impl futures_util::Stream for SseStream {
    type Item = Result<String, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(msg)) => Poll::Ready(Some(Ok(msg.to_sse_string()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create an asynchronous SSE channel pair.
pub fn sse_channel(buffer: usize) -> (mpsc::Sender<SseMsg>, SseStream) {
    let (tx, rx) = mpsc::channel(buffer);
    (tx, SseStream::new(rx))
}

/// Convert an [`SseStream`] into a streaming HTTP 200 [`Response`].
pub fn sse_response(stream: SseStream) -> Response {
    let body = Body::from_stream(stream);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/event-stream; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache, no-transform"),
            (header::CONNECTION, "keep-alive"),
            (header::HeaderName::from_static("x-accel-buffering"), "no"),
        ],
        body,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_msg_formatting() {
        let msg = SseMsg::new("Hello, world!")
            .event("greeting")
            .id("101")
            .retry(3000);

        let sse_str = msg.to_sse_string();
        assert!(sse_str.contains("event: greeting\n"));
        assert!(sse_str.contains("id: 101\n"));
        assert!(sse_str.contains("retry: 3000\n"));
        assert!(sse_str.contains("data: Hello, world!\n\n"));
    }

    #[test]
    fn test_sse_multiline_data() {
        let msg = SseMsg::new("line 1\nline 2");
        let sse_str = msg.to_sse_string();
        assert!(sse_str.contains("data: line 1\ndata: line 2\n\n"));
    }

    #[test]
    fn test_sse_injection_sanitization() {
        let msg = SseMsg::new("hello")
            .event("notice\ndata: injected\n")
            .id("1\ninjected: true\n");
        let sse_str = msg.to_sse_string();
        assert!(sse_str.contains("event: noticedata: injected\n"));
        assert!(sse_str.contains("id: 1injected: true\n"));
        // Ensure newlines did not inject new header lines
        assert!(!sse_str.contains("\ndata: injected\n"));
        assert!(!sse_str.contains("\ninjected: true\n"));
    }
}
