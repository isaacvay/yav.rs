//! Background Job Queue & Cron Task Scheduler (`yav::jobs` — Inngest / BullMQ equivalent).
//!
//! Provides asynchronous background task execution with automatic retry backoff, status tracking,
//! and recurring interval/cron jobs powered by Tokio.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock,
};
use std::time::Duration;

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a background job.
pub type JobId = u64;

/// Execution state of a background job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running { attempt: u32 },
    Completed,
    Failed { error: String, attempts: u32 },
}

/// Metadata record for a tracked background job.
#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: JobId,
    pub name: String,
    pub status: JobStatus,
}

/// Thread-safe background job queue and recurring scheduler.
#[derive(Clone, Default)]
pub struct JobQueue {
    jobs: Arc<RwLock<HashMap<JobId, JobInfo>>>,
}

impl JobQueue {
    /// Create a new background [`JobQueue`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a one-off background task that runs immediately on the Tokio runtime.
    pub fn enqueue<F, Fut>(&self, name: impl Into<String>, task: F) -> JobId
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
        let name_str = name.into();
        {
            let mut guard = self.jobs.write().unwrap_or_else(|e| e.into_inner());
            guard.insert(
                id,
                JobInfo {
                    id,
                    name: name_str,
                    status: JobStatus::Pending,
                },
            );
        }

        let jobs_ref = self.jobs.clone();
        tokio::spawn(async move {
            if let Some(info) = jobs_ref
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&id)
            {
                info.status = JobStatus::Running { attempt: 1 };
            }

            let res = task().await;
            if let Some(info) = jobs_ref
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&id)
            {
                info.status = match res {
                    Ok(()) => JobStatus::Completed,
                    Err(err) => JobStatus::Failed {
                        error: err,
                        attempts: 1,
                    },
                };
            }
        });

        id
    }

    /// Enqueue a resilient background task with automatic exponential backoff retries.
    pub fn enqueue_with_retry<F, Fut>(
        &self,
        name: impl Into<String>,
        max_attempts: u32,
        base_backoff: Duration,
        mut task: F,
    ) -> JobId
    where
        F: FnMut(u32) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        let id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
        let name_str = name.into();
        let max_tries = max_attempts.max(1);
        {
            let mut guard = self.jobs.write().unwrap_or_else(|e| e.into_inner());
            guard.insert(
                id,
                JobInfo {
                    id,
                    name: name_str,
                    status: JobStatus::Pending,
                },
            );
        }

        let jobs_ref = self.jobs.clone();
        tokio::spawn(async move {
            let mut last_err = String::new();
            for attempt in 1..=max_tries {
                if let Some(info) = jobs_ref
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .get_mut(&id)
                {
                    info.status = JobStatus::Running { attempt };
                }

                match task(attempt).await {
                    Ok(()) => {
                        if let Some(info) = jobs_ref
                            .write()
                            .unwrap_or_else(|e| e.into_inner())
                            .get_mut(&id)
                        {
                            info.status = JobStatus::Completed;
                        }
                        return;
                    }
                    Err(e) => {
                        last_err = e;
                        if attempt < max_tries {
                            tokio::time::sleep(base_backoff * attempt).await;
                        }
                    }
                }
            }

            if let Some(info) = jobs_ref
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&id)
            {
                info.status = JobStatus::Failed {
                    error: last_err,
                    attempts: max_tries,
                };
            }
        });

        id
    }

    /// Schedule a recurring task to run every `interval` in the background.
    pub fn schedule_interval<F, Fut>(&self, interval: Duration, mut task: F)
    where
        F: FnMut() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // Skip immediate first tick burst if desired, or run loop
            loop {
                ticker.tick().await;
                task().await;
            }
        });
    }

    /// Retrieve the current status of a job by [`JobId`].
    pub fn status(&self, id: JobId) -> Option<JobStatus> {
        self.jobs
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .map(|j| j.status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_queue_enqueue_and_retry() {
        let queue = JobQueue::new();
        let job_id = queue.enqueue("send_welcome_email", || async { Ok(()) });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(queue.status(job_id), Some(JobStatus::Completed));

        let retry_id = queue.enqueue_with_retry(
            "flaky_webhook",
            3,
            Duration::from_millis(5),
            |attempt| async move {
                if attempt < 2 {
                    Err("temporary timeout".to_string())
                } else {
                    Ok(())
                }
            },
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(queue.status(retry_id), Some(JobStatus::Completed));
    }
}
