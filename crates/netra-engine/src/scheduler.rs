//! [`Scheduler`] — drains the job queue into the runner under a concurrency cap.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use netra_agent::AgentSpec;
use netra_core::ids::JobId;
use netra_core::model::Job;
use netra_core::traits::JobQueue;
use netra_core::{CoreError, Result};

use crate::runner::JobRunner;

/// Resolves the [`AgentSpec`] for a job — typically by looking up its project.
///
/// Kept abstract so the scheduler does not depend on `netra-store`; the API
/// layer provides the concrete, project-aware implementation.
#[async_trait]
pub trait SpecResolver: Send + Sync {
    /// Builds the launch spec for `job`.
    async fn resolve(&self, job: &Job) -> Result<AgentSpec>;
}

/// Continuously moves jobs from the queue to the [`JobRunner`].
///
/// At most `concurrency` jobs run at once; the queue itself is the backlog.
pub struct Scheduler {
    queue: Arc<dyn JobQueue>,
    runner: Arc<JobRunner>,
    resolver: Arc<dyn SpecResolver>,
    concurrency: usize,
    running: Mutex<HashMap<JobId, CancellationToken>>,
}

impl Scheduler {
    /// Assembles a scheduler. `concurrency` is clamped to at least 1.
    #[must_use]
    pub fn new(
        queue: Arc<dyn JobQueue>,
        runner: Arc<JobRunner>,
        resolver: Arc<dyn SpecResolver>,
        concurrency: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            queue,
            runner,
            resolver,
            concurrency: concurrency.max(1),
            running: Mutex::new(HashMap::new()),
        })
    }

    /// Runs the dispatch loop until `shutdown` is cancelled.
    ///
    /// Jobs already in flight are left to finish; only the loop stops.
    pub async fn run(self: Arc<Self>, shutdown: CancellationToken) {
        let permits = Arc::new(Semaphore::new(self.concurrency));

        loop {
            // Hold a permit before taking a job, so no more than `concurrency`
            // jobs are ever in flight.
            let permit = tokio::select! {
                biased;
                () = shutdown.cancelled() => break,
                permit = permits.clone().acquire_owned() => match permit {
                    Ok(permit) => permit,
                    Err(_) => break,
                },
            };

            let job = match self.queue.dequeue().await {
                Ok(Some(job)) => job,
                Ok(None) => {
                    drop(permit);
                    tokio::select! {
                        biased;
                        () = shutdown.cancelled() => break,
                        () = self.queue.wait_for_job() => continue,
                    }
                }
                Err(error) => {
                    tracing::error!(%error, "failed to dequeue job");
                    drop(permit);
                    continue;
                }
            };

            let scheduler = self.clone();
            tokio::spawn(async move {
                let _permit = permit; // released when the job task ends
                scheduler.execute(job).await;
            });
        }

        tracing::debug!("scheduler dispatch loop stopped");
    }

    /// Resolves the spec for `job` and runs it, tracking its cancellation token.
    async fn execute(&self, job: Job) {
        let job_id = job.id;
        let spec = match self.resolver.resolve(&job).await {
            Ok(spec) => spec,
            Err(error) => {
                tracing::error!(job = %job_id, %error, "could not resolve agent spec");
                return;
            }
        };

        let cancel = CancellationToken::new();
        self.running.lock().await.insert(job_id, cancel.clone());

        if let Err(error) = self.runner.run(job, spec, cancel).await {
            tracing::warn!(job = %job_id, %error, "job finished with error");
        }

        self.running.lock().await.remove(&job_id);
    }

    /// Requests cancellation of a currently-running job.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if no job with `id` is running.
    pub async fn cancel_running(&self, id: JobId) -> Result<()> {
        match self.running.lock().await.get(&id) {
            Some(token) => {
                token.cancel();
                Ok(())
            }
            None => Err(CoreError::NotFound(format!("running job {id}"))),
        }
    }

    /// The number of jobs currently executing.
    pub async fn running_count(&self) -> usize {
        self.running.lock().await.len()
    }
}
