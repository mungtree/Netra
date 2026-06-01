//! `netra-store` — persistence and logging for NETRA.
//!
//! Provides SQLite-backed implementations (via `sqlx`) of the repository
//! traits in [`netra_core::traits`], plus [`FileLogSink`], a
//! [`OutputSink`](netra_core::traits::OutputSink) that writes per-job event
//! logs.
//!
//! # Storage model
//! Every entity is stored as a JSON `data` blob — the source of truth —
//! alongside a few promoted columns for indexing and foreign keys. Domain
//! structs can gain fields without a schema migration.
//!
//! Open a [`Database`], then take repositories from it:
//! ```no_run
//! # async fn demo() -> netra_core::Result<()> {
//! let db = netra_store::Database::connect("netra.db").await?;
//! let projects = db.projects();
//! # let _ = projects;
//! # Ok(())
//! # }
//! ```

mod db;
mod queue;
mod repo;
mod sink;

pub use netra_core;
pub use db::Database;
pub use queue::SqliteJobQueue;
pub use repo::{SqliteBatchRepo, SqliteJobRepo, SqliteProjectRepo, SqliteTemplateRepo};
pub use sink::FileLogSink;
