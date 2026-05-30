//! `chatur-engine` — scheduling, batch orchestration, and aggregation.
//!
//! Single-job execution (P3):
//! - [`InMemoryJobQueue`] — the pending-work queue.
//! - [`BroadcastEventBus`] — fan-out of [`DomainEvent`](chatur_core::traits::DomainEvent)s.
//! - [`RetryPolicy`] — exponential backoff for transient failures.
//! - [`JobRunner`] — runs one job: agent turn, sinks, retry, cancellation.
//! - [`Scheduler`] — drains the queue into the runner under a concurrency cap.
//!
//! Batch orchestration (P5):
//! - [`BatchExecutor`] — fans a batch's prompts × targets into jobs, then
//!   reduces their outputs.
//! - [`AggregatorRegistry`] — the keyed set of reduce strategies, with the
//!   built-in [`ConcatAggregator`] and [`SchemaMergeAggregator`].

mod aggregate;
mod batch;
mod bus;
mod planner;
mod queue;
mod retry;
mod runner;
mod scheduler;

pub use chatur_core;

pub use aggregate::{AggregatorRegistry, ConcatAggregator, SchemaMergeAggregator};
pub use batch::BatchExecutor;
pub use bus::BroadcastEventBus;
pub use planner::{MockPlanner, NullPlanner, OutlinesHttpPlanner, StructuredPlanner};
pub use queue::InMemoryJobQueue;
pub use retry::RetryPolicy;
pub use runner::JobRunner;
pub use scheduler::{Scheduler, SpecResolver};
