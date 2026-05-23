//! [`ProjectSpecResolver`] — builds an [`AgentSpec`] from a job's project.

use std::path::PathBuf;

use async_trait::async_trait;

use chatur_agent::AgentSpec;
use chatur_core::Result;
use chatur_core::model::{Job, ModelRef};
use chatur_core::traits::ProjectRepo;
use chatur_engine::SpecResolver;
use chatur_store::SqliteProjectRepo;

use crate::config::AgentConfig;

/// Resolves a job's [`AgentSpec`] by looking up its project.
///
/// Model precedence: the job's own override, then the project default, then
/// the application-wide default from configuration. Tool policy and the
/// system-prompt append text come from the application-wide [`AgentConfig`]
/// (one knob in `chatur.toml` rather than per-project).
pub struct ProjectSpecResolver {
    projects: SqliteProjectRepo,
    pi_binary: PathBuf,
    default_model: Option<ModelRef>,
    agent: AgentConfig,
}

impl ProjectSpecResolver {
    /// Creates a resolver over `projects`.
    #[must_use]
    pub fn new(
        projects: SqliteProjectRepo,
        pi_binary: PathBuf,
        default_model: Option<ModelRef>,
        agent: AgentConfig,
    ) -> Self {
        Self {
            projects,
            pi_binary,
            default_model,
            agent,
        }
    }
}

#[async_trait]
impl SpecResolver for ProjectSpecResolver {
    async fn resolve(&self, job: &Job) -> Result<AgentSpec> {
        let project = self.projects.get(job.project_id).await?;

        let model = job
            .model
            .clone()
            .or_else(|| project.default_model.clone())
            .or_else(|| self.default_model.clone());

        let mut spec = AgentSpec::new(self.pi_binary.clone(), project.root_path.clone())
            .with_tool_policy(self.agent.tools.to_tool_policy());
        if let Some(model) = model {
            spec = spec.with_model(model);
        }
        if let Some(session) = &job.session_ref {
            spec = spec.with_session(session.clone());
        }
        if let Some(text) = &self.agent.system_prompt_append {
            if !text.trim().is_empty() {
                spec = spec.with_system_prompt_append(text.clone());
            }
        }
        Ok(spec)
    }
}
