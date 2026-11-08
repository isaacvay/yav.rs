//! Multi-Tier Distributed Cache & XFetch Probabilistic Early Expiration.
//!
//! Provides two-tier caching (L1 local in-memory + L2 distributed backing store)
//! and the XFetch algorithm to eliminate "thundering herd" / cache stampede effects.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

/// Internal cached entry with compute delta time for XFetch calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheItem<T> {
    pub value: T,
    /// Unix timestamp when the item expires (in milliseconds).
    pub expires_at_ms: u64,
    /// Duration it took to compute or fetch the value (in milliseconds).
    pub delta_ms: u64,
}

impl<T> CacheItem<T> {
    #[must_use]
    pub fn new(value: T, ttl: Duration, delta: Duration) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            value,
            expires_at_ms: now_ms + ttl.as_millis() as u64,
            delta_ms: delta.as_millis() as u64,
        }
    }

    /// Check if the entry is already past its hard expiration timestamp.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        now_ms >= self.expires_at_ms
    }

    /// XFetch optimal probabilistic early expiration check:
    /// Returns `true` if early recomputation is recommended before hard expiration.
    ///
    /// Formula: `now - delta * beta * ln(rand) > expires_at`
    /// where `beta > 0` (typically 1.0) and `rand in (0, 1]`.
    #[must_use]
    pub fn should_recompute_xfetch(&self, beta: f64) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        if now_ms >= self.expires_at_ms {
            return true;
        }

        // Generate pseudo-random float in (0.0, 1.0]
        let rand_val = fast_pseudo_rand_f64();
        let early_offset_ms = -((self.delta_ms as f64) * beta * rand_val.ln());

        (now_ms as f64 + early_offset_ms) >= (self.expires_at_ms as f64)
    }
}

static XFETCH_RAND_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Fast pseudo-random generator returning (0.0, 1.0] without pulling external crates.
fn fast_pseudo_rand_f64() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let ctr = XFETCH_RAND_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let val = (((nanos ^ (ctr.wrapping_mul(0x5DEECE66Du64))) ^ 0x5DEECE66Du64) & 0x7FFFFFFF) as f64
        / 2147483648.0;
    val.clamp(0.0001, 1.0)
}

/// In-memory L1 cache store with capacity limit.
#[derive(Debug, Clone)]
pub struct MemoryL1Store<T> {
    items: Arc<RwLock<HashMap<String, CacheItem<T>>>>,
    max_capacity: usize,
}

impl<T: Clone> MemoryL1Store<T> {
    #[must_use]
    pub fn new(max_capacity: usize) -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            max_capacity: max_capacity.max(16),
        }
    }

    pub fn get(&self, key: &str) -> Option<CacheItem<T>> {
        let lock = self.items.read().unwrap_or_else(|e| e.into_inner());
        if let Some(item) = lock.get(key) {
            if !item.is_expired() {
                return Some(item.clone());
            }
        }
        None
    }

    pub fn put(&self, key: impl Into<String>, item: CacheItem<T>) {
        let mut lock = self.items.write().unwrap_or_else(|e| e.into_inner());
        if lock.len() >= self.max_capacity {
            // Evict expired entries or arbitrary key
            lock.retain(|_, v| !v.is_expired());
            if lock.len() >= self.max_capacity {
                if let Some(first_key) = lock.keys().next().cloned() {
                    lock.remove(&first_key);
                }
            }
        }
        lock.insert(key.into(), item);
    }

    pub fn delete(&self, key: &str) -> bool {
        let mut lock = self.items.write().unwrap_or_else(|e| e.into_inner());
        lock.remove(key).is_some()
    }

    pub fn clear(&self) {
        let mut lock = self.items.write().unwrap_or_else(|e| e.into_inner());
        lock.clear();
    }
}

/// Pluggable distributed L2 cache backend (Redis, Memcached, SQLite, etc.).
pub trait CacheL2Backend<T>: std::fmt::Debug + Send + Sync {
    /// Retrieve item from L2 cache.
    fn get(&self, key: &str) -> Option<CacheItem<T>>;
    /// Store item in L2 cache.
    fn put(&self, key: &str, item: &CacheItem<T>);
    /// Remove item from L2 cache.
    fn delete(&self, key: &str) -> bool;
    /// Clear all L2 entries.
    fn clear(&self);
}

/// Multi-tier distributed cache orchestrator (L1 local memory + optional L2 distributed backing store).
#[derive(Debug, Clone)]
pub struct DistributedCache<T> {
    l1: MemoryL1Store<T>,
    l2: Option<Arc<dyn CacheL2Backend<T>>>,
    beta: f64,
    default_ttl: Duration,
}

impl<T: Clone> DistributedCache<T> {
    /// Create a new distributed cache with default configuration.
    #[must_use]
    pub fn new(max_l1_capacity: usize, default_ttl: Duration) -> Self {
        Self {
            l1: MemoryL1Store::new(max_l1_capacity),
            l2: None,
            beta: 1.0,
            default_ttl,
        }
    }

    /// Attach a pluggable L2 distributed backend (Redis, etc.).
    #[must_use]
    pub fn with_l2_backend(mut self, backend: impl CacheL2Backend<T> + 'static) -> Self {
        self.l2 = Some(Arc::new(backend));
        self
    }

    /// Set XFetch beta tuning parameter (default: 1.0).
    #[must_use]
    pub fn with_beta(mut self, beta: f64) -> Self {
        self.beta = beta.max(0.1);
        self
    }

    /// Retrieve item from L1/L2 cache if available and not flagged for early recomputation.
    pub fn get(&self, key: &str) -> Option<T> {
        if let Some(item) = self.l1.get(key) {
            if item.should_recompute_xfetch(self.beta) {
                // Signal cache miss to prompt fresh recomputation
                return None;
            } else {
                return Some(item.value);
            }
        }

        // Check L2 backing store on L1 miss
        if let Some(l2) = &self.l2 {
            if let Some(item) = l2.get(key) {
                if !item.is_expired() {
                    self.l1.put(key, item.clone());
                    if !item.should_recompute_xfetch(self.beta) {
                        return Some(item.value);
                    }
                }
            }
        }

        None
    }

    /// Set an item in the cache with specified TTL and compute duration delta.
    pub fn set(
        &self,
        key: impl Into<String>,
        value: T,
        ttl: Option<Duration>,
        compute_delta: Duration,
    ) {
        let key_str = key.into();
        let ttl = ttl.unwrap_or(self.default_ttl);
        let item = CacheItem::new(value, ttl, compute_delta);
        self.l1.put(&key_str, item.clone());
        if let Some(l2) = &self.l2 {
            l2.put(&key_str, &item);
        }
    }

    /// Synchronously get or compute with exact delta measurement.
    pub fn get_or_compute<F>(&self, key: &str, compute_fn: F) -> T
    where
        F: FnOnce() -> T,
    {
        if let Some(val) = self.get(key) {
            return val;
        }

        let start = Instant::now();
        let computed = compute_fn();
        let delta = start.elapsed();

        self.set(key, computed.clone(), None, delta);
        computed
    }

    /// Invalidate a key across both L1 and L2 caches.
    pub fn invalidate(&self, key: &str) -> bool {
        let l1_res = self.l1.delete(key);
        let l2_res = self.l2.as_ref().map(|l2| l2.delete(key)).unwrap_or(false);
        l1_res || l2_res
    }

    /// Clear all cached items in L1 and L2.
    pub fn clear(&self) {
        self.l1.clear();
        if let Some(l2) = &self.l2 {
            l2.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_item_expiration() {
        let item = CacheItem::new("data", Duration::from_millis(50), Duration::from_millis(5));
        assert!(!item.is_expired());
        std::thread::sleep(Duration::from_millis(60));
        assert!(item.is_expired());
    }

    #[test]
    fn test_xfetch_early_recompute() {
        // Very large delta compared to remaining TTL triggers early refresh
        let item = CacheItem {
            value: "expensive_computation",
            expires_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + 10,
            delta_ms: 5000, // 5s computation time
        };
        assert!(item.should_recompute_xfetch(2.0));
    }

    #[test]
    fn test_distributed_cache_get_or_compute() {
        let cache = DistributedCache::new(100, Duration::from_secs(60));
        let mut count = 0;

        let val1 = cache.get_or_compute("counter", || {
            count += 1;
            format!("Computed {}", count)
        });
        assert_eq!(val1, "Computed 1");

        // Subsequent get should hit L1
        let val2 = cache.get_or_compute("counter", || {
            count += 1;
            format!("Computed {}", count)
        });
        assert_eq!(val2, "Computed 1");
        assert_eq!(count, 1);

        // Invalidate key
        assert!(cache.invalidate("counter"));
        let val3 = cache.get_or_compute("counter", || {
            count += 1;
            format!("Computed {}", count)
        });
        assert_eq!(val3, "Computed 2");
        assert_eq!(count, 2);
    }
}
