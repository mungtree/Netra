//! The domain-event bus.

use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::ids::{BatchId, JobId};
use crate::model::AgentEvent;

/// A high-level event broadcast to interested observers (the UI, loggers).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DomainEvent {
    /// A job entered the queue.
    JobQueued {
        /// The job.
        job_id: JobId,
    },
    /// A job began executing.
    JobStarted {
        /// The job.
        job_id: JobId,
    },
    /// A job emitted an agent event.
    JobProgress {
        /// The job.
        job_id: JobId,
        /// The underlying agent event.
        event: AgentEvent,
    },
    /// A job finished successfully.
    JobCompleted {
        /// The job.
        job_id: JobId,
    },
    /// A job failed.
    JobFailed {
        /// The job.
        job_id: JobId,
        /// Failure description.
        error: String,
    },
    /// A batch began executing its map step.
    BatchStarted {
        /// The batch.
        batch_id: BatchId,
    },
    /// A batch finished, including its aggregated result.
    BatchCompleted {
        /// The batch.
        batch_id: BatchId,
    },
    /// A batch could not produce a result.
    BatchFailed {
        /// The batch.
        batch_id: BatchId,
        /// Failure description.
        error: String,
    },
    /// Structured planner began consolidating a batch's outputs.
    PlannerStarted {
        /// The batch whose reduce step is running.
        batch_id: BatchId,
        /// Number of source outputs feeding the planner.
        source_count: usize,
    },
    /// Structured planner finished (success or fail). UI uses this to clear
    /// the "reviewing…" spinner.
    PlannerFinished {
        /// The batch whose reduce step is running.
        batch_id: BatchId,
        /// True when the planner returned a valid report.
        success: bool,
    },
    /// A chroma-enabled job ran without the ChromaDB context it asked for —
    /// because the server was down, the helper venv was unavailable, etc.
    /// Emitted at spec-resolve time so the UI can warn the user.
    ChromaPromptDegraded {
        /// The job whose chroma context was dropped.
        job_id: JobId,
        /// Human-readable reason.
        reason: String,
    },
}

/// Publish/subscribe channel for [`DomainEvent`]s.
///
/// `netra-engine` publishes; `src-tauri` subscribes to forward events to the
/// frontend.
pub trait EventBus: Send + Sync {
    /// Broadcasts an event to all current subscribers.
    fn publish(&self, event: DomainEvent);

    /// Returns a stream of events published from now on.
    fn subscribe(&self) -> BoxStream<'static, DomainEvent>;
}
