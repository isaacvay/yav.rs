//! Database Migration Runner, Parameterized SQL Query Builder, and Embedded Document Store.
//!
//! Provides an integrated Rust data layer rivaling Prisma / Drizzle:
//! - [`Migrator`] & [`Migration`]: Parses `migrations/*.sql` (`-- Up` / `-- Down`), verifies SHA-256
//!   checksums, tracks state in `migrations/.yav_migrations.json`, and runs up/down migrations.
//! - [`QueryBuilder`]: Fluent, SQL-injection-safe parameterized query builder (`SELECT`, `INSERT`,
//!   `UPDATE`, `DELETE`, `WHERE`, `ORDER BY`, cursor/offset pagination).
//! - [`EmbeddedStore`]: Thread-safe, zero-config in-memory and JSON-file-backed document store.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

// ─────────────────────────────────────────────────────────────────────────────
// 1. SQL Migration Engine (`Migrator` & `Migration`)
// ─────────────────────────────────────────────────────────────────────────────

/// Represents a parsed `.sql` migration file with `-- Up` and `-- Down` sections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Migration {
    /// Migration version / filename stem (e.g. `20261104120000_create_users`).
    pub version: String,
    /// Path to the `.sql` file on disk.
    pub file_path: PathBuf,
    /// Forward SQL statements (`-- Up` section).
    pub up_sql: String,
    /// Rollback SQL statements (`-- Down` section).
    pub down_sql: String,
    /// Hex-encoded SHA-256 checksum of the migration file contents.
    pub checksum: String,
}

impl Migration {
    /// Parse a migration from raw SQL text containing `-- Up` and optional `-- Down` markers.
    pub fn parse(version: impl Into<String>, file_path: impl Into<PathBuf>, raw_sql: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(raw_sql.as_bytes());
        let digest = hasher.finalize();
        let mut checksum = String::with_capacity(64);
        for b in digest {
            use std::fmt::Write;
            let _ = write!(&mut checksum, "{:02x}", b);
        }

        let mut up_lines = Vec::new();
        let mut down_lines = Vec::new();
        let mut in_down = false;

        for line in raw_sql.lines() {
            let trimmed = line.trim();
            if trimmed.eq_ignore_ascii_case("-- down")
                || trimmed.eq_ignore_ascii_case("-- migrate:down")
            {
                in_down = true;
                continue;
            }
            if trimmed.eq_ignore_ascii_case("-- up")
                || trimmed.eq_ignore_ascii_case("-- migrate:up")
            {
                in_down = false;
                continue;
            }
            if in_down {
                down_lines.push(line);
            } else {
                up_lines.push(line);
            }
        }

        Self {
            version: version.into(),
            file_path: file_path.into(),
            up_sql: up_lines.join("\n").trim().to_string(),
            down_sql: down_lines.join("\n").trim().to_string(),
            checksum,
        }
    }
}

/// Record of an applied migration stored in `.yav_migrations.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedMigration {
    pub version: String,
    pub checksum: String,
    pub applied_at_epoch: u64,
}

/// Status of a discovered migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationStatus {
    pub version: String,
    pub applied: bool,
    pub checksum_valid: bool,
}

/// Manages discovering, validating, applying, and rolling back SQL migrations in a directory.
#[derive(Debug, Clone)]
pub struct Migrator {
    migrations_dir: PathBuf,
}

impl Migrator {
    /// Create a new migrator rooted at `migrations_dir` (typically `"migrations"`).
    pub fn new(migrations_dir: impl Into<PathBuf>) -> Self {
        Self {
            migrations_dir: migrations_dir.into(),
        }
    }

    fn journal_path(&self) -> PathBuf {
        self.migrations_dir.join(".yav_migrations.json")
    }

    /// Discover all `*.sql` files in `migrations_dir` sorted lexicographically by version.
    pub fn discover(&self) -> Result<Vec<Migration>, String> {
        if !self.migrations_dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(&self.migrations_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|de| de.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "sql"))
            .collect();
        entries.sort();

        let mut migrations = Vec::with_capacity(entries.len());
        for path in entries {
            let version = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
            migrations.push(Migration::parse(version, path, &raw));
        }
        Ok(migrations)
    }

    /// Load the list of already-applied migrations from the journal file.
    pub fn load_applied(&self) -> Result<Vec<AppliedMigration>, String> {
        let jp = self.journal_path();
        if !jp.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&jp).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    fn save_applied(&self, applied: &[AppliedMigration]) -> Result<(), String> {
        fs::create_dir_all(&self.migrations_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(applied).map_err(|e| e.to_string())?;
        fs::write(self.journal_path(), json).map_err(|e| e.to_string())
    }

    /// Persist an explicit applied-migration journal.
    ///
    /// This is the public journal-writing hook used by out-of-tree migration
    /// runners (e.g. `crate::roadmap::persistence`) that drive a real
    /// [`crate::adapters::DatabaseAdapter`] instead of a closure. The runner
    /// owns the decision of which statements succeeded; this method only
    /// records the resulting state on disk.
    pub fn record_applied(&self, applied: &[AppliedMigration]) -> Result<(), String> {
        self.save_applied(applied)
    }

    /// Clear the applied-migration journal (writes an empty journal).
    pub fn clear_applied(&self) -> Result<(), String> {
        self.save_applied(&[])
    }

    /// Compute the status of all discovered migrations against the applied journal.
    pub fn status(&self) -> Result<Vec<MigrationStatus>, String> {
        let discovered = self.discover()?;
        let applied = self.load_applied()?;
        let applied_map: BTreeMap<&str, &AppliedMigration> =
            applied.iter().map(|a| (a.version.as_str(), a)).collect();

        Ok(discovered
            .into_iter()
            .map(|m| {
                if let Some(app) = applied_map.get(m.version.as_str()) {
                    MigrationStatus {
                        version: m.version,
                        applied: true,
                        checksum_valid: app.checksum == m.checksum,
                    }
                } else {
                    MigrationStatus {
                        version: m.version,
                        applied: false,
                        checksum_valid: true,
                    }
                }
            })
            .collect())
    }

    /// Apply all pending migrations using the provided SQL execution callback, recording them in the journal.
    /// Returns the versions of newly applied migrations.
    pub fn migrate_up_with<F>(&self, mut executor: F) -> Result<Vec<String>, String>
    where
        F: FnMut(&str) -> Result<(), String>,
    {
        let discovered = self.discover()?;
        let mut applied = self.load_applied()?;
        let mut newly_applied = Vec::new();

        for m in discovered {
            if let Some(existing) = applied.iter().find(|a| a.version == m.version) {
                if existing.checksum != m.checksum {
                    return Err(format!(
                        "Checksum mismatch for already-applied migration '{}'",
                        m.version
                    ));
                }
                continue;
            }
            executor(&m.up_sql)?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            applied.push(AppliedMigration {
                version: m.version.clone(),
                checksum: m.checksum.clone(),
                applied_at_epoch: now,
            });
            // Persist the journal incrementally so a later failure never hides applied migrations.
            self.save_applied(&applied)?;
            newly_applied.push(m.version);
        }

        Ok(newly_applied)
    }

    /// Roll back the most recently applied migration using the provided SQL execution callback.
    /// Returns `Ok(Some(version))` if a migration was rolled back, or `Ok(None)` if none were applied.
    pub fn rollback_last_with<F>(&self, mut executor: F) -> Result<Option<String>, String>
    where
        F: FnMut(&str) -> Result<(), String>,
    {
        let discovered = self.discover()?;
        let mut applied = self.load_applied()?;
        let Some(last) = applied.pop() else {
            return Ok(None);
        };

        if let Some(m) = discovered.into_iter().find(|d| d.version == last.version) {
            if !m.down_sql.is_empty() {
                executor(&m.down_sql)?;
            }
        }
        self.save_applied(&applied)?;
        Ok(Some(last.version))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Parameterized SQL QueryBuilder
// ─────────────────────────────────────────────────────────────────────────────

/// Sort direction for `ORDER BY` clauses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueryKind {
    Select,
    Insert,
    Update,
    Delete,
}

/// Fluent, SQL-injection-safe parameterized SQL query builder.
///
/// Generates `$1, $2, ...` positional bind placeholders alongside a `Vec<serde_json::Value>`
/// parameter vector, preventing raw string interpolation vulnerabilities.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    kind: QueryKind,
    table: String,
    columns: Vec<String>,
    set_clauses: Vec<(String, serde_json::Value)>,
    where_clauses: Vec<(String, &'static str, serde_json::Value)>,
    joins: Vec<String>,
    order_by: Vec<(String, Order)>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl QueryBuilder {
    /// Start a `SELECT` query on `table`.
    pub fn select(table: impl Into<String>) -> Self {
        Self {
            kind: QueryKind::Select,
            table: sanitize_ident(&table.into()),
            columns: Vec::new(),
            set_clauses: Vec::new(),
            where_clauses: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Start an `INSERT INTO` query on `table`.
    pub fn insert_into(table: impl Into<String>) -> Self {
        Self {
            kind: QueryKind::Insert,
            ..Self::select(table)
        }
    }

    /// Start an `UPDATE` query on `table`.
    pub fn update(table: impl Into<String>) -> Self {
        Self {
            kind: QueryKind::Update,
            ..Self::select(table)
        }
    }

    /// Start a `DELETE FROM` query on `table`.
    pub fn delete_from(table: impl Into<String>) -> Self {
        Self {
            kind: QueryKind::Delete,
            ..Self::select(table)
        }
    }

    /// Specify columns to project in a `SELECT` query (defaults to `*` when empty).
    #[must_use]
    pub fn columns(mut self, cols: &[&str]) -> Self {
        self.columns = cols.iter().map(|c| sanitize_ident(c)).collect();
        self
    }

    /// Bind a column value for `INSERT` or `UPDATE` statements.
    #[must_use]
    pub fn value(mut self, col: &str, val: impl Into<serde_json::Value>) -> Self {
        self.set_clauses.push((sanitize_ident(col), val.into()));
        self
    }

    /// Add a parameterized `WHERE col = $N` filter.
    #[must_use]
    pub fn where_eq(mut self, col: &str, val: impl Into<serde_json::Value>) -> Self {
        self.where_clauses
            .push((sanitize_ident(col), "=", val.into()));
        self
    }

    /// Add a parameterized `WHERE col > $N` filter.
    #[must_use]
    pub fn where_gt(mut self, col: &str, val: impl Into<serde_json::Value>) -> Self {
        self.where_clauses
            .push((sanitize_ident(col), ">", val.into()));
        self
    }

    /// Add a parameterized `WHERE col LIKE $N` filter.
    #[must_use]
    pub fn where_like(mut self, col: &str, pattern: impl Into<String>) -> Self {
        self.where_clauses.push((
            sanitize_ident(col),
            "LIKE",
            serde_json::Value::String(pattern.into()),
        ));
        self
    }

    /// Add an `INNER JOIN` clause.
    #[must_use]
    pub fn inner_join(mut self, other_table: &str, on_left: &str, on_right: &str) -> Self {
        self.joins.push(format!(
            "INNER JOIN {} ON {} = {}",
            sanitize_ident(other_table),
            sanitize_ident(on_left),
            sanitize_ident(on_right)
        ));
        self
    }

    /// Add an `ORDER BY` clause.
    #[must_use]
    pub fn order_by(mut self, col: &str, order: Order) -> Self {
        self.order_by.push((sanitize_ident(col), order));
        self
    }

    /// Set `LIMIT`.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set `OFFSET`.
    #[must_use]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Configure 1-based page pagination (`LIMIT per_page OFFSET (page - 1) * per_page`).
    #[must_use]
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        let clean_page = page.max(1);
        let clean_per_page = per_page.max(1);
        self.limit = Some(clean_per_page);
        self.offset = Some((clean_page - 1) * clean_per_page);
        self
    }

    /// Compile the query into `(sql_string, bound_parameters)`.
    pub fn build(&self) -> (String, Vec<serde_json::Value>) {
        let mut params = Vec::new();
        let mut sql = String::new();

        match self.kind {
            QueryKind::Select => {
                let cols = if self.columns.is_empty() {
                    "*".to_string()
                } else {
                    self.columns.join(", ")
                };
                sql.push_str(&format!("SELECT {} FROM {}", cols, self.table));
                for j in &self.joins {
                    sql.push(' ');
                    sql.push_str(j);
                }
            }
            QueryKind::Insert => {
                if self.set_clauses.is_empty() {
                    sql.push_str(&format!("INSERT INTO {} DEFAULT VALUES", self.table));
                } else {
                    let cols: Vec<&str> =
                        self.set_clauses.iter().map(|(c, _)| c.as_str()).collect();
                    let mut placeholders = Vec::with_capacity(self.set_clauses.len());
                    for (_, val) in &self.set_clauses {
                        params.push(val.clone());
                        placeholders.push(format!("${}", params.len()));
                    }
                    sql.push_str(&format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        self.table,
                        cols.join(", "),
                        placeholders.join(", ")
                    ));
                }
            }
            QueryKind::Update => {
                assert!(
                    !self.set_clauses.is_empty(),
                    "QueryBuilder::update requires at least one .value(col, val) assignment"
                );
                let mut sets = Vec::with_capacity(self.set_clauses.len());
                for (col, val) in &self.set_clauses {
                    params.push(val.clone());
                    sets.push(format!("{} = ${}", col, params.len()));
                }
                sql.push_str(&format!("UPDATE {} SET {}", self.table, sets.join(", ")));
            }
            QueryKind::Delete => {
                sql.push_str(&format!("DELETE FROM {}", self.table));
            }
        }

        if !self.where_clauses.is_empty() {
            let mut conds = Vec::with_capacity(self.where_clauses.len());
            for (col, op, val) in &self.where_clauses {
                params.push(val.clone());
                conds.push(format!("{} {} ${}", col, op, params.len()));
            }
            sql.push_str(" WHERE ");
            sql.push_str(&conds.join(" AND "));
        }

        if !self.order_by.is_empty() {
            let orders: Vec<String> = self
                .order_by
                .iter()
                .map(|(c, dir)| {
                    let d = match dir {
                        Order::Asc => "ASC",
                        Order::Desc => "DESC",
                    };
                    format!("{c} {d}")
                })
                .collect();
            sql.push_str(" ORDER BY ");
            sql.push_str(&orders.join(", "));
        }

        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {lim}"));
        }
        if let Some(off) = self.offset {
            sql.push_str(&format!(" OFFSET {off}"));
        }

        (sql, params)
    }
}

fn sanitize_ident(ident: &str) -> String {
    ident
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. EmbeddedStore (Zero-Config Persistent/In-Memory Collection Store)
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe, zero-config in-memory and JSON-file-persisted document store.
///
/// Ideal for rapid full-stack prototyping, local dev databases, or lightweight production apps.
#[derive(Clone, Default)]
pub struct EmbeddedStore {
    collections: Arc<RwLock<BTreeMap<String, BTreeMap<String, serde_json::Value>>>>,
    persist_path: Option<PathBuf>,
}

impl EmbeddedStore {
    /// Create a new in-memory [`EmbeddedStore`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an [`EmbeddedStore`] backed by a JSON file on disk, loading existing records if present.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let p = path.as_ref().to_path_buf();
        let store = Self {
            collections: Arc::new(RwLock::new(BTreeMap::new())),
            persist_path: Some(p.clone()),
        };
        if p.exists() {
            let raw = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            if !raw.trim().is_empty() {
                let parsed: BTreeMap<String, BTreeMap<String, serde_json::Value>> =
                    serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                *store.collections.write().unwrap_or_else(|e| e.into_inner()) = parsed;
            }
        }
        Ok(store)
    }

    /// Insert or update a typed record by `id` in `collection`.
    pub fn insert<T: Serialize>(
        &self,
        collection: &str,
        id: &str,
        record: &T,
    ) -> Result<(), String> {
        let val = serde_json::to_value(record).map_err(|e| e.to_string())?;
        {
            let mut guard = self.collections.write().unwrap_or_else(|e| e.into_inner());
            guard
                .entry(collection.to_string())
                .or_default()
                .insert(id.to_string(), val);
        }
        self.flush_if_persistent()
    }

    /// Retrieve and deserialize a record by `id` from `collection`.
    pub fn find_by_id<T: DeserializeOwned>(
        &self,
        collection: &str,
        id: &str,
    ) -> Result<Option<T>, String> {
        let guard = self.collections.read().unwrap_or_else(|e| e.into_inner());
        let Some(col) = guard.get(collection) else {
            return Ok(None);
        };
        let Some(val) = col.get(id) else {
            return Ok(None);
        };
        let decoded = serde_json::from_value(val.clone()).map_err(|e| e.to_string())?;
        Ok(Some(decoded))
    }

    /// Return all records in `collection`.
    pub fn find_all<T: DeserializeOwned>(&self, collection: &str) -> Result<Vec<T>, String> {
        let guard = self.collections.read().unwrap_or_else(|e| e.into_inner());
        let Some(col) = guard.get(collection) else {
            return Ok(Vec::new());
        };
        let mut results = Vec::with_capacity(col.len());
        for val in col.values() {
            results.push(serde_json::from_value(val.clone()).map_err(|e| e.to_string())?);
        }
        Ok(results)
    }

    /// Delete a record by `id` from `collection`. Returns `true` if a record was removed.
    pub fn delete(&self, collection: &str, id: &str) -> Result<bool, String> {
        let removed = {
            let mut guard = self.collections.write().unwrap_or_else(|e| e.into_inner());
            guard
                .get_mut(collection)
                .and_then(|c| c.remove(id))
                .is_some()
        };
        if removed {
            self.flush_if_persistent()?;
        }
        Ok(removed)
    }

    fn flush_if_persistent(&self) -> Result<(), String> {
        static FLUSH_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if let Some(ref p) = self.persist_path {
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let json = {
                let guard = self.collections.read().unwrap_or_else(|e| e.into_inner());
                serde_json::to_string_pretty(&*guard).map_err(|e| e.to_string())?
            };
            let seq = FLUSH_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let tmp_path = p.with_extension(format!("tmp.{}.{seq}", std::process::id()));
            fs::write(&tmp_path, json).map_err(|e| e.to_string())?;
            fs::rename(&tmp_path, p).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_select_and_insert() {
        let (sql, params) = QueryBuilder::select("users")
            .columns(&["id", "email", "role"])
            .where_eq("active", true)
            .where_gt("age", 18)
            .order_by("created_at", Order::Desc)
            .paginate(2, 25)
            .build();

        assert_eq!(
            sql,
            "SELECT id, email, role FROM users WHERE active = $1 AND age > $2 ORDER BY created_at DESC LIMIT 25 OFFSET 25"
        );
        assert_eq!(params.len(), 2);

        let (ins_sql, ins_params) = QueryBuilder::insert_into("users")
            .value("email", "alice@yav.rs")
            .value("role", "admin")
            .build();
        assert_eq!(ins_sql, "INSERT INTO users (email, role) VALUES ($1, $2)");
        assert_eq!(ins_params.len(), 2);
    }

    #[test]
    fn test_migration_parser_and_embedded_store() {
        let raw = "-- Up\nCREATE TABLE users (id INT PRIMARY KEY);\n-- Down\nDROP TABLE users;";
        let m = Migration::parse("20261104_init", "migrations/20261104_init.sql", raw);
        assert_eq!(m.up_sql, "CREATE TABLE users (id INT PRIMARY KEY);");
        assert_eq!(m.down_sql, "DROP TABLE users;");
        assert_eq!(m.checksum.len(), 64);

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TodoItem {
            title: String,
            done: bool,
        }

        let store = EmbeddedStore::new();
        store
            .insert(
                "todos",
                "1",
                &TodoItem {
                    title: "Ship Yav".to_string(),
                    done: true,
                },
            )
            .unwrap();
        let fetched: Option<TodoItem> = store.find_by_id("todos", "1").unwrap();
        assert_eq!(fetched.unwrap().title, "Ship Yav");
        assert_eq!(store.find_all::<TodoItem>("todos").unwrap().len(), 1);
        assert!(store.delete("todos", "1").unwrap());
    }
}
