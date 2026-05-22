//! `chatur-engine` — scheduling, batch orchestration, and aggregation.
//!
//! P3 delivers single-job execution:
//! - [`InMemoryJobQueue`] — the pending-work queue.
//! - [`BroadcastEventBus`] — fan-out of [`DomainEvent`](chatur_core::traits::DomainEvent)s.
//! - [`RetryPolicy`] — exponential backoff for transient failures.
//! - [`JobRunner`] — runs one job: agent turn, sinks, retry, cancellation.
//! - [`Scheduler`] — drains the queue into the runner under a concurrency cap.
//!
//! Batch orchestration and the aggregator registry land in P5.

mod bus;
mod queue;
mod retry;
mod runner;
mod scheduler;

pub use chatur_core;

pub use bus::BroadcastEventBus;
pub use queue::InMemoryJobQueue;
pub use retry::RetryPolicy;
pub use runner::JobRunner;
pub use scheduler::{Scheduler, SpecResolver};
