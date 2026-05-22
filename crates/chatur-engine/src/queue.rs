//! [`InMemoryJobQueue`] — an in-memory implementation of [`JobQueue`].

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use chatur_core::ids::JobId;
use chatur_core::model::Job;
use chatur_core::traits::JobQueue;
use chatur_core::{CoreError, Result};

/// Shared state behind every clone of an [`InMemoryJobQueue`].
#[derive(Debug, Default)]
struct Inner {
    /// Pending jobs, front (index 0) first.
    jobs: Mutex<Vec<Job>>,
    /// Signalled on every enqueue so a scheduler can block instead of poll.
    notify: Notify,
}

/// An in-memory FIFO job queue with manual reordering.
///
/// Cloning is cheap and shares state — all clones see the same queue, so the
/// API layer and the scheduler can each hold one.
#[derive(Debug, Clone, Default)]
pub struct InMemoryJobQueue {
    inner: Arc<Inner>,
}

impl InMemoryJobQueue {
    /// Creates an empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves the next time a job is enqueued.
    ///
    /// Lets a scheduler await new work rather than busy-polling. Standard
    /// [`Notify`] semantics apply: an enqueue that happens before this is
    /// awaited still wakes the next call.
    pub async fn wait_for_job(&self) {
        self.inner.notify.notified().await;
    }
}

#[async_trait]
impl JobQueue for InMemoryJobQueue {
    async fn enqueue(&self, job: Job) -> Result<()> {
        self.inner.jobs.lock().await.push(job);
        self.inner.notify.notify_one();
        Ok(())
    }

    async fn dequeue(&self) -> Result<Option<Job>> {
        let mut jobs = self.inner.jobs.lock().await;
        if jobs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(jobs.remove(0)))
        }
    }

    async fn peek(&self) -> Result<Option<Job>> {
        Ok(self.inner.jobs.lock().await.first().cloned())
    }

    async fn cancel(&self, id: JobId) -> Result<()> {
        let mut jobs = self.inner.jobs.lock().await;
        let before = jobs.len();
        jobs.retain(|job| job.id != id);
        if jobs.len() == before {
            return Err(CoreError::NotFound(format!("queued job {id}")));
        }
        Ok(())
    }

    async fn reorder(&self, id: JobId, position: usize) -> Result<()> {
        let mut jobs = self.inner.jobs.lock().await;
        let current = jobs
            .iter()
            .position(|job| job.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("queued job {id}")))?;
        let job = jobs.remove(current);
        // Clamp so a position past the end means "move to the back".
        let target = position.min(jobs.len());
        jobs.insert(target, job);
        Ok(())
    }

    async fn len(&self) -> Result<usize> {
        Ok(self.inner.jobs.lock().await.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chatur_core::ids::ProjectId;
    use std::time::Duration;

    /// A queued job whose prompt is `label`, for order assertions.
    fn job(label: &str) -> Job {
        Job::new(ProjectId::new(), label)
    }

    #[tokio::test]
    async fn dequeue_returns_jobs_in_fifo_order() {
        let queue = InMemoryJobQueue::new();
        for label in ["a", "b", "c"] {
            queue.enqueue(job(label)).await.unwrap();
        }
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "a");
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "b");
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "c");
    }

    #[tokio::test]
    async fn dequeue_on_empty_queue_is_none() {
        let queue = InMemoryJobQueue::new();
        assert!(queue.dequeue().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn peek_reports_front_without_removing() {
        let queue = InMemoryJobQueue::new();
        queue.enqueue(job("a")).await.unwrap();
        queue.enqueue(job("b")).await.unwrap();

        assert_eq!(queue.peek().await.unwrap().unwrap().prompt, "a");
        assert_eq!(queue.len().await.unwrap(), 2);
        assert!(!queue.is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn cancel_removes_a_queued_job() {
        let queue = InMemoryJobQueue::new();
        let target = job("doomed");
        let target_id = target.id;
        queue.enqueue(job("keep")).await.unwrap();
        queue.enqueue(target).await.unwrap();

        queue.cancel(target_id).await.unwrap();
        assert_eq!(queue.len().await.unwrap(), 1);
        assert_eq!(queue.peek().await.unwrap().unwrap().prompt, "keep");
    }

    #[tokio::test]
    async fn cancelling_an_unknown_job_is_not_found() {
        let queue = InMemoryJobQueue::new();
        assert!(matches!(
            queue.cancel(JobId::new()).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn reorder_moves_a_job_to_the_front() {
        let queue = InMemoryJobQueue::new();
        queue.enqueue(job("a")).await.unwrap();
        queue.enqueue(job("b")).await.unwrap();
        let last = job("c");
        let last_id = last.id;
        queue.enqueue(last).await.unwrap();

        queue.reorder(last_id, 0).await.unwrap();
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "c");
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "a");
    }

    #[tokio::test]
    async fn reorder_past_the_end_moves_to_the_back() {
        let queue = InMemoryJobQueue::new();
        let first = job("a");
        let first_id = first.id;
        queue.enqueue(first).await.unwrap();
        queue.enqueue(job("b")).await.unwrap();

        queue.reorder(first_id, 999).await.unwrap();
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "b");
        assert_eq!(queue.dequeue().await.unwrap().unwrap().prompt, "a");
    }

    #[tokio::test]
    async fn reordering_an_unknown_job_is_not_found() {
        let queue = InMemoryJobQueue::new();
        queue.enqueue(job("a")).await.unwrap();
        assert!(matches!(
            queue.reorder(JobId::new(), 0).await,
            Err(CoreError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn clones_share_one_queue() {
        let queue = InMemoryJobQueue::new();
        let clone = queue.clone();
        queue.enqueue(job("a")).await.unwrap();
        assert_eq!(clone.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn wait_for_job_resolves_when_a_job_arrives() {
        let queue = InMemoryJobQueue::new();
        let watcher = queue.clone();
        let waiter = tokio::spawn(async move { watcher.wait_for_job().await });

        // Let the waiter park, then enqueue.
        tokio::task::yield_now().await;
        queue.enqueue(job("a")).await.unwrap();

        tokio::time::timeout(Duration::from_millis(500), waiter)
            .await
            .expect("wait_for_job should resolve after an enqueue")
            .expect("waiter task should not panic");
    }
}
