//! `chatur-store` — persistence and logging for Mini ChatUR.
//!
//! Provides SQLite-backed implementations (via `sqlx`) of the repository
//! traits in [`chatur_core::traits`], plus [`FileLogSink`], a
//! [`OutputSink`](chatur_core::traits::OutputSink) that writes per-job event
//! logs.
//!
//! # Storage model
//! Every entity is stored as a JSON `data` blob — the source of truth —
//! alongside a few promoted columns for indexing and foreign keys. Domain
//! structs can gain fields without a schema migration.
//!
//! Open a [`Database`], then take repositories from it:
//! ```no_run
//! # async fn demo() -> chatur_core::Result<()> {
//! let db = chatur_store::Database::connect("chatur.db").await?;
//! let projects = db.projects();
//! # let _ = projects;
//! # Ok(())
//! # }
//! ```

mod db;
mod repo;
mod sink;

pub use chatur_core;
pub use db::Database;
pub use repo::{SqliteBatchRepo, SqliteJobRepo, SqliteProjectRepo, SqliteTemplateRepo};
pub use sink::FileLogSink;
