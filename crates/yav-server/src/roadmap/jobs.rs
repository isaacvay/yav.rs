//! Durable background jobs backed by a real [`DatabaseAdapter`] (feature `durable-jobs`).
//!
//! Unlike the in-memory [`crate::jobs::JobQueue`], every state transition here
//! is a SQL statement. No method reports success unless the underlying adapter
//! confirms the write. All columns are `TEXT` and bind parameters are strings,
//! so the same schema works on both SQLite and PostgreSQL.

#![cfg(feature = "durable-jobs")]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::adapters::DatabaseAdapter;

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn new_job_id() -> String {
    format!(
        "job_{}_{}",
        now_epoch(),
        NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed)
    )
}

/// A row claimed by a worker from `yav_jobs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableJob {
    pub id: String,
    pub name: String,
    pub payload: String,
    /// Attempt number of this claim (1-based).
    pub attempts: u32,
}

/// Durable FIFO job queue on top of `yav_jobs`.
///
/// Claiming is optimistic: a job is selected, then claimed with a conditional
/// `UPDATE ... WHERE status = 'pending'`. If another worker won the race, the
/// queue re-selects; a bounded number of retries keeps this lock-free and
/// portable across SQL backends (no `RETURNING` / transaction requirement).
pub struct SqliteJobQueue {
    adapter: Arc<dyn DatabaseAdapter>,
}

impl SqliteJobQueue {
    pub fn new(adapter: Arc<dyn DatabaseAdapter>) -> Self {
        Self { adapter }
    }

    /// Create the `yav_jobs` table if it does not exist yet.
    pub fn ensure_schema(&self) -> Result<(), String> {
        self.adapter.execute(
            "CREATE TABLE IF NOT EXISTS yav_jobs (\
             id TEXT PRIMARY KEY, \
             name TEXT NOT NULL, \
             payload TEXT NOT NULL, \
             status TEXT NOT NULL, \
             attempts TEXT NOT NULL, \
             run_at TEXT NOT NULL, \
             claimed_by TEXT NOT NULL, \
             updated_at TEXT NOT NULL\
             )",
            &[],
        )?;
        Ok(())
    }

    /// Insert a new pending job and return its generated id.
    pub fn enqueue(
        &self,
        name: impl Into<String>,
        payload: impl Into<String>,
    ) -> Result<String, String> {
        let id = new_job_id();
        let now = now_epoch().to_string();
        let name = name.into();
        let payload = payload.into();
        self.adapter.execute(
            "INSERT INTO yav_jobs \
             (id, name, payload, status, attempts, run_at, claimed_by, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                id.clone(),
                name,
                payload,
                "pending".to_string(),
                "0".to_string(),
                now.clone(),
                String::new(),
                now,
            ],
        )?;
        Ok(id)
    }

    /// Claim the next runnable job, propagating any SQL error.
    ///
    /// This is the honest counterpart of [`Self::claim`]: callers that need to
    /// distinguish "no job available" from "database failed" should use it.
    pub fn try_claim(&self, worker_id: &str) -> Result<Option<DurableJob>, String> {
        const MAX_RACES: usize = 8;
        let now = now_epoch();

        for _ in 0..MAX_RACES {
            let rows = self.adapter.query_rows(
                "SELECT id, name, payload, attempts FROM yav_jobs \
                 WHERE status = 'pending' AND run_at <= $1 \
                 ORDER BY run_at ASC LIMIT 1",
                &[now.to_string()],
            )?;

            let Some(row) = rows.into_iter().next() else {
                return Ok(None);
            };
            let id = row.get("id").cloned().unwrap_or_default();
            if id.is_empty() {
                return Err("yav_jobs row is missing an id".to_string());
            }
            let name = row.get("name").cloned().unwrap_or_default();
            let payload = row.get("payload").cloned().unwrap_or_default();
            let attempts = row
                .get("attempts")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(0)
                .saturating_add(1);

            let affected = self.adapter.execute(
                "UPDATE yav_jobs \
                 SET status = 'running', claimed_by = $1, attempts = $2, updated_at = $3 \
                 WHERE id = $4 AND status = 'pending'",
                &[
                    worker_id.to_string(),
                    attempts.to_string(),
                    now.to_string(),
                    id.clone(),
                ],
            )?;

            if affected == 1 {
                return Ok(Some(DurableJob {
                    id,
                    name,
                    payload,
                    attempts,
                }));
            }
            // Another worker claimed this row first: re-select on the next pass.
        }

        Ok(None)
    }

    /// Claim the next runnable job.
    ///
    /// Convenience wrapper around [`Self::try_claim`]. A SQL error is logged
    /// and surfaces as `None`; use `try_claim` when the distinction matters.
    pub fn claim(&self, worker_id: &str) -> Option<DurableJob> {
        match self.try_claim(worker_id) {
            Ok(job) => job,
            Err(err) => {
                tracing::error!(error = %err, "SqliteJobQueue::claim failed");
                None
            }
        }
    }

    /// Acknowledge a completed job by deleting it.
    ///
    /// Deletion is idempotent: acknowledging an already-gone job is `Ok(())`.
    pub fn ack(&self, id: &str) -> Result<(), String> {
        self.adapter
            .execute("DELETE FROM yav_jobs WHERE id = $1", &[id.to_string()])?;
        Ok(())
    }

    /// Requeue a failed job with a delay (in seconds) before it is claimable again.
    ///
    /// Returns an error if the job id does not exist rather than claiming success.
    pub fn retry(&self, id: &str, delay_secs: u64) -> Result<(), String> {
        let run_at = now_epoch().saturating_add(delay_secs).to_string();
        let affected = self.adapter.execute(
            "UPDATE yav_jobs \
             SET status = 'pending', claimed_by = $1, run_at = $2, updated_at = $3 \
             WHERE id = $4",
            &[
                String::new(),
                run_at,
                now_epoch().to_string(),
                id.to_string(),
            ],
        )?;
        if affected == 0 {
            return Err(format!("cannot retry unknown job '{id}'"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubAdapter {
        executed: Mutex<Vec<String>>,
        rows: Vec<HashMap<String, String>>,
        fail: bool,
    }

    impl DatabaseAdapter for StubAdapter {
        fn execute(&self, sql: &str, _params: &[String]) -> Result<u64, String> {
            if self.fail {
                return Err("stub SQL failure".to_string());
            }
            self.executed
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
                return Err("stub SQL failure".to_string());
            }
            Ok(self.rows.clone())
        }
    }

    fn queue(adapter: Arc<StubAdapter>) -> SqliteJobQueue {
        SqliteJobQueue::new(adapter)
    }

    #[test]
    fn test_schema_enqueue_and_ack_issue_real_sql() {
        let adapter = Arc::new(StubAdapter::default());
        let jobs = queue(adapter.clone());

        jobs.ensure_schema().unwrap();
        let id = jobs
            .enqueue("send_email", "{\"to\":\"dev@yav.rs\"}")
            .unwrap();
        assert!(id.starts_with("job_"));
        jobs.ack(&id).unwrap();

        let executed = adapter.executed.lock().unwrap();
        assert!(executed[0].contains("CREATE TABLE IF NOT EXISTS yav_jobs"));
        assert!(executed[1].contains("INSERT INTO yav_jobs"));
        assert!(executed[2].contains("DELETE FROM yav_jobs"));
    }

    #[test]
    fn test_claim_returns_none_when_no_row_and_propagates_errors() {
        let jobs = queue(Arc::new(StubAdapter::default()));
        assert!(jobs.try_claim("worker-1").unwrap().is_none());

        let jobs = queue(Arc::new(StubAdapter {
            fail: true,
            ..Default::default()
        }));
        assert!(jobs.try_claim("worker-1").is_err());
        assert!(jobs.ensure_schema().is_err());
        assert!(jobs.enqueue("x", "y").is_err());
        assert!(jobs.retry("job_missing", 5).is_err());
        // The Option-returning wrapper never fabricates a job.
        assert!(jobs.claim("worker-1").is_none());
    }

    #[test]
    fn test_claim_constructs_durable_job_from_row() {
        let mut row = HashMap::new();
        row.insert("id".to_string(), "job_1".to_string());
        row.insert("name".to_string(), "resize".to_string());
        row.insert("payload".to_string(), "{\"k\":1}".to_string());
        row.insert("attempts".to_string(), "2".to_string());
        let jobs = queue(Arc::new(StubAdapter {
            rows: vec![row],
            ..Default::default()
        }));

        let claimed = jobs
            .try_claim("worker-2")
            .unwrap()
            .expect("one job claimed");
        assert_eq!(claimed.id, "job_1");
        assert_eq!(claimed.name, "resize");
        assert_eq!(claimed.payload, "{\"k\":1}");
        assert_eq!(claimed.attempts, 3);
    }
}
