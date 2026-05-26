//! [`ProjectSpecResolver`] — builds an [`AgentSpec`] from a job's project.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use chatur_agent::AgentSpec;
use chatur_chroma::{chromadb_system_prompt, ChromaConfig, ChromaHandle};
use chatur_core::Result;
use chatur_core::model::{Job, ModelRef};
use chatur_core::traits::{DomainEvent, EventBus, ProjectRepo};
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
    chroma: Option<ChromaHandle>,
    bus: Arc<dyn EventBus>,
}

impl ProjectSpecResolver {
    /// Creates a resolver over `projects`.
    #[must_use]
    pub fn new(
        projects: SqliteProjectRepo,
        pi_binary: PathBuf,
        default_model: Option<ModelRef>,
        agent: AgentConfig,
        chroma: Option<ChromaHandle>,
        bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            projects,
            pi_binary,
            default_model,
            agent,
            chroma,
            bus,
        }
    }

    fn degrade(&self, job: &Job, reason: impl Into<String>) {
        let reason = reason.into();
        tracing::warn!(
            target: "chatur::chroma::resolver",
            job_id = %job.id,
            "chroma degraded: {reason}"
        );
        self.bus.publish(DomainEvent::ChromaPromptDegraded {
            job_id: job.id,
            reason,
        });
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

        // Combine the global system-prompt append (from config) with the
        // optional ChromaDB hint (per-job opt-in).
        let mut appended: Vec<String> = Vec::new();
        if let Some(text) = &self.agent.system_prompt_append {
            if !text.trim().is_empty() {
                appended.push(text.clone());
            }
        }
        if job.use_chromadb {
            match &self.chroma {
                None => self.degrade(job, "ChromaDB integration is disabled in config"),
                Some(chroma) => {
                    if !chroma.is_running().await {
                        self.degrade(job, "ChromaDB server is not running");
                    } else {
                        let coll = ChromaConfig::collection_name(&job.project_id.to_string());
                        let cfg = chroma.config().await;
                        match chatur_chroma::bootstrap::ensure_shim() {
                            Ok(shim) => {
                                appended.push(chromadb_system_prompt(&coll, &shim));
                                spec = spec
                                    .with_env("CHATUR_CHROMA_HOST", cfg.host.clone())
                                    .with_env("CHATUR_CHROMA_PORT", cfg.port.to_string())
                                    .with_env("CHATUR_CHROMA_MODEL", cfg.resolved_model())
                                    .with_env("CHATUR_CHROMA_COLLECTION", coll.clone());
                            }
                            Err(e) => {
                                self.degrade(job, format!("chroma shim unavailable: {e}"));
                            }
                        }
                    }
                }
            }
        }
        if !appended.is_empty() {
            spec = spec.with_system_prompt_append(appended.join("\n\n"));
        }
        Ok(spec)
    }
}
