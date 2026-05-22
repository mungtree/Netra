//! `chatur-engine` — scheduling, batch orchestration, and aggregation.
//!
//! Hosts the job queue and scheduler/worker pool, the `JobRunner`, the
//! `BatchExecutor` (map/reduce over targets), and the registry of
//! [`Aggregator`](chatur_core::traits::Aggregator) strategies.
//!
//! P3 builds the job queue first; the scheduler, `JobRunner`, retry, and
//! cancellation follow. Batches and aggregators land in P5.

mod queue;

pub use chatur_core;
pub use queue::InMemoryJobQueue;
