//! Real SQL and tagged-cache infrastructure integrations for the roadmap.
//!
//! Everything in this module is honest: no method fabricates a successful
//! query, a healthy connection, or a cached value. Each implementation is
//! compiled only when its Cargo feature is enabled.
//!
//! - `sqlite` / `postgres`: `SqliteDatabaseAdapter` / `PostgresDatabaseAdapter`
//!   execute real SQL through a pooled `sqlx` connection.
//! - Always available: `Migrator` extension methods that run discovered SQL
//!   migrations through any `DatabaseAdapter` and journal the result.
//! - `tagged-cache`: `MokaCacheAdapter` with per-entry TTL and tag indexes.

use std::collections::HashMap;
#[cfg(feature = "tagged-cache")]
use std::collections::HashSet;
#[cfg(feature = "tagged-cache")]
use std::sync::RwLock;

use crate::adapters::{CacheAdapter, DatabaseAdapter};
use crate::db::{AppliedMigration, Migrator};

/// Current Unix time in seconds (never panics; falls back to 0 on clock error).
fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Split a SQL section into individual statements while respecting `'...'` string
/// literals, `"..."` quoted identifiers, `--` line comments and `/* ... */` blocks.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < chars.len() {
        let ch = chars[i];
        let next = chars.get(i + 1).copied();

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                current.push('\n');
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if in_single {
            current.push(ch);
            if ch == '\'' {
                if next == Some('\'') {
                    current.push('\'');
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            current.push(ch);
            if ch == '"' {
                if next == Some('"') {
                    current.push('"');
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }

        if ch == '-' && next == Some('-') {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if ch == '/' && next == Some('*') {
            in_block_comment = true;
            i += 2;
            continue;
        }
        if ch == '\'' {
            in_single = true;
            current.push(ch);
            i += 1;
            continue;
        }
        if ch == '"' {
            in_double = true;
            current.push(ch);
            i += 1;
            continue;
        }
        if ch == ';' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
            i += 1;
            continue;
        }
        current.push(ch);
        i += 1;
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }
    statements
}

impl Migrator {
    /// Apply every pending migration through a real [`DatabaseAdapter`].
    ///
    /// For each discovered migration:
    /// 1. an already-applied migration with a different checksum is an error;
    /// 2. the `-- Up` section is split into statements and executed in order;
    /// 3. immediately after every statement in a migration succeeds, the migration is journaled.
    ///
    /// Returns the versions that were newly applied.
    pub fn apply_all_with(&self, adapter: &dyn DatabaseAdapter) -> Result<Vec<String>, String> {
        let discovered = self.discover()?;
        let mut applied = self.load_applied()?;
        let mut newly_applied = Vec::new();

        for migration in discovered {
            if let Some(existing) = applied.iter().find(|a| a.version == migration.version) {
                if existing.checksum != migration.checksum {
                    return Err(format!(
                        "Checksum mismatch for already-applied migration '{}'",
                        migration.version
                    ));
                }
                continue;
            }

            for statement in split_sql_statements(&migration.up_sql) {
                adapter.execute(&statement, &[])?;
            }

            applied.push(AppliedMigration {
                version: migration.version.clone(),
                checksum: migration.checksum.clone(),
                applied_at_epoch: now_epoch(),
            });
            self.record_applied(&applied)?;
            newly_applied.push(migration.version);
        }

        Ok(newly_applied)
    }

    /// Roll back the most recently applied migration through a [`DatabaseAdapter`].
    ///
    /// Named `*_with_adapter` because `Migrator::rollback_last_with` already
    /// exists in `db.rs` for closure-based executors. The checksum of the
    /// journaled entry is verified against the migration on disk before any
    /// `-- Down` statement is executed.
    pub fn rollback_last_with_adapter(
        &self,
        adapter: &dyn DatabaseAdapter,
    ) -> Result<Option<String>, String> {
        let discovered = self.discover()?;
        let mut applied = self.load_applied()?;
        let Some(last) = applied.pop() else {
            return Ok(None);
        };

        if let Some(migration) = discovered.into_iter().find(|d| d.version == last.version) {
            if migration.checksum != last.checksum {
                return Err(format!(
                    "Checksum mismatch for migration '{}' during rollback",
                    last.version
                ));
            }
            for statement in split_sql_statements(&migration.down_sql) {
                adapter.execute(&statement, &[])?;
            }
        }

        self.record_applied(&applied)?;
        Ok(Some(last.version))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SQLx database adapters
// ─────────────────────────────────────────────────────────────────────────────

/// Real SQLite driver backed by a `sqlx` connection pool.
#[cfg(feature = "sqlite")]
pub struct SqliteDatabaseAdapter {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteDatabaseAdapter {
    /// Open a pooled SQLite connection asynchronously (safe inside any Axum/Tokio task).
    pub async fn connect_async(url: &str) -> Result<Self, String> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(url)
            .await
            .map_err(|e| format!("failed to connect to SQLite at '{url}': {e}"))?;
        Ok(Self { pool })
    }

    /// Open a pooled SQLite connection synchronously (safe both outside and inside Tokio).
    pub fn connect(url: &str) -> Result<Self, String> {
        let owned = url.to_string();
        crate::roadmap::block_on_shared(Self::connect_async(&owned))?
    }

    /// Asynchronously execute a parameterized statement.
    pub async fn execute_async(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        for param in params {
            query = query.bind(param);
        }
        query
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
            .map_err(|e| format!("SQLite execute failed: {e}"))
    }

    /// Asynchronously run a parameterized query returning rows as string maps.
    pub async fn query_rows_async(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        for param in params {
            query = query.bind(param);
        }
        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("SQLite query failed: {e}"))?;
        use sqlx::{Column, Row};
        let mut collected = Vec::with_capacity(rows.len());
        for row in rows {
            let mut record = HashMap::new();
            for column in row.columns() {
                let name = column.name();
                let value: Option<String> = row
                    .try_get(name)
                    .map_err(|e| format!("SQLite query failed: {e}"))?;
                record.insert(name.to_string(), value.unwrap_or_default());
            }
            collected.push(record);
        }
        Ok(collected)
    }
}

#[cfg(feature = "sqlite")]
impl DatabaseAdapter for SqliteDatabaseAdapter {
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        crate::roadmap::block_on_shared(self.execute_async(sql, params))?
    }

    fn query_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        crate::roadmap::block_on_shared(self.query_rows_async(sql, params))?
    }

    fn health_check(&self) -> bool {
        self.execute("SELECT 1", &[]).is_ok()
    }
}

/// Real PostgreSQL driver backed by a `sqlx` connection pool.
#[cfg(feature = "postgres")]
pub struct PostgresDatabaseAdapter {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresDatabaseAdapter {
    /// Open a pooled PostgreSQL connection asynchronously.
    pub async fn connect_async(url: &str) -> Result<Self, String> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(url)
            .await
            .map_err(|e| format!("failed to connect to PostgreSQL: {e}"))?;
        Ok(Self { pool })
    }

    /// Open a pooled PostgreSQL connection synchronously (safe both outside and inside Tokio).
    pub fn connect(url: &str) -> Result<Self, String> {
        let owned = url.to_string();
        crate::roadmap::block_on_shared(Self::connect_async(&owned))?
    }

    /// Asynchronously execute a parameterized statement.
    pub async fn execute_async(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        for param in params {
            query = query.bind(param);
        }
        query
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
            .map_err(|e| format!("PostgreSQL execute failed: {e}"))
    }

    /// Asynchronously run a parameterized query returning rows as string maps.
    pub async fn query_rows_async(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
        for param in params {
            query = query.bind(param);
        }
        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("PostgreSQL query failed: {e}"))?;
        use sqlx::{Column, Row};
        let mut collected = Vec::with_capacity(rows.len());
        for row in rows {
            let mut record = HashMap::new();
            for column in row.columns() {
                let name = column.name();
                let value: Option<String> = row
                    .try_get(name)
                    .map_err(|e| format!("PostgreSQL query failed: {e}"))?;
                record.insert(name.to_string(), value.unwrap_or_default());
            }
            collected.push(record);
        }
        Ok(collected)
    }
}

#[cfg(feature = "postgres")]
impl DatabaseAdapter for PostgresDatabaseAdapter {
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        crate::roadmap::block_on_shared(self.execute_async(sql, params))?
    }

    fn query_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        crate::roadmap::block_on_shared(self.query_rows_async(sql, params))?
    }

    fn health_check(&self) -> bool {
        self.execute("SELECT 1", &[]).is_ok()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Moka tagged cache (feature = "tagged-cache")
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "tagged-cache")]
use std::time::{Duration, Instant};

#[cfg(feature = "tagged-cache")]
use moka::Expiry;

/// Cache entry carrying its own (optional) time-to-live.
#[cfg(feature = "tagged-cache")]
#[derive(Clone, Debug)]
struct TaggedValue {
    value: String,
    ttl: Option<Duration>,
}

/// Per-entry expiration policy: honour each entry's TTL instead of relying on
/// a single global `time_to_live`. Reads do **not** extend the lifetime.
#[cfg(feature = "tagged-cache")]
#[derive(Clone, Default)]
struct ValueTtlExpiry;

#[cfg(feature = "tagged-cache")]
impl Expiry<String, TaggedValue> for ValueTtlExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &TaggedValue,
        _created_at: Instant,
    ) -> Option<Duration> {
        value.ttl
    }

    fn expire_after_read(
        &self,
        _key: &String,
        _value: &TaggedValue,
        _read_at: Instant,
        duration_until_expiry: Option<Duration>,
        _last_modified_at: Instant,
    ) -> Option<Duration> {
        // Preserve the previously computed expiration: a read must not extend it.
        duration_until_expiry
    }

    fn expire_after_update(
        &self,
        _key: &String,
        value: &TaggedValue,
        _updated_at: Instant,
        _duration_until_expiry: Option<Duration>,
    ) -> Option<Duration> {
        value.ttl
    }
}

/// Thread-safe cache with tag-based invalidation and per-entry TTL, backed by
/// `moka::sync::Cache`.
///
/// `ttl_secs == 0` means "no expiration"; any other value is that entry's TTL
/// in seconds. Expiration is enforced by moka's `Expiry` policy above, so no
/// background timer or fake value is involved.
#[cfg(feature = "tagged-cache")]
pub struct MokaCacheAdapter {
    cache: moka::sync::Cache<String, TaggedValue>,
    tag_index: RwLock<HashMap<String, HashSet<String>>>,
}

#[cfg(feature = "tagged-cache")]
impl Default for MokaCacheAdapter {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(feature = "tagged-cache")]
impl MokaCacheAdapter {
    /// Create a cache holding at most `max_entries` entries.
    pub fn new(max_entries: u64) -> Self {
        let cache = moka::sync::Cache::builder()
            .max_capacity(max_entries)
            .expire_after(ValueTtlExpiry)
            .build();
        Self {
            cache,
            tag_index: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(feature = "tagged-cache")]
impl CacheAdapter for MokaCacheAdapter {
    fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|entry| entry.value)
    }

    fn set(&self, key: &str, value: String, ttl_secs: u64, tags: Vec<String>) {
        let ttl = if ttl_secs == 0 {
            None
        } else {
            Some(Duration::from_secs(ttl_secs))
        };
        self.cache
            .insert(key.to_string(), TaggedValue { value, ttl });

        let mut index = self.tag_index.write().unwrap_or_else(|e| e.into_inner());
        for tag in tags {
            index.entry(tag).or_default().insert(key.to_string());
        }
    }

    fn invalidate_key(&self, key: &str) -> bool {
        let existed = self.cache.get(key).is_some();
        self.cache.invalidate(key);
        let mut index = self.tag_index.write().unwrap_or_else(|e| e.into_inner());
        for keys in index.values_mut() {
            keys.remove(key);
        }
        index.retain(|_, keys| !keys.is_empty());
        existed
    }

    fn invalidate_tag(&self, tag: &str) -> usize {
        self.cache.run_pending_tasks();
        let keys = self
            .tag_index
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(tag)
            .unwrap_or_default();
        if keys.is_empty() {
            return 0;
        }
        let mut count = 0usize;
        for key in &keys {
            if self.cache.get(key).is_some() {
                count += 1;
            }
            self.cache.invalidate(key);
        }
        // The same key may be indexed under other tags. Drop those stale
        // references too, otherwise a later `invalidate_tag` reports keys
        // that were already removed.
        let mut index = self.tag_index.write().unwrap_or_else(|e| e.into_inner());
        for remaining in index.values_mut() {
            for key in &keys {
                remaining.remove(key);
            }
        }
        index.retain(|_, remaining| !remaining.is_empty());
        count
    }
}

#[cfg(all(test, feature = "tagged-cache"))]
mod cache_tests {
    use super::*;

    #[test]
    fn test_set_get_and_tag_invalidation() {
        let cache = MokaCacheAdapter::new(64);
        cache.set(
            "/home",
            "<h1>hi</h1>".to_string(),
            60,
            vec!["pages".to_string(), "public".to_string()],
        );
        cache.set(
            "/admin",
            "secret".to_string(),
            60,
            vec!["pages".to_string()],
        );

        assert_eq!(cache.get("/home").as_deref(), Some("<h1>hi</h1>"));
        assert_eq!(cache.invalidate_tag("public"), 1);
        assert_eq!(cache.get("/home"), None);
        assert_eq!(cache.get("/admin").as_deref(), Some("secret"));

        assert!(cache.invalidate_key("/admin"));
        assert!(!cache.invalidate_key("/admin"));
        assert_eq!(cache.get("/admin"), None);
        assert_eq!(cache.invalidate_tag("pages"), 0);
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;
    use std::sync::Mutex;

    /// In-test adapter that records statements instead of touching a database.
    #[derive(Default)]
    struct RecordingAdapter {
        statements: Mutex<Vec<String>>,
        fail: bool,
    }

    impl DatabaseAdapter for RecordingAdapter {
        fn execute(&self, sql: &str, _params: &[String]) -> Result<u64, String> {
            if self.fail {
                return Err("injected SQL failure".to_string());
            }
            self.statements
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(sql.to_string());
            Ok(1)
        }

        fn query_rows(
            &self,
            _sql: &str,
            _params: &[String],
        ) -> Result<Vec<HashMap<String, String>>, String> {
            if self.fail {
                return Err("injected SQL failure".to_string());
            }
            Ok(Vec::new())
        }
    }

    fn temp_migrator(name: &str, sql: &str) -> (Migrator, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "yav_roadmap_migration_{}_{}",
            name,
            std::process::id()
        ));
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir).expect("create temp migration dir");
        std::fs::write(dir.join("0001_init.sql"), sql).expect("write migration");
        (Migrator::new(dir.clone()), dir)
    }

    #[test]
    fn test_split_sql_statements_ignores_comments_and_empty_fragments() {
        let statements = split_sql_statements(
            "-- Up\n-- a comment\nCREATE TABLE a (id TEXT);\n-- another comment\n\nINSERT INTO a (id) VALUES ('x');\n-- trailing comment\n",
        );
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE TABLE a"));
        assert!(statements[1].contains("INSERT INTO a"));
    }

    #[test]
    fn test_apply_all_with_records_statements_and_journal() {
        let (migrator, dir) = temp_migrator(
            "apply",
            "-- Up\nCREATE TABLE a (id TEXT);\nINSERT INTO a (id) VALUES ('x');\n-- Down\nDROP TABLE a;",
        );
        let adapter = RecordingAdapter::default();

        let applied = migrator.apply_all_with(&adapter).expect("apply ok");
        assert_eq!(applied, vec!["0001_init".to_string()]);
        assert_eq!(adapter.statements.lock().unwrap().len(), 2);
        assert_eq!(migrator.load_applied().unwrap().len(), 1);

        // Re-applying is a no-op, not a duplicate execution.
        assert!(migrator.apply_all_with(&adapter).unwrap().is_empty());
        assert_eq!(adapter.statements.lock().unwrap().len(), 2);

        let rolled = migrator
            .rollback_last_with_adapter(&adapter)
            .unwrap()
            .expect("one migration rolled back");
        assert_eq!(rolled, "0001_init");
        assert!(migrator.load_applied().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_sql_error_propagates_without_journaling() {
        let (migrator, dir) = temp_migrator("fail", "-- Up\nCREATE TABLE a (id TEXT);");
        let adapter = RecordingAdapter {
            fail: true,
            ..Default::default()
        };

        let err = migrator.apply_all_with(&adapter).expect_err("must fail");
        assert!(err.contains("injected SQL failure"));
        assert!(
            migrator.load_applied().unwrap().is_empty(),
            "no journal on error"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
