//! # YAV Pluggable Infrastructure Adapters (`yav::adapters`)
//!
//! Trait-based driver abstractions allowing applications to seamlessly swap
//! between Yav's zero-dependency in-memory/local engines and distributed
//! production backends (PostgreSQL/SQLite via SQLx, Redis, AWS S3 / Cloudflare
//! R2, SMTP / Resend / Postmark, and OpenAI / Anthropic / Gemini).

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Instant;

use axum::body::Bytes;
use axum::http::StatusCode;

use crate::ai::ChatMessage;
use crate::cache::{CacheEntry, SsrCache};
use crate::db::EmbeddedStore;
use crate::email::{EmailMessage, Mailer};
use crate::storage::{ObjectMeta, ObjectStorage, PresignedUrlSigner};

/// Pluggable SQL / NoSQL database driver trait (implementable for Postgres, SQLite, MySQL, Turso).
pub trait DatabaseAdapter: Send + Sync {
    /// Execute a parameterized statement (`INSERT`, `UPDATE`, `DELETE`, `DDL`) and return affected rows.
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String>;

    /// Query rows as key-value maps for a parameterized `SELECT` query.
    fn query_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String>;

    /// Verify connectivity to the underlying database.
    fn health_check(&self) -> bool {
        true
    }
}

/// Pluggable distributed cache backend trait (implementable for Redis, Valkey, Memcached, CDN).
pub trait CacheAdapter: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: String, ttl_secs: u64, tags: Vec<String>);
    fn invalidate_key(&self, key: &str) -> bool;
    fn invalidate_tag(&self, tag: &str) -> usize;
}

/// Pluggable object storage backend trait (implementable for AWS S3, Cloudflare R2, MinIO, GCS).
pub trait StorageAdapter: Send + Sync {
    fn put_object(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<ObjectMeta, String>;
    fn get_object(&self, key: &str) -> Option<(ObjectMeta, Vec<u8>)>;
    fn delete_object(&self, key: &str) -> bool;
    fn presign_put(
        &self,
        key: &str,
        content_type: &str,
        max_bytes: usize,
        expires_at: u64,
    ) -> String;
}

/// Pluggable LLM & Embedding provider trait (implementable for OpenAI, Anthropic, Gemini, Ollama).
pub trait LlmAdapter: Send + Sync {
    fn complete(&self, model: &str, messages: &[ChatMessage]) -> Result<String, String>;
    fn embed(&self, model: &str, text: &str) -> Result<Vec<f32>, String>;
}

/// Pluggable transactional email provider trait (implementable for SMTP, Resend, Postmark, SES).
pub trait MailAdapter: Send + Sync {
    fn send_email(&self, message: EmailMessage) -> Result<String, String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Zero-Config Reference Implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Default embedded database adapter backed by [`EmbeddedStore`].
#[derive(Clone, Default)]
pub struct EmbeddedDatabaseAdapter {
    store: EmbeddedStore,
    seq: Arc<AtomicUsize>,
}

impl EmbeddedDatabaseAdapter {
    pub fn new() -> Self {
        Self {
            store: EmbeddedStore::new(),
            seq: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn store(&self) -> &EmbeddedStore {
        &self.store
    }
}

impl DatabaseAdapter for EmbeddedDatabaseAdapter {
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        let id = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let payload = format!("{sql} | {params:?}");
        self.store.insert("sql_log", &id.to_string(), &payload)?;
        Ok(1)
    }

    fn query_rows(
        &self,
        _sql: &str,
        _params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        Ok(Vec::new())
    }
}

/// Default cache adapter backed by Yav's bounded LRU [`SsrCache`].
#[derive(Clone)]
pub struct MemoryCacheAdapter {
    cache: SsrCache,
}

impl Default for MemoryCacheAdapter {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl MemoryCacheAdapter {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: SsrCache::with_capacity(max_entries),
        }
    }
}

impl CacheAdapter for MemoryCacheAdapter {
    fn get(&self, key: &str) -> Option<String> {
        self.cache
            .get(key)
            .map(|e| String::from_utf8_lossy(&e.body).into_owned())
    }

    fn set(&self, key: &str, value: String, _ttl_secs: u64, tags: Vec<String>) {
        let entry = CacheEntry {
            status: StatusCode::OK,
            content_type: "text/html; charset=utf-8".to_string(),
            body: Bytes::from(value),
            created_at: Instant::now(),
            revalidating: false,
        };
        self.cache.insert_with_tags(key, entry, tags);
    }

    fn invalidate_key(&self, key: &str) -> bool {
        self.cache.invalidate(key)
    }

    fn invalidate_tag(&self, tag: &str) -> usize {
        self.cache.revalidate_tag(tag)
    }
}

/// Default storage adapter backed by [`ObjectStorage`] and [`PresignedUrlSigner`].
#[derive(Clone)]
pub struct MemoryStorageAdapter {
    storage: ObjectStorage,
    signer: PresignedUrlSigner,
}

impl Default for MemoryStorageAdapter {
    fn default() -> Self {
        Self::new(b"yav-default-storage-secret", "https://storage.local")
    }
}

impl MemoryStorageAdapter {
    pub fn new(secret: &[u8], base_url: &str) -> Self {
        Self {
            storage: ObjectStorage::new(),
            signer: PresignedUrlSigner::new(secret.to_vec(), base_url),
        }
    }
}

impl StorageAdapter for MemoryStorageAdapter {
    fn put_object(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<ObjectMeta, String> {
        Ok(self.storage.put(key, content_type, bytes))
    }

    fn get_object(&self, key: &str) -> Option<(ObjectMeta, Vec<u8>)> {
        self.storage.get(key)
    }

    fn delete_object(&self, key: &str) -> bool {
        self.storage.delete(key)
    }

    fn presign_put(
        &self,
        key: &str,
        content_type: &str,
        max_bytes: usize,
        expires_at: u64,
    ) -> String {
        self.signer
            .sign_put_url_at(key, content_type, max_bytes, expires_at)
    }
}

/// Deterministic local LLM adapter for development and testing.
#[derive(Clone, Default)]
pub struct LocalLlmAdapter;

impl LlmAdapter for LocalLlmAdapter {
    fn complete(&self, model: &str, messages: &[ChatMessage]) -> Result<String, String> {
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        Ok(format!("[{model}] Processed: {last}"))
    }

    fn embed(&self, _model: &str, text: &str) -> Result<Vec<f32>, String> {
        let b = text.as_bytes();
        let v0 = b.first().copied().unwrap_or(1) as f32 / 255.0;
        let v1 = b.get(1).copied().unwrap_or(1) as f32 / 255.0;
        let v2 = (b.len() as f32).min(255.0) / 255.0;
        Ok(vec![v0, v1, v2])
    }
}

/// Default mail adapter backed by [`Mailer`].
#[derive(Clone, Default)]
pub struct MemoryMailAdapter {
    mailer: Mailer,
}

impl MemoryMailAdapter {
    pub fn new() -> Self {
        Self {
            mailer: Mailer::new(),
        }
    }

    pub fn outbox(&self) -> Vec<EmailMessage> {
        self.mailer.sent_messages()
    }
}

impl MailAdapter for MemoryMailAdapter {
    fn send_email(&self, message: EmailMessage) -> Result<String, String> {
        self.mailer.send(message)?;
        Ok(format!("msg_{}", self.mailer.sent_count()))
    }
}

/// Unified container holding all pluggable infrastructure adapters for injection into [`crate::YavApp`].
#[derive(Clone)]
pub struct ServiceAdapters {
    pub db: Arc<dyn DatabaseAdapter>,
    pub cache: Arc<dyn CacheAdapter>,
    pub storage: Arc<dyn StorageAdapter>,
    pub llm: Arc<dyn LlmAdapter>,
    pub mail: Arc<dyn MailAdapter>,
}

impl Default for ServiceAdapters {
    fn default() -> Self {
        Self {
            db: Arc::new(EmbeddedDatabaseAdapter::new()),
            cache: Arc::new(MemoryCacheAdapter::default()),
            storage: Arc::new(MemoryStorageAdapter::default()),
            llm: Arc::new(LocalLlmAdapter),
            mail: Arc::new(MemoryMailAdapter::new()),
        }
    }
}

impl ServiceAdapters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_db(mut self, db: Arc<dyn DatabaseAdapter>) -> Self {
        self.db = db;
        self
    }

    pub fn with_cache(mut self, cache: Arc<dyn CacheAdapter>) -> Self {
        self.cache = cache;
        self
    }

    pub fn with_storage(mut self, storage: Arc<dyn StorageAdapter>) -> Self {
        self.storage = storage;
        self
    }

    pub fn with_llm(mut self, llm: Arc<dyn LlmAdapter>) -> Self {
        self.llm = llm;
        self
    }

    pub fn with_mail(mut self, mail: Arc<dyn MailAdapter>) -> Self {
        self.mail = mail;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_adapters_end_to_end() {
        let services = ServiceAdapters::new();
        assert!(services.db.health_check());
        assert_eq!(
            services
                .db
                .execute("INSERT INTO t VALUES ($1)", &["a".into()]),
            Ok(1)
        );

        services
            .cache
            .set("/home", "<h1>Cached</h1>".into(), 60, vec!["home".into()]);
        assert_eq!(
            services.cache.get("/home").as_deref(),
            Some("<h1>Cached</h1>")
        );
        assert_eq!(services.cache.invalidate_tag("home"), 1);

        let meta = services
            .storage
            .put_object("docs/spec.pdf", "application/pdf", vec![1, 2, 3])
            .unwrap();
        assert_eq!(meta.size, 3);

        let reply = services
            .llm
            .complete("gpt-4o", &[ChatMessage::user("Hello Yav")])
            .unwrap();
        assert!(reply.contains("Hello Yav"));

        let msg = EmailMessage::new()
            .from("no-reply@yav.rs")
            .to("dev@yav.rs")
            .subject("Welcome");
        assert!(services.mail.send_email(msg).is_ok());
    }
}
