# Security & Reliability

Security is designed into Yav at the architecture level, preventing entire classes of vulnerabilities by default.

## 1. Automatic HTML Escaping & XSS Protection

All dynamic text expressions and attribute values inside `rsx!` are escaped automatically by `yav-core`:

- **HTML Body Context**: Characters `&`, `<`, `>`, `"`, `'` are translated to their corresponding HTML entities (`&amp;`, `&lt;`, etc.).
- **URL Attributes (`href`, `src`, `action`)**: Dynamic URLs are checked against malicious pseudo-protocols like `javascript:`, `vbscript:`, or `data:text/html`. Dangerous URLs are automatically sanitized.
- **Raw HTML**: Only content explicitly wrapped in `Node::raw(...)` bypasses escaping. Use with extreme caution.

## 2. Cryptographic Sessions (HMAC-SHA256)

Yav implements tamper-proof stateless cookies:
- Sessions are signed with HMAC-SHA256 using `SESSION_SECRET` (or `YAV_SECRET`).
- Signatures are verified using **constant-time comparisons** (`subtle::constant_time_eq`) to eliminate timing attacks.
- Tampered or expired cookies are rejected immediately with clean error handling.
- Flags: `HttpOnly; SameSite=Lax; Secure` (in production).

## 3. Sliding-Window Rate Limiting

Protect sensitive endpoints from brute-force and DDoS attacks:

```rust
use yav::server::{YavApp, RateLimitConfig};
use std::time::Duration;

let app = YavApp::new()
    .rate_limit(RateLimitConfig {
        window: Duration::from_secs(60),
        max_requests: 100,
        cleanup_interval: Duration::from_secs(300),
        path_limits: vec![
            ("/api/login".to_string(), (10, Duration::from_secs(60))),
        ],
    });
```

Headers emitted:
- `X-RateLimit-Limit`
- `X-RateLimit-Remaining`
- `X-RateLimit-Reset`
- `Retry-After` (upon HTTP 429)

## 4. CSRF Protection for Forms

Yav includes built-in CSRF token generation and validation helpers:

```rust
use yav::prelude::*;

// In your form component
rsx! {
    <form method="POST" action="/submit">
        {csrf_field(&session)}
        <input name="email" type="email" />
        <button type="submit">"Submit"</button>
    </form>
}

// In your server action / handler
if !verify_csrf_token(&form_token, &session) {
    return (StatusCode::FORBIDDEN, "Invalid CSRF token").into_response();
}
```

## 5. Path Traversal Prevention

Static asset serving ([`crates/yav-server/src/static_files.rs`](file:///d:/deep/yav.rs/crates/yav-server/src/static_files.rs)) verifies that all resolved paths remain strictly bounded within the configured `public` directory:
- Null bytes (`\0`) are blocked.
- Parent directory traversal sequences (`..`, `/../`, `..\`) are detected and rejected with `404 Not Found`.
- Windows backslash traversals are normalized and tested.

## 6. Secure HTTP Headers

Yav applies security headers by default on every response:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: SAMEORIGIN`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: strict-origin-when-cross-origin`
