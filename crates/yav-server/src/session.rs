//! Cryptographically signed session management for Axum.
//!
//! Provides tamper-proof cookie-backed sessions using HMAC-SHA256 signatures,
//! protecting user authentication state and sensitive variables against client manipulation.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
};

/// Process-wide ephemeral secret used when no `SESSION_SECRET` is configured.
/// Generated once from the OS CSPRNG; sessions therefore do not survive a restart,
/// which is far safer than panicking on every request (release uses `panic = "abort"`).
static EPHEMERAL_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

/// Single source of truth for the session secret used by middleware, extractors and
/// CSRF helpers. Never returns the well-known development constant in production.
pub fn fallback_secret_bytes() -> Vec<u8> {
    if let Ok(s) = std::env::var("SESSION_SECRET").or_else(|_| std::env::var("YAV_SECRET")) {
        if !s.is_empty() {
            return s.into_bytes();
        }
    }
    if !is_production_mode() {
        return DEFAULT_DEV_SESSION_SECRET.to_vec();
    }
    EPHEMERAL_SECRET
        .get_or_init(|| {
            let mut bytes = [0u8; 32];
            if getrandom::getrandom(&mut bytes).is_err() {
                // Extremely rare: no OS entropy. Mix in process-unique values instead
                // of reusing a publicly known constant.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                bytes[..16].copy_from_slice(&now.to_le_bytes());
                bytes[16..24].copy_from_slice(&std::process::id().to_le_bytes());
            }
            tracing::warn!(
                "SESSION_SECRET is not set in production: generated an ephemeral secret. \
                 Sessions are invalidated on restart. Set SESSION_SECRET (>= 32 bytes) to persist them."
            );
            bytes.to_vec()
        })
        .clone()
}

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::Response,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Standard cookie name for Yav sessions.
pub const SESSION_COOKIE_NAME: &str = "_yav_session";

/// Maximum allowed encoded cookie payload length in bytes (4 KB per RFC 6265).
pub const MAX_COOKIE_VALUE_LEN: usize = 4096;

fn sanitize_cookie_attr(s: &str) -> String {
    s.chars()
        .filter(|&c| c != ';' && c != '\r' && c != '\n' && c != '\0')
        .collect()
}

/// Thread-safe user session container.
#[derive(Debug, Clone)]
pub struct Session {
    data: Arc<Mutex<HashMap<String, String>>>,
    modified: Arc<AtomicBool>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Create a new, empty session.
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            modified: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Read a string value by key from the session.
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().ok()?.get(key).cloned()
    }

    /// Store a string value into the session, marking it as modified.
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        if let Ok(mut lock) = self.data.lock() {
            lock.insert(key.into(), value.into());
            self.modified.store(true, Ordering::Relaxed);
        }
    }

    /// Remove a key from the session.
    pub fn remove(&self, key: &str) {
        if let Ok(mut lock) = self.data.lock() {
            if lock.remove(key).is_some() {
                self.modified.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Clear all session data.
    pub fn clear(&self) {
        if let Ok(mut lock) = self.data.lock() {
            lock.clear();
            self.modified.store(true, Ordering::Relaxed);
        }
    }

    /// Check if the session was modified during this request cycle.
    pub fn is_modified(&self) -> bool {
        self.modified.load(Ordering::Relaxed)
    }

    /// Check whether the session has expired according to `_expires_at`.
    pub fn is_expired(&self) -> bool {
        if let Some(exp_str) = self.get("_expires_at") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                {
                    return now.as_secs() > exp;
                }
            }
        }
        false
    }

    /// Set an expiration timeout (in seconds from now).
    pub fn set_ttl(&self, ttl_seconds: u64) {
        if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            let exp = now.as_secs() + ttl_seconds;
            self.set("_expires_at", exp.to_string());
        }
    }

    /// Return a snapshot of all current session key/values.
    pub fn to_map(&self) -> HashMap<String, String> {
        self.data.lock().map(|d| d.clone()).unwrap_or_default()
    }

    /// Generate signed `Set-Cookie` header value using HMAC-SHA256 with default session configuration.
    pub fn cookie_header(&self, secret: &[u8]) -> Option<HeaderValue> {
        self.cookie_header_with_config(secret, &SessionConfig::default())
    }

    /// Generate signed `Set-Cookie` header value using HMAC-SHA256 with custom [`SessionConfig`].
    pub fn cookie_header_with_config(
        &self,
        secret: &[u8],
        config: &SessionConfig,
    ) -> Option<HeaderValue> {
        let mut map = self.data.lock().ok()?.clone();
        let is_prod = config.secure.unwrap_or_else(is_production_mode);
        let secure_flag = if is_prod { "; Secure" } else { "" };
        let http_only = if config.http_only { "; HttpOnly" } else { "" };
        let cookie_name = sanitize_cookie_attr(&config.cookie_name);
        let path_attr = format!("; Path={}", sanitize_cookie_attr(&config.path));
        let samesite_attr = format!("; SameSite={}", config.same_site);
        let domain_attr = config
            .domain
            .as_ref()
            .map(|d| format!("; Domain={}", sanitize_cookie_attr(d)))
            .unwrap_or_default();

        if map.is_empty() {
            let val = format!(
                "{cookie_name}={path_attr}; Max-Age=0{http_only}{samesite_attr}{secure_flag}{domain_attr}",
            );
            return HeaderValue::from_str(&val).ok();
        }

        let max_age_attr = if let Some(exp_str) = map.get("_expires_at") {
            if let Ok(exp) = exp_str.parse::<u64>() {
                if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                {
                    let remaining = exp.saturating_sub(now.as_secs());
                    format!("; Max-Age={remaining}")
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else if let Some(ttl) = config.default_ttl {
            if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                map.insert("_expires_at".to_string(), (now.as_secs() + ttl).to_string());
            }
            format!("; Max-Age={ttl}")
        } else {
            String::new()
        };

        // A stable, unguessable session id makes configured revocation lists actually work.
        if !map.contains_key("session_id") {
            let id = random_bytes::<16>()
                .map(|b| hex_encode(&b))
                .unwrap_or_default();
            if !id.is_empty() {
                map.insert("session_id".to_string(), id);
            }
        }

        let json = serde_json::to_string(&map).ok()?;
        let sealed = seal_session_payload(&json, secret)?;
        if sealed.len() > MAX_COOKIE_VALUE_LEN {
            tracing::warn!(
                "Session cookie payload ({} bytes) exceeds {} bytes limit; refusing to emit oversized cookie",
                sealed.len(),
                MAX_COOKIE_VALUE_LEN
            );
            return None;
        }
        let cookie_val = format!(
            "{cookie_name}={sealed}{path_attr}{http_only}{samesite_attr}{secure_flag}{domain_attr}{max_age_attr}",
        );
        HeaderValue::from_str(&cookie_val).ok()
    }

    /// Attach this session's `Set-Cookie` header to an existing Axum [`Response`].
    pub fn attach_to_response(&self, secret: &[u8], response: Response) -> Response {
        self.attach_to_response_with_config(secret, &SessionConfig::default(), response)
    }

    /// Attach this session's `Set-Cookie` header using custom [`SessionConfig`].
    pub fn attach_to_response_with_config(
        &self,
        secret: &[u8],
        config: &SessionConfig,
        mut response: Response,
    ) -> Response {
        if self.is_modified() {
            if let Some(cookie) = self.cookie_header_with_config(secret, config) {
                response.headers_mut().append(header::SET_COOKIE, cookie);
            }
        }
        response
    }
}

/// Compute standard HMAC-SHA256 signature using the `hmac` crate.
pub fn compute_hmac_sha256(data: &[u8], key: &[u8]) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can accept key of any length");
    mac.update(data);
    let result = mac.finalize();
    hex_encode(&result.into_bytes())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

// --- Authenticated encryption of cookie payloads ---------------------------------
//
// Legacy cookies were hex-encoded JSON + an HMAC: tamper-proof but fully readable by
// the client. New cookies are AES-256-GCM sealed and still HMAC-signed (defence in
// depth). The `v2.` prefix lets us keep reading old cookies after an upgrade.

const SEALED_COOKIE_PREFIX: &str = "v2.";

fn derive_aead_key(secret: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"yav/session-cookie/aes256gcm/v2");
    hasher.update(secret);
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).ok()?;
    Some(bytes)
}

/// Seal a JSON session payload into an opaque, authenticated-encryption cookie value
/// (`v2.<base64url(nonce||ciphertext)>.<hmac>`). Returns `None` if the CSPRNG fails.
fn seal_session_payload(json: &str, secret: &[u8]) -> Option<String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;

    let nonce_bytes = random_bytes::<12>()?;
    let key = derive_aead_key(secret);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).ok()?;
    let sealed = cipher
        .encrypt(aes_gcm::Nonce::from_slice(&nonce_bytes), json.as_bytes())
        .ok()?;

    let mut blob = Vec::with_capacity(12 + sealed.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&sealed);
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&blob);
    let signed_part = format!("{SEALED_COOKIE_PREFIX}{encoded}");
    let signature = compute_hmac_sha256(signed_part.as_bytes(), secret);
    Some(format!("{signed_part}.{signature}"))
}

/// Verify and decrypt a `v2.` cookie value. Returns the plaintext JSON on success.
fn open_session_payload(payload: &str, sig: &str, secret: &[u8]) -> Option<String> {
    use aes_gcm::aead::{Aead, KeyInit};
    use base64::Engine;

    let signed_part = format!("{SEALED_COOKIE_PREFIX}{payload}");
    let expected = compute_hmac_sha256(signed_part.as_bytes(), secret);
    if !constant_time_eq(sig.as_bytes(), expected.as_bytes()) {
        tracing::warn!("Rejected sealed session cookie with invalid HMAC-SHA256 signature");
        return None;
    }
    let blob = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .ok()?;
    if blob.len() < 12 + 16 {
        return None;
    }
    let key = derive_aead_key(secret);
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).ok()?;
    let plaintext = cipher
        .decrypt(aes_gcm::Nonce::from_slice(&blob[..12]), &blob[12..])
        .ok()?;
    String::from_utf8(plaintext).ok()
}

/// Cryptographic signing secret key for session verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSecret(pub Vec<u8>);

impl SessionSecret {
    /// Create a new session secret from raw bytes or string.
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self(secret.into())
    }

    /// Get reference to raw secret bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Default session Time-To-Live (7 days in seconds).
pub const DEFAULT_SESSION_TTL_SECS: u64 = 7 * 24 * 3600;

/// Check if the server is running in production mode.
///
/// Returns `true` when:
/// - `YAV_ENV` or `NODE_ENV` is set to `"production"` or `"prod"`, OR
/// - neither environment variable is set to `"development"` / `"dev"` / `"test"` AND
///   the binary was compiled in release mode (`#[cfg(not(debug_assertions))]`).
pub fn is_production_mode() -> bool {
    if let Ok(env) = std::env::var("YAV_ENV")
        .or_else(|_| std::env::var("APP_ENV"))
        .or_else(|_| std::env::var("NODE_ENV"))
    {
        let env = env.trim();
        if env.eq_ignore_ascii_case("production") || env.eq_ignore_ascii_case("prod") {
            return true;
        }
        if env.eq_ignore_ascii_case("development")
            || env.eq_ignore_ascii_case("dev")
            || env.eq_ignore_ascii_case("test")
        {
            return false;
        }
    }
    cfg!(all(not(debug_assertions), not(test)))
}

/// Fallback secret used in dev/tests if no explicit secret is configured.
pub const DEFAULT_DEV_SESSION_SECRET: &[u8] =
    b"yav_default_dev_session_secret_change_in_production_32b";

use subtle::ConstantTimeEq;

/// Constant-time byte slice comparison to mitigate timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// SameSite cookie policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    /// Cookies are sent with top-level navigations and GET requests from third-party sites.
    Lax,
    /// Cookies are only sent in a first-party context.
    Strict,
    /// Cookies are sent in all contexts (requires `Secure`).
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SameSite::Lax => write!(f, "Lax"),
            SameSite::Strict => write!(f, "Strict"),
            SameSite::None => write!(f, "None"),
        }
    }
}

/// Thread-safe server-side revocation list for invalidated session IDs or tokens.
#[derive(Debug, Clone, Default)]
pub struct SessionRevocationList {
    revoked: Arc<RwLock<HashSet<String>>>,
}

impl PartialEq for SessionRevocationList {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.revoked, &other.revoked)
    }
}

impl Eq for SessionRevocationList {}

impl SessionRevocationList {
    /// Create a new empty revocation list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Revoke a session identifier or token.
    pub fn revoke(&self, id: impl Into<String>) {
        if let Ok(mut set) = self.revoked.write() {
            set.insert(id.into());
        }
    }

    /// Check if a session identifier or token is revoked.
    #[must_use]
    pub fn is_revoked(&self, id: &str) -> bool {
        self.revoked
            .read()
            .map(|set| set.contains(id))
            .unwrap_or(false)
    }

    /// Remove a revoked identifier.
    pub fn unrevoke(&self, id: &str) -> bool {
        if let Ok(mut set) = self.revoked.write() {
            set.remove(id)
        } else {
            false
        }
    }

    /// Total count of currently revoked sessions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.revoked.read().map(|set| set.len()).unwrap_or(0)
    }

    /// Check if the revocation list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Configuration for HTTP session cookies and behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionConfig {
    /// Name of the session cookie (default: `"_yav_session"`).
    pub cookie_name: String,
    /// Path attribute for the cookie (default: `"/"`).
    pub path: String,
    /// Domain attribute for the cookie (optional).
    pub domain: Option<String>,
    /// SameSite policy (default: [`SameSite::Lax`]).
    pub same_site: SameSite,
    /// HttpOnly flag (default: `true`).
    pub http_only: bool,
    /// Explicit secure flag override (default: `None`, which auto-detects based on `YAV_ENV=production`).
    pub secure: Option<bool>,
    /// Default Time-To-Live in seconds for sessions (optional).
    pub default_ttl: Option<u64>,
    /// Optional server-side session revocation list.
    pub revocation_list: Option<SessionRevocationList>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: SESSION_COOKIE_NAME.to_string(),
            path: "/".to_string(),
            domain: None,
            same_site: SameSite::Lax,
            http_only: true,
            secure: None,
            default_ttl: Some(DEFAULT_SESSION_TTL_SECS),
            revocation_list: None,
        }
    }
}

impl SessionConfig {
    /// Create a new session configuration with standard defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the session cookie name.
    #[must_use]
    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Set the cookie path attribute.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Set the cookie domain attribute.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the SameSite policy.
    #[must_use]
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    /// Set whether the cookie has the HttpOnly attribute.
    #[must_use]
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Explicitly enable or disable the Secure cookie attribute.
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = Some(secure);
        self
    }

    /// Set a default expiration TTL (in seconds) for sessions.
    #[must_use]
    pub fn ttl(mut self, ttl_seconds: u64) -> Self {
        self.default_ttl = Some(ttl_seconds);
        self
    }

    /// Disable automatic session expiration TTL (browser-session cookie).
    #[must_use]
    pub fn without_ttl(mut self) -> Self {
        self.default_ttl = None;
        self
    }

    /// Attach a server-side session revocation list to reject invalidated sessions.
    #[must_use]
    pub fn revocation_list(mut self, list: SessionRevocationList) -> Self {
        self.revocation_list = Some(list);
        self
    }
}

/// Helper to extract a cryptographically signed [`Session`] from a `Cookie` header.
pub fn extract_session_from_cookie(
    cookie_hdr: Option<&HeaderValue>,
    secret: &[u8],
    config: &SessionConfig,
) -> Session {
    let mut session_map = HashMap::new();

    if let Some(cookie_hdr) = cookie_hdr {
        if let Ok(cookie_str) = cookie_hdr.to_str() {
            for pair in cookie_str.split(';') {
                let mut parts_iter = pair.trim().splitn(2, '=');
                if let (Some(name), Some(val)) = (parts_iter.next(), parts_iter.next()) {
                    if name == config.cookie_name {
                        // `v2.<b64(nonce||ciphertext)>.<hmac>` or legacy `<hex>.<hmac>`.
                        // `rsplit_once` is required because sealed values contain a dot.
                        if let Some((payload, sig)) = val.rsplit_once('.') {
                            if payload.len() > MAX_COOKIE_VALUE_LEN {
                                tracing::warn!("Rejected oversized session cookie payload");
                                continue;
                            }
                            let decoded_json = if let Some(encoded) =
                                payload.strip_prefix(SEALED_COOKIE_PREFIX)
                            {
                                open_session_payload(encoded, sig, secret)
                            } else {
                                // Backwards compatibility: still accept signed-only cookies
                                // issued by older versions of the framework.
                                let expected_sig = compute_hmac_sha256(payload.as_bytes(), secret);
                                if constant_time_eq(sig.as_bytes(), expected_sig.as_bytes()) {
                                    hex_decode(payload).and_then(|b| String::from_utf8(b).ok())
                                } else {
                                    tracing::warn!(
                                            "Rejected session cookie with invalid HMAC-SHA256 signature"
                                        );
                                    None
                                }
                            };

                            if let Some(json) = decoded_json {
                                if let Ok(map) =
                                    serde_json::from_str::<HashMap<String, String>>(&json)
                                {
                                    let expired = if let Some(exp_str) = map.get("_expires_at") {
                                        if let Ok(exp) = exp_str.parse::<u64>() {
                                            if let Ok(now) = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                            {
                                                now.as_secs() > exp
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };

                                    let is_revoked =
                                        if let Some(ref rev_list) = config.revocation_list {
                                            let id_revoked = map
                                                .get("session_id")
                                                .or_else(|| map.get("id"))
                                                .or_else(|| map.get("_id"))
                                                .map(|id| rev_list.is_revoked(id))
                                                .unwrap_or(false);
                                            id_revoked || rev_list.is_revoked(payload)
                                        } else {
                                            false
                                        };

                                    if is_revoked {
                                        tracing::info!(
                                            "Session is revoked; starting with empty session"
                                        );
                                    } else if !expired {
                                        session_map = map;
                                    } else {
                                        tracing::debug!(
                                            "Session expired; starting with empty session"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Session {
        data: Arc::new(Mutex::new(session_map)),
        modified: Arc::new(AtomicBool::new(false)),
    }
}

/// Axum middleware that extracts the session, attaches it to request extensions,
/// and automatically appends the `Set-Cookie` header to the response if the session was modified.
pub async fn session_middleware(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let secret = req
        .extensions()
        .get::<SessionSecret>()
        .cloned()
        .or_else(|| {
            std::env::var("SESSION_SECRET")
                .or_else(|_| std::env::var("YAV_SECRET"))
                .ok()
                .map(|s| SessionSecret::new(s.into_bytes()))
        })
        .unwrap_or_else(|| SessionSecret::new(fallback_secret_bytes()));

    let config = req
        .extensions()
        .get::<SessionConfig>()
        .cloned()
        .unwrap_or_default();

    let session = if let Some(existing) = req.extensions().get::<Session>() {
        existing.clone()
    } else {
        let cookie_hdr = req.headers().get(header::COOKIE);
        let s = extract_session_from_cookie(cookie_hdr, secret.as_bytes(), &config);
        req.extensions_mut().insert(s.clone());
        s
    };

    let flash = if let Some(existing) = req.extensions().get::<crate::flash::Flash>() {
        existing.clone()
    } else {
        let cookie_hdr = req.headers().get(header::COOKIE);
        let f = crate::flash::extract_flash_from_cookie(cookie_hdr, secret.as_bytes());
        req.extensions_mut().insert(f.clone());
        f
    };

    let mut response = next.run(req).await;

    if session.is_modified() {
        if let Some(cookie) = session.cookie_header_with_config(secret.as_bytes(), &config) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
    }

    if flash.should_commit() && !flash.is_committed() {
        if let Some(cookie) = flash.cookie_header_with_secret(secret.as_bytes()) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
            flash.mark_committed();
        }
    }

    response
}

impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(existing) = parts.extensions.get::<Session>() {
            return Ok(existing.clone());
        }

        let secret: Vec<u8> = parts
            .extensions
            .get::<SessionSecret>()
            .map(|s| s.0.clone())
            .or_else(|| {
                std::env::var("SESSION_SECRET")
                    .or_else(|_| std::env::var("YAV_SECRET"))
                    .ok()
                    .map(|s| s.into_bytes())
            })
            .unwrap_or_else(fallback_secret_bytes);

        let config = parts
            .extensions
            .get::<SessionConfig>()
            .cloned()
            .unwrap_or_default();

        let session =
            extract_session_from_cookie(parts.headers.get(header::COOKIE), &secret, &config);
        parts.extensions.insert(session.clone());
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let session = Session::new();
        assert_eq!(session.get("user_id"), None);

        session.set("user_id", "alice123");
        assert_eq!(session.get("user_id"), Some("alice123".into()));
        assert!(session.is_modified());

        let secret = b"my_super_secret_signing_key_32_bytes";
        let header = session
            .cookie_header(secret)
            .expect("Cookie header generated");
        let val = header.to_str().unwrap();
        assert!(val.contains(SESSION_COOKIE_NAME));
        assert!(val.contains("Path=/"));
        assert!(val.contains("HttpOnly"));

        session.remove("user_id");
        assert_eq!(session.get("user_id"), None);
    }

    #[test]
    fn test_hmac_sha256() {
        let sig1 = compute_hmac_sha256(b"hello world", b"secret");
        let sig2 = compute_hmac_sha256(b"hello world", b"secret");
        let sig3 = compute_hmac_sha256(b"different data", b"secret");

        assert_eq!(sig1, sig2);
        assert_ne!(sig1, sig3);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"password", b"password"));
        assert!(!constant_time_eq(b"password", b"passwore"));
        assert!(!constant_time_eq(b"pass", b"password"));
    }

    #[tokio::test]
    async fn test_from_request_parts_hmac_verification() {
        use axum::http::Request;

        let secret = b"super_secret_key_12345";
        let session = Session::new();
        session.set("user_id", "admin");
        let cookie_hdr = session.cookie_header(secret).unwrap();

        // 1. Valid cookie with SessionSecret extension
        let req = Request::builder()
            .header(header::COOKIE, cookie_hdr.to_str().unwrap())
            .extension(SessionSecret::new(secret.to_vec()))
            .body(())
            .unwrap();
        let (mut parts, _) = req.into_parts();
        let extracted = Session::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(extracted.get("user_id"), Some("admin".to_string()));

        // 2. Forged signature should be rejected!
        let forged_cookie = format!(
            "{}=61646d696e.0000000000000000000000000000000000000000000000000000000000000000",
            SESSION_COOKIE_NAME
        );
        let req = Request::builder()
            .header(header::COOKIE, forged_cookie)
            .extension(SessionSecret::new(secret.to_vec()))
            .body(())
            .unwrap();
        let (mut parts, _) = req.into_parts();
        let extracted = Session::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(extracted.get("user_id"), None);
    }

    #[tokio::test]
    async fn test_session_ttl_and_expiration() {
        use axum::http::Request;

        let secret = b"test_secret_key_123";
        let session = Session::new();
        session.set("user", "bob");
        session.set_ttl(3600); // 1 hour
        assert!(!session.is_expired());

        let header = session.cookie_header(secret).unwrap();
        let header_str = header.to_str().unwrap();
        assert!(header_str.contains("Max-Age="));

        // Test already expired session
        let expired_session = Session::new();
        expired_session.set("user", "alice");
        // Expired in 1970
        expired_session.set("_expires_at", "1000");
        assert!(expired_session.is_expired());

        let exp_hdr = expired_session.cookie_header(secret).unwrap();
        let req = Request::builder()
            .header(header::COOKIE, exp_hdr.to_str().unwrap())
            .extension(SessionSecret::new(secret.to_vec()))
            .body(())
            .unwrap();
        let (mut parts, _) = req.into_parts();
        let extracted = Session::from_request_parts(&mut parts, &()).await.unwrap();
        // Expired session data must be rejected
        assert_eq!(extracted.get("user"), None);
    }

    #[test]
    fn test_session_config_builder() {
        let config = SessionConfig::new()
            .cookie_name("custom_sid")
            .path("/app")
            .same_site(SameSite::Strict)
            .http_only(false)
            .secure(true)
            .ttl(7200);

        assert_eq!(config.cookie_name, "custom_sid");
        assert_eq!(config.path, "/app");
        assert_eq!(config.same_site, SameSite::Strict);
        assert!(!config.http_only);
        assert_eq!(config.secure, Some(true));
        assert_eq!(config.default_ttl, Some(7200));

        let session = Session::new();
        session.set("theme", "dark");
        let secret = b"super_secret_for_config_test_32b";
        let header = session
            .cookie_header_with_config(secret, &config)
            .expect("cookie header");
        let header_str = header.to_str().unwrap();

        assert!(header_str.starts_with("custom_sid="));
        assert!(header_str.contains("Path=/app"));
        assert!(header_str.contains("SameSite=Strict"));
        assert!(header_str.contains("Secure"));
        assert!(!header_str.contains("HttpOnly"));
        assert!(header_str.contains("Max-Age=7200"));
    }

    #[tokio::test]
    async fn test_session_middleware_auto_commit() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
            middleware,
            response::IntoResponse,
            routing::get,
            Extension, Router,
        };
        use tower::ServiceExt;

        let secret = SessionSecret::new(b"auto_commit_test_secret_32_bytes".to_vec());

        let app = Router::new()
            .route(
                "/login",
                get(|session: Session| async move {
                    session.set("user_id", "user_999");
                    (StatusCode::OK, "Logged in").into_response()
                }),
            )
            .route(
                "/profile",
                get(|session: Session| async move {
                    let user = session.get("user_id").unwrap_or_else(|| "guest".into());
                    (StatusCode::OK, user).into_response()
                }),
            )
            .layer(middleware::from_fn(session_middleware))
            .layer(Extension(secret));

        // 1. Call /login - verify Set-Cookie is automatically returned!
        let req1 = Request::builder()
            .uri("/login")
            .body(Body::empty())
            .unwrap();

        let resp1 = app.clone().oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        let set_cookie = resp1
            .headers()
            .get(header::SET_COOKIE)
            .expect("Set-Cookie MUST be automatically committed by session_middleware");
        let cookie_str = set_cookie.to_str().unwrap();
        assert!(cookie_str.starts_with("_yav_session="));

        // 2. Call /profile with the returned cookie - verify session state is maintained!
        let cookie_val = cookie_str.split(';').next().unwrap();
        let req2 = Request::builder()
            .uri("/profile")
            .header(header::COOKIE, cookie_val)
            .body(Body::empty())
            .unwrap();

        let resp2 = app.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(resp2.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body_bytes, "user_999");
    }

    #[test]
    fn test_cookie_attribute_sanitization_and_size_limit() {
        let config = SessionConfig::new()
            .cookie_name("sid; EvilAttr=1\r\n")
            .path("/app; HttpOnly=false")
            .domain("example.com; injected=true");

        let session = Session::new();
        session.set("k", "v");
        let secret = b"sanitization_test_secret_32_byte";
        let header = session
            .cookie_header_with_config(secret, &config)
            .expect("header");
        let val = header.to_str().unwrap();
        assert!(val.starts_with("sid EvilAttr=1="));
        assert!(val.contains("Path=/app HttpOnly=false"));
        assert!(val.contains("Domain=example.com injected=true"));

        // Oversized payload (> 4096 hex chars) should return None
        let huge_session = Session::new();
        huge_session.set("big", "a".repeat(3000));
        assert!(huge_session
            .cookie_header_with_config(secret, &config)
            .is_none());
    }

    #[test]
    fn test_session_revocation_list() {
        let revocation_list = SessionRevocationList::new();
        let config = SessionConfig::new().revocation_list(revocation_list.clone());
        let secret = b"revocation_test_secret_32_bytes_";

        let session = Session::new();
        session.set("session_id", "sess_abc123");
        session.set("user", "alice");

        let header = session
            .cookie_header_with_config(secret, &config)
            .expect("cookie header");

        // Prior to revocation: session is valid and extracts user
        let extracted1 = extract_session_from_cookie(Some(&header), secret, &config);
        assert_eq!(extracted1.get("user").as_deref(), Some("alice"));

        // Revoke the session ID
        revocation_list.revoke("sess_abc123");
        assert!(revocation_list.is_revoked("sess_abc123"));
        assert_eq!(revocation_list.len(), 1);

        // After revocation: session is rejected and returns empty session
        let extracted2 = extract_session_from_cookie(Some(&header), secret, &config);
        assert_eq!(extracted2.get("user"), None);

        // Unrevoke
        assert!(revocation_list.unrevoke("sess_abc123"));
        let extracted3 = extract_session_from_cookie(Some(&header), secret, &config);
        assert_eq!(extracted3.get("user").as_deref(), Some("alice"));
    }

    #[test]
    fn test_cookie_payload_is_encrypted_and_authenticated() {
        let secret = b"encryption_test_secret_at_least_32_bytes";
        let session = Session::new();
        session.set("user", "alice");
        session.set("token", "super-secret-value");

        let header = session.cookie_header(secret).expect("cookie header");
        let raw = header.to_str().unwrap();
        // The client must not be able to read session values from the cookie.
        assert!(!raw.contains("alice"));
        assert!(!raw.contains("super-secret-value"));
        assert!(raw.contains("v2."));
        // A stable session id is emitted so revocation lists are usable.
        let extracted =
            extract_session_from_cookie(Some(&header), secret, &SessionConfig::default());
        assert_eq!(extracted.get("user").as_deref(), Some("alice"));
        assert!(extracted.get("session_id").is_some());

        // Wrong key must not decrypt (and must not panic).
        let wrong = extract_session_from_cookie(
            Some(&header),
            b"a-completely-different-secret-value",
            &SessionConfig::default(),
        );
        assert_eq!(wrong.get("user"), None);

        // Tampering with the sealed payload must be rejected.
        let tampered = HeaderValue::from_str(&raw.replace("v2.", "v2.A")).unwrap();
        let rejected =
            extract_session_from_cookie(Some(&tampered), secret, &SessionConfig::default());
        assert_eq!(rejected.get("user"), None);
    }

    #[test]
    fn test_legacy_signed_cookie_is_still_accepted() {
        let secret = b"legacy_upgrade_secret_32_bytes_______";
        let json = r#"{"user":"bob"}"#;
        let payload = hex_encode(json.as_bytes());
        let sig = compute_hmac_sha256(payload.as_bytes(), secret);
        let legacy =
            HeaderValue::from_str(&format!("_yav_session={payload}.{sig}; Path=/; HttpOnly"))
                .unwrap();
        let extracted =
            extract_session_from_cookie(Some(&legacy), secret, &SessionConfig::default());
        assert_eq!(extracted.get("user").as_deref(), Some("bob"));
    }

    #[test]
    fn test_fallback_secret_is_never_empty() {
        assert!(!fallback_secret_bytes().is_empty());
    }
}
