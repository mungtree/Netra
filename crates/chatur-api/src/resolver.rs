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

        // Module scope hint. cwd stays at the project root so tooling sees the
        // whole repo; the module path is passed only as a prompt hint + env var.
        if let Some(name) = &job.module_name {
            let module_root = job
                .module_root
                .clone()
                .unwrap_or_else(|| project.root_path.clone());
            let root_subdir = module_root
                .strip_prefix(&project.root_path)
                .unwrap_or(&module_root)
                .display();
            appended.push(format!(
                "Scope: focus on module `{name}` rooted at `{}` (relative to repo: `{}`).\n\
                 You still have read access to the entire repo at `{}` if cross-module \
                 context is needed, but your work should target this module.",
                module_root.display(),
                root_subdir,
                project.root_path.display(),
            ));
            spec = spec.with_env("CHATUR_MODULE_ROOT", module_root.display().to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use chatur_core::model::{Job, Project};
    use chatur_engine::BroadcastEventBus;
    use chatur_store::Database;

    async fn resolver(db: &Database) -> ProjectSpecResolver {
        ProjectSpecResolver::new(
            db.projects(),
            PathBuf::from("pi"),
            None,
            AgentConfig::default(),
            None,
            Arc::new(BroadcastEventBus::new(16)),
        )
    }

    #[tokio::test]
    async fn module_block_present_when_job_is_scoped() {
        let db = Database::in_memory().await.unwrap();
        let project = Project::new("p", "/tmp/p");
        db.projects().create(&project).await.unwrap();
        let resolver = resolver(&db).await;

        let mut job = Job::new(project.id, "do work");
        job.module_name = Some("backend".into());
        job.module_root = Some(PathBuf::from("/tmp/p/server"));

        let spec = resolver.resolve(&job).await.unwrap();
        let prompt = spec.system_prompt_append.unwrap_or_default();
        assert!(prompt.contains("Scope: focus on module `backend`"), "{prompt}");
        assert!(prompt.contains("server"), "{prompt}");
        assert_eq!(
            spec.env.get("CHATUR_MODULE_ROOT").map(String::as_str),
            Some("/tmp/p/server")
        );
    }

    #[tokio::test]
    async fn module_block_absent_for_unscoped_job() {
        let db = Database::in_memory().await.unwrap();
        let project = Project::new("p", "/tmp/p");
        db.projects().create(&project).await.unwrap();
        let resolver = resolver(&db).await;

        let job = Job::new(project.id, "do work");
        let spec = resolver.resolve(&job).await.unwrap();
        let prompt = spec.system_prompt_append.unwrap_or_default();
        assert!(!prompt.contains("Scope: focus on module"));
        assert!(!spec.env.contains_key("CHATUR_MODULE_ROOT"));
    }
}
