//! Streaming SSR engine for chunked HTML transfer and async Suspense-like resolution.
//!
//! Allows immediate flushing of the document `<head>` and top layout shell (<5ms TTFB),
//! streaming dynamic HTML fragments into placeholders as background asynchronous tasks finish.

use std::{
    convert::Infallible,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tokio::sync::mpsc;

static CHUNK_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// A streaming HTML response builder.
pub struct StreamingHtml {
    initial_shell: String,
    receiver: mpsc::Receiver<String>,
    closing_shell: String,
    sent_initial: bool,
}

impl StreamingHtml {
    /// Create a new streaming HTML response with an initial shell and an async channel for chunks.
    pub fn new(
        initial_shell: impl Into<String>,
        receiver: mpsc::Receiver<String>,
        closing_shell: impl Into<String>,
    ) -> Self {
        Self {
            initial_shell: initial_shell.into(),
            receiver,
            closing_shell: closing_shell.into(),
            sent_initial: false,
        }
    }

    /// Helper to produce a replacement chunk that swaps a placeholder element by ID.
    pub fn swap_placeholder(target_id: &str, html: &str) -> String {
        let id_num = CHUNK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_id = format!("yav-chunk-{id_num}");
        let safe_target_id =
            serde_json::to_string(target_id).unwrap_or_else(|_| "\"\"".to_string());
        let safe_html = html
            .replace("</template>", "<\\/template>")
            .replace("</TEMPLATE>", "<\\/TEMPLATE>");
        format!(
            r#"<template id="{temp_id}">{safe_html}</template><script>(function(){{var t=document.getElementById('{temp_id}');var target=document.getElementById({safe_target_id});if(t&&target){{target.replaceWith(t.content.cloneNode(true));t.remove();}}}})();</script>"#
        )
    }

    /// Convert into an Axum [`Response`] with `Transfer-Encoding: chunked`.
    pub fn into_response(self) -> Response {
        let body = Body::from_stream(self);
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache, no-transform"),
                (header::HeaderName::from_static("x-accel-buffering"), "no"),
            ],
            body,
        )
            .into_response()
    }
}

impl futures_util::Stream for StreamingHtml {
    type Item = Result<String, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // 1. Send the initial shell immediately
        if !self.sent_initial {
            self.sent_initial = true;
            let initial = std::mem::take(&mut self.initial_shell);
            return Poll::Ready(Some(Ok(initial)));
        }

        // 2. Poll the channel for subsequent async fragments
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(chunk))),
            Poll::Ready(None) => {
                // 3. Channel closed: send closing shell if not empty
                if !self.closing_shell.is_empty() {
                    let closing = std::mem::take(&mut self.closing_shell);
                    Poll::Ready(Some(Ok(closing)))
                } else {
                    Poll::Ready(None)
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Helper creating a channel pair and returning a sender alongside the [`StreamingHtml`] instance.
pub fn streaming_html(
    initial_shell: impl Into<String>,
    closing_shell: impl Into<String>,
    buffer: usize,
) -> (mpsc::Sender<String>, StreamingHtml) {
    let (tx, rx) = mpsc::channel(buffer);
    let streaming = StreamingHtml::new(initial_shell, rx, closing_shell);
    (tx, streaming)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_placeholder() {
        let chunk =
            StreamingHtml::swap_placeholder("metrics-skeleton", "<div>1,234 Visiteurs</div>");
        assert!(chunk.contains("metrics-skeleton"));
        assert!(chunk.contains("1,234 Visiteurs"));
        assert!(chunk.contains("template id="));
        assert!(chunk.contains("target.replaceWith"));
    }

    #[test]
    fn test_swap_placeholder_escaping() {
        let dangerous_id = r#"foo'); alert('xss'); ('"#;
        let dangerous_html = "<div>Content with </template> embedded</div>";
        let chunk = StreamingHtml::swap_placeholder(dangerous_id, dangerous_html);
        assert!(!chunk.contains("getElementById('foo')"));
        assert!(chunk.contains(r#"getElementById("foo'); alert('xss'); ('")"#));
        assert!(!chunk.contains("</template> embedded"));
        assert!(chunk.contains(r#"<\/template> embedded"#));

        // Test double quote breakout attempt
        let dbl_quote_id = r#"foo"); alert('xss'); ("#;
        let chunk2 = StreamingHtml::swap_placeholder(dbl_quote_id, "<div>ok</div>");
        assert!(chunk2.contains(r#"getElementById("foo\"); alert('xss'); ("#));
    }
}
