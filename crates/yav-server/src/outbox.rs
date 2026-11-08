//! Transactional Outbox Pattern & Reliable Asynchronous Event Bus.
//!
//! Guarantees atomicity between local database transactions and distributed event publishing
//! (avoiding Dual-Write inconsistencies with Kafka, RabbitMQ, Redis, or Webhooks).

use std::{
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

/// Delivery status of a transactional outbox message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxStatus {
    /// Message written during local transaction, waiting to be dispatched.
    Pending,
    /// Message currently claimed by a background worker for delivery.
    Processing,
    /// Successfully published and acknowledged by downstream event broker.
    Delivered,
    /// Reached maximum retry attempts without success.
    Failed,
}

/// A persistent event record written atomically within a business transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxMessage {
    /// Unique event identifier.
    pub id: String,
    /// Domain event name (e.g. `"order.paid"`, `"user.registered"`).
    pub event_type: String,
    /// Aggregate root category (e.g. `"order"`, `"account"`).
    pub aggregate_type: String,
    /// Aggregate root unique identifier.
    pub aggregate_id: String,
    /// Serialized JSON payload.
    pub payload: String,
    /// Unix timestamp when the message was recorded.
    pub created_at: u64,
    /// Scheduled next delivery attempt timestamp.
    pub next_attempt_at: u64,
    /// Number of delivery attempts executed so far.
    pub attempts: u32,
    /// Maximum permitted retry attempts before transitioning to Failed.
    pub max_attempts: u32,
    /// Current delivery status.
    pub status: OutboxStatus,
    /// Last error encountered during delivery attempt.
    pub last_error: Option<String>,
}

impl OutboxMessage {
    pub fn new(
        id: impl Into<String>,
        event_type: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: id.into(),
            event_type: event_type.into(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            payload: payload.into(),
            created_at: now,
            next_attempt_at: now,
            attempts: 0,
            max_attempts: 5,
            status: OutboxStatus::Pending,
            last_error: None,
        }
    }
}

/// Statistics snapshot of the transactional outbox queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxStats {
    pub pending: usize,
    pub processing: usize,
    pub delivered: usize,
    pub failed: usize,
}

/// Central Transactional Outbox engine managing reliable message dispatch.
#[derive(Debug, Clone, Default)]
pub struct TransactionalOutbox {
    messages: Arc<RwLock<Vec<OutboxMessage>>>,
}

impl TransactionalOutbox {
    /// Create a new in-memory transactional outbox.
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a domain event inside the outbox (call this inside your DB transaction).
    pub fn publish(
        &self,
        event_type: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        payload: impl Into<String>,
    ) -> String {
        let id = format!(
            "evt_{}_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros(),
            fastrand_hex(6)
        );

        let msg = OutboxMessage::new(
            id.clone(),
            event_type,
            aggregate_type,
            aggregate_id,
            payload,
        );
        self.messages.write().expect("Lock poisoned").push(msg);
        id
    }

    /// Claim next batch of pending messages ready for transmission.
    pub fn poll_pending(&self, batch_size: usize) -> Vec<OutboxMessage> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut lock = self.messages.write().expect("Lock poisoned");
        let mut claimed = Vec::new();

        for msg in lock.iter_mut() {
            if claimed.len() >= batch_size {
                break;
            }
            if msg.status == OutboxStatus::Pending && now >= msg.next_attempt_at {
                msg.status = OutboxStatus::Processing;
                msg.attempts += 1;
                claimed.push(msg.clone());
            }
        }

        claimed
    }

    /// Mark an outbox message as successfully delivered.
    pub fn mark_delivered(&self, message_id: &str) {
        let mut lock = self.messages.write().expect("Lock poisoned");
        if let Some(msg) = lock.iter_mut().find(|m| m.id == message_id) {
            msg.status = OutboxStatus::Delivered;
            msg.last_error = None;
        }
    }

    /// Mark an outbox message as failed with exponential retry backoff.
    pub fn mark_failed(&self, message_id: &str, error: impl Into<String>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut lock = self.messages.write().expect("Lock poisoned");
        if let Some(msg) = lock.iter_mut().find(|m| m.id == message_id) {
            let err_str = error.into();
            msg.last_error = Some(err_str);

            if msg.attempts >= msg.max_attempts {
                msg.status = OutboxStatus::Failed;
            } else {
                msg.status = OutboxStatus::Pending;
                // Exponential backoff: 2^attempts * 5 seconds
                let delay = (1u64 << msg.attempts.min(6)) * 5;
                msg.next_attempt_at = now + delay;
            }
        }
    }

    /// Query statistics overview of the outbox queue.
    pub fn stats(&self) -> OutboxStats {
        let lock = self.messages.read().expect("Lock poisoned");
        let mut stats = OutboxStats {
            pending: 0,
            processing: 0,
            delivered: 0,
            failed: 0,
        };

        for m in lock.iter() {
            match m.status {
                OutboxStatus::Pending => stats.pending += 1,
                OutboxStatus::Processing => stats.processing += 1,
                OutboxStatus::Delivered => stats.delivered += 1,
                OutboxStatus::Failed => stats.failed += 1,
            }
        }

        stats
    }
}

fn fastrand_hex(len: usize) -> String {
    let chars = b"0123456789abcdef";
    let mut out = String::with_capacity(len);
    for i in 0..len {
        let idx = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize
            + i * 31)
            % chars.len();
        out.push(chars[idx] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transactional_outbox_lifecycle() {
        let outbox = TransactionalOutbox::new();
        let id = outbox.publish("order.created", "order", "ord_100", r#"{"amount": 4200}"#);

        let initial_stats = outbox.stats();
        assert_eq!(initial_stats.pending, 1);
        assert_eq!(initial_stats.delivered, 0);

        // Poll pending messages
        let batch = outbox.poll_pending(10);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].id, id);
        assert_eq!(batch[0].status, OutboxStatus::Processing);

        // Mark successfully delivered
        outbox.mark_delivered(&id);
        let final_stats = outbox.stats();
        assert_eq!(final_stats.pending, 0);
        assert_eq!(final_stats.delivered, 1);
    }

    #[test]
    fn test_transactional_outbox_retry_and_failure() {
        let outbox = TransactionalOutbox::new();
        let id = outbox.publish("invoice.email", "invoice", "inv_99", "{}");

        let batch = outbox.poll_pending(1);
        assert_eq!(batch.len(), 1);

        // Fail first attempt
        outbox.mark_failed(&id, "SMTP connection timed out");
        let stats = outbox.stats();
        assert_eq!(stats.pending, 1); // Re-queued with backoff
    }
}
