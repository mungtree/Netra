//! A single agent query against one project.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use std::path::PathBuf;

use crate::ids::{BatchId, JobId, ModuleId, ProjectId};
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
    /// When the job first transitioned to `Running`.
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    /// When the job reached a terminal status.
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    /// Opt-in: signal to the agent that the ChromaDB MCP server is available
    /// and should be used to retrieve project context before answering.
    /// Default `false` — set per-job (or per-batch) by the user.
    #[serde(default)]
    pub use_chromadb: bool,
    /// Module this job is scoped to, when the batch fanned out over modules.
    #[serde(default)]
    pub module_id: Option<ModuleId>,
    /// Absolute path of the module root, resolved at fanout time. Used as a
    /// prompt hint; cwd stays at the project root for full-repo tool access.
    #[serde(default)]
    pub module_root: Option<PathBuf>,
    /// Module name, for prompt injection and the UI module badge.
    #[serde(default)]
    pub module_name: Option<String>,
    /// Explicit FIFO override for the durable queue. `None` falls back to
    /// `created_at` ordering.
    #[serde(default)]
    pub queue_position: Option<i64>,
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
            started_at: None,
            finished_at: None,
            use_chromadb: false,
            module_id: None,
            module_root: None,
            module_name: None,
            queue_position: None,
        }
    }

    /// Builder-style toggle for ChromaDB usage on a single job.
    #[must_use]
    pub fn with_chromadb(mut self, enabled: bool) -> Self {
        self.use_chromadb = enabled;
        self
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
