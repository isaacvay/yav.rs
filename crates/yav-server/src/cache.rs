//! In-memory SSR Page Cache and Incremental Static Regeneration (ISR) engine.
//!
//! Enables caching rendered SSR responses with a configurable Time-To-Live (TTL)
//! and optional `stale-while-revalidate` background regeneration.

use axum::body::Bytes;
use axum::http::{HeaderValue, StatusCode};
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

/// A cached SSR HTTP response snapshot.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub status: StatusCode,
    pub content_type: String,
    pub body: Bytes,
    pub created_at: Instant,
    pub revalidating: bool,
}

impl CacheEntry {
    /// Returns `true` if this entry is still within `ttl`.
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() < ttl
    }

    /// Convert this cached snapshot into an HTTP [`Response`] with cache telemetry headers.
    pub fn into_response_with_status(&self, cache_status: &'static str, ttl: Duration) -> Response {
        let mut resp = (
            self.status,
            [(
                axum::http::header::CONTENT_TYPE,
                self.content_type.as_str(),
            )],
            self.body.clone(),
        )
            .into_response();

        resp.headers_mut()
            .insert("x-yav-cache", HeaderValue::from_static(cache_status));
        if let Ok(cc) = HeaderValue::from_str(&format!(
            "public, max-age={}, stale-while-revalidate={}",
            ttl.as_secs(),
            ttl.as_secs().max(60)
        )) {
            resp.headers_mut()
                .insert(axum::http::header::CACHE_CONTROL, cc);
        }
        resp
    }
}

/// Thread-safe in-memory SSR response cache.
#[derive(Clone, Default)]
pub struct SsrCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl SsrCache {
    /// Create a new empty SSR cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieve a cached entry by key (`path` or `path?query`).
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        self.entries
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }

    /// Insert or update a cached entry.
    pub fn insert(&self, key: impl Into<String>, entry: CacheEntry) {
        self.entries
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.into(), entry);
    }

    /// Mark an existing entry as currently revalidating in the background.
    /// Returns `true` if this caller transitioned `revalidating` from `false` to `true`.
    pub fn try_begin_revalidation(&self, key: &str) -> bool {
        let mut guard = self.entries.write().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = guard.get_mut(key) {
            if !entry.revalidating {
                entry.revalidating = true;
                return true;
            }
        }
        false
    }

    /// Invalidate a specific cache key (e.g. after a POST mutation).
    pub fn invalidate(&self, key: &str) -> bool {
        self.entries
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(key)
            .is_some()
    }

    /// Clear all cached SSR pages.
    pub fn clear(&self) {
        self.entries
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Return the number of cached routes currently in memory.
    pub fn len(&self) -> usize {
        self.entries
            .read()
            .unwrap_or_else(|e| e.into_inner())
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
        };
        cache.insert("/docs", entry);
        assert_eq!(cache.len(), 1);

        let fetched = cache.get("/docs").unwrap();
        assert!(fetched.is_fresh(Duration::from_secs(60)));

        assert!(cache.invalidate("/docs"));
        assert!(cache.is_empty());
    }
}
