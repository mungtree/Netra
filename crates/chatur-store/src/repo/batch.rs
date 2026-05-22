//! [`BatchRepo`] over the `batches` and `batch_items` tables.

use async_trait::async_trait;
use sqlx::SqlitePool;

use chatur_core::ids::BatchId;
use chatur_core::model::{Batch, BatchItem};
use chatur_core::traits::BatchRepo;
use chatur_core::{CoreError, Result};

use super::{decode_data, store_err};

/// SQLite-backed [`BatchRepo`].
#[derive(Debug, Clone)]
pub struct SqliteBatchRepo {
    pool: SqlitePool,
}

impl SqliteBatchRepo {
    /// Wraps a connection pool.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BatchRepo for SqliteBatchRepo {
    async fn create(&self, batch: &Batch) -> Result<()> {
        sqlx::query("INSERT INTO batches (id, name, data) VALUES (?1, ?2, ?3)")
            .bind(batch.id.to_string())
            .bind(batch.name.as_str())
            .bind(serde_json::to_string(batch)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn get(&self, id: BatchId) -> Result<Batch> {
        let row = sqlx::query("SELECT data FROM batches WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(store_err)?
            .ok_or_else(|| CoreError::NotFound(format!("batch {id}")))?;
        decode_data(&row)
    }

    async fn update(&self, batch: &Batch) -> Result<()> {
        let affected = sqlx::query("UPDATE batches SET name = ?2, data = ?3 WHERE id = ?1")
            .bind(batch.id.to_string())
            .bind(batch.name.as_str())
            .bind(serde_json::to_string(batch)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?
            .rows_affected();
        if affected == 0 {
            return Err(CoreError::NotFound(format!("batch {}", batch.id)));
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Batch>> {
        let rows = sqlx::query("SELECT data FROM batches ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }

    async fn add_item(&self, item: &BatchItem) -> Result<()> {
        sqlx::query("INSERT INTO batch_items (id, batch_id, data) VALUES (?1, ?2, ?3)")
            .bind(item.id.to_string())
            .bind(item.batch_id.to_string())
            .bind(serde_json::to_string(item)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?;
        Ok(())
    }

    async fn update_item(&self, item: &BatchItem) -> Result<()> {
        let affected = sqlx::query("UPDATE batch_items SET data = ?2 WHERE id = ?1")
            .bind(item.id.to_string())
            .bind(serde_json::to_string(item)?)
            .execute(&self.pool)
            .await
            .map_err(store_err)?
            .rows_affected();
        if affected == 0 {
            return Err(CoreError::NotFound(format!("batch item {}", item.id)));
        }
        Ok(())
    }

    async fn items(&self, batch_id: BatchId) -> Result<Vec<BatchItem>> {
        let rows = sqlx::query("SELECT data FROM batch_items WHERE batch_id = ?1")
            .bind(batch_id.to_string())
            .fetch_all(&self.pool)
            .await
            .map_err(store_err)?;
        rows.iter().map(decode_data).collect()
    }
}
