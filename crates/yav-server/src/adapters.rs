//! # YAV Pluggable Infrastructure Adapters (`yav::adapters`)
//!
//! Trait-based driver abstractions allowing applications to seamlessly swap
//! between Yav's zero-dependency in-memory/local engines and distributed
//! production backends (PostgreSQL/SQLite via SQLx, Redis, AWS S3 / Cloudflare
//! R2, SMTP / Resend / Postmark, and OpenAI / Anthropic / Gemini).
//!
//! # Honest defaults
//!
//! [`ServiceAdapters::default()`](ServiceAdapters::default) ships real in-memory
//! cache/storage/mail helpers but **test-only mocks** for `db`/`llm` that return
//! explicit errors. It never provides a silently-working fake database or LLM driver.

use std::collections::HashMap;
use std::sync::Arc;
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
    ///
    /// The default is `false` (unhealthy) rather than a fabricated `true`: an
    /// implementation must override this only after a real connectivity probe.
    fn health_check(&self) -> bool {
        false
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
// Real in-memory reference implementations + explicit test MOCKS
// ─────────────────────────────────────────────────────────────────────────────

/// Test-only database MOCK: no database driver, no SQL execution, no network access.
///
/// Every method returns an explicit error instead of a fake `Ok`, so a misconfigured
/// [`ServiceAdapters::default`] can never pretend a statement ran. Intended for unit
/// tests and offline scaffolding only — wire a real `DatabaseAdapter` via
/// [`ServiceAdapters::with_db`] for anything else.
#[derive(Clone, Default)]
pub struct MockDatabaseAdapter {
    store: EmbeddedStore,
}

impl MockDatabaseAdapter {
    pub fn new() -> Self {
        Self {
            store: EmbeddedStore::new(),
        }
    }

    /// The (always empty) embedded store kept for API compatibility.
    /// No SQL is ever written to it by this mock.
    pub fn store(&self) -> &EmbeddedStore {
        &self.store
    }
}

impl DatabaseAdapter for MockDatabaseAdapter {
    fn execute(&self, _sql: &str, _params: &[String]) -> Result<u64, String> {
        Err("MockDatabaseAdapter: no real database driver configured; SQL was NOT executed".into())
    }

    fn query_rows(
        &self,
        _sql: &str,
        _params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        Err("MockDatabaseAdapter: no real database driver configured; query was NOT run".into())
    }

    /// Always unhealthy: there is no real database to connect to.
    fn health_check(&self) -> bool {
        false
    }
}

/// Deprecated compatibility name for [`MockDatabaseAdapter`].
///
/// The old name suggested a real "embedded database" driver; the type is actually a
/// test mock that never executes SQL. Use [`MockDatabaseAdapter`] instead.
#[deprecated(
    note = "renamed to MockDatabaseAdapter: test mock, no real database driver, no SQL execution"
)]
pub type EmbeddedDatabaseAdapter = MockDatabaseAdapter;

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
            headers: axum::http::HeaderMap::new(),
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

/// Test-only LLM MOCK: no provider, no inference, no network access.
///
/// Every method returns an explicit error instead of fabricating a completion or an
/// embedding, so callers can never mistake it for a working LLM driver.
#[derive(Clone, Default)]
pub struct MockLlmAdapter;

impl LlmAdapter for MockLlmAdapter {
    fn complete(&self, _model: &str, _messages: &[ChatMessage]) -> Result<String, String> {
        Err("MockLlmAdapter: no real LLM provider configured; no completion was generated".into())
    }

    fn embed(&self, _model: &str, _text: &str) -> Result<Vec<f32>, String> {
        Err("MockLlmAdapter: no real LLM provider configured; no embedding was computed".into())
    }
}

/// Deprecated compatibility name for [`MockLlmAdapter`].
///
/// The old name suggested a real "local LLM"; the type is actually a test mock that
/// never contacts any model. Use [`MockLlmAdapter`] instead.
#[deprecated(note = "renamed to MockLlmAdapter: test mock, no real LLM provider, no inference")]
pub type LocalLlmAdapter = MockLlmAdapter;

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
///
/// [`ServiceAdapters::default`] is a **development/test** configuration: `cache`, `storage`
/// and `mail` are real in-memory implementations, while `db` and `llm` are explicit mocks
/// that always fail loudly. Supply real drivers with [`ServiceAdapters::with_db`] /
/// [`ServiceAdapters::with_llm`] before using those slots.
#[derive(Clone)]
pub struct ServiceAdapters {
    pub db: Arc<dyn DatabaseAdapter>,
    pub cache: Arc<dyn CacheAdapter>,
    pub storage: Arc<dyn StorageAdapter>,
    pub llm: Arc<dyn LlmAdapter>,
    pub mail: Arc<dyn MailAdapter>,
}

impl Default for ServiceAdapters {
    /// Development defaults. The `db`/`llm` slots hold mocks that return explicit
    /// errors rather than silently faking success; no real database or LLM is configured.
    fn default() -> Self {
        Self {
            db: Arc::new(MockDatabaseAdapter::new()),
            cache: Arc::new(MemoryCacheAdapter::default()),
            storage: Arc::new(MemoryStorageAdapter::default()),
            llm: Arc::new(MockLlmAdapter),
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

        // Honest defaults: the db/llm mocks never pretend to have done real work.
        assert!(!services.db.health_check());
        let exec = services
            .db
            .execute("INSERT INTO t VALUES ($1)", &["a".into()]);
        assert!(exec.is_err(), "mock must not claim SQL was executed");
        assert!(exec.unwrap_err().contains("no real database driver"));
        assert!(services.db.query_rows("SELECT * FROM t", &[]).is_err());

        let llm_reply = services
            .llm
            .complete("gpt-4o", &[ChatMessage::user("Hello Yav")]);
        assert!(llm_reply.is_err(), "mock must not fabricate a completion");

        // These are real in-memory implementations and keep working as documented.
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

        let msg = EmailMessage::new()
            .from("no-reply@yav.rs")
            .to("dev@yav.rs")
            .subject("Welcome");
        // In-memory development transport (documented in `email.rs`), not SMTP delivery.
        assert!(services.mail.send_email(msg).is_ok());
    }

    #[test]
    fn test_mocks_never_claim_sql_execution() {
        let db = MockDatabaseAdapter::new();
        let err = db
            .execute("DELETE FROM users", &[])
            .expect_err("mock must not return Ok(1)");
        assert!(err.contains("NOT executed"));
        assert!(db.query_rows("SELECT 1", &[]).is_err());
        assert!(!db.health_check());

        // Nothing was written: the `sql_log` collection must remain empty.
        let logged: Vec<String> = db.store().find_all("sql_log").unwrap();
        assert!(logged.is_empty(), "mock must not log/pretend to run SQL");

        let llm = MockLlmAdapter;
        assert!(llm
            .complete("m", &[])
            .unwrap_err()
            .contains("no real LLM provider"));
        assert!(llm
            .embed("m", "text")
            .unwrap_err()
            .contains("no real LLM provider"));
    }
}
