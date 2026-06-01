//! [`JobRepo`] over the `jobs` table.

use async_trait::async_trait;
use sqlx::SqlitePool;

use netra_core::ids::{JobId, ProjectId};
use netra_core::model::{Job, JobStatus};
use netra_core::traits::JobRepo;
use netra_core::{CoreError, Result};

use super::{decode_data, store_err};

/// SQLite-backed [`JobRepo`].
#[derive(Debug, Clone)]
pub struct SqliteJobRepo {
    pool: SqlitePool,
}

impl SqliteJobRepo {
    /// Wraps a connection pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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

#[async_trait]
impl JobRepo for SqliteJobRepo {
    async fn create(&self, job: &Job) -> Result<()> {
        sqlx::query(
            "INSERT INTO jobs (id, project_id, batch_id, status, created_at, data) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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

    async fn get(&self, id: JobId) -> Result<Job> {
        let row = sqlx::query("SELECT data FROM jobs WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?
            .ok_or_else(|| CoreError::NotFound(format!("job {id}")))?;
        decode_data(&row)
    }

    async fn update(&self, job: &Job) -> Result<()> {
        let result = sqlx::query(
            "UPDATE jobs SET project_id = ?1, batch_id = ?2, status = ?3, data = ?4 \
             WHERE id = ?5",
        )
        .bind(job.project_id.to_string())
        .bind(job.batch_id.map(|id| id.to_string()))
        .bind(status_str(job.status))
        .bind(serde_json::to_string(job)?)
        .bind(job.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(store_err)?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!("job {}", job.id)));
        }
        Ok(())
    }

    async fn list_by_status(&self, status: JobStatus) -> Result<Vec<Job>> {
        let rows = sqlx::query("SELECT data FROM jobs WHERE status = ?1 ORDER BY created_at")
            .bind(status_str(status))
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }

    async fn list_by_project(&self, project_id: ProjectId) -> Result<Vec<Job>> {
        let rows = sqlx::query("SELECT data FROM jobs WHERE project_id = ?1 ORDER BY created_at")
            .bind(project_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }

    async fn delete(&self, id: JobId) -> Result<()> {
        let result = sqlx::query("DELETE FROM jobs WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!("job {id}")));
        }
        Ok(())
    }

    async fn delete_by_status_in_project(
        &self,
        project_id: ProjectId,
        statuses: &[JobStatus],
    ) -> Result<u64> {
        if statuses.is_empty() {
            return Ok(0);
        }
        let placeholders = (0..statuses.len())
            .map(|i| format!("?{}", i + 2))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "DELETE FROM jobs WHERE project_id = ?1 AND status IN ({placeholders})"
        );
        let mut query = sqlx::query(&sql).bind(project_id.to_string());
        for status in statuses {
            query = query.bind(status_str(*status));
        }
        let result = query.execute(&self.pool).await.map_err(store_err)?;
        Ok(result.rows_affected())
    }
}
