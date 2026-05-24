//! [`JobRunner`] — executes one job: agent turn, sinks, retry, cancellation.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

const WRAP_UP_MESSAGE: &str = "You have been working on this task for too long. \
    Stop immediately and provide your final response now. Summarize what you have \
    found so far and do not begin any new work. If the task is incomplete, give \
    your best answer based on what you have gathered so far.";


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
    /// When `Some`, a soft interrupt fires after this duration: the current
    /// turn is aborted and a wrap-up prompt is sent so the model can finish.
    interrupt_timeout: Option<Duration>,
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
        interrupt_timeout: Option<Duration>,
    ) -> Self {
        Self {
            pool,
            jobs,
            bus,
            sinks,
            retry,
            interrupt_timeout,
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
        let start = Utc::now();
        job.status = JobStatus::Running;
        job.updated_at = start;
        if job.started_at.is_none() {
            job.started_at = Some(start);
        }
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

        let end = Utc::now();
        job.updated_at = end;
        job.finished_at = Some(end);
        match &outcome {
            Ok(output) => {
                job.status = JobStatus::Completed;
                job.output = Some(output.clone());
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

        let prompt_event = AgentEvent::Prompt { text: job.prompt.clone() };
        self.dispatch(job.id, &prompt_event, &mut job.prompt.clone(), &mut TokenUsage::default(), &mut None).await;

        let mut text = String::new();
        let mut usage = TokenUsage::default();
        let mut failure: Option<String> = None;
        let mut cancelled = false;

        // Build an optional sleep future for the soft-interrupt timeout.
        let sleep = self.interrupt_timeout.map(tokio::time::sleep);
        tokio::pin!(sleep);
        let mut timed_out = false;

        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => {
                    cancelled = true;
                    break;
                }
                // Soft interrupt: abort the current turn and send a wrap-up prompt.
                () = async {
                    if let Some(s) = sleep.as_mut().as_pin_mut() { s.await }
                    else { std::future::pending::<()>().await }
                }, if !timed_out => {
                    timed_out = true;
                    tracing::info!(job = %job.id, "job timeout — sending soft interrupt");
                    // transport.abort().await.ok();
                    // Drain remaining events so the transport is ready for a new turn.
                    // while stream.next().await.is_some() {}

                    match transport.send_steer(PromptRequest::new(WRAP_UP_MESSAGE)).await {
                        Ok(_s) => { 
                            // stream = s;

                            // Notify of wrapup prompt
                            let mut wrapup_text = WRAP_UP_MESSAGE.to_string();
                            let wrapup_event = AgentEvent::Prompt { text: wrapup_text.clone() };
                            self.dispatch(job.id, &wrapup_event, &mut wrapup_text, &mut TokenUsage::default(), &mut None).await;

                        },
                        Err(e) => {
                            tracing::warn!(job = %job.id, %e, "wrap-up prompt failed, ending turn");
                            let mut wrapup_err_str = "wrap-up prompt failed, ending turn".to_string();
                            let wrapup_event = AgentEvent::Error { message: wrapup_err_str.clone() };
                            self.dispatch(job.id, &wrapup_event, &mut wrapup_err_str, &mut TokenUsage::default(), &mut None).await;

                            break;
                        }
                    }
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
