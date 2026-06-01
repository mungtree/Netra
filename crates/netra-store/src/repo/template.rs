//! [`TemplateRepo`] over the `templates` table.

use async_trait::async_trait;
use sqlx::SqlitePool;

use netra_core::ids::TemplateId;
use netra_core::model::PromptTemplate;
use netra_core::traits::TemplateRepo;
use netra_core::{CoreError, Result};

use super::{decode_data, store_err};

/// SQLite-backed [`TemplateRepo`].
#[derive(Debug, Clone)]
pub struct SqliteTemplateRepo {
    pool: SqlitePool,
}

impl SqliteTemplateRepo {
    /// Wraps a connection pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TemplateRepo for SqliteTemplateRepo {
    async fn create(&self, template: &PromptTemplate) -> Result<()> {
        sqlx::query("INSERT INTO templates (id, name, data) VALUES (?1, ?2, ?3)")
            .bind(template.id.to_string())
            .bind(template.name.as_str())
            .bind(serde_json::to_string(template)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn get(&self, id: TemplateId) -> Result<PromptTemplate> {
        let row = sqlx::query("SELECT data FROM templates WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?
            .ok_or_else(|| CoreError::NotFound(format!("template {id}")))?;
        decode_data(&row)
    }

    async fn list(&self) -> Result<Vec<PromptTemplate>> {
        let rows = sqlx::query("SELECT data FROM templates ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }
}
