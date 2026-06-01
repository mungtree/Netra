//! [`ProjectRepo`] over the `projects` table.

use async_trait::async_trait;
use sqlx::SqlitePool;

use netra_core::ids::ProjectId;
use netra_core::model::Project;
use netra_core::traits::ProjectRepo;
use netra_core::{CoreError, Result};

use super::{decode_data, store_err};

/// SQLite-backed [`ProjectRepo`].
#[derive(Debug, Clone)]
pub struct SqliteProjectRepo {
    pool: SqlitePool,
}

impl SqliteProjectRepo {
    /// Wraps a connection pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepo for SqliteProjectRepo {
    async fn create(&self, project: &Project) -> Result<()> {
        sqlx::query("INSERT INTO projects (id, name, data) VALUES (?1, ?2, ?3)")
            .bind(project.id.to_string())
            .bind(project.name.as_str())
            .bind(serde_json::to_string(project)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn get(&self, id: ProjectId) -> Result<Project> {
        let row = sqlx::query("SELECT data FROM projects WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?
            .ok_or_else(|| CoreError::NotFound(format!("project {id}")))?;
        decode_data(&row)
    }

    async fn update(&self, project: &Project) -> Result<()> {
        let result = sqlx::query("UPDATE projects SET name = ?2, data = ?3 WHERE id = ?1")
            .bind(project.id.to_string())
            .bind(project.name.as_str())
            .bind(serde_json::to_string(project)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!("project {}", project.id)));
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Project>> {
        let rows = sqlx::query("SELECT data FROM projects ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }

    async fn delete(&self, id: ProjectId) -> Result<()> {
        let result = sqlx::query("DELETE FROM projects WHERE id = ?1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(format!("project {id}")));
        }
        Ok(())
    }
}
