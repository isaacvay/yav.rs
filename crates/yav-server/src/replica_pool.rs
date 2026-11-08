//! Multi-Region Read/Write Database Replica Pool & Smart Router.
//!
//! Directs write mutations (`POST`, `PUT`, `DELETE`) to the primary master database
//! while load-balancing read queries (`GET`, `HEAD`) across low-latency geo-distributed read replicas,
//! featuring a "Read-Your-Own-Writes" session stickiness window.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

/// Target database node determined by the routing policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteTarget {
    /// Route to the primary read/write master database.
    Primary,
    /// Route to a specific geo-replicated read replica.
    Replica { id: String, region: String },
}

/// Metadata describing a single database node in the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub id: String,
    pub region: String,
    pub dsn: String,
    pub is_primary: bool,
    pub weight: u32,
    pub healthy: bool,
}

/// Database replica pool routing queries across primary and read replicas.
#[derive(Debug, Clone)]
pub struct DbReplicaPool {
    primary: ReplicaNode,
    replicas: Arc<RwLock<Vec<ReplicaNode>>>,
    /// Session stickiness: mapping session_id -> expiration timestamp for Read-Your-Own-Writes.
    write_stickiness: Arc<RwLock<HashMap<String, u64>>>,
    /// Duration in seconds to enforce primary reads after a write.
    stickiness_window_secs: u64,
}

impl DbReplicaPool {
    /// Create a new pool initialized with the primary master node.
    pub fn new(primary_dsn: impl Into<String>, primary_region: impl Into<String>) -> Self {
        Self {
            primary: ReplicaNode {
                id: "primary".to_string(),
                region: primary_region.into(),
                dsn: primary_dsn.into(),
                is_primary: true,
                weight: 100,
                healthy: true,
            },
            replicas: Arc::new(RwLock::new(Vec::new())),
            write_stickiness: Arc::new(RwLock::new(HashMap::new())),
            stickiness_window_secs: 5, // 5 seconds default window
        }
    }

    /// Set duration of the Read-Your-Own-Writes stickiness window.
    #[must_use]
    pub fn with_stickiness_window(mut self, secs: u64) -> Self {
        self.stickiness_window_secs = secs;
        self
    }

    /// Return the primary master node.
    #[must_use]
    pub fn primary(&self) -> &ReplicaNode {
        &self.primary
    }

    /// Register a read-only replica located in a specific region.
    pub fn add_replica(
        &self,
        id: impl Into<String>,
        region: impl Into<String>,
        dsn: impl Into<String>,
        weight: u32,
    ) {
        let node = ReplicaNode {
            id: id.into(),
            region: region.into(),
            dsn: dsn.into(),
            is_primary: false,
            weight,
            healthy: true,
        };
        self.replicas
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(node);
    }

    /// Record that a session just performed a database mutation (activating stickiness).
    pub fn record_mutation(&self, session_id: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut guard = self
            .write_stickiness
            .write()
            .unwrap_or_else(|e| e.into_inner());
        guard.retain(|_, expires_at| now < *expires_at);
        guard.insert(session_id.to_string(), now + self.stickiness_window_secs);
    }

    /// Check whether a session is currently within its post-write stickiness window.
    pub fn is_sticky_to_primary(&self, session_id: &str) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let lock = self
            .write_stickiness
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(&expires_at) = lock.get(session_id) {
            now < expires_at
        } else {
            false
        }
    }

    /// Resolve optimal target database node according to HTTP method, region, and session state.
    pub fn route(
        &self,
        http_method: &str,
        user_region: Option<&str>,
        session_id: Option<&str>,
    ) -> RouteTarget {
        let is_write = matches!(
            http_method.to_uppercase().as_str(),
            "POST" | "PUT" | "DELETE" | "PATCH"
        );

        // 1. Mutations must ALWAYS target Primary
        if is_write {
            if let Some(sid) = session_id {
                self.record_mutation(sid);
            }
            return RouteTarget::Primary;
        }

        // 2. Read-Your-Own-Writes stickiness enforcement
        if let Some(sid) = session_id {
            if self.is_sticky_to_primary(sid) {
                return RouteTarget::Primary;
            }
        }

        // 3. Match healthy replica closest to user region
        let lock = self.replicas.read().unwrap_or_else(|e| e.into_inner());
        let healthy_replicas: Vec<&ReplicaNode> = lock.iter().filter(|r| r.healthy).collect();

        if healthy_replicas.is_empty() {
            return RouteTarget::Primary;
        }

        if let Some(reg) = user_region {
            if let Some(matched) = healthy_replicas
                .iter()
                .find(|r| r.region.eq_ignore_ascii_case(reg))
            {
                return RouteTarget::Replica {
                    id: matched.id.clone(),
                    region: matched.region.clone(),
                };
            }
        }

        // 4. Default to first healthy replica
        RouteTarget::Replica {
            id: healthy_replicas[0].id.clone(),
            region: healthy_replicas[0].region.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replica_pool_write_and_read_routing() {
        let pool = DbReplicaPool::new("postgres://master:5432/app", "us-east-1");
        pool.add_replica("replica-eu", "eu-west-1", "postgres://eu:5432/app", 100);
        pool.add_replica(
            "replica-ap",
            "ap-southeast-1",
            "postgres://ap:5432/app",
            100,
        );

        // Write mutation routes to Primary
        let target_write = pool.route("POST", Some("eu-west-1"), None);
        assert_eq!(target_write, RouteTarget::Primary);

        // Read without session matches regional replica
        let target_read_eu = pool.route("GET", Some("eu-west-1"), None);
        assert_eq!(
            target_read_eu,
            RouteTarget::Replica {
                id: "replica-eu".to_string(),
                region: "eu-west-1".to_string()
            }
        );
    }

    #[test]
    fn test_read_your_own_writes_stickiness() {
        let pool = DbReplicaPool::new("postgres://master:5432/app", "us-east-1")
            .with_stickiness_window(10);
        pool.add_replica("replica-eu", "eu-west-1", "postgres://eu:5432/app", 100);

        let session = "user_sess_42";

        // User performs a write
        pool.route("POST", Some("eu-west-1"), Some(session));
        assert!(pool.is_sticky_to_primary(session));

        // Immediate subsequent read routes to Primary to prevent stale cache view!
        let target_subsequent_read = pool.route("GET", Some("eu-west-1"), Some(session));
        assert_eq!(target_subsequent_read, RouteTarget::Primary);
    }
}
