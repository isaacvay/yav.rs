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
    nonce: Option<String>,
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
            nonce: None,
        }
    }

    /// Attach the per-request CSP nonce to this stream.
    ///
    /// Every emitted chunk (including the initial and closing shells) has the
    /// nonce injected into its `<script>` tags, so a strict `script-src
    /// 'nonce-…'` policy works with streaming Suspense instead of blocking the
    /// swap scripts.
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }

    fn apply_nonce(&self, chunk: String) -> String {
        match self.nonce.as_deref() {
            Some(n) if !n.is_empty() => crate::handler::inject_csp_nonce_into_scripts(&chunk, n),
            _ => chunk,
        }
    }
}

/// Neutralises an HTML end-tag that could close the surrounding container early.
///
/// `to_ascii_lowercase()` keeps UTF-8 byte offsets identical to the source string, so
/// slicing back into `html` is always on a valid char boundary.
fn escape_breakout_tag(html: &str, needle: &str) -> String {
    let lower = html.to_ascii_lowercase();
    if !lower.contains(needle) {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len() + 16);
    let mut last_idx = 0;
    for (match_start, _) in lower.match_indices(needle) {
        out.push_str(&html[last_idx..match_start]);
        out.push_str("<\\/");
        out.push_str(&html[match_start + 2..match_start + needle.len()]);
        last_idx = match_start + needle.len();
    }
    out.push_str(&html[last_idx..]);
    out
}

/// Escapes a streamed HTML fragment so it cannot close either the `<template>` that
/// holds it or a following inline `<script>` element.
fn escape_template_tag(html: &str) -> String {
    let without_template = escape_breakout_tag(html, "</template");
    escape_breakout_tag(&without_template, "</script")
}

/// Serialises a value as a JavaScript string literal that is safe inside an inline
/// `<script>` element. `serde_json` escapes quotes/backslashes/control characters but
/// deliberately leaves `<`, `>`, `&`, `/` and the JS line separators untouched; an
/// attacker-supplied `</script>` would otherwise terminate the script element and
/// inject arbitrary markup (XSS).
///
/// `pub(crate)` so the PPR swap scripts (`crate::ppr`) reuse the exact same
/// escaping instead of a raw `serde_json::to_string`.
pub(crate) fn escape_js_string_literal(value: &str) -> String {
    let json = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string());
    let mut out = String::with_capacity(json.len() + 16);
    for ch in json.chars() {
        match ch {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '/' => out.push_str("\\/"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(ch),
        }
    }
    out
}

impl StreamingHtml {
    /// Helper to produce a replacement chunk that swaps a placeholder element by ID.
    pub fn swap_placeholder(target_id: &str, html: &str) -> String {
        let id_num = CHUNK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_id = format!("yav-chunk-{id_num}");
        let safe_target_id = escape_js_string_literal(target_id);
        let safe_html = escape_template_tag(html);
        format!(
            r#"<template id="{temp_id}">{safe_html}</template><script>(function(){{var t=document.getElementById('{temp_id}');var target=document.getElementById({safe_target_id});if(t&&target){{target.replaceWith(t.content.cloneNode(true));t.remove();}}}})();</script>"#
        )
    }

    /// Convert into an Axum [`Response`] with `Transfer-Encoding: chunked`.
    ///
    /// The nonce is taken from the current SSR context (i.e. the one installed by
    /// the security middleware) or generated server-side if the stream is built
    /// outside a request. It is advertised to the middleware through an internal
    /// header so that a matching strict CSP can be emitted.
    pub fn into_response(mut self) -> Response {
        let nonce = yav_core::use_context::<crate::middleware::CspNonce>()
            .map(|n| n.0.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| crate::middleware::CspNonce::generate().0);
        self.nonce = Some(nonce.clone());
        let body = Body::from_stream(self);
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache, no-transform"),
                (header::HeaderName::from_static("x-accel-buffering"), "no"),
                (
                    header::HeaderName::from_static(crate::middleware::STREAMING_NONCE_HEADER),
                    nonce.as_str(),
                ),
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
            return Poll::Ready(Some(Ok(self.apply_nonce(initial))));
        }

        // 2. Poll the channel for subsequent async fragments
        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(self.apply_nonce(chunk)))),
            Poll::Ready(None) => {
                // 3. Channel closed: send closing shell if not empty
                if !self.closing_shell.is_empty() {
                    let closing = std::mem::take(&mut self.closing_shell);
                    Poll::Ready(Some(Ok(self.apply_nonce(closing))))
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

    #[test]
    fn test_swap_placeholder_blocks_script_breakout_in_id() {
        let attacker_id = r#"</script><img src=x onerror=alert(1)>"#;
        let chunk = StreamingHtml::swap_placeholder(attacker_id, "<div>ok</div>");
        assert!(!chunk.contains("</script><img"));
        assert!(chunk.contains(r#"\u003c\/script\u003e"#));
        assert!(chunk.contains(r#"onerror=alert(1)"#));
    }

    #[test]
    fn test_swap_placeholder_blocks_script_breakout_in_html() {
        let payload = "</template></script><script>alert(1)</script>";
        let chunk = StreamingHtml::swap_placeholder("target", payload);
        assert!(!chunk.contains("</template></script>"));
        assert!(chunk.contains(r#"<\/template>"#));
        assert!(chunk.contains(r#"<\/script>"#));
    }

    #[test]
    fn test_js_string_literal_escapes_js_line_separators() {
        let literal = escape_js_string_literal("a\u{2028}alert(1)");
        assert!(literal.contains(r"\u2028"));
        assert!(!literal.contains('\u{2028}'));
    }

    #[tokio::test]
    async fn test_streaming_response_injects_nonce_into_every_chunk() {
        let (tx, streaming) = streaming_html("<html><script>boot()</script>", "", 4);
        tx.send(StreamingHtml::swap_placeholder("slot", "<div>done</div>"))
            .await
            .unwrap();
        drop(tx);

        let response = streaming.into_response();
        let advertised = response
            .headers()
            .get(crate::middleware::STREAMING_NONCE_HEADER)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(!advertised.is_empty());

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(
            html.matches(&format!(r#"nonce="{advertised}""#)).count(),
            2,
            "initial shell and swap chunk must both carry the nonce"
        );
    }
}
