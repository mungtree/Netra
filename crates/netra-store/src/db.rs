//! [`Database`] — opens the SQLite pool, runs migrations, vends repositories.

use std::path::Path;
use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use netra_core::{CoreError, Result};

use crate::repo::{SqliteBatchRepo, SqliteJobRepo, SqliteProjectRepo, SqliteTemplateRepo};

/// Embedded schema migrations, applied on every [`Database::connect`].
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A connection pool to the NETRA SQLite database.
///
/// Cloning a repository obtained from [`projects`](Self::projects) and friends
/// is cheap — they share this pool.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Opens (creating if absent) the database file at `path` and migrates it.
    ///
    /// # Errors
    /// Returns [`CoreError::Storage`] if the file cannot be opened or migrated.
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        Self::with_options(options, 5).await
    }

    /// Opens a private, in-memory database — intended for tests.
    ///
    /// # Errors
    /// Returns [`CoreError::Storage`] if the database cannot be initialized.
    pub async fn in_memory() -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .map_err(|e| CoreError::Storage(e.to_string()))?
            .foreign_keys(true);
        // One connection keeps the in-memory database alive for the pool's life.
        Self::with_options(options, 1).await
    }

    /// Shared constructor: builds the pool and runs migrations.
    async fn with_options(options: SqliteConnectOptions, max_connections: u32) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await
            .map_err(|e| CoreError::Storage(format!("failed to open database: {e}")))?;

        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| CoreError::Storage(format!("migration failed: {e}")))?;
        tracing::debug!("database ready");

        Ok(Self { pool })
    }

    /// The underlying connection pool, for advanced use.
    #[must_use]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// A repository over `projects`.
    #[must_use]
    pub fn projects(&self) -> SqliteProjectRepo {
        SqliteProjectRepo::new(self.pool.clone())
    }

    /// A repository over `jobs`.
    #[must_use]
    pub fn jobs(&self) -> SqliteJobRepo {
        SqliteJobRepo::new(self.pool.clone())
    }

    /// A repository over `batches` and `batch_items`.
    #[must_use]
    pub fn batches(&self) -> SqliteBatchRepo {
        SqliteBatchRepo::new(self.pool.clone())
    }

    /// A repository over `templates`.
    #[must_use]
    pub fn templates(&self) -> SqliteTemplateRepo {
        SqliteTemplateRepo::new(self.pool.clone())
    }
}
