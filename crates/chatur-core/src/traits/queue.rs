//! The job queue interface.

use async_trait::async_trait;

use crate::Result;
use crate::ids::JobId;
use crate::model::Job;

/// An ordered, mutable queue of pending [`Job`]s.
///
/// Implementations may be in-memory or persistent; the scheduler in
/// `chatur-engine` depends only on this trait.
#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Appends a job to the back of the queue.
    async fn enqueue(&self, job: Job) -> Result<()>;

    /// Removes and returns the next job, or `None` if the queue is empty.
    async fn dequeue(&self) -> Result<Option<Job>>;

    /// Returns the next job without removing it.
    async fn peek(&self) -> Result<Option<Job>>;

    /// Removes a queued job by id.
    async fn cancel(&self, id: JobId) -> Result<()>;

    /// Moves a queued job to a new zero-based position.
    async fn reorder(&self, id: JobId, position: usize) -> Result<()>;

    /// Returns the number of queued jobs.
    async fn len(&self) -> Result<usize>;

    /// Returns `true` when the queue holds no jobs.
    async fn is_empty(&self) -> Result<bool> {
        Ok(self.len().await? == 0)
    }
}
