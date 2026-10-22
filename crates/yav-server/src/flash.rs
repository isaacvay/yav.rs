//! Flash messages system for full-stack request/response cycles.
//!
//! Provides cookie-backed one-time notification messages that persist across HTTP redirects
//! and are consumed by the subsequent page request.

use crate::session::{
    compute_hmac_sha256, constant_time_eq, is_production_mode, SessionSecret,
    DEFAULT_DEV_SESSION_SECRET,
};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Cookie name used to store serialized flash messages.
pub const FLASH_COOKIE_NAME: &str = "_yav_flash";

/// Severity level for flash messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A single flash message payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashMessage {
    pub level: FlashLevel,
    pub message: String,
}

impl FlashMessage {
    /// Create a new flash message with severity level.
    pub fn new(level: FlashLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
        }
    }
}

/// Resolve the HMAC secret key for flash cookie signing and verification.
fn resolve_flash_secret(custom: Option<&[u8]>) -> Vec<u8> {
    if let Some(secret) = custom {
        return secret.to_vec();
    }
    std::env::var("SESSION_SECRET")
        .or_else(|_| std::env::var("YAV_SECRET"))
        .ok()
        .map(|s| s.into_bytes())
        .unwrap_or_else(|| {
            if is_production_mode() {
                panic!("FATAL: SESSION_SECRET must be explicitly set when running in production mode! Refusing to sign flash cookies with default dev secret.");
            }
            DEFAULT_DEV_SESSION_SECRET.to_vec()
        })
}

/// Axum extractor and manager for cryptographically signed flash messages.
#[derive(Debug, Clone, Default)]
pub struct Flash {
    incoming: Vec<FlashMessage>,
    outgoing: Arc<Mutex<Vec<FlashMessage>>>,
    secret: Option<Vec<u8>>,
}

impl Flash {
    /// Create an empty flash manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an empty flash manager with an explicit HMAC signing secret.
    pub fn with_secret(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            incoming: Vec::new(),
            outgoing: Arc::new(Mutex::new(Vec::new())),
            secret: Some(secret.into()),
        }
    }

    /// Messages delivered by the previous request.
    pub fn incoming(&self) -> &[FlashMessage] {
        &self.incoming
    }

    /// Add a success flash message for the next request.
    pub fn success(&self, message: impl Into<String>) {
        self.push(FlashLevel::Success, message);
    }

    /// Add an error flash message for the next request.
    pub fn error(&self, message: impl Into<String>) {
        self.push(FlashLevel::Error, message);
    }

    /// Add a warning flash message for the next request.
    pub fn warning(&self, message: impl Into<String>) {
        self.push(FlashLevel::Warning, message);
    }

    /// Add an info flash message for the next request.
    pub fn info(&self, message: impl Into<String>) {
        self.push(FlashLevel::Info, message);
    }

    fn push(&self, level: FlashLevel, message: impl Into<String>) {
        if let Ok(mut lock) = self.outgoing.lock() {
            lock.push(FlashMessage::new(level, message));
        }
    }

    /// Generate the HMAC-SHA256 signed `Set-Cookie` header value to update or clear flash cookies.
    pub fn cookie_header(&self) -> Option<HeaderValue> {
        let secret = resolve_flash_secret(self.secret.as_deref());
        self.cookie_header_with_secret(&secret)
    }

    /// Generate the HMAC-SHA256 signed `Set-Cookie` header value with an explicit signing secret.
    pub fn cookie_header_with_secret(&self, secret: &[u8]) -> Option<HeaderValue> {
        let outgoing = self.outgoing.lock().ok()?.clone();
        let secure_flag = if is_production_mode() { "; Secure" } else { "" };
        if !outgoing.is_empty() {
            if let Ok(json) = serde_json::to_string(&outgoing) {
                let encoded = urlencoding_simple(&json);
                let signature = compute_hmac_sha256(encoded.as_bytes(), secret);
                let val = format!(
                    "{FLASH_COOKIE_NAME}={encoded}.{signature}; Path=/; HttpOnly; SameSite=Lax{secure_flag}"
                );
                return HeaderValue::from_str(&val).ok();
            }
        } else if !self.incoming.is_empty() {
            // Clear the consumed cookie
            let val = format!(
                "{FLASH_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax{secure_flag}"
            );
            return HeaderValue::from_str(&val).ok();
        }
        None
    }

    /// Attach this flash manager's `Set-Cookie` header to an existing Axum [`Response`].
    pub fn attach_to_response(&self, mut response: Response) -> Response {
        if let Some(cookie) = self.cookie_header() {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        response
    }
}

fn urlencoding_simple(s: &str) -> String {
    // Simple hex encoding or standard escape
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn urldecoding_simple(s: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chars = s.chars();
    let mut buf = [0u8; 4];
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next()?;
            let h2 = chars.next()?;
            let hex_str = format!("{}{}", h1, h2);
            let byte = u8::from_str_radix(&hex_str, 16).ok()?;
            bytes.push(byte);
        } else {
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }
    String::from_utf8(bytes).ok()
}

impl<S> FromRequestParts<S> for Flash
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ext_secret = parts.extensions.get::<SessionSecret>().map(|s| s.0.clone());
        let secret = resolve_flash_secret(ext_secret.as_deref());

        let mut incoming = Vec::new();
        if let Some(cookie_hdr) = parts.headers.get(header::COOKIE) {
            if let Ok(cookie_str) = cookie_hdr.to_str() {
                for pair in cookie_str.split(';') {
                    let mut parts_iter = pair.trim().splitn(2, '=');
                    if let (Some(name), Some(val)) = (parts_iter.next(), parts_iter.next()) {
                        if name == FLASH_COOKIE_NAME {
                            if let Some((payload, sig)) = val.rsplit_once('.') {
                                let expected_sig = compute_hmac_sha256(payload.as_bytes(), &secret);
                                if constant_time_eq(sig.as_bytes(), expected_sig.as_bytes()) {
                                    if let Some(decoded) = urldecoding_simple(payload) {
                                        if let Ok(msgs) =
                                            serde_json::from_str::<Vec<FlashMessage>>(&decoded)
                                        {
                                            incoming = msgs;
                                        }
                                    }
                                } else {
                                    tracing::warn!(
                                        "Rejected flash cookie with invalid HMAC-SHA256 signature"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Flash {
            incoming,
            outgoing: Arc::new(Mutex::new(Vec::new())),
            secret: Some(secret),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_push_and_cookie() {
        let flash = Flash::new();
        assert!(flash.incoming().is_empty());

        flash.success("Opération réussie !");
        let hdr = flash.cookie_header().expect("Expected Set-Cookie header");
        let val = hdr.to_str().unwrap();
        assert!(val.contains(FLASH_COOKIE_NAME));
        assert!(val.contains("Path=/"));
    }

    #[tokio::test]
    async fn test_flash_hmac_verification_and_rejection() {
        use axum::http::Request;

        let secret = b"flash_test_secret_32_bytes_long!";
        let flash = Flash::with_secret(secret.to_vec());
        flash.success("Signed message");
        let cookie_hdr = flash.cookie_header().unwrap();
        let cookie_pair = cookie_hdr.to_str().unwrap().split(';').next().unwrap();

        // 1. Valid signed cookie is accepted
        let req = Request::builder()
            .header(header::COOKIE, cookie_pair)
            .extension(SessionSecret::new(secret.to_vec()))
            .body(())
            .unwrap();
        let (mut parts, _) = req.into_parts();
        let extracted = Flash::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(extracted.incoming().len(), 1);
        assert_eq!(extracted.incoming()[0].message, "Signed message");

        // 2. Unsigned or tampered cookie is rejected
        let raw_payload = urlencoding_simple(r#"[{"level":"error","message":"Forged!"}]"#);
        let forged_cookie = format!(
            "{FLASH_COOKIE_NAME}={raw_payload}.0000000000000000000000000000000000000000000000000000000000000000"
        );
        let req_forged = Request::builder()
            .header(header::COOKIE, forged_cookie)
            .extension(SessionSecret::new(secret.to_vec()))
            .body(())
            .unwrap();
        let (mut parts_forged, _) = req_forged.into_parts();
        let extracted_forged = Flash::from_request_parts(&mut parts_forged, &())
            .await
            .unwrap();
        assert!(extracted_forged.incoming().is_empty());
    }

    #[test]
    fn test_urlencode_urldecode() {
        let original = r#"[{"level":"success","message":"Bonjour le monde !"}]"#;
        let encoded = urlencoding_simple(original);
        let decoded = urldecoding_simple(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_unicode_accents_and_emojis() {
        let original = "Élément supprimé avec succès ! 🚀 À bientôt à l'aérogare";
        let encoded = urlencoding_simple(original);
        let decoded = urldecoding_simple(&encoded).unwrap();
        assert_eq!(original, decoded);
    }
}
