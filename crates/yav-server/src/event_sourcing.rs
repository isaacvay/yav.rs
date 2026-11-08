//! # YAV Event Sourcing & CQRS Framework
//!
//! Provides an append-only event store with optimistic concurrency control,
//! stream rehydration, and state projection materialization.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Error types for Event Sourcing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStoreError {
    /// Concurrency conflict: the actual aggregate version differs from the expected version.
    ConcurrencyConflict {
        aggregate_id: String,
        expected: u64,
        actual: u64,
    },
    /// Invalid payload or event formatting.
    InvalidEvent(String),
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrencyConflict {
                aggregate_id,
                expected,
                actual,
            } => write!(
                f,
                "Concurrency conflict on aggregate '{aggregate_id}': expected version {expected}, actual {actual}"
            ),
            Self::InvalidEvent(msg) => write!(f, "Invalid event: {msg}"),
        }
    }
}

impl std::error::Error for EventStoreError {}

/// Immutable domain event recorded in the append-only log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub version: u64,
    pub timestamp_ms: u64,
}

impl DomainEvent {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        aggregate_id: impl Into<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
        version: u64,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: id.into(),
            aggregate_id: aggregate_id.into(),
            event_type: event_type.into(),
            payload,
            version,
            timestamp_ms,
        }
    }
}

/// Thread-safe in-memory event store with optimistic concurrency verification.
#[derive(Debug, Clone, Default)]
pub struct EventStore {
    streams: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
    global_log: Arc<RwLock<Vec<DomainEvent>>>,
}

impl EventStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            global_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Append a new event to an aggregate stream with optimistic locking check.
    pub fn append(
        &self,
        aggregate_id: &str,
        expected_version: u64,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<DomainEvent, EventStoreError> {
        let mut streams = self.streams.write().expect("Lock poisoned");
        let stream = streams.entry(aggregate_id.to_string()).or_default();

        let current_version = stream.len() as u64;
        if current_version != expected_version {
            return Err(EventStoreError::ConcurrencyConflict {
                aggregate_id: aggregate_id.to_string(),
                expected: expected_version,
                actual: current_version,
            });
        }

        let new_version = current_version + 1;
        let event_id = format!("{aggregate_id}-{new_version}");
        let event = DomainEvent::new(event_id, aggregate_id, event_type, payload, new_version);

        stream.push(event.clone());

        let mut global = self.global_log.write().expect("Lock poisoned");
        global.push(event.clone());

        Ok(event)
    }

    /// Retrieve all events for a given aggregate in chronological sequence.
    #[must_use]
    pub fn get_stream(&self, aggregate_id: &str) -> Vec<DomainEvent> {
        let streams = self.streams.read().expect("Lock poisoned");
        streams.get(aggregate_id).cloned().unwrap_or_default()
    }

    /// Retrieve events for an aggregate starting after a specific version.
    #[must_use]
    pub fn get_stream_from(&self, aggregate_id: &str, from_version: u64) -> Vec<DomainEvent> {
        self.get_stream(aggregate_id)
            .into_iter()
            .filter(|e| e.version > from_version)
            .collect()
    }

    /// Get current version of an aggregate stream.
    #[must_use]
    pub fn get_version(&self, aggregate_id: &str) -> u64 {
        let streams = self.streams.read().expect("Lock poisoned");
        streams.get(aggregate_id).map_or(0, |s| s.len() as u64)
    }

    /// Get total number of events recorded across all streams.
    #[must_use]
    pub fn total_events(&self) -> usize {
        let global = self.global_log.read().expect("Lock poisoned");
        global.len()
    }
}

/// A read-model projection that folds over domain events to build current state.
pub trait Projection: Default {
    fn apply(&mut self, event: &DomainEvent);

    /// Rehydrate current state by replaying a stream of events.
    fn rehydrate(events: &[DomainEvent]) -> Self {
        let mut state = Self::default();
        for event in events {
            state.apply(event);
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, Eq)]
    struct BankAccountState {
        account_id: String,
        balance_cents: i64,
        is_closed: bool,
    }

    impl Projection for BankAccountState {
        fn apply(&mut self, event: &DomainEvent) {
            match event.event_type.as_str() {
                "AccountOpened" => {
                    self.account_id = event.aggregate_id.clone();
                    self.balance_cents = event.payload["initial_balance"].as_i64().unwrap_or(0);
                }
                "MoneyDeposited" => {
                    self.balance_cents += event.payload["amount"].as_i64().unwrap_or(0);
                }
                "MoneyWithdrawn" => {
                    self.balance_cents -= event.payload["amount"].as_i64().unwrap_or(0);
                }
                "AccountClosed" => {
                    self.is_closed = true;
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_event_store_append_and_concurrency() {
        let store = EventStore::new();

        // 1. Initial event at version 0
        let ev1 = store
            .append(
                "acc_100",
                0,
                "AccountOpened",
                serde_json::json!({ "initial_balance": 5000 }),
            )
            .expect("First event should succeed");
        assert_eq!(ev1.version, 1);
        assert_eq!(store.get_version("acc_100"), 1);

        // 2. Second event at version 1
        let ev2 = store
            .append(
                "acc_100",
                1,
                "MoneyDeposited",
                serde_json::json!({ "amount": 2500 }),
            )
            .expect("Second event should succeed");
        assert_eq!(ev2.version, 2);

        // 3. Concurrency conflict: attempting to append with stale expected version 1
        let err = store
            .append(
                "acc_100",
                1,
                "MoneyWithdrawn",
                serde_json::json!({ "amount": 1000 }),
            )
            .unwrap_err();

        match err {
            EventStoreError::ConcurrencyConflict {
                aggregate_id,
                expected,
                actual,
            } => {
                assert_eq!(aggregate_id, "acc_100");
                assert_eq!(expected, 1);
                assert_eq!(actual, 2);
            }
            _ => panic!("Expected ConcurrencyConflict"),
        }

        // 4. Valid append with expected version 2
        let ev3 = store
            .append(
                "acc_100",
                2,
                "MoneyWithdrawn",
                serde_json::json!({ "amount": 1000 }),
            )
            .expect("Third event should succeed");
        assert_eq!(ev3.version, 3);

        // 5. Rehydrate projection
        let stream = store.get_stream("acc_100");
        assert_eq!(stream.len(), 3);

        let account = BankAccountState::rehydrate(&stream);
        assert_eq!(account.account_id, "acc_100");
        assert_eq!(account.balance_cents, 6500); // 5000 + 2500 - 1000
        assert!(!account.is_closed);
    }
}
