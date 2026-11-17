# Security & Reliability

Security is designed into Yav at the architecture level, preventing entire classes of vulnerabilities by default.

## 1. Automatic HTML Escaping & XSS Protection

All dynamic text expressions and attribute values inside `rsx!` are escaped automatically by `yav-core`:

- **HTML Body Context**: Characters `&`, `<`, `>`, `"`, `'` are translated to their corresponding HTML entities (`&amp;`, `&lt;`, etc.).
- **URL Attributes (`href`, `src`, `action`, `srcset`, …)**: Dynamic URLs are checked against malicious pseudo-protocols like `javascript:`, `vbscript:`, or `data:text/html` and dropped when unsafe. `srcdoc` is double-HTML-escaped, `srcset` is validated URL-by-URL, and `style` values are filtered against `expression(`, `javascript:` and `@import`.
- **Raw HTML**: Only content explicitly wrapped in `Node::raw(...)` bypasses escaping. Use with extreme caution.

## 2. Encrypted Sessions & Signed Flash Cookies

Yav implements tamper-proof stateless cookies for both user sessions (`Session`) and one-time flash notifications (`Flash`):
- Session payloads are **AES-256-GCM encrypted** and authenticated; flash cookies are HMAC-SHA256 signed. Both derive their key from `SESSION_SECRET` (or `YAV_SECRET`).
- If no secret is configured, the process generates a single ephemeral CSPRNG secret, logs a warning and starts. Sessions and flash cookies are then invalidated on restart and are not shared between instances. Generated systemd/Docker units require the variable explicitly.
- Legacy flash payloads without an `expires_at` are rejected, so a captured signed cookie cannot be replayed forever.
- Signatures are verified using **constant-time comparisons** (`yav_server::session::constant_time_eq`) to eliminate timing side-channels.
- Cookie attributes (`cookie_name`, `path`, `domain`) are sanitized against header/attribute injection (`;`, `\r`, `\n`, `\0`), and payloads exceeding 4 KB (`MAX_COOKIE_VALUE_LEN = 4096`) are rejected before HMAC computation or emission.
- Signed `Flash` cookies embed a 5-minute expiration timestamp (`DEFAULT_FLASH_TTL_SECS = 300`) to prevent replay attacks and are automatically committed or cleared by `session_middleware`.
- Flags: `HttpOnly; SameSite=Lax; Secure` (automatically enabled in production).

## 3. Sliding-Window Rate Limiting

Rate limiting is **optional** and is not enabled by `YavApp::new()`. Configure
and mount it explicitly:

```rust
use yav::server::{YavApp, RateLimitConfig};
use std::time::Duration;

let cfg = RateLimitConfig::new()
    .limit_per_window(100)
    .window_duration(Duration::from_secs(60))
    .trust_proxy(false); // or .trusted_proxies(["10.0.0.1"]) behind a proxy
let app = YavApp::new().rate_limit(cfg);
```

Headers emitted (lowercase names):
- `ratelimit-limit`
- `ratelimit-remaining`
- `ratelimit-reset`
- `retry-after` (upon HTTP 429)

## 4. CSRF Protection for Forms with CSPRNG Salt

Yav includes built-in cryptographic CSRF token generation and constant-time verification via `CsrfToken`:
- Each generated `CsrfToken` incorporates a cryptographically secure random (CSPRNG) per-token salt, ensuring two distinct tokens generated for the same or different sessions are unique and unpredictable.
- Tokens are signed and verified in constant time (`constant_time_eq`) to prevent timing attacks.

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

---

## 5. Typed Form Validation (`FromFormData` & `FormErrors`)

Safely parse, deserialize, and validate user input before processing business logic:

```rust
use yav::server::{FormData, FormErrors, FromFormData};

#[derive(Debug)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
}

impl FromFormData for RegisterForm {
    fn from_form(form: &FormData) -> Result<Self, FormErrors> {
        let mut errors = FormErrors::new();
        let username = form.get_or_empty("username");
        let email = form.get_or_empty("email");

        errors.require("username", username, "Le nom d'utilisateur est requis");
        errors.require("email", email, "L'adresse email est requise");

        if !email.is_empty() && !email.contains('@') {
            errors.add("email", "L'adresse email est invalide");
        }

        if errors.has_errors() {
            Err(errors)
        } else {
            Ok(Self {
                username: username.to_string(),
                email: email.to_string(),
            })
        }
    }
}

// In your POST handler:
pub async fn register_action(form: FormData) -> impl IntoResponse {
    match form.parse_validated::<RegisterForm>() {
        Ok(valid_user) => {
            // Process clean, validated input safely
            redirect("/welcome")
        }
        Err(validation_errors) => {
            // Render form with specific field error messages
            let msg = validation_errors.first("email").unwrap_or("Erreur de formulaire");
            HtmlFragment::new(rsx! { <p class="text-red-500">{msg}</p> }).into_response()
        }
    }
}
```

---

## 6. XSS-Safe i18n JSON Serialization

When passing server-side translation catalogs or data dictionaries into client-side `<script>` tags, malicious strings like `</script><script>alert(1)</script>` can break out of script contexts.

Yav's `I18n::to_json()` automatically sanitizes forward slashes into `<\/script>`, rendering inline JSON dictionaries 100% immune to HTML tag injection:

```rust
use yav_core::I18n;

let i18n = I18n::new("fr").add_translation("fr", "key", "Test </script> alert");
let safe_json = i18n.to_json();
assert!(safe_json.contains(r#"<\/script>"#));
```

---

## 7. Path Traversal & Static Asset Hardening

Static asset serving ([`crates/yav-server/src/static_files.rs`](../crates/yav-server/src/static_files.rs)) verifies that all resolved paths remain strictly bounded within the configured `public` directory:
- Null bytes (`\0`) and absolute/prefix components are blocked.
- Parent directory traversal sequences (`..`, `/../`, `..\`) are rejected with `400 Bad Request`.
- Both the target file and any pre-compressed `.br` / `.gz` variants undergo strict `canonicalize()` verification against the canonical base directory, preventing symlink escape attacks.
- Static SVG files (`image/svg+xml`) are served with a strict `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; sandbox` header to prevent stored XSS when navigated to directly.
- Files larger than 1 MB are streamed asynchronously in 64 KB chunks to prevent memory exhaustion (OOM DoS).

---

## 8. Default HTTP Security Headers

Yav applies hardened OWASP security headers by default on every response via `security_headers_middleware`:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 0` (per modern OWASP guidance)
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=(), geolocation=()`
- `Cross-Origin-Opener-Policy: same-origin`
- `Strict-Transport-Security: max-age=63072000; includeSubDomains` (automatically enabled in production mode)

---

## 9. Cryptographic CSP Nonce Injection

To satisfy strict Content Security Policy (`script-src 'nonce-...'`), Yav provides automated per-request CSP nonces via `CspNonce`:
- Each incoming request is assigned a stable, cryptographically generated nonce (or reuses `x-nonce` / `csp-nonce` headers from upstream proxies).
- The `HtmlPage::render()` document wrapper inspects the SSR context and **automatically injects `nonce="<nonce>"` onto every `<script>` tag** in the document, including framework micro-runtimes (`yav-micro-actions`, `yav-signals-runtime`, etc.).
- Inline scripts run safely without requiring `unsafe-inline` in your CSP headers.

---

## 10. Complete Authentication, JWT, OAuth2 PKCE & TOTP 2FA (`yav-server::auth`)

Yav includes a production-grade authentication suite:

### A. Password Hashing (`PasswordHasher`)
Hardened PBKDF2-HMAC-SHA256 with CSPRNG per-password salt and 100,000 iterations (`DEFAULT_PBKDF2_ITERATIONS`):
- **CSPRNG-Seeded Salt Generator (`generate_crypto_salt_16()`)**: Uses operating system entropy via standard library `RandomState` combined with SHA-256 digests and nanosecond monotonic clocks to generate completely unpredictable, cryptographically random 16-byte hex salts without relying on predictable PRNG seeds.
- **Constant-Time Verification**: Password verification uses constant-time byte comparisons (`constant_time_eq`), completely neutralizing timing side-channel attacks.

```rust
use yav_server::auth::PasswordHasher;

let hasher = PasswordHasher::new();
let hash = hasher.hash_password("SuperSecretPass123!")?;
assert!(hasher.verify_password("SuperSecretPass123!", &hash));
assert!(!hasher.verify_password("WrongPassword", &hash));
```

### B. JWT Tokens & Role-Based Access Control (`JwtManager`, `AuthUser`)
Cryptographically signed HS256 JWT tokens with subject, role arrays, and expiration checks:
```rust
use yav_server::auth::{JwtManager, JwtClaims, JwtValidation};

let jwt = JwtManager::new("your-secure-jwt-signing-key-32-chars");
let claims = JwtClaims::new("u1", 3600).with_role("admin");
let token = jwt.sign(&claims)?;

let verified = jwt.verify_with_config(&token, &JwtValidation::default())?;
assert!(verified.has_role("admin"));
```

### C. OAuth2 with PKCE (`OAuthConfig`, `OAuthProvider`)
RFC 7636 Proof Key for Code Exchange (PKCE) with SHA-256 code challenge generation and verification, ready for Google, GitHub, or generic OIDC providers.

### D. Time-Based One-Time Password 2FA (`Totp`)
RFC 6238 TOTP engine with base32 secret generation, 6-digit TOTP code calculation, and tolerance window verification (+/- 1 time step) to compensate for clock drift:
```rust
use yav_server::auth::Totp;
use std::time::{SystemTime, UNIX_EPOCH};

let secret = [0u8; 20]; // generate and store a real random secret in production
let totp = Totp::new(secret);
let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
let code = totp.generate_at(now);
assert!(totp.verify_at(&code, now));
```

---

## 11. Tamper-Evident Hash-Chained Audit Trail (`AuditTrail`)

Track compliance and security-sensitive user actions with a cryptographically verified audit log where each event's hash is chained to the previous event (`prev_hash`):
```rust
use yav_server::flags::AuditTrail;
use std::collections::BTreeMap;

let audit = AuditTrail::new();
let mut meta = BTreeMap::new();
meta.insert("ip".to_string(), "192.168.1.1".to_string());
audit.record("user-123", "tenant-a", "user_login", "session", meta);
audit.record("user-123", "tenant-a", "password_change", "account", BTreeMap::new());

// Verifies the hash chain from genesis to tail:
assert!(audit.verify_integrity());
```
*Any external tampering with past events invalidates subsequent hashes and is immediately detected.*

---

## 12. Attribute-Based Access Control (`PolicyEngine`)

Define fine-grained ABAC permissions taking into account subject roles, tenant IDs, resource owners, and action verbs:
```rust
use yav_server::flags::{PolicyEngine, PolicySubject, PolicyResource};

let mut policy = PolicyEngine::new();
policy.allow_role("admin", "*");
policy.allow_owner("editor", "article", "update");

let subject = PolicySubject::new("u2").role("editor");
let resource = PolicyResource::new("article", "art-99").owner("u2");

assert!(policy.evaluate(&subject, "update", &resource));
```

---

## 13. Open Redirect & Header Injection Protection

Unvalidated redirects expose users to phishing attacks. Yav provides strict redirection validation helpers:
- `is_safe_redirect_target(url)`: Validates URLs against control characters, CRLF injection (`\r`, `\n`), protocol-relative bypasses (`//evil.com`, `/\evil.com`), and dangerous URI schemes (`javascript:`, `data:`, `vbscript:`).
- `is_local_redirect_path(path)`: Ensures the target strictly begins with `/` and stays within the local application domain.
- `safe_redirect(url)`: Redirects to the local path or safely defaults to `"/"` if an external or malicious target is supplied.

---

## 14. Webhook Signature Verification (`WebhookVerifier`)

Verify inbound webhooks from Stripe, Svix, GitHub, or Slack with HMAC-SHA256 and configurable replay tolerance:
```rust
use yav_server::billing::WebhookVerifier;
use std::time::{SystemTime, UNIX_EPOCH};

let verifier = WebhookVerifier::new(b"whsec_test_secret_123".to_vec(), 300);
let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

assert!(verifier.verify_at(signature_header, payload_bytes, now).is_ok());
```

---

## 15. DoS Mitigation: LiveQuery Auto-Pruning & Disk-Spooling Resumable Uploads

Yav incorporates proactive defenses against memory exhaustion and slowloris-style denial of service:

### A. LiveQuery Idle Channel Pruning (`prune_idle_channels`)
Long-lived real-time connections (Server-Sent Events) can accumulate orphaned broadcast channels if clients disconnect abruptly without completing closure handshakes.
- `LiveQueryManager` periodically executes `prune_idle_channels()` to reclaim memory occupied by channels with zero active subscribers.
- An automatic safety threshold (`auto_prune_threshold = 512`) triggers garbage collection whenever channel counts surge, guaranteeing stable memory profiles even under churn.

### B. Resumable Chunked Uploads Disk Spooling (`ChunkedUploader::with_spool_dir`)
Buffering partial multipart chunks in server RAM poses a severe out-of-memory (OOM) attack vector.
- The `ChunkedUploader` writes incoming Tus protocol chunks directly to a sandboxed spooling directory on disk (`with_spool_dir`).
- Memory overhead remains fixed at $O(1)$ regardless of whether uploaded files are 10 MB or 10 GB.
- When an upload finishes or is cancelled, temporary spool files are cleaned up safely with atomic renames.

---

## 16. Relationship-Based Access Control (ReBAC) & Graph Security

Yav implements Google Zanzibar-style relationship-based access control ([`yav_server::rebac::RebacGraph`](../crates/yav-server/src/rebac.rs)) to secure multi-tenant SaaS environments:

- **Cycle Detection**: Traversal routines track visited nodes to prevent stack overflow attacks caused by circular relationship graphs (e.g. `team:A` member of `team:B` and `team:B` member of `team:A`).
- **Transitive Permission Resolution**: Evaluates complex authorization paths (e.g. `user` in `team` that owns `organization` that owns `project` that contains `document`) in sub-millisecond memory-bounded traversals.
- **Fail-Safe Default**: Any undefined relationship or unresolvable tuple path evaluates strictly to `false` (Deny by default).

