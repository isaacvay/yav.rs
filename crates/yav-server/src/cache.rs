//! In-memory SSR Page Cache and Incremental Static Regeneration (ISR) engine.
//!
//! Enables caching rendered SSR responses with a configurable Time-To-Live (TTL)
//! and optional `stale-while-revalidate` background regeneration.

use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// Configuration policy for cached SSR routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachePolicy {
    /// Duration for which a cached response is considered fresh.
    pub ttl: Duration,
    /// If `true`, expired entries are served immediately with `X-Yav-Cache: STALE`
    /// while a background task regenerates the fresh entry.
    pub stale_while_revalidate: bool,
}

impl CachePolicy {
    /// Create a new cache policy with the given TTL and `stale_while_revalidate = true`.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            stale_while_revalidate: true,
        }
    }

    /// Enable or disable `stale-while-revalidate` behavior.
    pub fn stale_while_revalidate(mut self, enabled: bool) -> Self {
        self.stale_while_revalidate = enabled;
        self
    }
}

/// Headers that must never be replayed verbatim from a cached response.
/// Hop-by-hop / framing headers are rebuilt by the server for the new body.
const NON_REPLAYABLE_HEADERS: [&str; 8] = [
    "content-length",
    "transfer-encoding",
    "connection",
    "content-encoding",
    "cache-control",
    "x-yav-cache",
    "vary",
    "set-cookie",
];

/// A cached SSR HTTP response snapshot.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub status: StatusCode,
    pub content_type: String,
    pub body: Bytes,
    pub created_at: Instant,
    pub revalidating: bool,
    /// Full response headers (except hop-by-hop ones) so a HIT is indistinguishable
    /// from a MISS for redirects, custom headers, CORS, etc.
    pub headers: HeaderMap,
}

impl CacheEntry {
    /// Returns `true` if this entry is still within `ttl`.
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() < ttl
    }

    /// Build a cache entry from a rendered response, dropping only hop-by-hop headers.
    pub fn from_response(status: StatusCode, headers: &HeaderMap, body: Bytes) -> Self {
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html; charset=utf-8")
            .to_string();
        let mut stored = headers.clone();
        for name in NON_REPLAYABLE_HEADERS {
            stored.remove(name);
        }
        Self {
            status,
            content_type,
            body,
            created_at: Instant::now(),
            revalidating: false,
            headers: stored,
        }
    }

    /// Convert this cached snapshot into an HTTP [`Response`] with cache telemetry headers.
    ///
    /// Cached SSR pages are always `private` and vary on `Cookie`/`Accept-Language`:
    /// a public cache directive would allow a shared proxy to serve one user's page
    /// to another. Responses that set cookies are never cached (see `get_cached`).
    pub fn into_response_with_status(&self, cache_status: &'static str, ttl: Duration) -> Response {
        let mut resp = (
            self.status,
            [(header::CONTENT_TYPE, self.content_type.as_str())],
            self.body.clone(),
        )
            .into_response();

        let out_headers = resp.headers_mut();
        for (name, value) in self.headers.iter() {
            if !NON_REPLAYABLE_HEADERS
                .iter()
                .any(|skip| name.as_str() == *skip)
            {
                out_headers.append(name.clone(), value.clone());
            }
        }

        out_headers.insert("x-yav-cache", HeaderValue::from_static(cache_status));
        if let Ok(cc) = HeaderValue::from_str(&format!(
            "private, max-age={}, stale-while-revalidate={}",
            ttl.as_secs(),
            ttl.as_secs().max(60)
        )) {
            out_headers.insert(header::CACHE_CONTROL, cc);
        }
        out_headers.insert(
            header::VARY,
            HeaderValue::from_static("Cookie, Accept-Language, HX-Request"),
        );
        resp
    }
}

/// Telemetry metrics for [`SsrCache`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[derive(Debug, Clone)]
struct StoredCacheRecord {
    entry: CacheEntry,
    tags: Vec<String>,
    last_accessed: Instant,
    access_count: u64,
}

#[derive(Debug)]
struct SsrCacheInner {
    records: HashMap<String, StoredCacheRecord>,
    max_capacity: usize,
    stats: CacheStats,
}

impl Default for SsrCacheInner {
    fn default() -> Self {
        Self {
            records: HashMap::new(),
            max_capacity: 4096,
            stats: CacheStats::default(),
        }
    }
}

/// Thread-safe in-memory SSR response cache with bounded LRU eviction and Next.js-style
/// tag/path invalidation (`revalidate_tag`, `revalidate_path`).
#[derive(Clone, Default)]
pub struct SsrCache {
    inner: Arc<RwLock<SsrCacheInner>>,
}

impl SsrCache {
    /// Create a new empty SSR cache with default capacity (4096 routes).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new SSR cache bounded to `max_capacity` entries using LRU eviction.
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(SsrCacheInner {
                records: HashMap::new(),
                max_capacity: max_capacity.max(1),
                stats: CacheStats::default(),
            })),
        }
    }

    /// Retrieve a cached entry by key (`path` or `path?query`), updating its LRU timestamp.
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(rec) = guard.records.get_mut(key) {
            rec.last_accessed = Instant::now();
            rec.access_count += 1;
            let entry = rec.entry.clone();
            guard.stats.hits += 1;
            Some(entry)
        } else {
            guard.stats.misses += 1;
            None
        }
    }

    /// Insert or update a cached entry without tags.
    pub fn insert(&self, key: impl Into<String>, entry: CacheEntry) {
        self.insert_with_tags(key, entry, Vec::<String>::new());
    }

    /// Insert or update a cached entry associated with one or more cache tags
    /// (for on-demand invalidation via [`SsrCache::revalidate_tag`]).
    pub fn insert_with_tags<I, S>(&self, key: impl Into<String>, entry: CacheEntry, tags: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let key_str = key.into();
        let tag_vec: Vec<String> = tags.into_iter().map(Into::into).collect();
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());

        // Evict least-recently-used entry if at capacity and inserting a new key
        if !guard.records.contains_key(&key_str) && guard.records.len() >= guard.max_capacity {
            if let Some(lru_key) = guard
                .records
                .iter()
                .min_by_key(|(_, rec)| rec.last_accessed)
                .map(|(k, _)| k.clone())
            {
                guard.records.remove(&lru_key);
                guard.stats.evictions += 1;
            }
        }

        guard.records.insert(
            key_str,
            StoredCacheRecord {
                entry,
                tags: tag_vec,
                last_accessed: Instant::now(),
                access_count: 1,
            },
        );
    }

    /// Mark an existing entry as currently revalidating in the background.
    /// Returns `true` if this caller transitioned `revalidating` from `false` to `true`.
    pub fn try_begin_revalidation(&self, key: &str) -> bool {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(rec) = guard.records.get_mut(key) {
            if !rec.entry.revalidating {
                rec.entry.revalidating = true;
                return true;
            }
        }
        false
    }

    /// Cancel a background revalidation in flight, resetting `revalidating` to `false`.
    pub fn cancel_revalidation(&self, key: &str) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(rec) = guard.records.get_mut(key) {
            rec.entry.revalidating = false;
        }
    }

    /// Invalidate a specific cache key (e.g. after a POST mutation).
    pub fn invalidate(&self, key: &str) -> bool {
        self.inner
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .records
            .remove(key)
            .is_some()
    }

    /// Revalidate (evict) all cached entries tagged with `tag` (Next.js `revalidateTag` equivalent).
    /// Returns the number of invalidated entries.
    pub fn revalidate_tag(&self, tag: &str) -> usize {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let before = guard.records.len();
        guard
            .records
            .retain(|_, rec| !rec.tags.iter().any(|t| t == tag));
        before - guard.records.len()
    }

    /// Revalidate (evict) all cached entries whose key matches or starts with `path_prefix`
    /// (Next.js `revalidatePath` equivalent). Returns the number of invalidated entries.
    pub fn revalidate_path(&self, path_prefix: &str) -> usize {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        let before = guard.records.len();
        guard.records.retain(|k, _| {
            !(k == path_prefix
                || k.starts_with(&format!("{path_prefix}?"))
                || k.starts_with(&format!("{path_prefix}/")))
        });
        before - guard.records.len()
    }

    /// Return current cache telemetry stats (`hits`, `misses`, `evictions`).
    pub fn stats(&self) -> CacheStats {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).stats
    }

    /// Clear all cached SSR pages.
    pub fn clear(&self) {
        self.inner
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .records
            .clear();
    }

    /// Return the number of cached routes currently in memory.
    pub fn len(&self) -> usize {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .records
            .len()
    }

    /// Returns `true` if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssr_cache_lifecycle() {
        let cache = SsrCache::new();
        assert!(cache.is_empty());

        let entry = CacheEntry {
            status: StatusCode::OK,
            content_type: "text/html; charset=utf-8".to_string(),
            body: Bytes::from_static(b"<h1>Cached</h1>"),
            created_at: Instant::now(),
            revalidating: false,
            headers: HeaderMap::new(),
        };
        cache.insert("/docs", entry);
        assert_eq!(cache.len(), 1);

        let fetched = cache.get("/docs").unwrap();
        assert!(fetched.is_fresh(Duration::from_secs(60)));

        assert!(cache.invalidate("/docs"));
        assert!(cache.is_empty());
    }

    #[test]
    fn test_ssr_cache_lru_and_tag_revalidation() {
        let cache = SsrCache::with_capacity(2);
        let mk_entry = || CacheEntry {
            status: StatusCode::OK,
            content_type: "text/html".to_string(),
            body: Bytes::from_static(b"ok"),
            created_at: Instant::now(),
            revalidating: false,
            headers: HeaderMap::new(),
        };

        cache.insert_with_tags("/posts/1", mk_entry(), ["posts", "post:1"]);
        cache.insert_with_tags("/posts/2", mk_entry(), ["posts", "post:2"]);
        // Access /posts/1 so /posts/2 is older in LRU order
        let _ = cache.get("/posts/1");

        // Inserting a 3rd entry evicts the least-recently-used (/posts/2)
        cache.insert_with_tags("/users", mk_entry(), ["users"]);
        assert_eq!(cache.len(), 2);
        assert!(cache.get("/posts/2").is_none());
        assert!(cache.get("/posts/1").is_some());
        assert_eq!(cache.stats().evictions, 1);

        // Revalidate by tag
        let evicted = cache.revalidate_tag("posts");
        assert_eq!(evicted, 1);
        assert!(cache.get("/posts/1").is_none());

        // Revalidate by path prefix
        cache.insert("/blog/rust", mk_entry());
        cache.insert("/blog/web", mk_entry());
        assert_eq!(cache.revalidate_path("/blog"), 2);
    }
}
