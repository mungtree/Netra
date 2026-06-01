//! SQLite-backed repository implementations.
//!
//! Each repository maps a domain struct to a JSON `data` column plus promoted
//! index/foreign-key columns. Reads deserialize `data`; writes refresh both.

mod batch;
mod job;
mod project;
mod template;

pub use batch::SqliteBatchRepo;
pub use job::SqliteJobRepo;
pub use project::SqliteProjectRepo;
pub use template::SqliteTemplateRepo;

use serde::de::DeserializeOwned;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

use netra_core::{CoreError, Result};

/// Maps an `sqlx` error to a [`CoreError::Storage`].
pub(crate) fn store_err(error: sqlx::Error) -> CoreError {
    CoreError::Storage(error.to_string())
}

/// Deserializes the `data` column of `row` into a domain type.
pub(crate) fn decode_data<T: DeserializeOwned>(row: &SqliteRow) -> Result<T> {
    let data: String = row.try_get("data").map_err(store_err)?;
    serde_json::from_str(&data).map_err(CoreError::from)
}
