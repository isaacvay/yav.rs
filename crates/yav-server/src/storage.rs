//! # YAV Object Storage & Presigned Upload URLs
//!
//! S3/R2-compatible object storage abstraction and HMAC-SHA256 presigned
//! upload/download URLs (rivaling Uploadthing, AWS S3 Presigned URLs, and
//! Cloudflare R2).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::auth::constant_time_eq;

type HmacSha256 = Hmac<Sha256>;

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

/// Metadata describing a stored object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub key: String,
    pub size: usize,
    pub content_type: String,
    pub sha256: String,
    pub uploaded_at: u64,
}

/// Thread-safe object storage supporting in-memory objects and prefix listing.
#[derive(Debug, Clone, Default)]
pub struct ObjectStorage {
    #[allow(clippy::type_complexity)]
    objects: Arc<RwLock<HashMap<String, (ObjectMeta, Vec<u8>)>>>,
}

impl ObjectStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an object under `key` with `content_type` and compute its SHA-256 digest.
    pub fn put(
        &self,
        key: impl Into<String>,
        content_type: impl Into<String>,
        bytes: Vec<u8>,
    ) -> ObjectMeta {
        let key_str = key.into();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = to_hex(&hasher.finalize());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let meta = ObjectMeta {
            key: key_str.clone(),
            size: bytes.len(),
            content_type: content_type.into(),
            sha256,
            uploaded_at: now,
        };

        if let Ok(mut guard) = self.objects.write() {
            guard.insert(key_str, (meta.clone(), bytes));
        }
        meta
    }

    /// Retrieve an object's metadata and raw payload by `key`.
    pub fn get(&self, key: &str) -> Option<(ObjectMeta, Vec<u8>)> {
        self.objects.read().ok()?.get(key).cloned()
    }

    /// Delete an object by `key`. Returns `true` if it existed.
    pub fn delete(&self, key: &str) -> bool {
        self.objects
            .write()
            .ok()
            .and_then(|mut g| g.remove(key))
            .is_some()
    }

    /// List all objects matching `prefix`, sorted by key.
    pub fn list_prefix(&self, prefix: &str) -> Vec<ObjectMeta> {
        let mut list: Vec<ObjectMeta> = self
            .objects
            .read()
            .map(|g| {
                g.iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .map(|(_, (meta, _))| meta.clone())
                    .collect()
            })
            .unwrap_or_default();
        list.sort_by(|a, b| a.key.cmp(&b.key));
        list
    }
}

/// Cryptographic presigned URL generator and verifier for direct client uploads/downloads.
#[derive(Debug, Clone)]
pub struct PresignedUrlSigner {
    secret: Vec<u8>,
    base_url: String,
}

impl PresignedUrlSigner {
    pub fn new(secret: impl Into<Vec<u8>>, base_url: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    fn compute_sig(
        &self,
        method: &str,
        key: &str,
        content_type: &str,
        max_bytes: usize,
        expires: u64,
    ) -> String {
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC accepts keys of any length");
        let canonical = format!("{method}\n{key}\n{content_type}\n{max_bytes}\n{expires}");
        mac.update(canonical.as_bytes());
        to_hex(&mac.finalize().into_bytes())
    }

    /// Generate a time-limited, size-bounded, MIME-typed presigned `PUT` upload URL.
    pub fn sign_put_url_at(
        &self,
        key: &str,
        content_type: &str,
        max_bytes: usize,
        expires_at: u64,
    ) -> String {
        let sig = self.compute_sig("PUT", key, content_type, max_bytes, expires_at);
        let clean_key = key.trim_start_matches('/');
        format!(
            "{}/{}?method=PUT&ct={}&max={}&exp={}&sig={}",
            self.base_url, clean_key, content_type, max_bytes, expires_at, sig
        )
    }

    /// Verify a presigned URL's signature, expiration, MIME type, and payload size.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_put_at(
        &self,
        key: &str,
        content_type: &str,
        actual_bytes: usize,
        max_bytes: usize,
        expires_at: u64,
        signature: &str,
        now_epoch_secs: u64,
    ) -> Result<(), String> {
        if now_epoch_secs > expires_at {
            return Err("presigned URL has expired".to_string());
        }
        if actual_bytes > max_bytes {
            return Err(format!(
                "upload size {} exceeds maximum allowed {} bytes",
                actual_bytes, max_bytes
            ));
        }
        let expected = self.compute_sig("PUT", key, content_type, max_bytes, expires_at);
        if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
            return Err("invalid presigned URL signature".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_storage_and_presigned_urls() {
        let storage = ObjectStorage::new();
        let meta = storage.put("avatars/u1.png", "image/png", vec![1, 2, 3, 4]);
        assert_eq!(meta.size, 4);
        assert_eq!(storage.list_prefix("avatars/").len(), 1);

        let signer = PresignedUrlSigner::new(b"s3_secret_key", "https://cdn.yav.rs");
        let url = signer.sign_put_url_at("avatars/u1.png", "image/png", 10_000, 2_000_000_000);
        let sig = url.split("sig=").nth(1).unwrap();

        assert!(signer
            .verify_put_at(
                "avatars/u1.png",
                "image/png",
                4096,
                10_000,
                2_000_000_000,
                sig,
                1_900_000_000
            )
            .is_ok());

        // Oversized upload rejected:
        assert!(signer
            .verify_put_at(
                "avatars/u1.png",
                "image/png",
                25_000,
                10_000,
                2_000_000_000,
                sig,
                1_900_000_000
            )
            .is_err());
    }
}
