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

/// Strict, nonce-based Content-Security-Policy applied by default to every response.
///
/// `{nonce}` is substituted with the per-request, server-generated nonce by
/// [`SecurityHeadersConfig::apply`]. `'strict-dynamic'` lets the framework
/// runtime (streaming Suspense swaps, PPR, HMR) create scripts dynamically.
/// `'unsafe-inline'` is kept only as a CSP Level 2 compatibility fallback:
/// CSP Level 3 browsers ignore it as soon as a nonce is present.
///
/// No scheme-wide `https:` source is allowed for scripts. In CSP Level 2 it
/// would permit any external HTTPS script, which turns an HTML injection into
/// executable code even when a nonce is used. Applications that genuinely need
/// a CDN must add the exact origin through a custom policy.
pub const DEFAULT_CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "form-action 'self'; ",
    "script-src 'nonce-{nonce}' 'strict-dynamic' 'unsafe-inline'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: https:; ",
    "font-src 'self' data:; ",
    "connect-src 'self' ws: wss:; ",
    "worker-src 'self' blob:; ",
    "manifest-src 'self'"
);

/// Fallback for responses whose body cannot be rewritten (very large or
/// third-party streams), i.e. when no nonce can be injected.
///
/// Inline scripts are therefore **not** allowed: `script-src 'self'` only.
/// This is weaker than the nonce policy for dynamically created scripts, but
/// far stronger than the previous `'self' 'unsafe-inline' https:` fallback,
/// which allowed both inline and arbitrary HTTPS scripts.
pub const FALLBACK_CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "frame-ancestors 'none'; ",
    "form-action 'self'; ",
    "script-src 'self'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: https:; ",
    "font-src 'self' data:; ",
    "connect-src 'self' ws: wss:; ",
    "worker-src 'self' blob:; ",
    "manifest-src 'self'"
);

/// Internal header used by streaming producers to advertise the nonce they
/// already injected into their chunks. Always removed before the response is
/// sent to the client.
pub const STREAMING_NONCE_HEADER: &str = "x-yav-streaming-nonce";

/// Maximum HTML body size we are willing to buffer in order to inject nonces.
const MAX_NONCE_REWRITE_BYTES: usize = 8 * 1024 * 1024;

/// Per-request cryptographic Content-Security-Policy (CSP) nonce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CspNonce(pub String);

impl CspNonce {
    /// Generate a fresh 128-bit cryptographic hex nonce seeded from OS CSPRNG entropy.
    pub fn generate() -> Self {
        let mut random_bytes = [0u8; 16];
        if getrandom::getrandom(&mut random_bytes).is_ok() {
            let mut hex = String::with_capacity(32);
            for b in random_bytes {
                use std::fmt::Write;
                let _ = write!(&mut hex, "{:02x}", b);
            }
            return Self(hex);
        }

        static FALLBACK_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        tracing::error!("OS CSPRNG getrandom failed; falling back to mixed monotonic nonce");
        let seq = FALLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut hasher = sha2::Sha256::new();
        hasher.update(now.to_le_bytes());
        hasher.update(seq.to_le_bytes());
        hasher.update(std::process::id().to_le_bytes());
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(32);
        for b in &digest[..16] {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{:02x}", b);
        }
        Self(hex)
    }

    /// Return the nonce string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// Value for `Content-Security-Policy` header (default: the strict nonce-based
    /// [`DEFAULT_CONTENT_SECURITY_POLICY`]). Supports the `{nonce}` placeholder.
    /// Set to `None` to disable the header entirely for this response.
    pub content_security_policy: Option<String>,
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
            // A CSP is opt-*out*, not opt-in: emitting no policy at all left
            // every SSR page exposed to inline XSS. The framework rewrites HTML
            // responses with the matching per-request nonce, so this strict
            // policy is functional out of the box rather than decorative.
            content_security_policy: Some(DEFAULT_CONTENT_SECURITY_POLICY.to_string()),
        }
    }
}

impl SecurityHeadersConfig {
    /// Create a new default security headers configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `Content-Security-Policy` header value (supports `{nonce}` interpolation).
    pub fn csp(mut self, policy: impl Into<String>) -> Self {
        self.content_security_policy = Some(policy.into());
        self
    }

    /// Set the `X-Frame-Options` header value.
    pub fn frame_options(mut self, val: impl Into<String>) -> Self {
        self.frame_options = Some(val.into());
        self
    }

    /// Apply configured security headers to an outgoing [`HeaderMap`](axum::http::HeaderMap).
    pub fn apply(&self, headers: &mut axum::http::HeaderMap, nonce: Option<&str>) {
        if let Some(ref v) = self.content_type_options {
            if !headers.contains_key(header::X_CONTENT_TYPE_OPTIONS) {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(header::X_CONTENT_TYPE_OPTIONS, hv);
                }
            }
        }
        if let Some(ref v) = self.frame_options {
            if !headers.contains_key(header::X_FRAME_OPTIONS) {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(header::X_FRAME_OPTIONS, hv);
                }
            }
        }
        if let Some(ref v) = self.xss_protection {
            if !headers.contains_key("x-xss-protection") {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(HeaderName::from_static("x-xss-protection"), hv);
                }
            }
        }
        if let Some(ref v) = self.referrer_policy {
            if !headers.contains_key(header::REFERRER_POLICY) {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(header::REFERRER_POLICY, hv);
                }
            }
        }
        if let Some(ref v) = self.permissions_policy {
            if !headers.contains_key("permissions-policy") {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(HeaderName::from_static("permissions-policy"), hv);
                }
            }
        }
        if let Some(ref v) = self.coop {
            if !headers.contains_key("cross-origin-opener-policy") {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(HeaderName::from_static("cross-origin-opener-policy"), hv);
                }
            }
        }
        if let Some(ref v) = self.hsts {
            if !headers.contains_key(header::STRICT_TRANSPORT_SECURITY) {
                if let Ok(hv) = HeaderValue::from_str(v) {
                    headers.insert(header::STRICT_TRANSPORT_SECURITY, hv);
                }
            }
        }
        if let Some(ref csp) = self.content_security_policy {
            if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
                let resolved = if let Some(n) = nonce {
                    csp.replace("{nonce}", n)
                } else {
                    csp.clone()
                };
                if let Ok(hv) = HeaderValue::from_str(&resolved) {
                    headers.insert(header::CONTENT_SECURITY_POLICY, hv);
                }
            }
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
/// - `Content-Security-Policy` (strict, nonce-based by default)
///
/// If a response already contains any of these headers, the existing value is preserved.
/// A nonce-based CSP is only emitted when the response body actually carries that
/// nonce on every `<script>` tag; see [`inject_nonce_into_html_response`].
pub async fn security_headers_middleware(mut req: Request, next: Next) -> Response {
    let nonce = req
        .extensions()
        .get::<CspNonce>()
        .cloned()
        .unwrap_or_else(CspNonce::generate);
    req.extensions_mut().insert(nonce.clone());
    let mut config = req
        .extensions()
        .get::<SecurityHeadersConfig>()
        .cloned()
        .unwrap_or_default();

    let mut response = next.run(req).await;

    // Streaming producers (Suspense/PPR) inject the nonce into each chunk and
    // advertise it here. Validate the value before letting it drive a CSP.
    let streaming_nonce = response
        .headers()
        .get(STREAMING_NONCE_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
        .map(str::to_string);
    response.headers_mut().remove(STREAMING_NONCE_HEADER);

    let effective_nonce = streaming_nonce.as_deref().unwrap_or(nonce.as_str());

    let wants_nonce_csp = config
        .content_security_policy
        .as_deref()
        .is_some_and(|p| p.contains("{nonce}"))
        && !response
            .headers()
            .contains_key(header::CONTENT_SECURITY_POLICY);

    if wants_nonce_csp {
        let ready = if streaming_nonce.is_some() {
            true
        } else {
            inject_nonce_into_html_response(&mut response, effective_nonce).await
        };
        if !ready {
            tracing::warn!(
                "Could not apply a per-request CSP nonce to this response; \
                 falling back to the non-nonce strict policy"
            );
            config.content_security_policy = Some(FALLBACK_CONTENT_SECURITY_POLICY.to_string());
        }
    }

    config.apply(response.headers_mut(), Some(effective_nonce));
    response
}

/// Rewrite an HTML response so that every `<script>` opening tag carries the
/// per-request CSP nonce.
///
/// Returns `true` when a nonce-based CSP is safe for this response. Returns
/// `false` for bodies we cannot safely buffer (streams, oversized bodies,
/// non-UTF-8 payloads), in which case the caller must downgrade the policy
/// instead of emitting a CSP that would block the page's scripts.
///
/// Non-HTML responses (JSON, images, JS, CSS, …) are not document renders, so
/// they need no rewrite and return `true`.
pub async fn inject_nonce_into_html_response(response: &mut Response, nonce: &str) -> bool {
    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("text/html"));
    if !is_html {
        return true;
    }

    let is_streaming = response
        .headers()
        .get("x-accel-buffering")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("no"))
        || response.headers().contains_key(header::TRANSFER_ENCODING)
        || response.headers().contains_key("x-yav-streaming");
    if is_streaming {
        return false;
    }

    if response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        .is_some_and(|len| len > MAX_NONCE_REWRITE_BYTES)
    {
        return false;
    }

    let had_content_length = response.headers().contains_key(header::CONTENT_LENGTH);
    let body = std::mem::replace(response.body_mut(), Body::empty());
    let bytes = match to_bytes(body, MAX_NONCE_REWRITE_BYTES).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!(error = %err, "Failed to buffer HTML body for CSP nonce injection");
            *response = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::empty())
                .unwrap();
            return false;
        }
    };

    let html = match std::str::from_utf8(&bytes) {
        Ok(html) => html,
        Err(_) => {
            // Restore the untouched body; a nonce CSP would be unsafe here.
            *response.body_mut() = Body::from(bytes);
            return false;
        }
    };

    let rewritten = crate::handler::inject_csp_nonce_into_scripts(html, nonce);
    if had_content_length {
        if let Ok(len) = HeaderValue::from_str(&rewritten.len().to_string()) {
            response.headers_mut().insert(header::CONTENT_LENGTH, len);
        }
    }

    // A nonce is single-use: a cached HTML document would replay an old nonce
    // and could be served to a different browser. Drop any ETag computed on the
    // pre-nonce body and never let a shared cache store this representation.
    response.headers_mut().remove(header::ETAG);
    if let Some(cc) = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
    {
        if cc.starts_with("public") {
            let private = cc.replacen("public", "private", 1);
            if let Ok(hv) = HeaderValue::from_str(&format!("{private}, no-store")) {
                response.headers_mut().insert(header::CACHE_CONTROL, hv);
            }
        }
    }

    *response.body_mut() = Body::from(rewritten);
    true
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
                let cc_val = response
                    .headers()
                    .get(header::CACHE_CONTROL)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or(default_cache_control);
                let mut not_modified = Response::builder()
                    .status(StatusCode::NOT_MODIFIED)
                    .header(header::ETAG, existing_etag)
                    .header(header::CACHE_CONTROL, cc_val)
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
        Err(err) => {
            // Never turn a body failure into an empty `200 OK`: the client would
            // cache a blank page as a successful response. Report a real error.
            tracing::error!(error = %err, "Failed to buffer response body for ETag computation");
            parts.status = StatusCode::BAD_GATEWAY;
            parts.headers.remove(header::CONTENT_LENGTH);
            parts.headers.remove(header::ETAG);
            parts
                .headers
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            let mut error_response = Response::from_parts(parts, Body::empty());
            apply_default_security_headers(error_response.headers_mut());
            return error_response;
        }
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

// ─────────────────────────────────────────────────────────────────────────────
// CSRF protection
// ─────────────────────────────────────────────────────────────────────────────

/// CSRF protection policy for unsafe HTTP methods.
///
/// Protection is **enabled by default** (`enabled: true`). Enabling it requires
/// every form to carry [`crate::forms::CSRF_FIELD_NAME`] or every AJAX request
/// to carry the `x-csrf-token` header. Disable it explicitly with
/// [`CsrfConfig::disabled`] / [`crate::app::YavApp::csrf_protection`] for pure
/// JSON APIs protected by bearer tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsrfConfig {
    /// Whether to enforce CSRF validation.
    pub enabled: bool,
    /// Path prefixes exempt from validation (e.g. third-party webhooks).
    pub exempt_paths: Vec<String>,
    /// Header carrying the token for AJAX / JSON requests.
    pub header_name: String,
    /// Form field carrying the token for `application/x-www-form-urlencoded` bodies.
    pub field_name: String,
}

impl Default for CsrfConfig {
    /// CSRF validation is **enabled by default** for state-changing requests
    /// (POST, PUT, PATCH, DELETE). Disable it explicitly with
    /// [`CsrfConfig::disabled`] if your application only serves a JSON API
    /// protected by bearer tokens.
    fn default() -> Self {
        Self {
            enabled: true,
            exempt_paths: Vec::new(),
            header_name: crate::forms::CSRF_HEADER_NAME.to_string(),
            field_name: crate::forms::CSRF_FIELD_NAME.to_string(),
        }
    }
}

impl CsrfConfig {
    /// Explicitly disable CSRF enforcement (e.g. for pure JSON APIs protected
    /// by bearer tokens).
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }
}

impl CsrfConfig {
    /// Enable CSRF enforcement (this is the default).
    #[must_use]
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Exempt a path prefix (exact match or `prefix/...`) from validation.
    #[must_use]
    pub fn exempt(mut self, path_prefix: impl Into<String>) -> Self {
        let normalized = format!("/{}", path_prefix.into().trim_matches('/'));
        // Exempting `/` would disable protection for the whole app — ignore it.
        if normalized != "/" {
            self.exempt_paths.push(normalized);
        }
        self
    }
}

fn csrf_rejected_response() -> Response {
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from("CSRF token missing or invalid"))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

/// Decode one `application/x-www-form-urlencoded` component (`+` → space, `%HH`).
fn form_url_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = (bytes[i + 1] as char).to_digit(16)?;
                let lo = (bytes[i + 2] as char).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Extract a named field from a URL-encoded body without consuming it.
fn urlencoded_field(body: &[u8], field: &str) -> Option<String> {
    let text = std::str::from_utf8(body).ok()?;
    for pair in text.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        // A malformed key in an earlier pair must not hide a valid token in a
        // later one — skip it instead of aborting the whole scan.
        if let Some(key) = form_url_decode(key) {
            if key == field {
                return form_url_decode(value);
            }
        }
    }
    None
}

fn multipart_field(body: &[u8], content_type: &str, field: &str) -> Option<String> {
    let boundary = content_type.split(';').find_map(|part| {
        let trimmed = part.trim();
        let rest = trimmed
            .strip_prefix("boundary=")
            .or_else(|| trimmed.strip_prefix("BOUNDARY="))?;
        Some(rest.trim_matches('"').trim())
    })?;
    if boundary.is_empty() {
        return None;
    }
    let delim = format!("--{boundary}");
    let text = String::from_utf8_lossy(body);
    for section in text.split(&delim) {
        let section = section.trim_start_matches("\r\n").trim_start_matches('\n');
        if section.is_empty() || section.starts_with("--") {
            continue;
        }
        let (headers_part, val_part) = if let Some((h, v)) = section.split_once("\r\n\r\n") {
            (h, v)
        } else if let Some((h, v)) = section.split_once("\n\n") {
            (h, v)
        } else {
            continue;
        };
        let mut is_target = false;
        for line in headers_part.lines() {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("content-disposition:") {
                let needle = format!("name=\"{}\"", field.to_ascii_lowercase());
                if lower.contains(&needle) {
                    is_target = true;
                    break;
                }
            }
        }
        if is_target {
            let cleaned = val_part.trim_end_matches("\r\n").trim_end_matches('\n');
            return Some(cleaned.to_string());
        }
    }
    None
}

/// Validate the anti-CSRF token on every state-changing request.
///
/// The expected token is read from the session (`_csrf_token`), which is
/// populated by [`crate::forms::CsrfToken::from_session_with_secret`]. Clients send it
/// either in the `x-csrf-token` header or as the `_csrf` form field (`application/x-www-form-urlencoded`
/// or `multipart/form-data`). Failed validation returns `403 Forbidden` before the handler runs.
/// Safe methods (GET/HEAD/OPTIONS/TRACE) and exempt paths are passed through untouched.
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let config = req
        .extensions()
        .get::<CsrfConfig>()
        .cloned()
        .unwrap_or_default();
    if !config.enabled {
        return next.run(req).await;
    }
    if !matches!(
        *req.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    ) {
        return next.run(req).await;
    }
    let path = req.uri().path().to_string();
    if config
        .exempt_paths
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
    {
        return next.run(req).await;
    }

    let session = req.extensions().get::<crate::session::Session>().cloned();
    let session_secret = req
        .extensions()
        .get::<crate::session::SessionSecret>()
        .cloned();
    let expected = match session.as_ref().and_then(|s| s.get("_csrf_token")) {
        Some(token) if !token.is_empty() => token,
        _ => {
            if let Some(s) = &session {
                if let Some(ref secret) = session_secret {
                    let _ = crate::forms::CsrfToken::from_session_with_secret(s, secret.as_bytes());
                } else {
                    let _ = crate::forms::CsrfToken::from_session(s);
                }
            }
            return csrf_rejected_response();
        }
    };

    // Header-based submissions (JSON / HTMX / fetch) need no body buffering.
    if let Some(submitted) = req
        .headers()
        .get(config.header_name.as_str())
        .and_then(|v| v.to_str().ok())
    {
        return if crate::forms::verify_csrf_token(&expected, submitted) {
            next.run(req).await
        } else {
            csrf_rejected_response()
        };
    }

    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    // Form submissions: buffer, inspect, and rebuild the request so the
    // handler still sees the original body.
    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, 2 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return csrf_rejected_response(),
    };
    let submitted = match content_type.as_deref() {
        Some(ct) if ct.to_ascii_lowercase().starts_with("multipart/form-data") => {
            multipart_field(&bytes, ct, &config.field_name)
        }
        _ => urlencoded_field(&bytes, &config.field_name),
    };
    let valid = submitted
        .map(|tok| crate::forms::verify_csrf_token(&expected, &tok))
        .unwrap_or(false);
    if !valid {
        return csrf_rejected_response();
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use axum::{routing::get, routing::post, Router};
    use tower::ServiceExt;

    /// Seed a session carrying a known CSRF token, standing in for the real
    /// session middleware (which is exercised separately in session.rs tests).
    async fn seed_session(mut req: Request, next: Next) -> Response {
        let session = crate::session::Session::new();
        session.set("_csrf_token", "test-token-123");
        req.extensions_mut().insert(session);
        next.run(req).await
    }

    fn csrf_test_app(csrf: CsrfConfig) -> Router {
        Router::new()
            .route("/submit", post(|| async { "ok" }))
            .route("/webhook", post(|| async { "ok" }))
            .route("/page", get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(csrf_middleware))
            .layer(axum::Extension(csrf))
            .layer(axum::middleware::from_fn(seed_session))
    }

    fn post_req(path: &str) -> Request {
        Request::builder()
            .method(Method::POST)
            .uri(path)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn csrf_enabled_by_default_rejects_post() {
        let app = csrf_test_app(CsrfConfig::default());
        let resp = app.oneshot(post_req("/submit")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn csrf_disabled_passes_post() {
        let app = csrf_test_app(CsrfConfig::disabled());
        let resp = app.oneshot(post_req("/submit")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn csrf_missing_token_is_rejected() {
        let app = csrf_test_app(CsrfConfig::enabled());
        let resp = app.oneshot(post_req("/submit")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn csrf_correct_header_token_passes() {
        let app = csrf_test_app(CsrfConfig::enabled());
        let req = Request::builder()
            .method(Method::POST)
            .uri("/submit")
            .header("x-csrf-token", "test-token-123")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn csrf_wrong_header_token_is_rejected() {
        let app = csrf_test_app(CsrfConfig::enabled());
        let req = Request::builder()
            .method(Method::POST)
            .uri("/submit")
            .header("x-csrf-token", "attacker-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(req).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn csrf_urlencoded_form_field_passes() {
        let app = csrf_test_app(CsrfConfig::enabled());
        let req = Request::builder()
            .method(Method::POST)
            .uri("/submit")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from("_csrf=test-token-123&name=alice+bob"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "valid form token was rejected"
        );
    }

    #[tokio::test]
    async fn csrf_safe_methods_are_not_checked() {
        let app = csrf_test_app(CsrfConfig::enabled());
        let req = Request::builder()
            .method(Method::GET)
            .uri("/page")
            .body(Body::empty())
            .unwrap();
        assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn csrf_exempt_path_is_not_checked() {
        let app = csrf_test_app(CsrfConfig::enabled().exempt("/webhook"));
        assert_eq!(
            app.clone()
                .oneshot(post_req("/webhook"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            app.oneshot(post_req("/submit")).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn urlencoded_field_decodes_values() {
        assert_eq!(
            urlencoded_field(b"_csrf=a%2Db&name=alice+bob", "_csrf").as_deref(),
            Some("a-b")
        );
        assert_eq!(
            urlencoded_field(b"name=alice+bob", "name").as_deref(),
            Some("alice bob")
        );
        assert_eq!(urlencoded_field(b"name=alice", "_csrf"), None);
        assert_eq!(urlencoded_field(b"_csrf=%ZZ", "_csrf"), None);
    }

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
    fn test_default_csp_is_strict_and_nonce_based() {
        let config = SecurityHeadersConfig::default();
        let csp = config.content_security_policy.as_deref().unwrap();
        assert!(csp.contains("{nonce}"));
        assert!(csp.contains("'strict-dynamic'"));
        assert!(csp.contains("object-src 'none'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(csp.contains("base-uri 'self'"));
        assert!(csp.contains("form-action 'self'"));

        let mut headers = HeaderMap::new();
        config.apply(&mut headers, Some("abc123"));
        let applied = headers
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(applied.contains("'nonce-abc123'"));
        assert!(!applied.contains("{nonce}"));
    }

    #[tokio::test]
    async fn test_inject_nonce_into_html_response() {
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::ETAG, "W/\"stale\"")
            .body(Body::from(
                r#"<html><body><script>alert('x')</script><script src="/app.js"></script></body></html>"#,
            ))
            .unwrap();

        assert!(inject_nonce_into_html_response(&mut response, "nonce42").await);
        assert!(!response.headers().contains_key(header::ETAG));

        let bytes = to_bytes(
            std::mem::replace(response.body_mut(), Body::empty()),
            usize::MAX,
        )
        .await
        .unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(html.matches("nonce=\"nonce42\"").count(), 2);
    }

    #[tokio::test]
    async fn test_inject_nonce_skips_streaming_and_non_html() {
        let mut streaming = Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .header("x-accel-buffering", "no")
            .body(Body::from("<script>x()</script>"))
            .unwrap();
        assert!(!inject_nonce_into_html_response(&mut streaming, "n").await);

        let mut json = Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert!(inject_nonce_into_html_response(&mut json, "n").await);
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
