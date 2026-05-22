//! Registered code projects the agent operates on.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::ProjectId;

/// A local code project that agent jobs can target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Stable identifier.
    pub id: ProjectId,
    /// Human-readable name.
    pub name: String,
    /// Absolute path to the project root on disk.
    pub root_path: PathBuf,
    /// Model used for this project's jobs unless a job overrides it.
    pub default_model: Option<ModelRef>,
    /// Which tools the agent may use against this project.
    pub tool_policy: ToolPolicy,
}

impl Project {
    /// Creates a project with a fresh id and default (read-only) tool policy.
    #[must_use]
    pub fn new(name: impl Into<String>, root_path: impl Into<PathBuf>) -> Self {
        Self {
            id: ProjectId::new(),
            name: name.into(),
            root_path: root_path.into(),
            default_model: None,
            tool_policy: ToolPolicy::default(),
        }
    }
}

/// A provider + model selection passed to `pi` (`--provider` / `--model`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRef {
    /// Provider name, e.g. `anthropic`, `google`.
    pub provider: String,
    /// Model id or pattern.
    pub model: String,
}

/// Controls which `pi` tools the agent is allowed to invoke.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPolicy {
    /// No mutating tools — analysis only (`pi --no-tools` or read-only set).
    #[default]
    ReadOnly,
    /// Only the named tools are enabled (`pi --tools`).
    Allowlist(Vec<String>),
    /// All built-in and extension tools enabled.
    Full,
}
