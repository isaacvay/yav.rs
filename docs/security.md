# Security & Reliability

Security is designed into Yav at the architecture level, preventing entire classes of vulnerabilities by default.

## 1. Automatic HTML Escaping & XSS Protection

All dynamic text expressions and attribute values inside `rsx!` are escaped automatically by `yav-core`:

- **HTML Body Context**: Characters `&`, `<`, `>`, `"`, `'` are translated to their corresponding HTML entities (`&amp;`, `&lt;`, etc.).
- **URL Attributes (`href`, `src`, `action`)**: Dynamic URLs are checked against malicious pseudo-protocols like `javascript:`, `vbscript:`, or `data:text/html`. Dangerous URLs are automatically sanitized.
- **Raw HTML**: Only content explicitly wrapped in `Node::raw(...)` bypasses escaping. Use with extreme caution.

## 2. Cryptographic Sessions & Signed Flash Cookies (HMAC-SHA256)

Yav implements tamper-proof stateless cookies for both user sessions (`Session`) and one-time flash notifications (`Flash`):
- Sessions and flash cookies are signed with HMAC-SHA256 using `SESSION_SECRET` (or `YAV_SECRET`).
- In production mode (`YAV_ENV=production` or release build), `SESSION_SECRET` is strictly enforced; the server refuses to start with a default development secret.
- Signatures are verified using **constant-time comparisons** (`yav_server::session::constant_time_eq`) to eliminate timing side-channels.
- Cookie attributes (`cookie_name`, `path`, `domain`) are sanitized against header/attribute injection (`;`, `\r`, `\n`, `\0`), and payloads exceeding 4 KB (`MAX_COOKIE_VALUE_LEN = 4096`) are rejected before HMAC computation or emission.
- Signed `Flash` cookies embed a 5-minute expiration timestamp (`DEFAULT_FLASH_TTL_SECS = 300`) to prevent replay attacks and are automatically committed or cleared by `session_middleware`.
- Flags: `HttpOnly; SameSite=Lax; Secure` (automatically enabled in production).

## 3. Sliding-Window Rate Limiting

Protect sensitive endpoints from brute-force and DDoS attacks using `RateLimitConfig`:

```rust
use yav::server::{YavApp, RateLimitConfig};
use std::time::Duration;

let app = YavApp::new()
    .rate_limit(
        RateLimitConfig::new()
            .limit_per_window(100)
            .window_duration(Duration::from_secs(60))
            .trust_proxy_headers(false) // Set to true only behind a trusted reverse proxy
    );
```

Headers emitted:
- `X-RateLimit-Limit`
- `X-RateLimit-Remaining`
- `Retry-After` (upon HTTP 429)

## 4. CSRF Protection for Forms

Yav includes built-in cryptographic CSRF token generation and constant-time verification via `CsrfToken`:

```rust
use yav::prelude::*;
use yav::server::{CsrfToken, FormData, Session};

// 1. In your GET page handler, bind a CSRF token to the user's session:
pub async fn form_page(session: Session) -> HtmlPage {
    let csrf = CsrfToken::from_session(&session);
    HtmlPage::new(rsx! {
        <form method="POST" action="/submit">
            {csrf.hidden_input()}
            <input name="email" type="email" required />
            <button type="submit">"Envoyer"</button>
        </form>
    })
}

// 2. In your POST handler, verify the submitted token in constant time:
pub async fn submit_handler(session: Session, form: FormData) -> Response {
    let csrf = CsrfToken::from_session(&session);
    if !csrf.verify(&form) {
        return YavError::Forbidden("Invalid CSRF token".into()).into_response();
    }
    redirect("/success")
}
```

## 5. Path Traversal & Static Asset Hardening

Static asset serving ([`crates/yav-server/src/static_files.rs`](file:///d:/deep/yav.rs/crates/yav-server/src/static_files.rs)) verifies that all resolved paths remain strictly bounded within the configured `public` directory:
- Null bytes (`\0`) and absolute/prefix components are blocked.
- Parent directory traversal sequences (`..`, `/../`, `..\`) are rejected with `400 Bad Request`.
- Both the target file and any pre-compressed `.br` / `.gz` variants undergo strict `canonicalize()` verification against the canonical base directory, preventing symlink escape attacks.
- Static SVG files (`image/svg+xml`) are served with a strict `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; sandbox` header to prevent stored XSS when navigated to directly.
- Files larger than 1 MB are streamed asynchronously in 64 KB chunks to prevent memory exhaustion (OOM DoS).

## 6. Default HTTP Security Headers

Yav applies hardened OWASP security headers by default on every response via `security_headers_middleware`:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 0` (per modern OWASP guidance)
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=(), geolocation=()`
- `Cross-Origin-Opener-Policy: same-origin`
- `Strict-Transport-Security: max-age=63072000; includeSubDomains` (automatically enabled in production mode)
