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

/// Split a SQL section into individual statements.
///
/// Fragments that are empty or contain only `--` comments are discarded.
/// This is intentionally simple: migration files are trusted, developer-owned
/// SQL, and statements are still passed to the driver as bound-parameter-free
/// strings (no interpolation of user input happens here).
fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|fragment| {
            fragment
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .filter(|statement| !statement.is_empty())
        .collect()
}

impl Migrator {
    /// Apply every pending migration through a real [`DatabaseAdapter`].
    ///
    /// For each discovered migration:
    /// 1. an already-applied migration with a different checksum is an error;
    /// 2. the `-- Up` section is split into statements and executed in order;
    /// 3. only after every statement succeeded is the migration journaled.
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
            newly_applied.push(migration.version);
        }

        self.record_applied(&applied)?;
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

/// Build a dedicated multi-threaded runtime used by the synchronous trait
/// methods. The pool is created on this same runtime so its background
/// maintenance tasks live on a driver that stays alive with the adapter.
#[cfg(any(feature = "sqlite", feature = "postgres"))]
fn dedicated_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to build dedicated Tokio runtime: {e}"))
}

/// Real SQLite driver backed by a `sqlx` connection pool.
#[cfg(feature = "sqlite")]
pub struct SqliteDatabaseAdapter {
    pool: sqlx::SqlitePool,
    runtime: tokio::runtime::Runtime,
}

#[cfg(feature = "sqlite")]
impl SqliteDatabaseAdapter {
    /// Open a pooled SQLite connection from a `sqlx` URL (e.g. `sqlite::memory:`
    /// or `sqlite://./app.db`). The connection is established eagerly; an
    /// unreachable/invalid database is an error, never a silent no-op.
    pub fn connect(url: &str) -> Result<Self, String> {
        let runtime = dedicated_runtime()?;
        let pool = runtime
            .block_on(sqlx::sqlite::SqlitePoolOptions::new().connect(url))
            .map_err(|e| format!("failed to connect to SQLite at '{url}': {e}"))?;
        Ok(Self { pool, runtime })
    }
}

#[cfg(feature = "sqlite")]
impl DatabaseAdapter for SqliteDatabaseAdapter {
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        let owned_sql = sql.to_string();
        let outcome = self.runtime.block_on(async move {
            let mut query = sqlx::query(sqlx::AssertSqlSafe(owned_sql));
            for param in params {
                query = query.bind(param.clone());
            }
            query
                .execute(&self.pool)
                .await
                .map(|result| result.rows_affected())
        });
        outcome.map_err(|e| format!("SQLite execute failed: {e}"))
    }

    fn query_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        let owned_sql = sql.to_string();
        let outcome = self.runtime.block_on(async move {
            let mut query = sqlx::query(sqlx::AssertSqlSafe(owned_sql));
            for param in params {
                query = query.bind(param.clone());
            }
            let rows = query.fetch_all(&self.pool).await?;
            use sqlx::{Column, Row};
            let mut collected = Vec::with_capacity(rows.len());
            for row in rows {
                let mut record = HashMap::new();
                for column in row.columns() {
                    let name = column.name();
                    let value: Option<String> = row.try_get(name)?;
                    record.insert(name.to_string(), value.unwrap_or_default());
                }
                collected.push(record);
            }
            Ok::<Vec<HashMap<String, String>>, sqlx::Error>(collected)
        });
        outcome.map_err(|e| format!("SQLite query failed: {e}"))
    }

    fn health_check(&self) -> bool {
        self.execute("SELECT 1", &[]).is_ok()
    }
}

/// Real PostgreSQL driver backed by a `sqlx` connection pool.
#[cfg(feature = "postgres")]
pub struct PostgresDatabaseAdapter {
    pool: sqlx::PgPool,
    runtime: tokio::runtime::Runtime,
}

#[cfg(feature = "postgres")]
impl PostgresDatabaseAdapter {
    /// Open a pooled PostgreSQL connection from a `sqlx` URL.
    pub fn connect(url: &str) -> Result<Self, String> {
        let runtime = dedicated_runtime()?;
        let pool = runtime
            .block_on(sqlx::postgres::PgPoolOptions::new().connect(url))
            .map_err(|e| format!("failed to connect to PostgreSQL: {e}"))?;
        Ok(Self { pool, runtime })
    }
}

#[cfg(feature = "postgres")]
impl DatabaseAdapter for PostgresDatabaseAdapter {
    fn execute(&self, sql: &str, params: &[String]) -> Result<u64, String> {
        let owned_sql = sql.to_string();
        let outcome = self.runtime.block_on(async move {
            let mut query = sqlx::query(sqlx::AssertSqlSafe(owned_sql));
            for param in params {
                query = query.bind(param.clone());
            }
            query
                .execute(&self.pool)
                .await
                .map(|result| result.rows_affected())
        });
        outcome.map_err(|e| format!("PostgreSQL execute failed: {e}"))
    }

    fn query_rows(
        &self,
        sql: &str,
        params: &[String],
    ) -> Result<Vec<HashMap<String, String>>, String> {
        let owned_sql = sql.to_string();
        let outcome = self.runtime.block_on(async move {
            let mut query = sqlx::query(sqlx::AssertSqlSafe(owned_sql));
            for param in params {
                query = query.bind(param.clone());
            }
            let rows = query.fetch_all(&self.pool).await?;
            use sqlx::{Column, Row};
            let mut collected = Vec::with_capacity(rows.len());
            for row in rows {
                let mut record = HashMap::new();
                for column in row.columns() {
                    let name = column.name();
                    let value: Option<String> = row.try_get(name)?;
                    record.insert(name.to_string(), value.unwrap_or_default());
                }
                collected.push(record);
            }
            Ok::<Vec<HashMap<String, String>>, sqlx::Error>(collected)
        });
        outcome.map_err(|e| format!("PostgreSQL query failed: {e}"))
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
        let keys = self
            .tag_index
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(tag)
            .unwrap_or_default();
        let count = keys.len();
        for key in keys {
            self.cache.invalidate(&key);
            // The same key may be indexed under other tags. Drop those stale
            // references too, otherwise a later `invalidate_tag` reports keys
            // that were already removed.
            let mut index = self
                .tag_index
                .write()
                .unwrap_or_else(|e| e.into_inner());
            for remaining in index.values_mut() {
                remaining.remove(&key);
            }
            index.retain(|_, remaining| !remaining.is_empty());
        }
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
