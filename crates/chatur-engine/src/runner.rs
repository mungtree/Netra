//! [`JobRunner`] — executes one job: agent turn, sinks, retry, cancellation.

use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use chatur_agent::{AgentPool, AgentSpec};
use chatur_core::ids::JobId;
use chatur_core::model::{AgentEvent, AgentOutput, Job, JobStatus, TokenUsage};
use chatur_core::traits::{DomainEvent, EventBus, JobRepo, OutputSink, PromptRequest};
use chatur_core::{CoreError, Result};

use crate::retry::RetryPolicy;

/// Runs a single job to completion.
///
/// For each job the runner acquires an agent from the [`AgentPool`], streams
/// the turn's events to every [`OutputSink`] and to the [`EventBus`], retries
/// transient failures per the [`RetryPolicy`], and persists the final status
/// through the [`JobRepo`].
pub struct JobRunner {
    pool: AgentPool,
    jobs: Arc<dyn JobRepo>,
    bus: Arc<dyn EventBus>,
    sinks: Vec<Arc<dyn OutputSink>>,
    retry: RetryPolicy,
}

impl JobRunner {
    /// Assembles a runner from its collaborators.
    #[must_use]
    pub fn new(
        pool: AgentPool,
        jobs: Arc<dyn JobRepo>,
        bus: Arc<dyn EventBus>,
        sinks: Vec<Arc<dyn OutputSink>>,
        retry: RetryPolicy,
    ) -> Self {
        Self {
            pool,
            jobs,
            bus,
            sinks,
            retry,
        }
    }

    /// Runs `job`, retrying transient failures, until it completes, fails, or
    /// is cancelled via `cancel`.
    ///
    /// The job's status is persisted as `Running` on entry and as its terminal
    /// state on exit.
    ///
    /// # Errors
    /// Returns the last failure if every attempt fails, or
    /// [`CoreError::Cancelled`] if `cancel` fired.
    pub async fn run(
        &self,
        mut job: Job,
        spec: AgentSpec,
        cancel: CancellationToken,
    ) -> Result<AgentOutput> {
        job.status = JobStatus::Running;
        job.updated_at = Utc::now();
        let _ = self.jobs.update(&job).await;
        self.bus.publish(DomainEvent::JobStarted { job_id: job.id });

        let mut attempt = 0u32;
        let outcome = loop {
            attempt += 1;
            job.attempts = attempt;
            match self.attempt(&job, &spec, &cancel).await {
                Ok(output) => break Ok(output),
                Err(error) => {
                    if RetryPolicy::is_retryable(&error) && attempt < self.retry.max_attempts {
                        let delay = self.retry.delay_for(attempt);
                        tracing::warn!(
                            job = %job.id, attempt, ?delay, %error,
                            "job attempt failed, retrying",
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    break Err(error);
                }
            }
        };

        job.updated_at = Utc::now();
        match &outcome {
            Ok(_) => {
                job.status = JobStatus::Completed;
                self.bus
                    .publish(DomainEvent::JobCompleted { job_id: job.id });
            }
            Err(CoreError::Cancelled) => {
                job.status = JobStatus::Cancelled;
                self.bus.publish(DomainEvent::JobFailed {
                    job_id: job.id,
                    error: "cancelled".to_string(),
                });
            }
            Err(error) => {
                job.status = JobStatus::Failed;
                self.bus.publish(DomainEvent::JobFailed {
                    job_id: job.id,
                    error: error.to_string(),
                });
            }
        }
        let _ = self.jobs.update(&job).await;
        outcome
    }

    /// One execution attempt — acquire an agent, run the turn, collect output.
    async fn attempt(
        &self,
        job: &Job,
        spec: &AgentSpec,
        cancel: &CancellationToken,
    ) -> Result<AgentOutput> {
        let lease = self.pool.acquire(job.project_id, spec.clone()).await?;
        let transport = lease.transport();
        let mut stream = transport
            .send_prompt(PromptRequest::new(job.prompt.clone()))
            .await?;

        let mut text = String::new();
        let mut usage = TokenUsage::default();
        let mut failure: Option<String> = None;
        let mut cancelled = false;

        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
                event = stream.next() => {
                    match event {
                        Some(event) => {
                            self.dispatch(job.id, &event, &mut text, &mut usage, &mut failure)
                                .await;
                        }
                        None => break,
                    }
                }
            }
        }

        for sink in &self.sinks {
            let _ = sink.flush(job.id).await;
        }

        if cancelled {
            let _ = transport.abort().await;
            let _ = lease.release().await;
            return Err(CoreError::Cancelled);
        }
        let _ = lease.release().await;

        if let Some(message) = failure {
            return Err(CoreError::Agent(message));
        }
        // If the job asked for a defined output, its text may itself be JSON.
        let structured = serde_json::from_str(text.trim()).ok();
        Ok(AgentOutput {
            text,
            structured,
            usage,
        })
    }

    /// Fans one event out to the sinks and the bus, and folds it into output.
    async fn dispatch(
        &self,
        job_id: JobId,
        event: &AgentEvent,
        text: &mut String,
        usage: &mut TokenUsage,
        failure: &mut Option<String>,
    ) {
        for sink in &self.sinks {
            let _ = sink.on_event(job_id, event).await;
        }
        self.bus.publish(DomainEvent::JobProgress {
            job_id,
            event: event.clone(),
        });
        match event {
            AgentEvent::AssistantText { text: delta } => text.push_str(delta),
            AgentEvent::Usage(reported) => *usage += *reported,
            AgentEvent::Error { message } => *failure = Some(message.clone()),
            _ => {}
        }
    }
}
