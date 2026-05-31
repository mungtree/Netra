//! [`SqliteJobQueue`] — a durable [`JobQueue`] backed by the `jobs` table.
//!
//! Unlike the in-memory queue, queued work survives a restart: `enqueue` writes
//! a `queued` row, `dequeue` flips the next one to `running` in a transaction,
//! and ordering is driven by a per-job `queue_position` (defaulted from a
//! monotonic counter at enqueue time, rewritten by `reorder`). No schema change
//! is needed — `queue_position` lives inside the JSON `data` blob.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use tokio::sync::Notify;

use chatur_core::ids::JobId;
use chatur_core::model::{Job, JobStatus};
use chatur_core::traits::JobQueue;
use chatur_core::{CoreError, Result};

use crate::repo::{decode_data, store_err};

/// Orders queued rows: explicit `queue_position` first (nulls last), then by
/// creation time as a stable tie-break.
const ORDER_BY: &str = "ORDER BY json_extract(data, '$.queue_position') IS NULL, \
                        json_extract(data, '$.queue_position'), created_at";

/// SQLite-backed durable job queue.
///
/// Cloning is cheap and shares state — all clones see the same rows and the
/// same in-process wake signal, so the API layer and the scheduler can each
/// hold one.
#[derive(Debug, Clone)]
pub struct SqliteJobQueue {
    pool: SqlitePool,
    /// Signalled on every enqueue so a scheduler can block instead of poll.
    notify: Arc<Notify>,
    /// Hands out strictly-increasing default positions so FIFO order is exact
    /// even within one millisecond.
    counter: Arc<AtomicI64>,
}

impl SqliteJobQueue {
    /// Wraps a connection pool. Seeds the position counter from the wall clock
    /// so freshly enqueued jobs sort after anything already persisted.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            notify: Arc::new(Notify::new()),
            counter: Arc::new(AtomicI64::new(Utc::now().timestamp_millis())),
        }
    }

    /// Persists a job row, inserting or refreshing it (idempotent on id).
    async fn upsert(&self, job: &Job) -> Result<()> {
        sqlx::query(
            "INSERT INTO jobs (id, project_id, batch_id, status, created_at, data) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET \
               project_id = excluded.project_id, \
               batch_id   = excluded.batch_id, \
               status     = excluded.status, \
               data       = excluded.data",
        )
        .bind(job.id.to_string())
        .bind(job.project_id.to_string())
        .bind(job.batch_id.map(|id| id.to_string()))
        .bind(status_str(job.status))
        .bind(job.created_at.to_rfc3339())
        .bind(serde_json::to_string(job)?)
        .execute(&self.pool)
        .await
        .map_err(store_err)?;
        Ok(())
    }
}

#[async_trait]
impl JobQueue for SqliteJobQueue {
    async fn enqueue(&self, mut job: Job) -> Result<()> {
        if job.queue_position.is_none() {
            job.queue_position = Some(self.counter.fetch_add(1, Ordering::Relaxed));
        }
        job.status = JobStatus::Queued;
        job.updated_at = Utc::now();
        self.upsert(&job).await?;
        self.notify.notify_one();
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<Job>> {
        let mut tx = self.pool.begin().await.map_err(store_err)?;
        let sql = format!("SELECT data FROM jobs WHERE status = 'queued' {ORDER_BY} LIMIT 1");
        let row = sqlx::query(&sql)
            .fetch_optional(&mut *tx)
            .await
            .map_err(store_err)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut job: Job = decode_data(&row)?;

        let now = Utc::now();
        job.status = JobStatus::Running;
        job.started_at = Some(now);
        job.updated_at = now;

        sqlx::query("UPDATE jobs SET status = 'running', data = ?2 WHERE id = ?1")
            .bind(job.id.to_string())
            .bind(serde_json::to_string(&job)?)
            .execute(&mut *tx)
            .await
            .map_err(store_err)?;
        tx.commit().await.map_err(store_err)?;
        Ok(Some(job))
    }

    async fn peek(&self) -> Result<Option<Job>> {
        let sql = format!("SELECT data FROM jobs WHERE status = 'queued' {ORDER_BY} LIMIT 1");
        let row = sqlx::query(&sql)
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?;
        row.as_ref().map(decode_data).transpose()
    }

    async fn cancel(&self, id: JobId) -> Result<()> {
        let row = sqlx::query("SELECT data FROM jobs WHERE id = ?1 AND status = 'queued'")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?
            .ok_or_else(|| CoreError::NotFound(format!("queued job {id}")))?;
        let mut job: Job = decode_data(&row)?;
        job.status = JobStatus::Cancelled;
        job.updated_at = Utc::now();
        job.finished_at = Some(Utc::now());
        sqlx::query("UPDATE jobs SET status = 'cancelled', data = ?2 WHERE id = ?1")
            .bind(id.to_string())
            .bind(serde_json::to_string(&job)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn reorder(&self, id: JobId, position: usize) -> Result<()> {
        let sql = format!("SELECT data FROM jobs WHERE status = 'queued' {ORDER_BY}");
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        let mut jobs: Vec<Job> = rows.iter().map(decode_data).collect::<Result<_>>()?;

        let current = jobs
            .iter()
            .position(|j| j.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("queued job {id}")))?;

        // Preserve the existing multiset of position keys and just permute which
        // job owns which — future enqueues (counter > all keys) still append.
        let mut keys: Vec<i64> = jobs
            .iter()
            .enumerate()
            .map(|(i, j)| j.queue_position.unwrap_or(i as i64))
            .collect();
        keys.sort_unstable();

        let job = jobs.remove(current);
        jobs.insert(position.min(jobs.len()), job);

        let mut tx = self.pool.begin().await.map_err(store_err)?;
        for (job, key) in jobs.iter_mut().zip(keys) {
            if job.queue_position == Some(key) {
                continue;
            }
            job.queue_position = Some(key);
            sqlx::query("UPDATE jobs SET data = ?2 WHERE id = ?1")
                .bind(job.id.to_string())
                .bind(serde_json::to_string(job)?)
                .execute(&mut *tx)
                .await
                .map_err(store_err)?;
        }
        tx.commit().await.map_err(store_err)?;
        Ok(())
    }

    async fn len(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM jobs WHERE status = 'queued'")
            .fetch_one(&self.pool)
            .await
            .map_err(store_err)?;
        let n: i64 = row.try_get("n").map_err(store_err)?;
        Ok(n as usize)
    }

    async fn wait_for_job(&self) {
        self.notify.notified().await;
    }
}

/// The stored string form of a [`JobStatus`] — matches the `serde` encoding.
fn status_str(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use chatur_core::ids::ProjectId;
    use chatur_core::model::Project;
    use chatur_core::traits::{JobRepo, ProjectRepo};

    async fn db_with_project() -> (Database, ProjectId) {
        let db = Database::in_memory().await.unwrap();
        let project = Project::new("p", "/tmp/p");
        db.projects().create(&project).await.unwrap();
        (db, project.id)
    }

    fn queued(project: ProjectId, label: &str) -> Job {
        Job::new(project, label)
    }

    #[tokio::test]
    async fn dequeue_returns_jobs_in_fifo_order() {
        let (db, project) = db_with_project().await;
        let q = SqliteJobQueue::new(db.pool().clone());
        for label in ["a", "b", "c"] {
            q.enqueue(queued(project, label)).await.unwrap();
        }
        assert_eq!(q.dequeue().await.unwrap().unwrap().prompt, "a");
        assert_eq!(q.dequeue().await.unwrap().unwrap().prompt, "b");
        assert_eq!(q.dequeue().await.unwrap().unwrap().prompt, "c");
        assert!(q.dequeue().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn dequeue_marks_job_running_and_persists() {
        let (db, project) = db_with_project().await;
        let q = SqliteJobQueue::new(db.pool().clone());
        let job = queued(project, "a");
        let id = job.id;
        q.enqueue(job).await.unwrap();

        let dq = q.dequeue().await.unwrap().unwrap();
        assert_eq!(dq.status, JobStatus::Running);
        assert!(dq.started_at.is_some());
        // The row in the DB reflects the running transition.
        let stored = db.jobs().get(id).await.unwrap();
        assert_eq!(stored.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn len_counts_only_queued() {
        let (db, project) = db_with_project().await;
        let q = SqliteJobQueue::new(db.pool().clone());
        q.enqueue(queued(project, "a")).await.unwrap();
        q.enqueue(queued(project, "b")).await.unwrap();
        assert_eq!(q.len().await.unwrap(), 2);
        q.dequeue().await.unwrap();
        assert_eq!(q.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn cancel_removes_a_queued_job() {
        let (db, project) = db_with_project().await;
        let q = SqliteJobQueue::new(db.pool().clone());
        let keep = queued(project, "keep");
        let doomed = queued(project, "doomed");
        let doomed_id = doomed.id;
        q.enqueue(keep).await.unwrap();
        q.enqueue(doomed).await.unwrap();

        q.cancel(doomed_id).await.unwrap();
        assert_eq!(q.len().await.unwrap(), 1);
        assert_eq!(q.peek().await.unwrap().unwrap().prompt, "keep");
        assert!(matches!(
            q.cancel(JobId::new()).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn reorder_moves_a_job_to_the_front() {
        let (db, project) = db_with_project().await;
        let q = SqliteJobQueue::new(db.pool().clone());
        q.enqueue(queued(project, "a")).await.unwrap();
        q.enqueue(queued(project, "b")).await.unwrap();
        let last = queued(project, "c");
        let last_id = last.id;
        q.enqueue(last).await.unwrap();

        q.reorder(last_id, 0).await.unwrap();
        assert_eq!(q.dequeue().await.unwrap().unwrap().prompt, "c");
        assert_eq!(q.dequeue().await.unwrap().unwrap().prompt, "a");
    }

    #[tokio::test]
    async fn survives_a_fresh_queue_over_the_same_pool() {
        // Simulates a restart: a new queue handle over the same DB still sees
        // the queued rows (durability).
        let (db, project) = db_with_project().await;
        {
            let q = SqliteJobQueue::new(db.pool().clone());
            q.enqueue(queued(project, "a")).await.unwrap();
            q.enqueue(queued(project, "b")).await.unwrap();
        }
        let q2 = SqliteJobQueue::new(db.pool().clone());
        assert_eq!(q2.len().await.unwrap(), 2);
        assert_eq!(q2.dequeue().await.unwrap().unwrap().prompt, "a");
    }
}
