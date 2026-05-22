//! A single agent query against one project.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BatchId, JobId, ProjectId};
use crate::model::{AgentOutput, ModelRef};

/// One unit of work: a prompt run against a project by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Stable identifier.
    pub id: JobId,
    /// Project this job runs against.
    pub project_id: ProjectId,
    /// Set when this job is part of a batch.
    pub batch_id: Option<BatchId>,
    /// The prompt sent to the agent.
    pub prompt: String,
    /// Model override; falls back to the project default when `None`.
    pub model: Option<ModelRef>,
    /// Current lifecycle state.
    pub status: JobStatus,
    /// `pi` session id, set once the job has run, enabling follow-ups.
    pub session_ref: Option<String>,
    /// The collected agent output, set once the job completes successfully.
    pub output: Option<AgentOutput>,
    /// How many times execution has been attempted.
    pub attempts: u32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last status-change timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Job {
    /// Creates a queued job with a fresh id and current timestamps.
    #[must_use]
    pub fn new(project_id: ProjectId, prompt: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: JobId::new(),
            project_id,
            batch_id: None,
            prompt: prompt.into(),
            model: None,
            status: JobStatus::Queued,
            session_ref: None,
            output: None,
            attempts: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Lifecycle state of a [`Job`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Waiting in the queue.
    Queued,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error after exhausting retries.
    Failed,
    /// Cancelled by the user before completion.
    Cancelled,
}

impl JobStatus {
    /// Returns `true` when no further state transition will occur.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}
