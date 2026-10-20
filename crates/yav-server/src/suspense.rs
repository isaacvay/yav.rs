//! Declarative Suspense boundaries for asynchronous SSR streaming.
//!
//! Provides progressive loading where immediate HTML shells are sent first,
//! while expensive server-rendered subtrees (database queries, external APIs)
//! resolve in background tasks and swap into placeholders via `<template>` replacement.

use std::future::Future;
use tokio::sync::mpsc;
use yav_core::{Element, IntoNode, Node};

use crate::streaming::StreamingHtml;

/// Declarative Suspense boundary helper for streaming SSR.
pub struct Suspense;

impl Suspense {
    /// Render the placeholder element containing the fallback UI.
    ///
    /// The resulting DOM node has `id="{target_id}"` and can be targeted
    /// by [`StreamingHtml::swap_placeholder`].
    pub fn placeholder(target_id: impl Into<String>, fallback: impl IntoNode) -> Node {
        let id_str = target_id.into();
        let el = Element::new("div")
            .attr("id", id_str)
            .attr("data-yav-suspense", "true")
            .child(fallback.into_node());
        Node::Element(el)
    }

    /// Spawn an asynchronous task to render dynamic content and stream the replacement chunk.
    ///
    /// When `future` resolves to a [`Node`], it is rendered to HTML and swapped into
    /// the placeholder matching `target_id`.
    pub fn spawn<Fut>(
        target_id: impl Into<String>,
        sender: mpsc::Sender<String>,
        future: Fut,
    ) -> tokio::task::JoinHandle<()>
    where
        Fut: Future<Output = Node> + Send + 'static,
    {
        let id = target_id.into();
        tokio::spawn(async move {
            let resolved_node = future.await;
            let html = yav_core::render_to_string(&resolved_node);
            let chunk = StreamingHtml::swap_placeholder(&id, &html);
            let _ = sender.send(chunk).await;
        })
    }

    /// Spawn an asynchronous task that produces an HTML string directly.
    pub fn spawn_html<Fut>(
        target_id: impl Into<String>,
        sender: mpsc::Sender<String>,
        future: Fut,
    ) -> tokio::task::JoinHandle<()>
    where
        Fut: Future<Output = String> + Send + 'static,
    {
        let id = target_id.into();
        tokio::spawn(async move {
            let html = future.await;
            let chunk = StreamingHtml::swap_placeholder(&id, &html);
            let _ = sender.send(chunk).await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspense_placeholder_rendering() {
        let fallback = Node::raw("<p class='skeleton'>Loading statistics...</p>");
        let node = Suspense::placeholder("stats-box", fallback);
        let html = yav_core::render_to_string(&node);

        assert!(html.contains(r#"id="stats-box""#));
        assert!(html.contains(r#"data-yav-suspense="true""#));
        assert!(html.contains("Loading statistics..."));
    }

    #[tokio::test]
    async fn test_suspense_streaming_resolution() {
        let (tx, mut rx) = mpsc::channel(4);

        let handle = Suspense::spawn("user-card", tx, async {
            Node::raw("<div class='card'>User: John Doe</div>")
        });

        handle.await.expect("join handle ok");

        let received = rx.recv().await.expect("should receive chunk");
        assert!(received.contains("user-card"));
        assert!(received.contains("User: John Doe"));
        assert!(received.contains("replaceWith"));
    }
}
