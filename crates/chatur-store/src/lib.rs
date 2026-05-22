//! `chatur-store` — persistence and logging for Mini ChatUR.
//!
//! Provides SQLite-backed implementations (via `sqlx`) of the repository
//! traits in [`chatur_core::traits`], plus a `FileLogSink` implementation of
//! [`OutputSink`](chatur_core::traits::OutputSink).
//!
//! **P0 scaffold.** Schema, migrations, and repositories land in P2.

/// Re-export of the domain crate this layer builds upon.
pub use chatur_core;
