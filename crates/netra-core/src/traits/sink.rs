//! Output sinks — observers of per-job event streams.

use async_trait::async_trait;

use crate::Result;
use crate::ids::JobId;
use crate::model::AgentEvent;

/// A consumer of a job's event stream (the Observer pattern).
///
/// `netra-store`'s `FileLogSink` writes events to per-job log files; future
/// sinks could forward to webhooks or metrics systems.
#[async_trait]
pub trait OutputSink: Send + Sync {
    /// Stable sink id, for configuration and diagnostics.
    fn id(&self) -> &str;

    /// Handles one event for the given job.
    async fn on_event(&self, job_id: JobId, event: &AgentEvent) -> Result<()>;

    /// Flushes any buffered output for a finished job.
    async fn flush(&self, job_id: JobId) -> Result<()>;
}
