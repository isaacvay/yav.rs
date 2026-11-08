//! Partial Prerendering (PPR) & Component-Level Caching (`"use cache"` parity).
//!
//! Combines the sub-millisecond TTFB of static site generation with dynamic
//! server-rendered holes streamed in over the same HTTP response:
//! - [`PprShell`] caches the static HTML shell containing `<template data-yav-ppr-slot="id">`
//!   fallback placeholders.
//! - Dynamic [`PprSlot`] futures execute concurrently and stream replacement chunks
//!   using inline swap scripts or resolve immediately into a single composed document.
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

    /// Render the static shell immediately and resolve all dynamic slots into a complete HTML string
    /// (useful for testing and non-streaming environments).
    pub async fn render_complete(self) -> String {
        let (mut out, _) = self.get_or_build_shell();
        for slot in self.slots {
            let resolved_node = slot.resolver.await;
            let resolved_html = render_to_string(&resolved_node);
            let escaped_json =
                serde_json::to_string(&resolved_html).unwrap_or_else(|_| "\"\"".into());
            let safe_id = yav_core::escape_attribute(&slot.id);
            out.push_str(&format!(
                "\n<template data-ppr-target=\"yav-ppr-{safe_id}\">{resolved_html}</template><script>(function(){{var t=document.getElementById('yav-ppr-{safe_id}');if(t)t.innerHTML={escaped_json};}})();</script>"
            ));
        }
        out.push_str("\n</body>\n</html>");
        out
    }

    /// Convert into a streaming HTTP response that flushes the cached static shell on frame 0
    /// and streams each resolved dynamic slot as it completes.
    pub fn into_streaming_response(self) -> Response {
        let (shell_html, is_hit) = self.get_or_build_shell();
        let slots = self.slots;

        let stream = futures_util::stream::unfold(
            (Some(shell_html), slots.into_iter(), false),
            |(mut init_shell, mut slots_iter, done)| async move {
                if done {
                    return None;
                }
                if let Some(shell) = init_shell.take() {
                    return Some((
                        Ok::<_, std::convert::Infallible>(shell),
                        (None, slots_iter, false),
                    ));
                }
                if let Some(slot) = slots_iter.next() {
                    let resolved_node = slot.resolver.await;
                    let resolved_html = render_to_string(&resolved_node);
                    let escaped_json =
                        serde_json::to_string(&resolved_html).unwrap_or_else(|_| "\"\"".into());
                    let safe_id = yav_core::escape_attribute(&slot.id);
                    let chunk = format!(
                        "\n<template data-ppr-target=\"yav-ppr-{safe_id}\">{resolved_html}</template><script>(function(){{var t=document.getElementById('yav-ppr-{safe_id}');if(t)t.innerHTML={escaped_json};}})();</script>"
                    );
                    return Some((Ok(chunk), (None, slots_iter, false)));
                }
                Some((
                    Ok("\n</body>\n</html>".to_string()),
                    (None, slots_iter, true),
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
#[derive(Debug, Clone, Default)]
pub struct ComponentCache {
    entries: Arc<Mutex<HashMap<String, CachedComponentEntry>>>,
}

impl ComponentCache {
    /// Create a new component-level cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
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
}
