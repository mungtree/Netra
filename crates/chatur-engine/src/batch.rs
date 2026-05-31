//! [`BatchExecutor`] — runs a batch's map step, then its reduce step.
//!
//! The **map** step turns every [`BatchItem`] into a [`Job`], enqueues it on
//! the shared [`JobQueue`] (so the [`Scheduler`](crate::Scheduler) runs it under
//! the global concurrency cap), and waits for all of them. The **reduce** step
//! combines the per-item outputs: pure strategies come from the
//! [`AggregatorRegistry`], while `reviewer` runs one more agent job that
//! consolidates the rest.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use chatur_core::ids::{BatchId, JobId};
use chatur_core::model::{
    AgentOutput, AggregatedResult, Batch, BatchItem, BatchStatus, Job, JobStatus, PromptSource,
    TokenUsage, findings_report_schema,
};
use chatur_core::traits::{
    BatchRepo, DomainEvent, EventBus, JobQueue, JobRepo, ProjectRepo, TemplateRepo,
};
use chatur_core::{CoreError, Result};

use crate::aggregate::AggregatorRegistry;
use crate::planner::StructuredPlanner;

/// The strategy id that triggers the agent-backed reviewer reduce step.
const REVIEWER_STRATEGY: &str = "reviewer";
/// The strategy id that runs a reviewer constrained to the FindingsReport schema.
const STRUCTURED_REVIEWER_STRATEGY: &str = "structured_reviewer";

/// Orchestrates one batch from map to reduce.
///
/// Cloning is cheap (every field is shared) so the API layer can hand a clone
/// to a background task per [`run`](Self::run).
#[derive(Clone)]
pub struct BatchExecutor {
    queue: Arc<dyn JobQueue>,
    jobs: Arc<dyn JobRepo>,
    batches: Arc<dyn BatchRepo>,
    projects: Arc<dyn ProjectRepo>,
    templates: Arc<dyn TemplateRepo>,
    bus: Arc<dyn EventBus>,
    registry: Arc<AggregatorRegistry>,
    planner: Arc<dyn StructuredPlanner>,
    /// When false, `structured_reviewer` falls back to the legacy prompt-only
    /// path: a normal `pi` job asked to emit JSON, then best-effort parsed.
    planner_enabled: bool,
    poll_interval: Duration,
}

impl BatchExecutor {
    /// Assembles an executor from its collaborators.
    ///
    /// `queue` must be the same queue the [`Scheduler`](crate::Scheduler)
    /// drains — that is how mapped jobs actually run.
    #[must_use]
    pub fn new(
        queue: Arc<dyn JobQueue>,
        jobs: Arc<dyn JobRepo>,
        batches: Arc<dyn BatchRepo>,
        projects: Arc<dyn ProjectRepo>,
        templates: Arc<dyn TemplateRepo>,
        bus: Arc<dyn EventBus>,
        registry: Arc<AggregatorRegistry>,
        planner: Arc<dyn StructuredPlanner>,
    ) -> Self {
        Self {
            queue,
            jobs,
            batches,
            projects,
            templates,
            bus,
            registry,
            planner,
            planner_enabled: true,
            poll_interval: Duration::from_millis(100),
        }
    }

    /// Disables the planner sidecar for `structured_reviewer` — falls back to
    /// a regular `pi` job prompted with the schema, parsed best-effort.
    #[must_use]
    pub fn with_planner_enabled(mut self, enabled: bool) -> Self {
        self.planner_enabled = enabled;
        self
    }

    /// Overrides how often job completion is polled (tests use a short value).
    #[must_use]
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Runs the batch identified by `batch_id` to completion.
    ///
    /// Persists the batch as `Running`, then as `Completed` (with its
    /// [`AggregatedResult`]) or `Failed`, publishing the matching
    /// [`DomainEvent`] at each transition.
    ///
    /// # Errors
    /// Returns an error — after marking the batch `Failed` — if the batch is
    /// unknown, has no items, every mapped job fails, or the reduce step fails.
    pub async fn run(&self, batch_id: BatchId) -> Result<AggregatedResult> {
        let mut batch = self.batches.get(batch_id).await?;
        batch.status = BatchStatus::Running;
        batch.updated_at = Utc::now();
        let _ = self.batches.update(&batch).await;
        self.bus.publish(DomainEvent::BatchStarted { batch_id });
        tracing::info!(%batch_id, items = batch.item_count(), "batch started");

        let outcome = self.execute(&batch).await;

        batch.updated_at = Utc::now();
        match &outcome {
            Ok(result) => {
                batch.status = BatchStatus::Completed;
                batch.result = Some(result.clone());
                let _ = self.batches.update(&batch).await;
                self.bus.publish(DomainEvent::BatchCompleted { batch_id });
                tracing::info!(%batch_id, "batch completed");
            }
            Err(error) => {
                batch.status = BatchStatus::Failed;
                let _ = self.batches.update(&batch).await;
                self.bus.publish(DomainEvent::BatchFailed {
                    batch_id,
                    error: error.to_string(),
                });
                tracing::warn!(%batch_id, %error, "batch failed");
            }
        }
        outcome
    }

    /// Map then reduce — the body of [`run`](Self::run), minus status bookkeeping.
    async fn execute(&self, batch: &Batch) -> Result<AggregatedResult> {
        let items = self.batches.items(batch.id).await?;
        if items.is_empty() {
            return Err(CoreError::Invalid(format!(
                "batch {} has no items to run",
                batch.id
            )));
        }

        // MAP — one job per item, enqueued onto the shared scheduler queue.
        // Cache each target project once so module lookups don't re-hit the DB.
        let mut project_cache: HashMap<_, chatur_core::model::Project> = HashMap::new();
        let mut job_ids = Vec::with_capacity(items.len());
        for mut item in items {
            let mut prompt = self.resolve_prompt(batch, &item).await?;

            // Resolve the module (when the batch fanned out over one) up front:
            // both the job's module scoping and PR/diff mode's pathspec need it.
            let mut module_id_name_root = None;
            if let Some(module_id) = item.module_id {
                let project = match project_cache.get(&item.target.project_id) {
                    Some(p) => p,
                    None => {
                        let p = self.projects.get(item.target.project_id).await?;
                        project_cache.entry(item.target.project_id).or_insert(p)
                    }
                };
                if let Some(module) = project.modules.iter().find(|m| m.id == module_id) {
                    module_id_name_root = Some((
                        module_id,
                        module.name.clone(),
                        project.root_path.join(&module.root_subdir),
                        module.root_subdir.clone(),
                    ));
                }
            }

            // PR/diff mode: prefix the prompt with `git diff <branch>` run in the
            // target project, scoped to the module subdir for module-fanned jobs.
            if let Some(branch) = &batch.diff_branch {
                let project = match project_cache.get(&item.target.project_id) {
                    Some(p) => p,
                    None => {
                        let p = self.projects.get(item.target.project_id).await?;
                        project_cache.entry(item.target.project_id).or_insert(p)
                    }
                };
                let subdir = module_id_name_root
                    .as_ref()
                    .map(|(_, _, _, root_subdir)| root_subdir.as_path());
                let preamble = diff_preamble(&project.root_path, subdir, branch).await?;
                prompt = format!("{preamble}{prompt}");
            }

            let mut job = Job::new(item.target.project_id, prompt)
                .with_chromadb(batch.use_chromadb);
            job.batch_id = Some(batch.id);
            if let Some((module_id, name, root, _)) = module_id_name_root {
                job.module_id = Some(module_id);
                job.module_name = Some(name);
                job.module_root = Some(root);
            }
            let job_id = job.id;

            self.jobs.create(&job).await?;
            item.job_id = Some(job_id);
            self.batches.update_item(&item).await?;
            self.queue.enqueue(job).await?;
            self.bus.publish(DomainEvent::JobQueued { job_id });
            job_ids.push(job_id);
        }

        let outputs = self.collect(&job_ids).await?;
        if outputs.is_empty() {
            return Err(CoreError::Agent(
                "every job in the batch failed; nothing to aggregate".to_string(),
            ));
        }

        // REDUCE — pick the strategy and combine.
        self.reduce(batch, outputs).await
    }

    /// Builds the prompt text for one item: resolve its source, substitute the
    /// target's variables, and append the output-schema instruction if any.
    async fn resolve_prompt(&self, batch: &Batch, item: &BatchItem) -> Result<String> {
        let prompt = batch
            .prompts
            .iter()
            .find(|p| p.name == item.prompt_name)
            .ok_or_else(|| {
                CoreError::Invalid(format!(
                    "batch references unknown prompt {}",
                    item.prompt_name
                ))
            })?;

        let body = match &prompt.source {
            PromptSource::Inline { body } => body.clone(),
            PromptSource::Template { id } => self.templates.get(*id).await?.body,
        };

        let mut text = render(&body, &item.target.variables);
        if let Some(schema) = &batch.output_schema {
            text.push_str("\n\nRespond ONLY with JSON matching this schema:\n");
            text.push_str(&serde_json::to_string_pretty(schema)?);
        }
        Ok(text)
    }

    /// Waits for every mapped job to finish, returning the successful outputs.
    ///
    /// Failed and cancelled jobs are skipped, not fatal — a batch still
    /// aggregates whatever succeeded.
    async fn collect(&self, job_ids: &[JobId]) -> Result<Vec<AgentOutput>> {
        let mut outputs = Vec::with_capacity(job_ids.len());
        for &job_id in job_ids {
            loop {
                let job = self.jobs.get(job_id).await?;
                if job.status.is_terminal() {
                    if let (JobStatus::Completed, Some(output)) = (job.status, job.output) {
                        outputs.push(output);
                    }
                    break;
                }
                tokio::time::sleep(self.poll_interval).await;
            }
        }
        Ok(outputs)
    }

    /// Combines `outputs` per the batch's configured strategy.
    async fn reduce(&self, batch: &Batch, outputs: Vec<AgentOutput>) -> Result<AggregatedResult> {
        let strategy = batch.aggregation.strategy.as_str();
        if strategy == REVIEWER_STRATEGY {
            return self.run_reviewer(batch, outputs).await;
        }
        if strategy == STRUCTURED_REVIEWER_STRATEGY {
            return self.run_structured_reviewer(batch, outputs).await;
        }
        let aggregator = self.registry.get(strategy).ok_or_else(|| {
            CoreError::Invalid(format!("unknown aggregation strategy {strategy}"))
        })?;
        aggregator
            .aggregate(outputs, &batch.aggregation.config)
            .await
    }

    /// The `reviewer` reduce step: enqueue one more agent job that consolidates
    /// every map output, and wrap its result.
    async fn run_reviewer(
        &self,
        batch: &Batch,
        outputs: Vec<AgentOutput>,
    ) -> Result<AggregatedResult> {
        let project_id = batch
            .targets
            .first()
            .ok_or_else(|| CoreError::Invalid("batch has no target to host the reviewer".into()))?
            .project_id;

        let source_count = outputs.len();
        let mut usage = TokenUsage::default();
        let mut joined = String::new();
        for (i, output) in outputs.iter().enumerate() {
            usage += output.usage;
            joined.push_str(&format!("=== Output {} ===\n{}\n\n", i + 1, output.text));
        }

        let prompt = format!(
            "You are a reviewer agent. Consolidate the following {source_count} agent \
             outputs into a single result: remove duplicate points, rank what remains \
             by importance, and present one clear summary.\n\n{joined}"
        );

        // The reviewer job is standalone — not a member of the batch.
        let job = Job::new(project_id, prompt).with_chromadb(batch.use_chromadb);
        let job_id = job.id;
        self.jobs.create(&job).await?;
        self.queue.enqueue(job).await?;
        self.bus.publish(DomainEvent::JobQueued { job_id });

        let reviewed = loop {
            let job = self.jobs.get(job_id).await?;
            if job.status.is_terminal() {
                break job;
            }
            tokio::time::sleep(self.poll_interval).await;
        };

        if reviewed.status != JobStatus::Completed {
            return Err(CoreError::Agent("the reviewer job did not complete".into()));
        }
        let output = reviewed
            .output
            .ok_or_else(|| CoreError::Agent("the reviewer job produced no output".into()))?;
        usage += output.usage;

        Ok(AggregatedResult {
            summary: output.text,
            structured: output.structured,
            source_count,
            usage,
        })
    }

    /// The `structured_reviewer` reduce step: the [`StructuredPlanner`]
    /// generates a [`FindingsReport`](chatur_core::model::FindingsReport)
    /// constrained to its JSON schema. No `pi` round-trip and no best-effort
    /// parsing — outlines guarantees the returned value matches the schema.
    async fn run_structured_reviewer(
        &self,
        batch: &Batch,
        outputs: Vec<AgentOutput>,
    ) -> Result<AggregatedResult> {
        if !self.planner_enabled {
            tracing::info!(
                batch_id = %batch.id,
                "structured_reviewer: planner disabled, using legacy prompt-only reviewer"
            );
            return self.run_structured_reviewer_legacy(batch, outputs).await;
        }
        self.run_structured_reviewer_planner(batch, outputs).await
    }

    async fn run_structured_reviewer_planner(
        &self,
        batch: &Batch,
        outputs: Vec<AgentOutput>,
    ) -> Result<AggregatedResult> {
        let batch_id = batch.id;
        let source_count = outputs.len();
        let mut usage = TokenUsage::default();
        let mut joined = String::new();
        for (i, output) in outputs.iter().enumerate() {
            usage += output.usage;
            joined.push_str(&format!("=== Output {} ===\n{}\n\n", i + 1, output.text));
        }

        let schema = findings_report_schema();
        let prompt = format!(
            "Consolidate the following {source_count} agent outputs into a \
             single findings report. Deduplicate items. Classify each as one \
             of: bug, vulnerability, idea, change, fix, suggestion, other. \
             Assign a severity (critical/high/medium/low/info). Include a \
             file:line location when one is mentioned in the source \
             outputs. Ensure you include a suggested fix, \
             tags which categorize the issue, and all other required properties.\n\n \
             Source outputs:\n\n{joined}"
        );

        tracing::info!(
            %batch_id,
            source_count,
            prompt_len = prompt.len(),
            "structured_reviewer: invoking planner sidecar"
        );
        self.bus.publish(DomainEvent::PlannerStarted {
            batch_id,
            source_count,
        });

        let started = std::time::Instant::now();
        let value = match self.planner.generate(&prompt, &schema).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(%batch_id, error = %e, "structured_reviewer: planner failed");
                self.bus.publish(DomainEvent::PlannerFinished {
                    batch_id,
                    success: false,
                });
                return Err(e);
            }
        };

        let report: chatur_core::model::FindingsReport = match serde_json::from_value(value.clone())
        {
            Ok(r) => r,
            Err(e) => {
                self.bus.publish(DomainEvent::PlannerFinished {
                    batch_id,
                    success: false,
                });
                return Err(CoreError::Agent(format!(
                    "planner returned value that does not match FindingsReport schema: {e}"
                )));
            }
        };

        tracing::info!(
            %batch_id,
            elapsed_ms = started.elapsed().as_millis() as u64,
            findings = report.findings.len(),
            "structured_reviewer: planner returned valid report"
        );
        self.bus.publish(DomainEvent::PlannerFinished {
            batch_id,
            success: true,
        });

        Ok(AggregatedResult {
            summary: report.summary.clone(),
            structured: Some(value),
            source_count,
            usage,
        })
    }

    /// Legacy `structured_reviewer` path used when the planner sidecar is
    /// disabled in config: enqueue a normal `pi` job prompted with the schema,
    /// then best-effort parse its text output. Falls back to plain text on
    /// parse failure.
    async fn run_structured_reviewer_legacy(
        &self,
        batch: &Batch,
        outputs: Vec<AgentOutput>,
    ) -> Result<AggregatedResult> {
        let project_id = batch
            .targets
            .first()
            .ok_or_else(|| CoreError::Invalid("batch has no target to host the reviewer".into()))?
            .project_id;

        let source_count = outputs.len();
        let mut usage = TokenUsage::default();
        let mut joined = String::new();
        for (i, output) in outputs.iter().enumerate() {
            usage += output.usage;
            joined.push_str(&format!("=== Output {} ===\n{}\n\n", i + 1, output.text));
        }

        let schema = findings_report_schema();
        let schema_pretty = serde_json::to_string_pretty(&schema)?;
        let prompt = format!(
            "You are a structured reviewer agent. Consolidate the following \
             {source_count} agent outputs into a single JSON report. \
             Deduplicate items, classify each as one of: bug, vulnerability, \
             idea, change, fix, suggestion, other. Assign a severity \
             (critical/high/medium/low/info). Include a file:line location \
             when one is mentioned in the source outputs. Respond with ONLY a \
             JSON object — no prose, no markdown fences — matching this \
             schema:\n\n{schema_pretty}\n\nSource outputs:\n\n{joined}"
        );

        let job = Job::new(project_id, prompt).with_chromadb(batch.use_chromadb);
        let job_id = job.id;
        self.jobs.create(&job).await?;
        self.queue.enqueue(job).await?;
        self.bus.publish(DomainEvent::JobQueued { job_id });

        let reviewed = loop {
            let job = self.jobs.get(job_id).await?;
            if job.status.is_terminal() {
                break job;
            }
            tokio::time::sleep(self.poll_interval).await;
        };

        if reviewed.status != JobStatus::Completed {
            return Err(CoreError::Agent(
                "the structured reviewer job did not complete".into(),
            ));
        }
        let output = reviewed
            .output
            .ok_or_else(|| CoreError::Agent("the reviewer job produced no output".into()))?;
        usage += output.usage;

        let (summary, structured) = parse_findings(&output.text)
            .map(|report| {
                (
                    report.summary.clone(),
                    Some(serde_json::to_value(report).unwrap_or(serde_json::Value::Null)),
                )
            })
            .unwrap_or_else(|| (output.text.clone(), output.structured.clone()));

        Ok(AggregatedResult {
            summary,
            structured,
            source_count,
            usage,
        })
    }
}

/// Best-effort parse of `text` into a [`FindingsReport`]. Tolerates fenced
/// code blocks and leading/trailing prose.
fn parse_findings(text: &str) -> Option<chatur_core::model::FindingsReport> {
    if let Ok(report) = serde_json::from_str(text) {
        return Some(report);
    }
    let stripped = strip_code_fence(text);
    if let Ok(report) = serde_json::from_str(stripped) {
        return Some(report);
    }
    let candidate = extract_json_object(stripped)?;
    serde_json::from_str(candidate).ok()
}

fn strip_code_fence(text: &str) -> &str {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    t.trim_end_matches("```").trim()
}

fn extract_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|b| *b == b'{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Builds the PR/diff-mode prompt prefix: a note of the exact git command run
/// plus the captured `git diff <branch>` output, fenced for the agent.
///
/// `subdir`, when set (a module-fanned job), narrows the diff to that path via a
/// pathspec; `None` or an empty subdir means the whole repo.
async fn diff_preamble(
    project_root: &std::path::Path,
    subdir: Option<&std::path::Path>,
    branch: &str,
) -> Result<String> {
    let scoped = subdir.filter(|s| !s.as_os_str().is_empty());
    let command = match scoped {
        Some(s) => format!("git diff {branch} -- {}", s.display()),
        None => format!("git diff {branch}"),
    };
    let diff = git_diff(project_root, scoped, branch).await?;
    let body = if diff.trim().is_empty() {
        format!("(no changes between the working tree and `{branch}`)")
    } else {
        format!("```diff\n{diff}\n```")
    };
    Ok(format!(
        "You are reviewing a code diff. The following was produced by running:\n\n    {command}\n\nin the project's working directory. Focus your analysis on these changes.\n\n{body}\n\n---\n\n",
    ))
}

/// Runs `git diff <branch>` (optionally pathspec-scoped) in `project_root` and
/// returns its stdout. A non-zero exit (e.g. unknown branch) is fatal.
async fn git_diff(
    project_root: &std::path::Path,
    subdir: Option<&std::path::Path>,
    branch: &str,
) -> Result<String> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.arg("diff").arg(branch).current_dir(project_root);
    if let Some(s) = subdir {
        cmd.arg("--").arg(s);
    }
    let out = cmd
        .output()
        .await
        .map_err(|e| CoreError::Agent(format!("failed to run git diff: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(CoreError::Invalid(format!(
            "`git diff {branch}` failed in {}: {}",
            project_root.display(),
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Substitutes `{{name}}` placeholders in `body` with values from `vars`.
fn render(body: &str, vars: &HashMap<String, String>) -> String {
    let mut out = body.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_substitutes_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("file".to_string(), "main.rs".to_string());
        assert_eq!(render("review {{file}}", &vars), "review main.rs");
    }

    #[test]
    fn render_leaves_unknown_placeholders_untouched() {
        assert_eq!(
            render("review {{file}}", &HashMap::new()),
            "review {{file}}"
        );
    }

    use std::path::Path;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    /// Sets up a repo with a `base` branch, then changes files in two subdirs on
    /// the working tree so a diff against `base` is non-empty.
    fn repo_with_changes() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        git(p, &["init", "-q", "-b", "base"]);
        std::fs::create_dir_all(p.join("a")).unwrap();
        std::fs::create_dir_all(p.join("b")).unwrap();
        std::fs::write(p.join("a/x.txt"), "one\n").unwrap();
        std::fs::write(p.join("b/y.txt"), "one\n").unwrap();
        git(p, &["add", "-A"]);
        git(p, &["commit", "-qm", "init"]);
        // Diverge the working tree from `base`.
        std::fs::write(p.join("a/x.txt"), "two\n").unwrap();
        std::fs::write(p.join("b/y.txt"), "two\n").unwrap();
        dir
    }

    #[tokio::test]
    async fn diff_preamble_includes_command_and_diff() {
        let dir = repo_with_changes();
        let out = diff_preamble(dir.path(), None, "base").await.unwrap();
        assert!(out.contains("git diff base"));
        assert!(out.contains("a/x.txt"));
        assert!(out.contains("b/y.txt"));
        assert!(out.ends_with("---\n\n"));
    }

    #[tokio::test]
    async fn diff_preamble_scopes_to_module_subdir() {
        let dir = repo_with_changes();
        let out = diff_preamble(dir.path(), Some(Path::new("a")), "base")
            .await
            .unwrap();
        assert!(out.contains("git diff base -- a"));
        assert!(out.contains("a/x.txt"));
        // The other subdir's change is excluded by the pathspec.
        assert!(!out.contains("b/y.txt"));
    }

    #[tokio::test]
    async fn git_diff_errors_on_unknown_branch() {
        let dir = repo_with_changes();
        let err = diff_preamble(dir.path(), None, "no-such-branch")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Invalid(_)));
    }
}
