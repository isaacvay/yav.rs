//! In-memory sliding window rate limiter and brute-force protection.
//!
//! Tracks client requests per IP and path, enforcing request thresholds with standard
//! `RateLimit-*` and `Retry-After` headers and returning HTTP 429 Too Many Requests.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};

/// Configuration for the rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Default maximum requests permitted within the window (e.g. 60).
    pub default_limit: u32,
    /// Sliding window duration (defaults to 60 seconds).
    pub window: Duration,
    /// Specific override thresholds for sensitive paths (e.g. `"/login"` => 5).
    pub path_limits: HashMap<String, u32>,
    /// Whether to trust proxy headers (X-Forwarded-For, X-Real-IP). Default: false.
    pub trust_proxy: bool,
    /// Optional whitelist of trusted proxy IP addresses.
    pub trusted_proxies: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            default_limit: 60,
            window: Duration::from_secs(60),
            path_limits: HashMap::new(),
            trust_proxy: false,
            trusted_proxies: Vec::new(),
        }
    }
}

impl RateLimitConfig {
    /// Create a new configuration with default 60 req/min.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default request limit per window.
    pub fn limit_per_window(mut self, limit: u32) -> Self {
        self.default_limit = limit;
        self
    }

    /// Set window duration.
    pub fn window_duration(mut self, duration: Duration) -> Self {
        self.window = duration;
        self
    }

    /// Set a custom limit for a sensitive route (e.g. `"/api/auth/login"`, 5).
    pub fn path_limit(mut self, path: impl Into<String>, limit: u32) -> Self {
        self.path_limits.insert(path.into(), limit);
        self
    }

    /// Enable or disable trusting proxy headers (`X-Forwarded-For`, `X-Real-IP`).
    pub fn trust_proxy(mut self, trust: bool) -> Self {
        self.trust_proxy = trust;
        self
    }

    /// Set an explicit list of trusted proxy IP addresses.
    pub fn trusted_proxies(mut self, proxies: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.trusted_proxies = proxies.into_iter().map(Into::into).collect();
        self.trust_proxy = true;
        self
    }
}

/// Result of checking a client request against rate limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitStatus {
    /// Request is allowed.
    Allowed {
        limit: u32,
        remaining: u32,
        reset_secs: u64,
    },
    /// Rate limit threshold exceeded.
    Exceeded { limit: u32, retry_after_secs: u64 },
}

/// Sliding-window in-memory rate limiter.
#[derive(Debug, Clone, Default)]
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

/// Maximum number of distinct client keys kept in memory to prevent memory exhaustion attacks.
pub const MAX_RATE_LIMITER_CLIENTS: usize = 10_000;

impl RateLimiter {
    /// Create a rate limiter with custom configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and record an incoming request for a client identifier (IP or user ID) and path.
    pub fn check(&self, client_id: &str, path: &str) -> RateLimitStatus {
        let now = Instant::now();
        let normalized_path = if path.len() > 1 && path.ends_with('/') {
            path.trim_end_matches('/')
        } else {
            path
        };

        let limit = self
            .config
            .path_limits
            .get(path)
            .or_else(|| self.config.path_limits.get(normalized_path))
            .copied()
            .unwrap_or(self.config.default_limit);

        let sanitized_client = if client_id.len() > 64 {
            let mut end = 64;
            while end > 0 && !client_id.is_char_boundary(end) {
                end -= 1;
            }
            &client_id[..end]
        } else {
            client_id
        };
        let key = format!("{sanitized_client}:{normalized_path}");
        let mut lock = self.clients.lock().unwrap_or_else(|e| e.into_inner());

        let window_start = now.checked_sub(self.config.window).unwrap_or(now);

        // Amortized periodic cleanup when the map grows (at powers of 2)
        if lock.len() >= 512 && lock.len().is_power_of_two() {
            lock.retain(|_, ts| {
                ts.retain(|&t| t >= window_start);
                !ts.is_empty()
            });
        }

        // Hard capacity safety cap against memory exhaustion attacks
        if lock.len() >= MAX_RATE_LIMITER_CLIENTS && !lock.contains_key(&key) {
            lock.retain(|_, ts| {
                ts.retain(|&t| t >= window_start);
                !ts.is_empty()
            });
            if lock.len() >= MAX_RATE_LIMITER_CLIENTS {
                let oldest_key = lock
                    .iter()
                    .min_by_key(|(_, ts)| ts.first().copied())
                    .map(|(k, _)| k.clone());
                if let Some(key_to_remove) = oldest_key {
                    lock.remove(&key_to_remove);
                }
            }
        }

        let timestamps = lock.entry(key).or_default();

        // Evict expired timestamps outside the sliding window
        timestamps.retain(|&t| t >= window_start);

        let count = timestamps.len() as u32;

        if count >= limit {
            // Find when the oldest timestamp in window expires
            let oldest = timestamps.first().copied().unwrap_or(now);
            let elapsed = now.duration_since(oldest);
            let retry_after = self
                .config
                .window
                .checked_sub(elapsed)
                .unwrap_or(Duration::from_secs(1))
                .as_secs()
                .max(1);

            RateLimitStatus::Exceeded {
                limit,
                retry_after_secs: retry_after,
            }
        } else {
            timestamps.push(now);
            let remaining = limit.saturating_sub(count + 1);
            RateLimitStatus::Allowed {
                limit,
                remaining,
                reset_secs: self.config.window.as_secs(),
            }
        }
    }

    /// Prune all expired timestamps and empty client keys from memory.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let window_start = now.checked_sub(self.config.window).unwrap_or(now);
        let mut lock = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        lock.retain(|_, ts| {
            ts.retain(|&t| t >= window_start);
            !ts.is_empty()
        });
    }

    /// Produce an HTTP 429 Too Many Requests response with standard headers and HTML alert.
    pub fn response_429(&self, retry_after_secs: u64, limit: u32) -> Response {
        let html_body = format!(
            r#"<!DOCTYPE html>
<html lang="fr" class="h-full">
<head>
  <meta charset="utf-8" />
  <title>429 - Trop de requêtes</title>
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <style>
    body {{ font-family: ui-sans-serif, system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #0f172a; color: #f8fafc; }}
    .card {{ background: #1e293b; padding: 2rem; border-radius: 0.75rem; max-width: 24rem; text-align: center; border: 1px solid #334155; }}
    h1 {{ font-size: 2.25rem; font-weight: 800; color: #ef4444; margin: 0 0 0.5rem; }}
    p {{ color: #94a3b8; font-size: 0.95rem; line-height: 1.5; }}
    .badge {{ display: inline-block; background: #334155; color: #cbd5e1; padding: 0.25rem 0.75rem; border-radius: 9999px; font-size: 0.85rem; margin-top: 1rem; }}
  </style>
</head>
<body>
  <div class="card">
    <h1>429</h1>
    <h2>Trop de requêtes</h2>
    <p>Vous avez dépassé la limite autorisée. Veuillez patienter avant de réessayer.</p>
    <div class="badge">Nouvel essai dans : {retry_after_secs} secondes</div>
  </div>
</body>
</html>"#
        );

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::RETRY_AFTER, &retry_after_secs.to_string()),
                (
                    header::HeaderName::from_static("ratelimit-limit"),
                    &limit.to_string(),
                ),
                (header::HeaderName::from_static("ratelimit-remaining"), "0"),
            ],
            Html(html_body),
        )
            .into_response()
    }
}

/// Axum middleware that enforces rate limits using a shared [`RateLimiter`] from request extensions.
pub async fn rate_limit_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let limiter = req.extensions().get::<RateLimiter>().cloned();

    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    let client_id = if limiter.config.trust_proxy {
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                req.headers()
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.trim().to_string())
            })
            .unwrap_or_else(|| "127.0.0.1".to_string())
    } else {
        "127.0.0.1".to_string()
    };

    let path = req.uri().path().to_string();

    match limiter.check(&client_id, &path) {
        RateLimitStatus::Allowed {
            limit,
            remaining,
            reset_secs,
        } => {
            let mut response = next.run(req).await;
            let headers = response.headers_mut();
            if let Ok(val) = limit.to_string().parse() {
                headers.insert(header::HeaderName::from_static("ratelimit-limit"), val);
            }
            if let Ok(val) = remaining.to_string().parse() {
                headers.insert(header::HeaderName::from_static("ratelimit-remaining"), val);
            }
            if let Ok(val) = reset_secs.to_string().parse() {
                headers.insert(header::HeaderName::from_static("ratelimit-reset"), val);
            }
            response
        }
        RateLimitStatus::Exceeded {
            limit,
            retry_after_secs,
        } => limiter.response_429(retry_after_secs, limit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_and_exceeds() {
        let config = RateLimitConfig::new()
            .limit_per_window(2)
            .window_duration(Duration::from_secs(5));
        let limiter = RateLimiter::new(config);

        // 1st request -> OK
        let res1 = limiter.check("127.0.0.1", "/api/test");
        assert!(matches!(
            res1,
            RateLimitStatus::Allowed { remaining: 1, .. }
        ));

        // 2nd request -> OK
        let res2 = limiter.check("127.0.0.1", "/api/test");
        assert!(matches!(
            res2,
            RateLimitStatus::Allowed { remaining: 0, .. }
        ));

        // 3rd request -> Exceeded!
        let res3 = limiter.check("127.0.0.1", "/api/test");
        assert!(matches!(res3, RateLimitStatus::Exceeded { .. }));
    }

    #[test]
    fn test_path_specific_limits() {
        let config = RateLimitConfig::new()
            .limit_per_window(10)
            .path_limit("/auth/login", 1);
        let limiter = RateLimiter::new(config);

        // First login -> OK
        assert!(matches!(
            limiter.check("1.2.3.4", "/auth/login"),
            RateLimitStatus::Allowed { .. }
        ));
        // Second login -> Exceeded
        assert!(matches!(
            limiter.check("1.2.3.4", "/auth/login"),
            RateLimitStatus::Exceeded { .. }
        ));

        // Other path still allowed
        assert!(matches!(
            limiter.check("1.2.3.4", "/other"),
            RateLimitStatus::Allowed { .. }
        ));
    }

    #[test]
    fn test_rate_limiter_trailing_slash_normalization() {
        let config = RateLimitConfig::new()
            .limit_per_window(10)
            .path_limit("/auth/login", 1);
        let limiter = RateLimiter::new(config);

        // Path /auth/login/ must match the configured limit for /auth/login
        assert!(matches!(
            limiter.check("5.6.7.8", "/auth/login/"),
            RateLimitStatus::Allowed { .. }
        ));
        assert!(matches!(
            limiter.check("5.6.7.8", "/auth/login/"),
            RateLimitStatus::Exceeded { .. }
        ));
    }
}
