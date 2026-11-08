//! Distributed Mutex Locks & Leader Election Protocol.
//!
//! Coordinates singleton cron jobs, background workers, and distributed resources
//! across clustered Yav server instances without single points of failure.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

/// State of an acquired distributed lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributedLock {
    pub resource: String,
    pub owner: String,
    pub token: String,
    pub expires_at_ms: u64,
}

impl DistributedLock {
    /// Check whether the lock has expired locally.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        now_ms >= self.expires_at_ms
    }

    /// Remaining time-to-live before lease expiration.
    #[must_use]
    pub fn ttl_remaining(&self) -> Duration {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if self.expires_at_ms > now_ms {
            Duration::from_millis(self.expires_at_ms - now_ms)
        } else {
            Duration::ZERO
        }
    }
}

/// Pluggable storage backend for distributed locking across cluster nodes (Redis, Etcd, SQL, etc.).
pub trait LockBackend: std::fmt::Debug + Send + Sync {
    /// Attempt to acquire a lease on `resource` with given `owner` and `ttl`.
    fn try_acquire(&self, resource: &str, owner: &str, ttl: Duration) -> Option<DistributedLock>;
    /// Release an actively held lock if `token` matches.
    fn release(&self, resource: &str, token: &str) -> bool;
    /// Extend lease on an actively held lock if `token` matches.
    fn renew(&self, resource: &str, token: &str, ttl: Duration) -> bool;
    /// Check if resource is currently leased.
    fn is_locked(&self, resource: &str) -> bool;
}

/// In-memory lock backend (default for single-node and local testing coordination).
#[derive(Debug, Clone, Default)]
pub struct MemoryLockBackend {
    locks: Arc<RwLock<HashMap<String, DistributedLock>>>,
}

impl MemoryLockBackend {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl LockBackend for MemoryLockBackend {
    fn try_acquire(&self, resource: &str, owner: &str, ttl: Duration) -> Option<DistributedLock> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut lock_map = self.locks.write().unwrap_or_else(|e| e.into_inner());
        lock_map.retain(|_, v| now_ms < v.expires_at_ms);

        if let Some(existing) = lock_map.get(resource) {
            if now_ms < existing.expires_at_ms {
                return None;
            }
        }

        let token = format!("tok_{:x}_{}", now_ms, fastrand_hex_lock(8));
        let lock = DistributedLock {
            resource: resource.to_string(),
            owner: owner.to_string(),
            token,
            expires_at_ms: now_ms + ttl.as_millis() as u64,
        };

        lock_map.insert(resource.to_string(), lock.clone());
        Some(lock)
    }

    fn release(&self, resource: &str, token: &str) -> bool {
        let mut lock_map = self.locks.write().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = lock_map.get(resource) {
            if existing.token == token {
                lock_map.remove(resource);
                return true;
            }
        }
        false
    }

    fn renew(&self, resource: &str, token: &str, ttl: Duration) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut lock_map = self.locks.write().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = lock_map.get_mut(resource) {
            if existing.token == token && now_ms < existing.expires_at_ms {
                existing.expires_at_ms = now_ms + ttl.as_millis() as u64;
                return true;
            }
        }
        false
    }

    fn is_locked(&self, resource: &str) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let lock_map = self.locks.read().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = lock_map.get(resource) {
            now_ms < existing.expires_at_ms
        } else {
            false
        }
    }
}

/// Central Lock Coordinator managing resource leases.
#[derive(Debug, Clone)]
pub struct LockCoordinator {
    backend: Arc<dyn LockBackend>,
}

impl Default for LockCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl LockCoordinator {
    /// Create a new lock coordinator using the default in-memory backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            backend: Arc::new(MemoryLockBackend::new()),
        }
    }

    /// Create a lock coordinator using a custom distributed backend (e.g. Redis, DB).
    pub fn with_backend(backend: impl LockBackend + 'static) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Attempt to acquire a lease on the specified resource.
    pub fn try_acquire(
        &self,
        resource: &str,
        owner: impl Into<String>,
        ttl: Duration,
    ) -> Option<DistributedLock> {
        self.backend.try_acquire(resource, &owner.into(), ttl)
    }

    /// Release an actively held lock if owner token matches.
    pub fn release(&self, lock: &DistributedLock) -> bool {
        self.backend.release(&lock.resource, &lock.token)
    }

    /// Extend lease on an actively held lock.
    pub fn renew(&self, lock: &mut DistributedLock, extend_ttl: Duration) -> bool {
        if self.backend.renew(&lock.resource, &lock.token, extend_ttl) {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            lock.expires_at_ms = now_ms + extend_ttl.as_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Check if a resource is currently locked.
    pub fn is_locked(&self, resource: &str) -> bool {
        self.backend.is_locked(resource)
    }
}

/// Dynamic Leader Election Coordinator for cluster singleton services.
#[derive(Debug, Clone)]
pub struct LeaderElector {
    coordinator: LockCoordinator,
    resource: String,
    candidate_id: String,
    active_lease: Arc<RwLock<Option<DistributedLock>>>,
}

impl LeaderElector {
    /// Create a new leader election participant.
    #[must_use]
    pub fn new(
        coordinator: LockCoordinator,
        leader_resource: impl Into<String>,
        candidate_id: impl Into<String>,
    ) -> Self {
        Self {
            coordinator,
            resource: leader_resource.into(),
            candidate_id: candidate_id.into(),
            active_lease: Arc::new(RwLock::new(None)),
        }
    }

    /// Campaign to become the cluster leader.
    pub fn campaign(&self, lease_ttl: Duration) -> bool {
        if let Some(lock) =
            self.coordinator
                .try_acquire(&self.resource, &self.candidate_id, lease_ttl)
        {
            let mut active = self.active_lease.write().unwrap_or_else(|e| e.into_inner());
            *active = Some(lock);
            true
        } else {
            false
        }
    }

    /// Check if this instance is currently the recognized cluster leader.
    pub fn is_leader(&self) -> bool {
        let active = self.active_lease.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ref lock) = *active {
            !lock.is_expired() && self.coordinator.is_locked(&self.resource)
        } else {
            false
        }
    }

    /// Heartbeat to renew leadership lease.
    pub fn heartbeat(&self, extend_ttl: Duration) -> bool {
        let mut active = self.active_lease.write().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut lock) = *active {
            if self.coordinator.renew(lock, extend_ttl) {
                return true;
            }
        }
        *active = None;
        false
    }

    /// Resign from leadership and immediately release the lease.
    pub fn resign(&self) -> bool {
        let mut active = self.active_lease.write().unwrap_or_else(|e| e.into_inner());
        if let Some(lock) = active.take() {
            self.coordinator.release(&lock)
        } else {
            false
        }
    }
}

static LOCK_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn fastrand_hex_lock(len: usize) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let ctr = LOCK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "{:016x}",
        (now as u64) ^ (ctr.wrapping_mul(0x517cc1b727220a95)) ^ 0xFEEDC0DECAFE
    )[0..len.min(16)]
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_acquire_and_release() {
        let coord = LockCoordinator::new();

        let lock1 = coord.try_acquire("job:billing", "node-1", Duration::from_secs(10));
        assert!(lock1.is_some());
        assert!(coord.is_locked("job:billing"));

        // Second node cannot acquire while lock is active
        let lock2 = coord.try_acquire("job:billing", "node-2", Duration::from_secs(10));
        assert!(lock2.is_none());

        // Node 1 releases lock
        assert!(coord.release(&lock1.unwrap()));
        assert!(!coord.is_locked("job:billing"));

        // Now node 2 can acquire
        let lock3 = coord.try_acquire("job:billing", "node-2", Duration::from_secs(10));
        assert!(lock3.is_some());
    }

    #[test]
    fn test_leader_election_lifecycle() {
        let coord = LockCoordinator::new();
        let elector_a = LeaderElector::new(coord.clone(), "cluster:leader", "node_a");
        let elector_b = LeaderElector::new(coord, "cluster:leader", "node_b");

        // Node A campaigns and wins
        assert!(elector_a.campaign(Duration::from_millis(50)));
        assert!(elector_a.is_leader());

        // Node B fails to steal leadership
        assert!(!elector_b.campaign(Duration::from_millis(50)));
        assert!(!elector_b.is_leader());

        // Node A resigns
        assert!(elector_a.resign());
        assert!(!elector_a.is_leader());

        // Node B campaigns again and wins
        assert!(elector_b.campaign(Duration::from_millis(50)));
        assert!(elector_b.is_leader());
    }
}
