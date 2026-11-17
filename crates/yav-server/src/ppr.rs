//! Partial Prerendering (PPR) & Component-Level Caching (`"use cache"` parity).
//!
//! Combines the sub-millisecond TTFB of static site generation with dynamic
//! server-rendered holes streamed in over the same HTTP response:
//! - [`PprShell`] caches the static HTML shell containing `<template data-yav-ppr-slot="id">`
//!   fallback placeholders.
//! - Dynamic [`PprSlot`] futures are resolved **concurrently**. The streaming
//!   path uses `FuturesUnordered` and flushes each swap script as soon as its
//!   slot completes (out of declaration order); the non-streaming
//!   [`PprPage::render_complete`] path uses `join_all` and emits the composed
//!   document in declaration order.
//! - [`ComponentCache`] provides fine-grained per-component memoization with TTL and
//!   tag-based invalidation (`revalidate_tag`).

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use yav_core::{render_to_string, Element, IntoNode, Node};

/// A boxed async future returning a rendered [`Node`] for a dynamic PPR slot.
pub type BoxSlotFuture = Pin<Box<dyn Future<Output = Node> + Send + 'static>>;

/// A dynamic hole inside a Partially Prerendered page.
pub struct PprSlot {
    /// Unique slot identifier (e.g. `"cart-badge"`, `"user-recommendations"`).
    pub id: String,
    /// Static fallback skeleton rendered inside the cached shell.
    pub fallback: Node,
    /// Asynchronous producer that resolves the personalized dynamic content.
    pub resolver: BoxSlotFuture,
}

impl PprSlot {
    /// Create a new dynamic PPR slot with a fallback skeleton and an async resolver.
    pub fn new<F, Fut>(id: impl Into<String>, fallback: Node, resolver: F) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Node> + Send + 'static,
    {
        Self {
            id: id.into(),
            fallback,
            resolver: Box::pin(resolver()),
        }
    }
}

/// Emit a placeholder container in RSX that marks where a [`PprSlot`] will be hydrated/swapped.
#[must_use]
pub fn ppr_placeholder(id: &str, fallback: Node) -> Node {
    Element::new("div")
        .attr("id", format!("yav-ppr-{id}"))
        .attr("data-yav-ppr-slot", id)
        .child(fallback)
        .into_node()
}

/// Partial Prerendering page builder.
pub struct PprPage {
    /// Route key used to cache the static shell.
    pub cache_key: String,
    /// Page title.
    pub title: String,
    /// Static shell node tree containing `ppr_placeholder(...)` markers.
    pub shell: Node,
    /// Dynamic slots to resolve per request.
    pub slots: Vec<PprSlot>,
}

/// Global cache for pre-rendered static shells.
static SHELL_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn shell_cache() -> &'static Mutex<HashMap<String, String>> {
    SHELL_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl PprPage {
    /// Create a new Partially Prerendered page.
    #[must_use]
    pub fn new(cache_key: impl Into<String>, title: impl Into<String>, shell: Node) -> Self {
        Self {
            cache_key: cache_key.into(),
            title: title.into(),
            shell,
            slots: Vec::new(),
        }
    }

    /// Register a dynamic slot to be resolved and streamed into the pre-rendered shell.
    #[must_use]
    pub fn slot<F, Fut>(mut self, id: impl Into<String>, fallback: Node, resolver: F) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Node> + Send + 'static,
    {
        self.slots.push(PprSlot::new(id, fallback, resolver));
        self
    }

    /// Retrieve or build the pre-rendered static HTML shell.
    #[must_use]
    pub fn get_or_build_shell(&self) -> (String, bool) {
        let mut lock = shell_cache().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached) = lock.get(&self.cache_key) {
            return (cached.clone(), true);
        }
        let body_html = render_to_string(&self.shell);
        let full_shell = format!(
            "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\" />\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n<title>{}</title>\n</head>\n<body>\n{}",
            yav_core::escape_html(&self.title),
            body_html
        );
        if lock.len() >= 1024 {
            if let Some(first) = lock.keys().next().cloned() {
                lock.remove(&first);
            }
        }
        lock.insert(self.cache_key.clone(), full_shell.clone());
        (full_shell, false)
    }

    /// Clear a cached static shell by key (or all shells if `None`).
    pub fn invalidate_shell(cache_key: Option<&str>) {
        let mut lock = shell_cache().lock().unwrap_or_else(|e| e.into_inner());
        match cache_key {
            Some(k) => {
                lock.remove(k);
            }
            None => lock.clear(),
        }
    }

    /// Render the static shell immediately and resolve all dynamic slots concurrently into a complete HTML string
    /// (useful for testing and non-streaming environments).
    pub async fn render_complete(self) -> String {
        let (mut out, _) = self.get_or_build_shell();
        let slot_ids: Vec<String> = self.slots.iter().map(|s| s.id.clone()).collect();
        let resolvers: Vec<_> = self.slots.into_iter().map(|s| s.resolver).collect();
        let resolved_nodes = futures_util::future::join_all(resolvers).await;
        for (id, resolved_node) in slot_ids.into_iter().zip(resolved_nodes) {
            let resolved_html = render_to_string(&resolved_node);
            let escaped_json = crate::streaming::escape_js_string_literal(&resolved_html);
            let safe_id = yav_core::escape_attribute(&id);
            out.push_str(&format!(
                "\n<template data-ppr-target=\"yav-ppr-{safe_id}\">{resolved_html}</template><script>(function(){{var t=document.getElementById('yav-ppr-{safe_id}');if(t)t.innerHTML={escaped_json};}})();</script>"
            ));
        }
        out.push_str("\n</body>\n</html>");

        // `into_streaming_response()` tags every chunk with the request CSP
        // nonce; the non-streaming path must do the same for its final HTML,
        // otherwise a strict `script-src 'nonce-…'` policy blocks the swap
        // scripts. Use the same context lookup / fallback as the streaming path.
        let nonce = yav_core::use_context::<crate::middleware::CspNonce>()
            .map(|n| n.0.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| crate::middleware::CspNonce::generate().0);
        crate::handler::inject_csp_nonce_into_scripts(&out, &nonce)
    }

    /// Convert into a streaming HTTP response that flushes the cached static shell on frame 0
    /// and streams each resolved dynamic slot concurrently as soon as each completes (out-of-order).
    pub fn into_streaming_response(self) -> Response {
        let (shell_html, is_hit) = self.get_or_build_shell();
        let slots = self.slots;
        let slot_count = slots.len();

        // Each streamed slot carries an inline `<script>` that performs the DOM
        // swap. Tag every chunk with the request nonce so the strict CSP does
        // not break Partial Prerendering, and advertise the nonce to the
        // security middleware (which then skips buffering the stream).
        let nonce = yav_core::use_context::<crate::middleware::CspNonce>()
            .map(|n| n.0.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| crate::middleware::CspNonce::generate().0);

        let (tx, rx) = tokio::sync::mpsc::channel::<String>(slot_count.max(1));
        let nonce_task = nonce.clone();

        tokio::spawn(async move {
            use futures_util::stream::FuturesUnordered;
            use futures_util::StreamExt;

            let mut tasks = FuturesUnordered::new();
            for slot in slots {
                let id = slot.id;
                let resolver = slot.resolver;
                tasks.push(async move {
                    let node = resolver.await;
                    (id, node)
                });
            }

            while let Some((id, resolved_node)) = tasks.next().await {
                let resolved_html = render_to_string(&resolved_node);
                let escaped_json = crate::streaming::escape_js_string_literal(&resolved_html);
                let safe_id = yav_core::escape_attribute(&id);
                let chunk = format!(
                    "\n<template data-ppr-target=\"yav-ppr-{safe_id}\">{resolved_html}</template><script>(function(){{var t=document.getElementById('yav-ppr-{safe_id}');if(t)t.innerHTML={escaped_json};}})();</script>"
                );
                let chunk = crate::handler::inject_csp_nonce_into_scripts(&chunk, &nonce_task);
                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
        });

        let stream = futures_util::stream::unfold(
            (Some(shell_html), rx, false, nonce.clone()),
            |(mut init_shell, mut rx_chan, done, nonce)| async move {
                if done {
                    return None;
                }
                if let Some(shell) = init_shell.take() {
                    let shell = crate::handler::inject_csp_nonce_into_scripts(&shell, &nonce);
                    return Some((
                        Ok::<_, std::convert::Infallible>(shell),
                        (None, rx_chan, false, nonce),
                    ));
                }
                if let Some(chunk) = rx_chan.recv().await {
                    return Some((Ok(chunk), (None, rx_chan, false, nonce)));
                }
                Some((
                    Ok("\n</body>\n</html>".to_string()),
                    (None, rx_chan, true, nonce),
                ))
            },
        );

        let mut resp = Response::new(Body::from_stream(stream));
        *resp.status_mut() = StatusCode::OK;
        resp.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        );
        resp.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-cache, no-transform"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static(crate::middleware::STREAMING_NONCE_HEADER),
            HeaderValue::from_str(&nonce).unwrap_or(HeaderValue::from_static("")),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("x-yav-ppr"),
            HeaderValue::from_static(if is_hit { "SHELL-HIT" } else { "SHELL-MISS" }),
        );
        resp
    }
}

use axum::http::HeaderName;

impl IntoResponse for PprPage {
    fn into_response(self) -> Response {
        self.into_streaming_response()
    }
}

/// Entry in the component-level cache (`"use cache"` equivalent).
#[derive(Debug, Clone)]
struct CachedComponentEntry {
    html: String,
    created_at: Instant,
    ttl: Duration,
    tags: HashSet<String>,
}

/// Fine-grained component-level HTML cache with TTL and tag invalidation.
#[derive(Debug, Clone)]
pub struct ComponentCache {
    entries: Arc<Mutex<HashMap<String, CachedComponentEntry>>>,
    max_capacity: usize,
}

impl Default for ComponentCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentCache {
    /// Create a new component-level cache with default capacity (1024 entries).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new component-level cache with a custom maximum capacity.
    #[must_use]
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            max_capacity: max_capacity.max(16),
        }
    }

    /// Render a component tree or return its cached HTML if fresh.
    pub fn get_or_render<F>(&self, key: &str, ttl: Duration, tags: &[&str], render_fn: F) -> Node
    where
        F: FnOnce() -> Node,
    {
        {
            let lock = self.entries.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = lock.get(key) {
                if entry.created_at.elapsed() < entry.ttl {
                    return Node::raw(entry.html.clone());
                }
            }
        }

        let node = render_fn();
        let html = render_to_string(&node);
        let tag_set: HashSet<String> = tags.iter().map(|t| (*t).to_string()).collect();

        let mut lock = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if lock.len() >= self.max_capacity {
            lock.retain(|_, entry| entry.created_at.elapsed() < entry.ttl);
            if lock.len() >= self.max_capacity {
                if let Some(oldest_key) = lock
                    .iter()
                    .min_by_key(|(_, e)| e.created_at)
                    .map(|(k, _)| k.clone())
                {
                    lock.remove(&oldest_key);
                }
            }
        }

        lock.insert(
            key.to_string(),
            CachedComponentEntry {
                html: html.clone(),
                created_at: Instant::now(),
                ttl,
                tags: tag_set,
            },
        );

        Node::raw(html)
    }

    /// Invalidate all cached components associated with a given tag.
    pub fn revalidate_tag(&self, tag: &str) -> usize {
        let mut lock = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let before = lock.len();
        lock.retain(|_, entry| !entry.tags.contains(tag));
        before - lock.len()
    }

    /// Clear all cached components.
    pub fn clear(&self) {
        let mut lock = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        lock.clear();
    }
}

/// Global singleton component cache for ergonomic `"use cache"` usage in components.
pub fn global_component_cache() -> &'static ComponentCache {
    static CACHE: OnceLock<ComponentCache> = OnceLock::new();
    CACHE.get_or_init(ComponentCache::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_ppr_page_shell_caching_and_slot_resolution() {
        PprPage::invalidate_shell(Some("/shop/item"));

        let shell = Element::new("main")
            .child(Element::new("h1").child(Node::text("Static Product Header")))
            .child(ppr_placeholder(
                "stock",
                Element::new("span")
                    .child(Node::text("Checking stock..."))
                    .into_node(),
            ))
            .into_node();

        let page = PprPage::new("/shop/item", "Product Page", shell).slot(
            "stock",
            Node::text("Loading..."),
            || async {
                Element::new("span")
                    .attr("class", "text-emerald-500")
                    .child(Node::text("In Stock (14 units)"))
                    .into_node()
            },
        );

        let html = page.render_complete().await;
        assert!(html.contains("Static Product Header"));
        assert!(html.contains("id=\"yav-ppr-stock\""));
        assert!(html.contains("In Stock (14 units)"));
    }

    /// Proves that `render_complete` polls every slot future concurrently.
    /// Each resolver waits on a two-party barrier: if slots were awaited one
    /// after another, the first would block forever waiting for a second party
    /// that is never polled, and the outer timeout would fire.
    #[tokio::test]
    async fn test_render_complete_resolves_slots_concurrently() {
        use std::sync::Arc as StdArc;
        use tokio::sync::Barrier;
        use tokio::time::{timeout, Duration};

        PprPage::invalidate_shell(Some("/concurrent/render"));
        let barrier = StdArc::new(Barrier::new(2));
        let shell = Element::new("main").into_node();

        let b1 = StdArc::clone(&barrier);
        let b2 = StdArc::clone(&barrier);
        let page = PprPage::new("/concurrent/render", "Concurrent", shell)
            .slot("first", Node::text("loading"), move || {
                let b = StdArc::clone(&b1);
                async move {
                    b.wait().await;
                    Node::text("first done")
                }
            })
            .slot("second", Node::text("loading"), move || {
                let b = StdArc::clone(&b2);
                async move {
                    b.wait().await;
                    Node::text("second done")
                }
            });

        let html = timeout(Duration::from_secs(2), page.render_complete())
            .await
            .expect("slots were resolved sequentially: the barrier never completed");
        assert!(html.contains("first done"));
        assert!(html.contains("second done"));
    }

    /// The streaming path uses `FuturesUnordered`, so a fast slot must be able
    /// to complete while a slow slot is still pending. The barrier again makes
    /// the property deterministic rather than timing-dependent.
    #[tokio::test]
    async fn test_streaming_response_resolves_slots_concurrently() {
        use std::sync::Arc as StdArc;
        use tokio::sync::Barrier;
        use tokio::time::{timeout, Duration};

        PprPage::invalidate_shell(Some("/concurrent/stream"));
        let barrier = StdArc::new(Barrier::new(2));
        let shell = Element::new("main").into_node();

        let b1 = StdArc::clone(&barrier);
        let b2 = StdArc::clone(&barrier);
        let page = PprPage::new("/concurrent/stream", "Concurrent", shell)
            .slot("first", Node::text("loading"), move || {
                let b = StdArc::clone(&b1);
                async move {
                    b.wait().await;
                    Node::text("first done")
                }
            })
            .slot("second", Node::text("loading"), move || {
                let b = StdArc::clone(&b2);
                async move {
                    b.wait().await;
                    Node::text("second done")
                }
            });

        let resp = page.into_streaming_response();
        let bytes = timeout(
            Duration::from_secs(2),
            axum::body::to_bytes(resp.into_body(), usize::MAX),
        )
        .await
        .expect("streaming slots were resolved sequentially")
        .expect("body collection failed");
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(html.contains("first done"));
        assert!(html.contains("second done"));
    }

    #[test]
    fn test_component_cache_memoization_and_tag_invalidation() {
        let cache = ComponentCache::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c1 = counter.clone();
        let node1 = cache.get_or_render(
            "pricing-table:pro",
            Duration::from_secs(60),
            &["pricing", "plans"],
            move || {
                c1.fetch_add(1, Ordering::SeqCst);
                Element::new("div").child(Node::text("$29/mo")).into_node()
            },
        );

        let c2 = counter.clone();
        let node2 = cache.get_or_render(
            "pricing-table:pro",
            Duration::from_secs(60),
            &["pricing", "plans"],
            move || {
                c2.fetch_add(1, Ordering::SeqCst);
                Element::new("div").child(Node::text("$99/mo")).into_node()
            },
        );

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(render_to_string(&node1), "<div>$29/mo</div>");
        assert_eq!(render_to_string(&node2), "<div>$29/mo</div>");

        // Invalidate by tag
        let evicted = cache.revalidate_tag("pricing");
        assert_eq!(evicted, 1);

        let c3 = counter.clone();
        let node3 = cache.get_or_render(
            "pricing-table:pro",
            Duration::from_secs(60),
            &["pricing"],
            move || {
                c3.fetch_add(1, Ordering::SeqCst);
                Element::new("div").child(Node::text("$39/mo")).into_node()
            },
        );
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert_eq!(render_to_string(&node3), "<div>$39/mo</div>");
    }

    #[tokio::test]
    async fn test_render_complete_escapes_script_breakout_and_injects_nonce() {
        PprPage::invalidate_shell(Some("/xss/complete"));
        let shell = Element::new("main")
            .child(Node::text("static shell"))
            .into_node();
        let page = PprPage::new("/xss/complete", "XSS", shell).slot(
            "slot",
            Node::text("loading"),
            || async { Node::raw("</script><img src=x onerror=alert(1)>") },
        );

        let html = page.render_complete().await;

        // The inline swap script carries the CSP nonce (render_complete must
        // not diverge from into_streaming_response on this).
        assert!(html.contains("nonce=\""));
        // `<` inside the JS string literal is escaped as \\u003c, so the
        // attacker's `</script>` can never terminate the <script> element.
        assert!(html.contains(r"\u003c"));
        assert!(html.contains(r"\u003c\/script\u003e"));

        // Isolate the inline <script> body: it must contain no raw breakout.
        let start = html.find("<script").unwrap();
        let gt = html[start..].find('>').unwrap() + start;
        let end = html[gt..].find("</script>").unwrap() + gt;
        let body = &html[gt + 1..end];
        assert!(!body.contains("</script>"));
    }

    #[tokio::test]
    async fn test_streaming_slots_escape_script_breakout() {
        PprPage::invalidate_shell(Some("/xss/stream"));
        let shell = Element::new("main")
            .child(Node::text("static shell"))
            .into_node();
        let page = PprPage::new("/xss/stream", "XSS", shell).slot(
            "slot",
            Node::text("loading"),
            || async { Node::raw("</script><img src=x onerror=alert(1)>") },
        );

        let resp = page.into_streaming_response();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(html.contains(r"\u003c"));
        assert!(html.contains(r"\u003c\/script\u003e"));
        let start = html.find("<script").unwrap();
        let gt = html[start..].find('>').unwrap() + start;
        let end = html[gt..].find("</script>").unwrap() + gt;
        let body = &html[gt + 1..end];
        assert!(!body.contains("</script>"));
    }
}
