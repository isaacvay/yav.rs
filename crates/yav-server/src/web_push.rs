//! # YAV WebPush & VAPID Notification Engine (RFC 8291 / RFC 8292)
//!
//! Provides browser push notification payload encryption, subscription management,
//! VAPID (Voluntary Application Server Identification) JWT authorization headers
//! with real ECDSA P-256 (ES256) signing, and actual HTTP delivery to push endpoints.
//!
//! ## Features
//! - **RFC 8292 VAPID**: ES256 JWT signed with a real P-256 ECDSA key (not HMAC).
//! - **RFC 8291 Payload Encryption**: aes128gcm content-encryption with ECDH
//!   key agreement on the subscriber's P-256 public key.
//! - **HTTP delivery**: `send_notification()` POSTs the encrypted payload to the
//!   push service endpoint with proper TTL and Authorization headers.
//! - **Subscription management**: `PushSubscription` models browser subscription
//!   data from `registration.pushManager.subscribe()`.

use aes_gcm::{aead::Aead, Aes128Gcm, KeyInit, Nonce};
use base64::Engine;
use p256::{
    ecdh::EphemeralSecret,
    ecdsa::{Signature, SigningKey},
    PublicKey,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Web Push subscription details gathered from `registration.pushManager.subscribe()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

impl PushSubscription {
    #[must_use]
    pub fn new(
        endpoint: impl Into<String>,
        p256dh: impl Into<String>,
        auth: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            p256dh: p256dh.into(),
            auth: auth.into(),
        }
    }
}

/// Notification payload displayed by the client Service Worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebPushNotification {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub badge: Option<String>,
    pub tag: Option<String>,
    pub data: Option<serde_json::Value>,
    pub ttl_secs: u32,
}

impl WebPushNotification {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            icon: None,
            badge: None,
            tag: None,
            data: None,
            ttl_secs: 86400,
        }
    }

    #[must_use]
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    #[must_use]
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    #[must_use]
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    #[must_use]
    pub fn ttl(mut self, seconds: u32) -> Self {
        self.ttl_secs = seconds.max(60);
        self
    }

    /// Serialize notification into standard JSON string.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// VAPID Server Keypair and authorization header generator (RFC 8292).
///
/// The `private_key` must be a 32-byte P-256 ECDSA secret key, base64url-encoded
/// without padding. The `public_key` must be the uncompressed (65-byte) SEC1
/// encoded P-256 public key, base64url-encoded without padding.
#[derive(Debug, Clone)]
pub struct VapidSigner {
    pub subject: String,
    pub public_key: String,
    pub private_key: String,
}

impl PartialEq for VapidSigner {
    fn eq(&self, other: &Self) -> bool {
        self.subject == other.subject
            && self.public_key == other.public_key
            && self.private_key == other.private_key
    }
}
impl Eq for VapidSigner {}

impl VapidSigner {
    /// Create a new VAPID signer.
    #[must_use]
    pub fn new(
        subject: impl Into<String>,
        public_key: impl Into<String>,
        private_key: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            public_key: public_key.into(),
            private_key: private_key.into(),
        }
    }

    /// Extract the audience origin (`scheme://host[:port]`) from a push endpoint.
    ///
    /// The audience is only derived from an endpoint that passed
    /// `validate_push_endpoint_syntax`; quotes, control characters, userinfo,
    /// fragments and unsafe hosts are therefore refused before any value can
    /// reach a JWT claim or the `Authorization` header. An invalid endpoint
    /// yields an empty string instead of a partially-parsed, injectable value.
    #[must_use]
    pub fn extract_audience(endpoint: &str) -> String {
        if validate_push_endpoint_syntax(endpoint).is_err() {
            return String::new();
        }
        let Ok(url) = reqwest::Url::parse(endpoint) else {
            return String::new();
        };
        let host = url.host_str().unwrap_or_default();
        match url.port() {
            Some(port) => format!("{}://{}:{}", url.scheme(), host, port),
            None => format!("{}://{}", url.scheme(), host),
        }
    }

    /// Decode the base64url-encoded private key into a `p256::ecdsa::SigningKey`.
    fn decode_signing_key(&self) -> Result<SigningKey, WebPushError> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(self.private_key.as_bytes())
            .map_err(|_| WebPushError::InvalidPrivateKey)?;
        if raw.len() != 32 {
            return Err(WebPushError::InvalidPrivateKey);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&raw);
        SigningKey::from_bytes(&arr.into()).map_err(|_| WebPushError::InvalidPrivateKey)
    }

    /// Construct a standards-compliant RFC 8292 ES256 VAPID Authorization header.
    ///
    /// This uses a real ECDSA P-256 signature (ES256) — not HMAC.
    pub fn create_authorization_header(
        &self,
        audience: &str,
        expiration_secs: u64,
    ) -> Result<String, WebPushError> {
        let signing_key = self.decode_signing_key()?;
        let signing_input = self.create_es256_signing_input(audience, expiration_secs);

        // Real ECDSA P-256 signature using the p256 crate
        use p256::ecdsa::signature::Signer;
        let signature: Signature = signing_key.sign(signing_input.as_bytes());
        let sig_bytes = signature.to_bytes();
        let sig_vec: Vec<u8> = sig_bytes.to_vec();
        let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&sig_vec);

        let jwt = format!("{signing_input}.{sig_b64}");
        Ok(format!("vapid t={jwt}, k={}", self.public_key))
    }

    /// Generate the exact signing input `base64url(header).base64url(claims)` for ES256 VAPID.
    #[must_use]
    pub fn create_es256_signing_input(&self, audience: &str, expiration_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let exp = now + expiration_secs;

        let header_json = r#"{"typ":"JWT","alg":"ES256"}"#;
        // Never build JSON with `format!`: `audience` and `subject` are
        // attacker-influenceable strings and a quote would break out of the
        // claim. `serde_json::json!` escapes every value correctly.
        let claims_json = serde_json::json!({
            "aud": audience,
            "exp": exp,
            "sub": self.subject.as_str(),
        })
        .to_string();

        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header_json);
        let claims_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims_json);
        format!("{header_b64}.{claims_b64}")
    }
}

/// Errors that can occur during Web Push operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebPushError {
    /// The provided private key is not a valid 32-byte P-256 secret key.
    InvalidPrivateKey,
    /// The provided public key is not a valid P-256 SEC1 encoded key.
    InvalidPublicKey,
    /// The subscriber's p256dh key could not be decoded.
    InvalidSubscriptionP256dh,
    /// The subscriber's auth secret could not be decoded.
    InvalidSubscriptionAuth,
    /// The payload is too large (RFC 8291 limits to 4078 bytes).
    PayloadTooLarge,
    /// Encryption failed.
    EncryptionFailed,
    /// The HTTP POST to the push endpoint failed.
    DeliveryFailed(String),
    /// The subscription endpoint was rejected by the SSRF guard (bad scheme,
    /// userinfo/fragment, private/loopback/link-local host, unsafe DNS result…).
    UnsafeEndpoint(String),
}

impl std::fmt::Display for WebPushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrivateKey => write!(
                f,
                "Invalid VAPID private key (expected 32-byte P-256 secret)"
            ),
            Self::InvalidPublicKey => write!(
                f,
                "Invalid VAPID public key (expected SEC1-encoded P-256 public)"
            ),
            Self::InvalidSubscriptionP256dh => write!(f, "Invalid subscription p256dh key"),
            Self::InvalidSubscriptionAuth => write!(f, "Invalid subscription auth secret"),
            Self::PayloadTooLarge => write!(
                f,
                "Notification payload exceeds 4078 bytes (RFC 8291 limit)"
            ),
            Self::EncryptionFailed => write!(f, "RFC 8291 aes128gcm encryption failed"),
            Self::DeliveryFailed(msg) => write!(f, "Push delivery failed: {msg}"),
            Self::UnsafeEndpoint(reason) => {
                write!(f, "Unsafe push endpoint rejected: {reason}")
            }
        }
    }
}

impl std::error::Error for WebPushError {}

/// RFC 8291 content-encryption: ECDH key agreement + HKDF-SHA-256 + aes128gcm.
///
/// Encrypts the notification payload for a specific subscriber using:
/// 1. ECDH key agreement between an ephemeral P-256 key and the subscriber's p256dh key.
/// 2. HKDF-SHA-256 to derive the content-encryption key (16 bytes) and nonce (12 bytes).
/// 3. aes128gcm authenticated encryption.
///
/// Returns the complete RFC 8291 encrypted message (binary).
pub fn encrypt_payload(
    subscription: &PushSubscription,
    payload: &[u8],
) -> Result<Vec<u8>, WebPushError> {
    if payload.len() > 4078 {
        return Err(WebPushError::PayloadTooLarge);
    }

    // 1. Decode subscriber's p256dh public key (base64url → SEC1 bytes)
    let p256dh_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(subscription.p256dh.as_bytes())
        .map_err(|_| WebPushError::InvalidSubscriptionP256dh)?;
    let subscriber_pubkey = PublicKey::from_sec1_bytes(&p256dh_bytes)
        .map_err(|_| WebPushError::InvalidSubscriptionP256dh)?;

    // 2. Decode subscriber's auth secret (base64url → 16 bytes)
    let auth_secret = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(subscription.auth.as_bytes())
        .map_err(|_| WebPushError::InvalidSubscriptionAuth)?;
    if auth_secret.len() != 16 {
        return Err(WebPushError::InvalidSubscriptionAuth);
    }

    // 3. Generate ephemeral P-256 key pair
    let ephemeral_secret = EphemeralSecret::random(&mut OsRng);
    let ephemeral_public = ephemeral_secret.public_key();
    let ephemeral_public_bytes = ephemeral_public.to_sec1_bytes();

    // 4. ECDH shared secret
    let shared_secret = ephemeral_secret.diffie_hellman(&subscriber_pubkey);
    let shared_secret_bytes = shared_secret.raw_secret_bytes();

    // 5. RFC 8291 key derivation — two HKDF stages, bound to `salt_random`.
    //
    // Stage 1 mixes the ECDH shared secret with the subscription auth secret:
    //   PRK_key = HKDF-Extract(auth_secret, ecdh_secret)
    //   IKM     = HKDF-Expand(PRK_key, "WebPush: info\0" || ua_public || as_public, 32)
    // `hkdf_sha256_expand(prk, info_prefix, context, len)` builds
    // `info_prefix || context || counter`, so the fixed "WebPush: info\0" label
    // is the prefix and the two public keys (with no label) are the context.
    let mut key_context = Vec::with_capacity(p256dh_bytes.len() + ephemeral_public_bytes.len());
    key_context.extend_from_slice(&p256dh_bytes);
    key_context.extend_from_slice(&ephemeral_public_bytes);

    let prk_key = hkdf_sha256_extract(&auth_secret, shared_secret_bytes);
    let ikm = hkdf_sha256_expand(&prk_key, b"WebPush: info\0", &key_context, 32);

    // The 16-byte salt is generated before the second stage: it is both the
    // HKDF salt and the value written into the header. A fresh salt therefore
    // changes the CEK for every message, which the old code never did.
    let mut salt_random = [0u8; 16];
    OsRng.fill_bytes(&mut salt_random);

    // Stage 2 (RFC 8188 §2): PRK = HKDF-Extract(salt, IKM), then CEK and
    // NONCE are expanded with an EMPTY context (not the WebPush info).
    let prk = hkdf_sha256_extract(&salt_random, &ikm);
    let content_key = hkdf_sha256_expand(&prk, b"Content-Encoding: aes128gcm\0", b"", 16);
    let nonce_bytes = hkdf_sha256_expand(&prk, b"Content-Encoding: nonce\0", b"", 12);

    // 6. aes128gcm encrypt. RFC 8188: the plaintext is the payload followed by
    // the 0x02 "last record" delimiter (the old 0x00,0x00 padding was wrong).
    let cipher =
        Aes128Gcm::new_from_slice(&content_key).map_err(|_| WebPushError::EncryptionFailed)?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut plaintext = Vec::with_capacity(payload.len() + 1);
    plaintext.extend_from_slice(payload);
    plaintext.push(0x02);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| WebPushError::EncryptionFailed)?;

    // 7. Build the RFC 8291 message structure.
    // Header: salt(16) || record_size(4) || key_id_len(1) || key_id(65)
    // Body: ciphertext. `record_size` is the total message length, computed
    // after encryption (the ciphertext length includes the 16-byte GCM tag).
    let key_id_len = ephemeral_public_bytes.len() as u8;
    let header_len = 16 + 4 + 1 + key_id_len as usize;
    let record_size = (header_len + ciphertext.len()) as u32;

    let mut message = Vec::with_capacity(header_len + ciphertext.len());
    message.extend_from_slice(&salt_random);
    message.extend_from_slice(&record_size.to_be_bytes());
    message.push(key_id_len);
    message.extend_from_slice(&ephemeral_public_bytes);
    message.extend_from_slice(&ciphertext);

    Ok(message)
}

/// HKDF-SHA-256 extract step: PRK = HMAC-SHA-256(salt, IKM).
fn hkdf_sha256_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(salt).expect("HMAC accepts any key size");
    mac.update(ikm);
    mac.finalize().into_bytes().into()
}

/// HKDF-SHA-256 expand step.
fn hkdf_sha256_expand(prk: &[u8], info_prefix: &[u8], context: &[u8], length: usize) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut output = Vec::with_capacity(length);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;

    while output.len() < length {
        let mut mac = <HmacSha256 as Mac>::new_from_slice(prk).expect("HMAC accepts any key size");
        mac.update(&prev);
        mac.update(info_prefix);
        mac.update(context);
        mac.update(&[counter]);
        let result = mac.finalize().into_bytes();
        output.extend_from_slice(&result);
        prev = result.to_vec();
        counter += 1;
    }
    output.truncate(length);
    output
}

// ── SSRF guards ─────────────────────────────────────────────────────────────

/// Returns `true` for IPv4 addresses that must never be contacted: loopback,
/// RFC 1918 private ranges, link-local, shared/CGNAT, multicast, unspecified
/// and reserved/broadcast ranges.
fn is_forbidden_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 0                                      // 0.0.0.0/8 (incl. unspecified)
        || o[0] == 10                               // 10.0.0.0/8 private
        || (o[0] == 100 && (o[1] & 0xc0) == 64)    // 100.64.0.0/10 shared (CGNAT)
        || o[0] == 127                              // 127.0.0.0/8 loopback
        || (o[0] == 169 && o[1] == 254)             // 169.254.0.0/16 link-local
        || (o[0] == 172 && (o[1] & 0xf0) == 16)    // 172.16.0.0/12 private
        || (o[0] == 192 && o[1] == 168)             // 192.168.0.0/16 private
        || ((o[0] & 0xf0) == 224)                   // 224.0.0.0/4 multicast
        || ((o[0] & 0xf0) == 240) // 240.0.0.0/4 reserved + broadcast
}

/// Returns `true` for forbidden IPv6 addresses: unspecified, loopback,
/// unique-local, link-local, multicast, and IPv4-mapped/compatible forms that
/// embed a forbidden IPv4 address.
fn is_forbidden_ipv6(ip: Ipv6Addr) -> bool {
    let o = ip.octets();
    let s = ip.segments();
    // :: (unspecified) and ::1 (loopback)
    if o.iter().all(|&b| b == 0) || (o[..15].iter().all(|&b| b == 0) && o[15] == 1) {
        return true;
    }
    // fc00::/7 unique-local, fe80::/10 link-local, ff00::/8 multicast
    if (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfe80 || (s[0] & 0xff00) == 0xff00 {
        return true;
    }
    // IPv4-mapped (::ffff:a.b.c.d) / IPv4-compatible (::a.b.c.d): re-evaluate
    // the embedded IPv4 address when the first 10 bytes are zero.
    if o[..10].iter().all(|&b| b == 0) {
        let v4 = Ipv4Addr::new(o[12], o[13], o[14], o[15]);
        if is_forbidden_ipv4(v4) {
            return true;
        }
    }
    false
}

fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_ipv4(v4),
        IpAddr::V6(v6) => is_forbidden_ipv6(v6),
    }
}

/// Parse and fully validate a client-supplied push endpoint.
///
/// Returns the lowercased host and the effective port on success. Reused by the
/// synchronous guard, the DNS guard and `VapidSigner::extract_audience`.
fn parse_validated_endpoint(endpoint: &str) -> Result<(String, u16), WebPushError> {
    let deny = |reason: &str| WebPushError::UnsafeEndpoint(reason.to_string());

    // WHATWG URL parsing silently strips tabs/newlines, leading/trailing C0
    // controls and spaces. Reject control/quote characters up front so a
    // smuggled payload cannot be normalised into a "safe" URL.
    if endpoint
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '"' | '\'' | '\\'))
    {
        return Err(deny(
            "endpoint contains control/whitespace or quote characters",
        ));
    }

    // Require an explicit https scheme (case-insensitive per RFC 3986).
    if endpoint
        .get(..8)
        .map(|s| !s.eq_ignore_ascii_case("https://"))
        .unwrap_or(true)
    {
        return Err(deny("endpoint must use https://"));
    }

    let url = reqwest::Url::parse(endpoint).map_err(|_| deny("malformed endpoint URL"))?;
    if url.scheme() != "https" {
        return Err(deny("endpoint must use https://"));
    }

    // Reject userinfo. `url.username()` handles `user:pw@`; the raw-authority
    // check also covers `https://@host`, which the parser normalises to an
    // empty username.
    if !url.username().is_empty() || url.password().is_some() {
        return Err(deny("userinfo is not allowed in endpoint"));
    }
    let authority = endpoint
        .get(8..)
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    if authority.contains('@') {
        return Err(deny("userinfo is not allowed in endpoint"));
    }
    if url.fragment().is_some() {
        return Err(deny("fragment is not allowed in endpoint"));
    }

    let host = url.host_str().ok_or_else(|| deny("empty host"))?;
    if host.is_empty() {
        return Err(deny("empty host"));
    }
    // `host_str()` keeps the brackets around IPv6 literals.
    let host_lower = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();

    if host_lower == "localhost"
        || host_lower.ends_with(".localhost")
        || host_lower.ends_with(".internal")
    {
        return Err(deny("localhost/.internal hosts are not allowed"));
    }

    if let Ok(ip) = host_lower.parse::<IpAddr>() {
        if is_forbidden_ip(ip) {
            return Err(deny(
                "private/loopback/link-local IP literal is not allowed",
            ));
        }
    }

    // A resolvable push endpoint is a dotted FQDN. Single-label hostnames
    // (intranet names, `localhost` aliases) are refused; no allowlist exists.
    if !host_lower.contains('.') {
        return Err(deny("host without a dot is not allowed"));
    }

    let port = url.port_or_known_default().unwrap_or(443);
    Ok((host_lower, port))
}

/// Synchronous, network-free validation of a push endpoint's syntax/host.
fn validate_push_endpoint_syntax(endpoint: &str) -> Result<(), WebPushError> {
    parse_validated_endpoint(endpoint).map(|_| ())
}

/// Resolve the endpoint's host and reject any result that lands on a private,
/// loopback, link-local, multicast or unspecified address (IPv4 and IPv6).
/// Returns `(host, validated_socket_addrs)` so the HTTP client can pin the
/// resolved IP addresses and prevent DNS-rebinding TOCTOU attacks.
async fn check_push_endpoint_dns(
    endpoint: &str,
) -> Result<(String, Vec<std::net::SocketAddr>), WebPushError> {
    let (host, port) = parse_validated_endpoint(endpoint)?;
    let addrs = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| WebPushError::UnsafeEndpoint(format!("DNS lookup failed: {e}")))?;
    let mut validated = Vec::new();
    for addr in addrs {
        if is_forbidden_ip(addr.ip()) {
            return Err(WebPushError::UnsafeEndpoint(
                "endpoint resolves to a forbidden address".into(),
            ));
        }
        validated.push(addr);
    }
    if validated.is_empty() {
        return Err(WebPushError::UnsafeEndpoint(
            "endpoint resolved to no address".into(),
        ));
    }
    Ok((host, validated))
}

/// Send a Web Push notification to a subscriber's push endpoint.
///
/// This encrypts the notification payload with RFC 8291 aes128gcm, generates
/// a VAPID ES256 authorization header, and POSTs the encrypted binary to the
/// push service endpoint.
///
/// # Errors
/// Returns `WebPushError::UnsafeEndpoint` if the subscription endpoint fails the
/// SSRF guard (bad scheme, userinfo, private/loopback host or DNS result),
/// `WebPushError::EncryptionFailed` if payload encryption fails, or
/// `WebPushError::DeliveryFailed` if the HTTP POST fails or returns non-2xx.
pub async fn send_notification(
    subscription: &PushSubscription,
    notification: &WebPushNotification,
    vapid: &VapidSigner,
) -> Result<(), WebPushError> {
    // The endpoint is supplied by the browser. Validate it BEFORE signing and
    // sending: `Authorization` is a signed VAPID credential and must never be
    // delivered to a client-chosen internal host (SSRF / cloud metadata).
    validate_push_endpoint_syntax(&subscription.endpoint)?;
    let (pinned_host, pinned_addrs) = check_push_endpoint_dns(&subscription.endpoint).await?;

    let payload = notification.to_json().into_bytes();
    let encrypted = encrypt_payload(subscription, &payload)?;

    let audience = VapidSigner::extract_audience(&subscription.endpoint);
    let auth_header = vapid.create_authorization_header(&audience, 12 * 3600)?;

    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(10));
    builder = builder.resolve_to_addrs(&pinned_host, &pinned_addrs);
    let client = builder
        .build()
        .map_err(|e| WebPushError::DeliveryFailed(e.to_string()))?;
    let resp = client
        .post(&subscription.endpoint)
        .header("Authorization", auth_header)
        .header("TTL", notification.ttl_secs.to_string())
        .header("Content-Encoding", "aes128gcm")
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", encrypted.len().to_string())
        .body(encrypted)
        .send()
        .await
        .map_err(|e| WebPushError::DeliveryFailed(e.to_string()))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(WebPushError::DeliveryFailed(format!(
            "HTTP {status}: {body}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::{ecdsa::VerifyingKey, EncodedPoint};

    fn generate_test_vapid_keypair() -> (String, String) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let public_point = EncodedPoint::from(&verifying_key);
        // p256 defaults to uncompressed (65 bytes including 0x04 tag)
        let public_bytes = public_point.to_bytes();

        let private_bytes = signing_key.to_bytes();
        let private_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(private_bytes.as_slice());
        let public_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&*public_bytes);
        (public_b64, private_b64)
    }

    fn generate_test_subscription_keys() -> (String, String) {
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let public_point = EncodedPoint::from(&verifying_key);
        let public_bytes = public_point.to_bytes();

        let mut auth = [0u8; 16];
        OsRng.fill_bytes(&mut auth);

        let p256dh_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&*public_bytes);
        let auth_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(auth);
        (p256dh_b64, auth_b64)
    }

    #[test]
    fn test_vapid_auth_header_uses_real_es256() {
        let (pub_b64, priv_b64) = generate_test_vapid_keypair();
        let signer = VapidSigner::new("mailto:admin@yav.rs", &pub_b64, &priv_b64);

        let endpoint = "https://fcm.googleapis.com/fcm/send/sub_12345";
        let aud = VapidSigner::extract_audience(endpoint);
        assert_eq!(aud, "https://fcm.googleapis.com");

        let auth_header = signer
            .create_authorization_header(&aud, 3600)
            .expect("ES256 signing must succeed with valid key");

        assert!(auth_header.starts_with("vapid t="));
        assert!(auth_header.contains(", k="));

        // The JWT must have 3 parts: header.claims.signature
        let vapid_part = auth_header.strip_prefix("vapid t=").unwrap();
        let (jwt, _) = vapid_part.split_once(", k=").unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have header.claims.signature");

        // Decode header and verify alg is ES256 (not HS256)
        let header_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .unwrap();
        let header_json: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header_json["alg"], "ES256");
        assert_eq!(header_json["typ"], "JWT");

        // Decode claims
        let claims_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .unwrap();
        let claims_json: serde_json::Value = serde_json::from_slice(&claims_bytes).unwrap();
        assert_eq!(claims_json["aud"], "https://fcm.googleapis.com");
        assert_eq!(claims_json["sub"], "mailto:admin@yav.rs");

        // Signature should be 64 bytes (raw r || s for P-256 ECDSA)
        let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[2])
            .unwrap();
        assert_eq!(
            sig_bytes.len(),
            64,
            "ES256 signature must be 64 bytes (raw r||s), got {}",
            sig_bytes.len()
        );
    }

    #[test]
    fn test_vapid_rejects_invalid_private_key() {
        let signer = VapidSigner::new("mailto:test@yav.rs", "valid_pub_b64", "too_short");
        let result = signer.create_authorization_header("https://example.com", 3600);
        assert_eq!(result, Err(WebPushError::InvalidPrivateKey));
    }

    #[test]
    fn test_web_push_notification_payload() {
        let notif = WebPushNotification::new("Order Shipped", "Your order #1042 is on the way!")
            .icon("/icons/truck.png")
            .badge("/icons/badge.png")
            .tag("order-update")
            .data(serde_json::json!({ "order_id": 1042 }))
            .ttl(3600);

        let json = notif.to_json();
        assert!(json.contains("Order Shipped"));
        assert!(json.contains("order-update"));
        assert!(json.contains("order_id"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_rfc8291_encryption_produces_valid_message() {
        let (_pub_b64, _priv_b64) = generate_test_vapid_keypair();
        let (sub_p256dh, sub_auth) = generate_test_subscription_keys();

        let subscription = PushSubscription::new(
            "https://fcm.googleapis.com/fcm/send/test123",
            sub_p256dh,
            sub_auth,
        );

        let payload = br#"{"title":"Test","body":"Hello"}"#;
        let encrypted = encrypt_payload(&subscription, payload).expect("encryption must succeed");

        // RFC 8291 message structure:
        // salt(16) + record_size(4) + key_id_len(1) + key_id(65 uncompressed) + ciphertext
        assert!(encrypted.len() > 16 + 4 + 1 + 65, "must have full header");

        // record_size field (bytes 16..20) should match total message length
        let record_size =
            u32::from_be_bytes([encrypted[16], encrypted[17], encrypted[18], encrypted[19]]);
        assert_eq!(
            record_size as usize,
            encrypted.len(),
            "record_size must match total message length"
        );

        // key_id_len (byte 20) should be 65 for uncompressed P-256 public key
        assert_eq!(
            encrypted[20], 65,
            "key_id_len must be 65 for uncompressed P-256"
        );

        // Ciphertext is the rest after the header
        let header_len = 16 + 4 + 1 + 65;
        let ciphertext = &encrypted[header_len..];
        // The ciphertext must be longer than the payload (padding + GCM tag)
        assert!(
            ciphertext.len() > payload.len(),
            "ciphertext must include padding and GCM tag"
        );
    }

    #[test]
    fn test_payload_too_large_rejected() {
        let (sub_p256dh, sub_auth) = generate_test_subscription_keys();
        let subscription = PushSubscription::new("https://example.com", sub_p256dh, sub_auth);

        let huge = vec![b'x'; 5000];
        let result = encrypt_payload(&subscription, &huge);
        assert_eq!(result, Err(WebPushError::PayloadTooLarge));
    }

    #[test]
    fn test_vapid_signing_input_format() {
        let signer = VapidSigner::new("mailto:test@yav.rs", "dummy_pub", "dummy_priv");
        let signing_input = signer.create_es256_signing_input("https://example.com", 3600);

        // Must be base64url(header).base64url(claims)
        let parts: Vec<&str> = signing_input.split('.').collect();
        assert_eq!(parts.len(), 2);

        // Decode header
        let header_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[0])
            .unwrap();
        let header_json: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header_json["alg"], "ES256");
    }
}
