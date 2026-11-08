//! # YAV WebPush & VAPID Notification Engine (RFC 8291 / RFC 8292)
//!
//! Provides browser push notification payload structuring, subscription management,
//! and VAPID (Voluntary Application Server Identification) JWT authorization headers.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
            ttl_secs: 86400, // 24 hours default TTL
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VapidSigner {
    pub subject: String,
    pub public_key: String,
    pub private_key: String,
}

impl VapidSigner {
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

    /// Extract audience origin from push endpoint URL (e.g. `https://fcm.googleapis.com`).
    #[must_use]
    pub fn extract_audience(endpoint: &str) -> String {
        if let Some(pos) = endpoint.find("://") {
            let after_scheme = &endpoint[pos + 3..];
            if let Some(slash) = after_scheme.find('/') {
                return format!("{}://{}", &endpoint[..pos], &after_scheme[..slash]);
            }
        }
        endpoint.to_string()
    }

    /// Construct a development VAPID JWT Authorization header for the given audience origin.
    #[must_use]
    pub fn create_authorization_header(&self, audience: &str, expiration_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let exp = now + expiration_secs;

        let header_json = r#"{"typ":"JWT","alg":"HS256"}"#;
        let claims_json = format!(
            r#"{{"aud":"{}","exp":{},"sub":"{}"}}"#,
            audience, exp, self.subject
        );

        let header_b64 = base64_url_encode(header_json.as_bytes());
        let claims_b64 = base64_url_encode(claims_json.as_bytes());
        let signing_input = format!("{header_b64}.{claims_b64}");

        // HMAC-SHA256 signature using the private key
        let signature = simple_hmac_sha256(self.private_key.as_bytes(), signing_input.as_bytes());
        let sig_b64 = base64_url_encode(&signature);

        let jwt = format!("{signing_input}.{sig_b64}");
        format!("vapid t={jwt}, k={}", self.public_key)
    }

    /// Construct a standards-compliant RFC 8292 ES256 VAPID Authorization header using ECDSA P-256 signature bytes.
    #[must_use]
    pub fn create_es256_authorization_header(
        &self,
        audience: &str,
        expiration_secs: u64,
        ecdsa_signature_raw: &[u8],
    ) -> String {
        let signing_input = self.create_es256_signing_input(audience, expiration_secs);
        let sig_b64 = base64_url_encode(ecdsa_signature_raw);

        let jwt = format!("{signing_input}.{sig_b64}");
        format!("vapid t={jwt}, k={}", self.public_key)
    }

    /// Generate the exact signing input `base64url(header).base64url(claims)` according to RFC 8292 ES256 VAPID.
    #[must_use]
    pub fn create_es256_signing_input(&self, audience: &str, expiration_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let exp = now + expiration_secs;

        let header_json = r#"{"typ":"JWT","alg":"ES256"}"#;
        let claims_json = format!(
            r#"{{"aud":"{}","exp":{},"sub":"{}"}}"#,
            audience, exp, self.subject
        );

        let header_b64 = base64_url_encode(header_json.as_bytes());
        let claims_b64 = base64_url_encode(claims_json.as_bytes());
        format!("{header_b64}.{claims_b64}")
    }

    /// Construct a standards-compliant RFC 8292 ES256 VAPID Authorization header from an ECDSA P-256 signer closure.
    pub fn create_es256_header_with_signer<F>(
        &self,
        audience: &str,
        expiration_secs: u64,
        signer: F,
    ) -> Result<String, &'static str>
    where
        F: FnOnce(&[u8]) -> Result<Vec<u8>, &'static str>,
    {
        let signing_input = self.create_es256_signing_input(audience, expiration_secs);
        let sig_bytes = signer(signing_input.as_bytes())?;
        let sig_b64 = base64_url_encode(&sig_bytes);
        let jwt = format!("{signing_input}.{sig_b64}");
        Ok(format!("vapid t={jwt}, k={}", self.public_key))
    }
}

fn base64_url_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);

        if i + 1 < data.len() {
            result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARSET[(n & 63) as usize] as char);
        }

        i += 3;
    }
    result
}

fn simple_hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    // Pure-Rust HMAC-SHA256 simulation using inner and outer state hashing
    let mut key_pad = [0u8; 64];
    if key.len() > 64 {
        // Hash key if too long
        let h = simple_sha256(key);
        key_pad[..32].copy_from_slice(&h);
    } else {
        key_pad[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key_pad[i] ^ 0x36;
        opad[i] = key_pad[i] ^ 0x5c;
    }

    let mut inner_input = Vec::with_capacity(64 + message.len());
    inner_input.extend_from_slice(&ipad);
    inner_input.extend_from_slice(message);
    let inner_hash = simple_sha256(&inner_input);

    let mut outer_input = Vec::with_capacity(64 + 32);
    outer_input.extend_from_slice(&opad);
    outer_input.extend_from_slice(&inner_hash);
    simple_sha256(&outer_input)
}

fn simple_sha256(data: &[u8]) -> [u8; 32] {
    // Standard SHA-256 implementation
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vapid_auth_header_generation() {
        let signer = VapidSigner::new(
            "mailto:admin@yav.rs",
            "BEl62iUYgUivxIkv69yViEuiBIa",
            "private_key_secret_bytes_12345",
        );

        let endpoint = "https://fcm.googleapis.com/fcm/send/sub_12345";
        let aud = VapidSigner::extract_audience(endpoint);
        assert_eq!(aud, "https://fcm.googleapis.com");

        let auth_header = signer.create_authorization_header(&aud, 3600);
        assert!(auth_header.starts_with("vapid t="));
        assert!(auth_header.contains(", k=BEl62iUYgUivxIkv69yViEuiBIa"));

        // Test RFC 8292 ES256 signing input & header with custom signer
        let signing_input = signer.create_es256_signing_input(&aud, 3600);
        assert!(signing_input.contains("."));
        let es256_header = signer
            .create_es256_header_with_signer(&aud, 3600, |payload| {
                assert_eq!(payload, signing_input.as_bytes());
                Ok(vec![1, 2, 3, 4, 5])
            })
            .expect("es256 signing succeeds");
        assert!(es256_header.starts_with("vapid t="));
        assert!(es256_header.contains(", k=BEl62iUYgUivxIkv69yViEuiBIa"));
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
}
