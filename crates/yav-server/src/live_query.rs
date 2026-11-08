//! Real-Time Live Queries & Change Data Capture (CDC) Server-Push.
//!
//! Provides reactive database invalidation and live server-driven UI patching
//! over Server-Sent Events (SSE) or WebSockets without full-page reloads.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use crate::sse::{sse_channel, sse_response, SseMsg};
use axum::{
    extract::{Query, State},
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// A change event emitted when data is mutated on the server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum LiveEvent {
    /// Invalidate any query reading from the given table.
    InvalidateTable { table: String },
    /// Invalidate a specific record by table and primary key.
    InvalidateRow { table: String, row_id: String },
    /// Direct DOM fragment replacement for a CSS selector.
    DomPatch { selector: String, html: String },
}

/// Central manager dispatching table and row invalidations to active live-query subscribers.
#[derive(Debug, Clone)]
pub struct LiveQueryManager {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<LiveEvent>>>>,
    capacity: usize,
}

impl Default for LiveQueryManager {
    fn default() -> Self {
        Self::new(256)
    }
}

impl LiveQueryManager {
    /// Create a new `LiveQueryManager` with bounded channel buffer capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// Subscribe to a specific channel (e.g. `"table:posts"`, `"user:42"`).
    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<LiveEvent> {
        let mut lock = self.channels.write().unwrap_or_else(|e| e.into_inner());
        // Auto-prune dead channels if capacity threshold is reached to prevent memory leaks
        if lock.len() >= 512 {
            lock.retain(|_, tx| tx.receiver_count() > 0);
        }
        if let Some(tx) = lock.get(channel) {
            tx.subscribe()
        } else {
            let (tx, rx) = broadcast::channel(self.capacity);
            lock.insert(channel.to_string(), tx);
            rx
        }
    }

    /// Prune idle channels with zero active subscribers to free memory.
    /// Returns the number of pruned channels.
    pub fn prune_idle_channels(&self) -> usize {
        let mut lock = self.channels.write().unwrap_or_else(|e| e.into_inner());
        let initial_count = lock.len();
        lock.retain(|_, tx| tx.receiver_count() > 0);
        initial_count.saturating_sub(lock.len())
    }

    /// Return the total number of currently registered channels.
    pub fn active_channel_count(&self) -> usize {
        let lock = self.channels.read().unwrap_or_else(|e| e.into_inner());
        lock.len()
    }

    /// Notify all subscribers of a table-level mutation (`INSERT`, `UPDATE`, `DELETE`).
    pub fn invalidate_table(&self, table: impl Into<String>) {
        let t = table.into();
        let chan = format!("table:{t}");
        let event = LiveEvent::InvalidateTable { table: t };
        self.publish(&chan, event);
    }

    /// Notify subscribers of a single row update.
    pub fn invalidate_row(&self, table: impl Into<String>, row_id: impl Into<String>) {
        let t = table.into();
        let id = row_id.into();
        let chan = format!("row:{t}:{id}");
        let event = LiveEvent::InvalidateRow {
            table: t,
            row_id: id,
        };
        self.publish(&chan, event);
    }

    /// Push an immediate HTML patch to a target CSS selector on clients subscribed to `channel`.
    pub fn push_patch(&self, channel: &str, selector: impl Into<String>, html: impl Into<String>) {
        let event = LiveEvent::DomPatch {
            selector: selector.into(),
            html: html.into(),
        };
        self.publish(channel, event);
    }

    /// Publish any [`LiveEvent`] to a specific channel.
    pub fn publish(&self, channel: &str, event: LiveEvent) {
        let lock = self.channels.read().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = lock.get(channel) {
            let _ = tx.send(event);
        }
    }
}

/// Global singleton instance of [`LiveQueryManager`].
pub fn global_live_queries() -> &'static LiveQueryManager {
    static INSTANCE: OnceLock<LiveQueryManager> = OnceLock::new();
    INSTANCE.get_or_init(LiveQueryManager::default)
}

/// Query parameters for connecting to the live-queries SSE endpoint.
#[derive(Debug, Deserialize)]
pub struct LiveQueryStreamParams {
    /// Channel name to stream (e.g. `"table:posts"`, `"user:10"`).
    pub channel: String,
}

/// Axum SSE handler streaming live events to connected browsers (`GET /_yav/live?channel=...`).
pub async fn live_query_sse_handler(
    State(manager): State<LiveQueryManager>,
    Query(params): Query<LiveQueryStreamParams>,
) -> Response {
    let mut rx = manager.subscribe(&params.channel);
    let (tx, stream) = sse_channel(32);

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let json = serde_json::to_string(&ev).unwrap_or_default();
                    let msg = SseMsg::new(json).event("yav:live");
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    sse_response(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_query_table_invalidation() {
        let manager = LiveQueryManager::new(16);
        let mut rx = manager.subscribe("table:users");

        manager.invalidate_table("users");

        let received = rx.try_recv().expect("Event should be received");
        assert_eq!(
            received,
            LiveEvent::InvalidateTable {
                table: "users".to_string()
            }
        );
    }

    #[test]
    fn test_live_query_dom_patch() {
        let manager = LiveQueryManager::new(16);
        let mut rx = manager.subscribe("feed:news");

        manager.push_patch(
            "feed:news",
            "#latest-headline",
            "<h1>Breaking Rust News</h1>",
        );

        let received = rx.try_recv().expect("Patch should be received");
        assert_eq!(
            received,
            LiveEvent::DomPatch {
                selector: "#latest-headline".to_string(),
                html: "<h1>Breaking Rust News</h1>".to_string(),
            }
        );
    }

    #[test]
    fn test_live_query_channel_pruning() {
        let manager = LiveQueryManager::new(16);
        let rx1 = manager.subscribe("temp:chan1");
        let _rx2 = manager.subscribe("temp:chan2");
        assert_eq!(manager.active_channel_count(), 2);

        // Drop rx1
        drop(rx1);

        let pruned = manager.prune_idle_channels();
        assert_eq!(pruned, 1);
        assert_eq!(manager.active_channel_count(), 1);
    }
}
