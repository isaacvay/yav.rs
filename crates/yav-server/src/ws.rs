//! Real-time RSX fragment broadcasting and WebSocket / live DOM swap helpers.
//!
//! Allows broadcasting rendered [`yav_core::Node`] fragments across connected clients
//! with declarative target and swap instructions (`innerHTML`, `outerHTML`, `beforeend`, `afterbegin`).

use tokio::sync::broadcast;
use yav_core::{render_to_string, Node};

/// Format a [`Node`] as a JSON envelope for real-time DOM fragment swapping over WebSockets or SSE.
///
/// Produces `{"target":"#id","swap":"innerHTML","html":"..."}`.
pub fn ws_swap_frame(target: &str, swap: &str, node: &Node) -> String {
    let html = render_to_string(node);
    let escaped_html = html
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    let escaped_target = target.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_swap = swap.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"target":"{}","swap":"{}","html":"{}"}}"#,
        escaped_target, escaped_swap, escaped_html
    )
}

/// Render a [`Node`] directly into a string frame for real-time transport.
pub fn ws_node_frame(node: &Node) -> String {
    render_to_string(node)
}

/// Multi-producer, multi-consumer real-time broadcaster for RSX [`Node`] fragments and messages.
#[derive(Clone, Debug)]
pub struct WsBroadcast {
    tx: broadcast::Sender<String>,
}

impl Default for WsBroadcast {
    fn default() -> Self {
        Self::new(256)
    }
}

impl WsBroadcast {
    /// Create a new [`WsBroadcast`] channel with the specified ring-buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    /// Subscribe to receive real-time frames.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// Broadcast a raw string message to all active subscribers.
    pub fn send_raw(&self, msg: impl Into<String>) -> usize {
        self.tx.send(msg.into()).unwrap_or(0)
    }

    /// Render and broadcast an RSX [`Node`] to all active subscribers.
    pub fn send_node(&self, node: &Node) -> usize {
        self.send_raw(ws_node_frame(node))
    }

    /// Render and broadcast a targeted DOM swap instruction (`target`, `swap`, `node`) to all subscribers.
    pub fn send_swap(&self, target: &str, swap: &str, node: &Node) -> usize {
        self.send_raw(ws_swap_frame(target, swap, node))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yav_core::Element;

    #[test]
    fn test_ws_broadcast_and_swap_frame() {
        let hub = WsBroadcast::new(16);
        let mut rx = hub.subscribe();

        let node = Node::Element(Element::new("span").child(Node::text("Live 42")));
        let sent = hub.send_swap("#counter", "innerHTML", &node);
        assert_eq!(sent, 1);

        let frame = rx.try_recv().unwrap();
        assert!(frame.contains(r##""target":"#counter""##));
        assert!(frame.contains(r#""swap":"innerHTML""#));
        assert!(frame.contains("<span>Live 42</span>"));
    }
}
