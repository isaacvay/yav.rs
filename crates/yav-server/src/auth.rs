//! Complete Authentication, Authorization, JWT, OAuth2/PKCE, Password Hashing, and 2FA (TOTP) suite.
//!
//! Provides an integrated Rust equivalent to Auth.js / Better-Auth / Clerk:
//! - [`PasswordHasher`]: Salted PBKDF2-HMAC-SHA256 password hashing and constant-time verification.
//! - [`JwtManager`] & [`JwtClaims`]: Stateless HMAC-SHA256 (`HS256`) JSON Web Token signing and verification.
//! - [`OAuthConfig`] & [`OAuthProvider`]: OAuth2 authorization URL builder with PKCE (`S256` code challenge).
//! - [`Totp`]: Multi-Factor Authentication (TOTP RFC 6238) code generator, verifier, and `otpauth://` URI builder.
//! - [`AuthUser`]: Role-Based Access Control (RBAC) identity guard (`has_role`, `require_role`).

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

// ─────────────────────────────────────────────────────────────────────────────
// Base64URL & Hex Helpers (Zero external crate overhead)
// ─────────────────────────────────────────────────────────────────────────────

const B64_URL_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Encode bytes as URL-safe Base64 without padding (RFC 7515 / JWT / PKCE standard).
pub fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(B64_URL_TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[(n & 0x3F) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(B64_URL_TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[((n >> 12) & 0x3F) as usize] as char);
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(B64_URL_TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_URL_TABLE[((n >> 6) & 0x3F) as usize] as char);
    }
    out
}

fn b64url_val(b: u8) -> Option<u32> {
    match b {
        b'A'..=b'Z' => Some((b - b'A') as u32),
        b'a'..=b'z' => Some((b - b'a' + 26) as u32),
        b'0'..=b'9' => Some((b - b'0' + 52) as u32),
        b'-' | b'+' => Some(62),
        b'_' | b'/' => Some(63),
        _ => None,
    }
}

/// Decode URL-safe Base64 (without or with `=` padding).
pub fn base64url_decode(input: &str) -> Result<Vec<u8>, String> {
    let clean = input.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= clean.len() {
        let a = b64url_val(clean[i]).ok_or("Invalid base64url byte")?;
        let b = b64url_val(clean[i + 1]).ok_or("Invalid base64url byte")?;
        let c = b64url_val(clean[i + 2]).ok_or("Invalid base64url byte")?;
        let d = b64url_val(clean[i + 3]).ok_or("Invalid base64url byte")?;
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push(((n >> 16) & 0xFF) as u8);
        out.push(((n >> 8) & 0xFF) as u8);
        out.push((n & 0xFF) as u8);
        i += 4;
    }
    let rem = clean.len() - i;
    if rem == 2 {
        let a = b64url_val(clean[i]).ok_or("Invalid base64url byte")?;
        let b = b64url_val(clean[i + 1]).ok_or("Invalid base64url byte")?;
        let n = (a << 18) | (b << 12);
        out.push(((n >> 16) & 0xFF) as u8);
    } else if rem == 3 {
        let a = b64url_val(clean[i]).ok_or("Invalid base64url byte")?;
        let b = b64url_val(clean[i + 1]).ok_or("Invalid base64url byte")?;
        let c = b64url_val(clean[i + 2]).ok_or("Invalid base64url byte")?;
        let n = (a << 18) | (b << 12) | (c << 6);
        out.push(((n >> 16) & 0xFF) as u8);
        out.push(((n >> 8) & 0xFF) as u8);
    } else if rem == 1 {
        return Err("Invalid base64url length".to_string());
    }
    Ok(out)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Odd hex length".to_string());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string())?;
        out.push(byte);
    }
    Ok(out)
}

use subtle::ConstantTimeEq;

/// Constant-time byte slice equality check to prevent timing side-channel attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. PasswordHasher (PBKDF2-HMAC-SHA256)
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::atomic::{AtomicU64, Ordering};

static SALT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Cryptographically secure 16-byte salt generator seeded from OS CSPRNG entropy.
pub fn generate_crypto_salt_16() -> [u8; 16] {
    let mut salt = [0u8; 16];
    if getrandom::getrandom(&mut salt).is_ok() {
        return salt;
    }

    let seq = SALT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let mut hasher = Sha256::new();
    hasher.update(nanos.to_le_bytes());
    hasher.update(seq.to_le_bytes());
    hasher.update(pid.to_le_bytes());

    let digest = hasher.finalize();
    salt.copy_from_slice(&digest[..16]);
    salt
}

/// Default PBKDF2 iteration count (100,000 iterations for high security).
pub const DEFAULT_PBKDF2_ITERATIONS: u32 = 100_000;

/// Salted PBKDF2-HMAC-SHA256 password hasher and verifier.
///
/// Produces PHC-style password hashes: `$yav-pbkdf2$i=<iterations>$<salt_hex>$<dk_hex>`.
#[derive(Debug, Clone)]
pub struct PasswordHasher {
    iterations: u32,
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_PBKDF2_ITERATIONS,
        }
    }
}

impl PasswordHasher {
    /// Create a password hasher with default iteration count (`100,000`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a password hasher with a custom PBKDF2 iteration count.
    pub fn with_iterations(iterations: u32) -> Self {
        Self {
            iterations: iterations.max(1_000),
        }
    }

    /// Hash a plaintext password with an automatically generated 16-byte cryptographic salt.
    pub fn hash_password(&self, password: &str) -> String {
        let salt = generate_crypto_salt_16();
        self.hash_with_salt(password, &salt)
    }

    /// Hash a plaintext password with an explicit salt slice.
    pub fn hash_with_salt(&self, password: &str, salt: &[u8]) -> String {
        let dk = pbkdf2_hmac_sha256(password.as_bytes(), salt, self.iterations);
        format!(
            "$yav-pbkdf2$i={}${}${}",
            self.iterations,
            hex_encode(salt),
            hex_encode(&dk)
        )
    }

    /// Verify a plaintext password against a stored `$yav-pbkdf2$...` hash in constant time.
    pub fn verify_password(password: &str, encoded_hash: &str) -> bool {
        let parts: Vec<&str> = encoded_hash.split('$').collect();
        // ["", "yav-pbkdf2", "i=10000", "<salt_hex>", "<dk_hex>"]
        if parts.len() != 5 || parts[1] != "yav-pbkdf2" {
            return false;
        }
        let Some(iter_str) = parts[2].strip_prefix("i=") else {
            return false;
        };
        let Ok(iterations) = iter_str.parse::<u32>() else {
            return false;
        };
        if !(1_000..=1_000_000).contains(&iterations) {
            return false;
        }
        let Ok(salt) = hex_decode(parts[3]) else {
            return false;
        };
        let Ok(expected_dk) = hex_decode(parts[4]) else {
            return false;
        };

        let computed = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations);
        constant_time_eq(&computed, &expected_dk)
    }
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    // PBKDF2 single block (32-byte SHA-256 output, block index = 1 BE)
    let mut prf = HmacSha256::new_from_slice(password).expect("HMAC accepts any key size");
    prf.update(salt);
    prf.update(&1u32.to_be_bytes());
    let mut u = prf.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&u);

    for _ in 1..iterations {
        let mut mac = HmacSha256::new_from_slice(password).expect("HMAC accepts any key size");
        mac.update(&u);
        u = mac.finalize().into_bytes();
        for (o, &b) in out.iter_mut().zip(u.iter()) {
            *o ^= b;
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. JwtManager & JwtClaims (HS256 Stateless JWT)
// ─────────────────────────────────────────────────────────────────────────────

/// Standard and custom JSON Web Token claims.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    /// Subject (user ID or email).
    pub sub: String,
    /// Expiration timestamp (Unix epoch seconds).
    pub exp: u64,
    /// Issued-at timestamp (Unix epoch seconds).
    pub iat: u64,
    /// Assigned RBAC roles (e.g. `["admin", "editor"]`).
    #[serde(default)]
    pub roles: Vec<String>,
    /// Additional custom string claims.
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

impl JwtClaims {
    /// Create new JWT claims for `sub` valid for `ttl_secs` from now.
    pub fn new(sub: impl Into<String>, ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            sub: sub.into(),
            iat: now,
            exp: now.saturating_add(ttl_secs),
            roles: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Attach a role to these claims.
    #[must_use]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Attach a custom claim key/value pair.
    #[must_use]
    pub fn with_claim(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// Returns `true` if the token has expired at the current wall-clock time.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now > self.exp
    }
}

/// Stateless HMAC-SHA256 (`HS256`) JSON Web Token manager.
#[derive(Debug, Clone)]
pub struct JwtManager {
    secret: Vec<u8>,
}

impl JwtManager {
    /// Create a JWT manager using the provided secret key.
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    /// Sign [`JwtClaims`] into a compact `header.payload.signature` JWT string.
    pub fn sign(&self, claims: &JwtClaims) -> Result<String, String> {
        let header_b64 = base64url_encode(br#"{"alg":"HS256","typ":"JWT"}"#);
        let payload_json = serde_json::to_vec(claims).map_err(|e| e.to_string())?;
        let payload_b64 = base64url_encode(&payload_json);

        let signing_input = format!("{header_b64}.{payload_b64}");
        let sig = self.compute_signature(signing_input.as_bytes());
        let sig_b64 = base64url_encode(&sig);

        Ok(format!("{signing_input}.{sig_b64}"))
    }

    /// Verify a compact JWT string's signature and expiration, returning the decoded [`JwtClaims`].
    pub fn verify(&self, token: &str) -> Result<JwtClaims, String> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid JWT structure: expected 3 segments".to_string());
        }

        let header_bytes = base64url_decode(parts[0])?;
        #[derive(Deserialize)]
        struct JwtHeader {
            alg: String,
        }
        let header: JwtHeader = serde_json::from_slice(&header_bytes)
            .map_err(|e| format!("Invalid JWT header: {e}"))?;
        if header.alg != "HS256" {
            return Err(format!("Unsupported JWT algorithm: {}", header.alg));
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = self.compute_signature(signing_input.as_bytes());
        let provided_sig = base64url_decode(parts[2])?;

        if !constant_time_eq(&expected_sig, &provided_sig) {
            return Err("Invalid JWT signature".to_string());
        }

        let payload_bytes = base64url_decode(parts[1])?;
        let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
            .map_err(|e| format!("Invalid JWT claims: {e}"))?;

        if claims.is_expired() {
            return Err("JWT has expired".to_string());
        }
        Ok(claims)
    }

    fn compute_signature(&self, data: &[u8]) -> [u8; 32] {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key size");
        mac.update(data);
        let bytes = mac.finalize().into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. OAuth2 + PKCE Provider Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Built-in OAuth2 / OIDC providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthProvider {
    GitHub,
    Google,
    Discord,
    Custom {
        authorize_url: String,
        token_url: String,
    },
}

impl OAuthProvider {
    /// Return the provider's OAuth2 authorization endpoint URL.
    pub fn authorize_url(&self) -> &str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/authorize",
            Self::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Self::Discord => "https://discord.com/api/oauth2/authorize",
            Self::Custom { authorize_url, .. } => authorize_url,
        }
    }

    /// Return the provider's OAuth2 token exchange endpoint URL.
    pub fn token_url(&self) -> &str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/access_token",
            Self::Google => "https://oauth2.googleapis.com/token",
            Self::Discord => "https://discord.com/api/oauth2/token",
            Self::Custom { token_url, .. } => token_url,
        }
    }
}

/// OAuth2 client configuration with PKCE (`S256`) support.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub provider: OAuthProvider,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OAuthConfig {
    /// Create a GitHub OAuth2 configuration.
    pub fn github(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            provider: OAuthProvider::GitHub,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            scopes: vec!["read:user".to_string(), "user:email".to_string()],
        }
    }

    /// Create a Google OAuth2 / OIDC configuration.
    pub fn google(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            provider: OAuthProvider::Google,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
        }
    }

    /// Create a Discord OAuth2 configuration.
    pub fn discord(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            provider: OAuthProvider::Discord,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            scopes: vec!["identify".to_string(), "email".to_string()],
        }
    }

    /// Generate a PKCE `(code_verifier, code_challenge)` pair using SHA-256 (`S256`).
    pub fn generate_pkce_pair(seed: &str) -> (String, String) {
        let mut verifier_hasher = Sha256::new();
        verifier_hasher.update(seed.as_bytes());
        verifier_hasher.update(b"yav-pkce-verifier");
        let verifier = base64url_encode(&verifier_hasher.finalize());

        let mut challenge_hasher = Sha256::new();
        challenge_hasher.update(verifier.as_bytes());
        let challenge = base64url_encode(&challenge_hasher.finalize());
        (verifier, challenge)
    }

    /// Construct the full OAuth2 authorization redirect URL with CSRF `state` and optional PKCE `code_challenge`.
    pub fn authorization_url(&self, state: &str, pkce_code_challenge: Option<&str>) -> String {
        let scope_str = self.scopes.join(" ");
        let mut url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.provider.authorize_url(),
            url_encode(&self.client_id),
            url_encode(&self.redirect_uri),
            url_encode(&scope_str),
            url_encode(state),
        );
        if let Some(challenge) = pkce_code_challenge {
            url.push_str(&format!(
                "&code_challenge={}&code_challenge_method=S256",
                url_encode(challenge)
            ));
        }
        url
    }
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(&mut out, "%{:02X}", b);
            }
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. TOTP (RFC 6238 Two-Factor Authentication)
// ─────────────────────────────────────────────────────────────────────────────

/// Time-based One-Time Password (TOTP RFC 6238) generator and verifier.
#[derive(Debug, Clone)]
pub struct Totp {
    secret: Vec<u8>,
    digits: u32,
    period: u64,
}

const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Encode bytes into RFC 4648 Base32 string without padding.
pub fn base32_encode(data: &[u8]) -> String {
    let mut s = String::new();
    let mut buffer: u64 = 0;
    let mut bits_left = 0;
    for &byte in data {
        buffer = (buffer << 8) | (byte as u64);
        bits_left += 8;
        while bits_left >= 5 {
            bits_left -= 5;
            let index = ((buffer >> bits_left) & 0x1F) as usize;
            s.push(BASE32_ALPHABET[index] as char);
        }
    }
    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1F) as usize;
        s.push(BASE32_ALPHABET[index] as char);
    }
    s
}

impl Totp {
    /// Create a 6-digit, 30-second TOTP instance from a shared secret.
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
            digits: 6,
            period: 30,
        }
    }

    /// Generate the current 6-digit TOTP code at the current system timestamp.
    pub fn generate_now(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.generate_at(now)
    }

    /// Generate the TOTP code for a specific Unix timestamp (in seconds).
    pub fn generate_at(&self, unix_seconds: u64) -> String {
        let counter = unix_seconds / self.period;
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts any key size");
        mac.update(&counter.to_be_bytes());
        let hash = mac.finalize().into_bytes();

        let offset = (hash[hash.len() - 1] & 0x0f) as usize;
        let binary = (((hash[offset] & 0x7f) as u32) << 24)
            | ((hash[offset + 1] as u32) << 16)
            | ((hash[offset + 2] as u32) << 8)
            | (hash[offset + 3] as u32);

        let modulo = 10u32.pow(self.digits);
        let code = binary % modulo;
        format!("{:0width$}", code, width = self.digits as usize)
    }

    /// Verify a TOTP code at `unix_seconds`, allowing a `+-1` time-step drift window.
    pub fn verify_at(&self, code: &str, unix_seconds: u64) -> bool {
        for step in [-1i64, 0, 1] {
            let ts = (unix_seconds as i64 + step * (self.period as i64)).max(0) as u64;
            let expected = self.generate_at(ts);
            if constant_time_eq(code.as_bytes(), expected.as_bytes()) {
                return true;
            }
        }
        false
    }

    /// Format an `otpauth://totp/...` URI for QR code authenticator enrollment.
    pub fn otpauth_uri(&self, issuer: &str, account: &str) -> String {
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA256&digits={}&period={}",
            url_encode(issuer),
            url_encode(account),
            base32_encode(&self.secret),
            url_encode(issuer),
            self.digits,
            self.period
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Authenticated User & Role-Based Access Control (RBAC)
// ─────────────────────────────────────────────────────────────────────────────

/// Represents an authenticated user principal with Role-Based Access Control (RBAC) helpers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
}

impl AuthUser {
    /// Create a new authenticated user principal.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            email: None,
            roles: Vec::new(),
        }
    }

    /// Construct an [`AuthUser`] from verified [`JwtClaims`].
    pub fn from_claims(claims: &JwtClaims) -> Self {
        Self {
            id: claims.sub.clone(),
            email: claims.extra.get("email").cloned(),
            roles: claims.roles.clone(),
        }
    }

    /// Check whether this user has the specified role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check whether this user holds any of the specified roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }

    /// Require a specific role, returning `Err(StatusCode::FORBIDDEN)` if missing.
    pub fn require_role(&self, role: &str) -> Result<(), axum::http::StatusCode> {
        if self.has_role(role) {
            Ok(())
        } else {
            Err(axum::http::StatusCode::FORBIDDEN)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hasher_roundtrip() {
        let hasher = PasswordHasher::with_iterations(2_000);
        let hashed = hasher.hash_password("CorrectHorseBatteryStaple!");
        assert!(hashed.starts_with("$yav-pbkdf2$i=2000$"));
        assert!(PasswordHasher::verify_password(
            "CorrectHorseBatteryStaple!",
            &hashed
        ));
        assert!(!PasswordHasher::verify_password("WrongPassword", &hashed));
    }

    #[test]
    fn test_jwt_sign_verify_and_rbac() {
        let jwt = JwtManager::new("super-secret-jwt-key-2026");
        let claims = JwtClaims::new("user_42", 3600)
            .with_role("admin")
            .with_claim("email", "alice@example.com");

        let token = jwt.sign(&claims).unwrap();
        let verified = jwt.verify(&token).unwrap();
        assert_eq!(verified.sub, "user_42");
        assert_eq!(verified.roles, vec!["admin".to_string()]);

        let user = AuthUser::from_claims(&verified);
        assert!(user.has_role("admin"));
        assert_eq!(user.email.as_deref(), Some("alice@example.com"));
        assert!(user.require_role("admin").is_ok());
        assert_eq!(
            user.require_role("superadmin"),
            Err(axum::http::StatusCode::FORBIDDEN)
        );
    }

    #[test]
    fn test_oauth_pkce_and_totp() {
        let cfg = OAuthConfig::github("client_123", "secret_456", "https://app.dev/callback");
        let (verifier, challenge) = OAuthConfig::generate_pkce_pair("random-session-seed");
        assert!(!verifier.is_empty());
        assert_ne!(verifier, challenge);

        let url = cfg.authorization_url("csrf_state_99", Some(&challenge));
        assert!(url.contains("client_id=client_123"));
        assert!(url.contains("code_challenge_method=S256"));

        let totp = Totp::new(b"12345678901234567890");
        let code = totp.generate_at(1_700_000_000);
        assert_eq!(code.len(), 6);
        assert!(totp.verify_at(&code, 1_700_000_015));
        assert!(!totp.verify_at("000000", 1_700_000_000));

        let uri = totp.otpauth_uri("Yav", "user@example.com");
        assert!(uri.starts_with("otpauth://totp/Yav:user%40example.com"));
        assert!(uri.contains("algorithm=SHA256"));
        assert!(uri.contains("secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"));
    }
}
