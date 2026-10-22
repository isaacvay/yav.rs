//! Common HTTP middleware for security headers and request logging.
//!
//! Provides Axum-compatible middleware functions for:
//! - Security headers (`X-Content-Type-Options`, `X-Frame-Options`, `X-XSS-Protection`, `Referrer-Policy`)
//! - Request latency and lifecycle logging using the `tracing` ecosystem

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::{
        header::{self, HeaderName, HeaderValue},
        Method, StatusCode,
    },
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

/// Configuration options for HTTP security headers.
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Value for `X-Content-Type-Options` header (default: `"nosniff"`).
    pub content_type_options: Option<String>,
    /// Value for `X-Frame-Options` header (default: `"DENY"`).
    pub frame_options: Option<String>,
    /// Value for `X-XSS-Protection` header (default: `"0"` per modern OWASP guidance).
    pub xss_protection: Option<String>,
    /// Value for `Referrer-Policy` header (default: `"strict-origin-when-cross-origin"`).
    pub referrer_policy: Option<String>,
    /// Optional value for `Permissions-Policy` header (default: `"camera=(), microphone=(), geolocation=()"`).
    pub permissions_policy: Option<String>,
    /// Optional value for `Strict-Transport-Security` header.
    pub hsts: Option<String>,
    /// Optional value for `Cross-Origin-Opener-Policy` header (default: `"same-origin"`).
    pub coop: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_type_options: Some("nosniff".to_string()),
            frame_options: Some("DENY".to_string()),
            xss_protection: Some("0".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=()".to_string()),
            hsts: if crate::session::is_production_mode() {
                Some("max-age=63072000; includeSubDomains".to_string())
            } else {
                None
            },
            coop: Some("same-origin".to_string()),
        }
    }
}

/// Axum middleware that injects recommended HTTP security headers onto every outgoing response.
///
/// Sets:
/// - `X-Content-Type-Options: nosniff`
/// - `X-Frame-Options: DENY`
/// - `X-XSS-Protection: 0`
/// - `Referrer-Policy: strict-origin-when-cross-origin`
/// - `Permissions-Policy: camera=(), microphone=(), geolocation=()`
/// - `Cross-Origin-Opener-Policy: same-origin`
/// - `Strict-Transport-Security: max-age=63072000; includeSubDomains` (in production mode)
///
/// If a response already contains any of these headers, the existing value is preserved.
pub async fn security_headers_middleware(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    apply_default_security_headers(response.headers_mut());
    response
}

/// Applies default security headers directly to an Axum/Hyper [`HeaderMap`](axum::http::HeaderMap).
pub fn apply_default_security_headers(headers: &mut axum::http::HeaderMap) {
    if !headers.contains_key(header::X_CONTENT_TYPE_OPTIONS) {
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }

    if !headers.contains_key(header::X_FRAME_OPTIONS) {
        headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    }

    if !headers.contains_key("x-xss-protection") {
        headers.insert(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("0"),
        );
    }

    if !headers.contains_key(header::REFERRER_POLICY) {
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        );
    }

    if !headers.contains_key("permissions-policy") {
        headers.insert(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        );
    }

    if !headers.contains_key("cross-origin-opener-policy") {
        headers.insert(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        );
    }

    if crate::session::is_production_mode()
        && !headers.contains_key(header::STRICT_TRANSPORT_SECURITY)
    {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }
}

/// Axum middleware that logs incoming requests, response status codes, and execution latency.
///
/// Emits structured tracing spans at `INFO`, `WARN` (for client 4xx errors), or `ERROR` (for server 5xx errors).
pub async fn request_logger_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let metrics = req
        .extensions()
        .get::<crate::health::ServerMetrics>()
        .cloned();
    if let Some(ref m) = metrics {
        m.record_request();
    }

    tracing::info!(
        method = %method,
        path = %uri.path(),
        "--> HTTP request started"
    );

    let response = next.run(req).await;

    let latency = start.elapsed();
    let status = response.status();

    if status.is_server_error() {
        if let Some(ref m) = metrics {
            m.record_error();
        }
        tracing::error!(
            method = %method,
            path = %uri.path(),
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            "<-- HTTP request server error"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            method = %method,
            path = %uri.path(),
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            "<-- HTTP request client error"
        );
    } else {
        tracing::info!(
            method = %method,
            path = %uri.path(),
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            "<-- HTTP request completed"
        );
    }

    response
}

/// Computes a weak ETag (`W/"<16-hex>"`) from raw body bytes using SHA-256.
pub fn compute_etag(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let hex_str = format!("{:x}", hash);
    let short_hash = &hex_str[..16];
    format!("W/\"{}\"", short_hash)
}

/// Checks whether an `If-None-Match` header matches a given target ETag per RFC 7232 §3.2.
///
/// Supports wildcard `*`, comma-separated lists of ETags, and weak/strong ETag equality.
pub fn etag_matches(if_none_match_header: &str, target_etag: &str) -> bool {
    let target_clean = target_etag.trim();
    let target_raw = target_clean.trim_start_matches("W/");

    for token in if_none_match_header.split(',') {
        let token_clean = token.trim();
        if token_clean.is_empty() {
            continue;
        }
        if token_clean == "*" {
            return true;
        }
        if token_clean == target_clean {
            return true;
        }
        let token_raw = token_clean.trim_start_matches("W/");
        if token_raw == target_raw {
            return true;
        }
    }
    false
}

/// Axum middleware that generates ETags for `GET`/`HEAD` `200 OK` responses and responds with
/// `304 Not Modified` if the request's `If-None-Match` header matches.
pub async fn etag_middleware(req: Request, next: Next) -> Response {
    let is_get_or_head = req.method() == Method::GET || req.method() == Method::HEAD;
    let has_req_auth = req.headers().contains_key(header::COOKIE)
        || req.headers().contains_key(header::AUTHORIZATION);
    let if_none_match = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;

    // Only process GET/HEAD responses that succeeded with 200 OK
    if !is_get_or_head || response.status() != StatusCode::OK {
        return response;
    }

    let is_private = has_req_auth || response.headers().contains_key(header::SET_COOKIE);
    let default_cache_control = if is_private {
        "private, no-cache, must-revalidate"
    } else {
        "public, max-age=0, must-revalidate"
    };

    // Bypass ETag buffering for streaming responses (SSE, chunked HTML, media) or no-cache/no-transform directives
    if let Some(ct) = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        if ct.starts_with("text/event-stream")
            || ct.starts_with("video/")
            || ct.starts_with("audio/")
        {
            return response;
        }
    }
    if let Some(cc) = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
    {
        if cc.contains("no-transform") || cc.contains("no-store") || cc.contains("no-cache") {
            return response;
        }
    }
    if let Some(xab) = response
        .headers()
        .get("x-accel-buffering")
        .and_then(|v| v.to_str().ok())
    {
        if xab == "no" {
            return response;
        }
    }
    if let Some(cl) = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
    {
        if cl > 10 * 1024 * 1024 {
            return response;
        }
    }

    // If an ETag is already present, respect it
    if let Some(existing_etag) = response
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ref req_etag) = if_none_match {
            if etag_matches(req_etag, existing_etag) {
                let mut not_modified = Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header(header::ETAG, existing_etag)
                    .header(header::CACHE_CONTROL, default_cache_control)
                    .body(Body::empty())
                    .unwrap_or(response);
                apply_default_security_headers(not_modified.headers_mut());
                return not_modified;
            }
        }
        return response;
    }

    // Extract body and compute ETag (cap reading at 10 MB)
    let (mut parts, body) = response.into_parts();
    let bytes = match to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return Response::from_parts(parts, Body::empty()),
    };

    let etag = compute_etag(&bytes);

    if let Some(ref req_etag) = if_none_match {
        if etag_matches(req_etag, &etag) {
            let mut not_modified = Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, etag)
                .header(header::CACHE_CONTROL, default_cache_control)
                .body(Body::empty())
                .unwrap();
            apply_default_security_headers(not_modified.headers_mut());
            return not_modified;
        }
    }

    if let Ok(etag_val) = HeaderValue::from_str(&etag) {
        parts.headers.insert(header::ETAG, etag_val);
    }
    if !parts.headers.contains_key(header::CACHE_CONTROL) {
        parts.headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(default_cache_control),
        );
    }

    Response::from_parts(parts, Body::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_apply_default_security_headers() {
        let mut headers = HeaderMap::new();
        apply_default_security_headers(&mut headers);

        assert_eq!(
            headers
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );
        assert_eq!(
            headers
                .get(header::X_FRAME_OPTIONS)
                .map(|v| v.to_str().unwrap()),
            Some("DENY")
        );
        assert_eq!(
            headers.get("x-xss-protection").map(|v| v.to_str().unwrap()),
            Some("0")
        );
        assert_eq!(
            headers
                .get(header::REFERRER_POLICY)
                .map(|v| v.to_str().unwrap()),
            Some("strict-origin-when-cross-origin")
        );
        assert_eq!(
            headers
                .get("permissions-policy")
                .map(|v| v.to_str().unwrap()),
            Some("camera=(), microphone=(), geolocation=()")
        );
        assert_eq!(
            headers
                .get("cross-origin-opener-policy")
                .map(|v| v.to_str().unwrap()),
            Some("same-origin")
        );
    }

    #[test]
    fn test_security_headers_preserve_existing() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("SAMEORIGIN"),
        );

        apply_default_security_headers(&mut headers);

        assert_eq!(
            headers
                .get(header::X_FRAME_OPTIONS)
                .map(|v| v.to_str().unwrap()),
            Some("SAMEORIGIN")
        );
        assert_eq!(
            headers
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );
    }

    #[test]
    fn test_security_headers_config_default() {
        let config = SecurityHeadersConfig::default();
        assert_eq!(config.content_type_options.as_deref(), Some("nosniff"));
        assert_eq!(config.frame_options.as_deref(), Some("DENY"));
        assert_eq!(config.xss_protection.as_deref(), Some("0"));
        assert_eq!(
            config.referrer_policy.as_deref(),
            Some("strict-origin-when-cross-origin")
        );
        assert_eq!(
            config.permissions_policy.as_deref(),
            Some("camera=(), microphone=(), geolocation=()")
        );
        assert_eq!(config.coop.as_deref(), Some("same-origin"));
    }

    #[test]
    fn test_compute_etag() {
        let tag1 = compute_etag(b"Hello world");
        let tag2 = compute_etag(b"Hello world");
        let tag3 = compute_etag(b"Different content");

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
        assert!(tag1.starts_with("W/\""));
        assert!(tag1.ends_with('"'));
    }

    #[test]
    fn test_etag_matches_rfc7232() {
        // Direct exact match
        assert!(etag_matches("\"abc123\"", "\"abc123\""));

        // Wildcard match
        assert!(etag_matches("*", "\"abc123\""));

        // Comma-separated list with whitespace
        assert!(etag_matches("\"xyz\", \"123\", \"abc123\"", "\"abc123\""));

        // Weak comparison: W/"tag" vs "tag"
        assert!(etag_matches("W/\"abc123\"", "\"abc123\""));
        assert!(etag_matches("\"abc123\"", "W/\"abc123\""));
        assert!(etag_matches(
            "\"first\", W/\"abc123\", \"other\"",
            "W/\"abc123\""
        ));

        // Prevention of substring false match ("10" should NOT match "1")
        assert!(!etag_matches("\"10\"", "\"1\""));
        assert!(!etag_matches("\"prefix1\"", "\"1\""));
        assert!(!etag_matches("\"1suffix\"", "\"1\""));
    }
}
