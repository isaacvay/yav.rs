//! Real object storage via `object_store` (feature `object-storage`).
//!
//! The crate is built with `default-features = false`, so backend selection
//! (local FS, S3, R2, GCS, ...) belongs to the caller. This adapter accepts any
//! `Arc<dyn ObjectStore>` and adds a key prefix on top of it. It never invents
//! an object or a presigned URL: failed reads return `None` and presigning is
//! rejected explicitly because the core `ObjectStore` trait has no signer.

#![cfg(feature = "object-storage")]

use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::adapters::StorageAdapter;
use crate::storage::ObjectMeta;

/// Join a configured prefix and a caller key into one object-store location.
fn prefixed_key(prefix: &str, key: &str) -> String {
    // Normalise both sides: callers may pass a prefix from configuration with
    // leading/trailing slashes, and object-store keys must not contain "//".
    let prefix = prefix.trim_matches('/');
    let key = key.trim_start_matches('/');
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}/{key}")
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Best-effort content type inferred from the object key.
///
/// `object_store::ObjectMeta` deliberately does not carry a content type, so
/// this is the only information available on reads. Callers that need exact
/// metadata should store it alongside the object.
fn content_type_from_key(key: &str) -> &'static str {
    let lower = key.to_ascii_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if lower.ends_with(".txt") || lower.ends_with(".md") {
        "text/plain; charset=utf-8"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else {
        "application/octet-stream"
    }
}

/// Storage adapter over an `object_store` backend.
pub struct ObjectStoreAdapter {
    store: Arc<dyn object_store::ObjectStore>,
    base_url: String,
    prefix: String,
}

impl ObjectStoreAdapter {
    /// Wrap an already-constructed backend.
    ///
    /// Callers build the concrete store themselves (e.g. `LocalFileSystem`,
    /// `AmazonS3`, `InMemory`) and enable the matching `object_store` feature
    /// in their own crate. `base_url` is retained for diagnostics only; no URL
    /// is fabricated for object access.
    pub fn new(
        store: Arc<dyn object_store::ObjectStore>,
        base_url: impl Into<String>,
        prefix: impl Into<String>,
    ) -> Result<Self, String> {
        Ok(Self {
            store,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            prefix: prefix.into().trim_matches('/').to_string(),
        })
    }

    /// URL-based construction is intentionally unsupported here.
    ///
    /// `object_store::parse_url` requires a cloud feature (`aws`, `gcp`,
    /// `azure`, `http`, or `fs`) that this crate does not enable (its
    /// dependency is `default-features = false`). Build the store in the
    /// calling crate and pass it to [`Self::new`] instead.
    pub fn from_url(_url: &str, _prefix: &str) -> Result<Self, String> {
        Err(
            "ObjectStoreAdapter::from_url requires an object_store backend feature \
             (aws/gcp/azure/http/fs) that is not enabled; construct an ObjectStore \
             yourself and call ObjectStoreAdapter::new(store, base_url, prefix)"
                .to_string(),
        )
    }

    /// The configured base URL (for diagnostics / external link building).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The configured key prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Native async object write helper safe inside any Axum handler.
    pub async fn put_object_async(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<ObjectMeta, String> {
        use object_store::{path::Path, ObjectStoreExt, PutPayload};

        let full_key = prefixed_key(&self.prefix, key);
        let size = bytes.len();
        let sha256 = hex_digest(&bytes);
        let uploaded_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let payload = PutPayload::from(bytes);
        self.store
            .put(&Path::from(full_key.clone()), payload)
            .await
            .map_err(|e| format!("object store put failed for '{full_key}': {e}"))?;

        Ok(ObjectMeta {
            key: full_key,
            size,
            content_type: content_type.to_string(),
            sha256,
            uploaded_at,
        })
    }

    /// Native async object read helper safe inside any Axum handler.
    pub async fn get_object_async(&self, key: &str) -> Option<(ObjectMeta, Vec<u8>)> {
        use object_store::{path::Path, ObjectStoreExt};

        let full_key = prefixed_key(&self.prefix, key);
        let result = self.store.get(&Path::from(full_key.clone())).await.ok()?;
        let meta = result.meta.clone();
        let bytes = result.bytes().await.ok()?.to_vec();

        let uploaded_at = u64::try_from(meta.last_modified.timestamp()).unwrap_or(0);
        let content_type = content_type_from_key(&full_key).to_string();
        let object_meta = ObjectMeta {
            key: full_key,
            size: bytes.len(),
            content_type,
            sha256: hex_digest(&bytes),
            uploaded_at,
        };
        Some((object_meta, bytes))
    }

    /// Native async object delete helper safe inside any Axum handler.
    pub async fn delete_object_async(&self, key: &str) -> bool {
        use object_store::{path::Path, ObjectStoreExt};

        let full_key = prefixed_key(&self.prefix, key);
        self.store.delete(&Path::from(full_key)).await.is_ok()
    }
}

impl StorageAdapter for ObjectStoreAdapter {
    fn put_object(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<ObjectMeta, String> {
        crate::roadmap::block_on_shared(self.put_object_async(key, content_type, bytes))?
    }

    fn get_object(&self, key: &str) -> Option<(ObjectMeta, Vec<u8>)> {
        crate::roadmap::block_on_shared(self.get_object_async(key))
            .ok()
            .flatten()
    }

    fn delete_object(&self, key: &str) -> bool {
        crate::roadmap::block_on_shared(self.delete_object_async(key)).unwrap_or(false)
    }

    fn presign_put(
        &self,
        _key: &str,
        _content_type: &str,
        _max_bytes: usize,
        _expires_at: u64,
    ) -> String {
        // The trait returns a plain String, so an error must be spelled out
        // rather than smuggled in as a plausible-looking URL.
        "error: presigned PUT is not supported by ObjectStoreAdapter; the object_store \
         core trait has no signer. Use a backend-specific presigning API and return \
         a real signed URL from the caller."
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefixed_key() {
        assert_eq!(prefixed_key("", "a/b.txt"), "a/b.txt");
        assert_eq!(prefixed_key("uploads", "a/b.txt"), "uploads/a/b.txt");
        assert_eq!(prefixed_key("uploads", "/a/b.txt"), "uploads/a/b.txt");
        assert_eq!(prefixed_key("/uploads/", "/a"), "uploads/a");
    }

    #[test]
    fn test_from_url_is_explicitly_unsupported() {
        let err = match ObjectStoreAdapter::from_url("s3://bucket", "uploads") {
            Ok(_) => panic!("from_url must fail without cloud features"),
            Err(err) => err,
        };
        assert!(err.contains("not enabled"));
        assert!(err.contains("new(store"));
    }
}
