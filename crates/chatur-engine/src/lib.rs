//! `chatur-engine` — scheduling, batch orchestration, and aggregation.
//!
//! Hosts the job scheduler/worker pool, the `JobRunner`, the `BatchExecutor`
//! (map/reduce over targets), and the registry of
//! [`Aggregator`](chatur_core::traits::Aggregator) strategies.
//!
//! **P0 scaffold.** The scheduler lands in P3, batches and aggregators in P5.

/// Re-export of the domain crate this layer builds upon.
pub use chatur_core;
