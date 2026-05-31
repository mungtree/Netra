//! Batches: a series of prompts fanned out across targets, then aggregated.
//!
//! A [`Batch`] is the map/reduce unit of work. Its **map** step expands the
//! cartesian product [`prompts`](Batch::prompts) × [`targets`](Batch::targets)
//! into one [`BatchItem`] (and, once run, one job) per pair; its **reduce**
//! step combines every per-item output via [`aggregation`](Batch::aggregation).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BatchId, BatchItemId, JobId, ModuleId, ProjectId, TemplateId};
use crate::model::AggregatedResult;
use crate::{CoreError, Result};

/// A map/reduce unit of work: run every [`prompt`](Self::prompts) against every
/// [`target`](Self::targets), then combine the outputs with
/// [`aggregation`](Self::aggregation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Stable identifier.
    pub id: BatchId,
    /// Human-readable name.
    pub name: String,
    /// The prompt-set run against each target (the "map" inputs).
    pub prompts: Vec<BatchPrompt>,
    /// The targets to fan out over.
    pub targets: Vec<BatchTarget>,
    /// How per-item outputs are combined (the "reduce" step).
    pub aggregation: AggregationSpec,
    /// Optional JSON schema each item output is asked to conform to.
    pub output_schema: Option<serde_json::Value>,
    /// Opt-in: every job mapped from this batch (and its reviewer) will be
    /// told the ChromaDB MCP server is available. Default `false`.
    #[serde(default)]
    pub use_chromadb: bool,
    /// When `true`, skip module fanout entirely — one job per `(prompt, target)`
    /// against the whole repo, regardless of each project's modules.
    #[serde(default)]
    pub global: bool,
    /// PR/diff mode: when set, each mapped job's prompt is prefixed with the
    /// output of `git diff <branch>` run in the target project's working dir
    /// (scoped to the module subdir for module-fanned jobs), plus a note of the
    /// exact git command. Default `None` — full-repo scan as before.
    #[serde(default)]
    pub diff_branch: Option<String>,
    /// Current lifecycle state.
    pub status: BatchStatus,
    /// The consolidated reduce-step result, set once the batch completes.
    pub result: Option<AggregatedResult>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last status-change timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Batch {
    /// Creates an empty, `Pending` batch with a fresh id.
    ///
    /// Prefer [`BatchBuilder`] for assembling a runnable batch.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: BatchId::new(),
            name: name.into(),
            prompts: Vec::new(),
            targets: Vec::new(),
            aggregation: AggregationSpec::default(),
            output_schema: None,
            use_chromadb: false,
            global: false,
            diff_branch: None,
            status: BatchStatus::Pending,
            result: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// How many `(prompt, target)` pairs this batch fans out into, **ignoring**
    /// module fanout. Exact only when [`global`](Self::global) is set; otherwise
    /// a lower bound — use [`materialize`](Self::materialize)`.len()` for the
    /// true job count once module lists are known.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.prompts.len() * self.targets.len()
    }

    /// Expands the `prompts × targets × modules` product into one [`BatchItem`]
    /// per triple.
    ///
    /// `modules` maps each target project to its available [`ModuleId`]s; it is
    /// only consulted for non-[`global`](Self::global) targets that don't pin
    /// their own [`module_ids`](BatchTarget::module_ids).
    ///
    /// - [`global`](Self::global) → one item per `(prompt, target)`,
    ///   `module_id = None`.
    /// - otherwise → for each target, the modules are
    ///   [`target.module_ids`](BatchTarget::module_ids) when set, else
    ///   `modules[project_id]`. A target whose project has no known modules
    ///   degrades to a single whole-repo item (`module_id = None`).
    #[must_use]
    pub fn materialize(&self, modules: &HashMap<ProjectId, Vec<ModuleId>>) -> Vec<BatchItem> {
        let mut items = Vec::new();
        for prompt in &self.prompts {
            for target in &self.targets {
                if self.global {
                    items.push(BatchItem::new(
                        self.id,
                        prompt.name.clone(),
                        target.clone(),
                        None,
                    ));
                    continue;
                }
                let module_ids: Vec<ModuleId> = target
                    .module_ids
                    .clone()
                    .or_else(|| modules.get(&target.project_id).cloned())
                    .unwrap_or_default();
                if module_ids.is_empty() {
                    items.push(BatchItem::new(
                        self.id,
                        prompt.name.clone(),
                        target.clone(),
                        None,
                    ));
                } else {
                    for module_id in module_ids {
                        items.push(BatchItem::new(
                            self.id,
                            prompt.name.clone(),
                            target.clone(),
                            Some(module_id),
                        ));
                    }
                }
            }
        }
        items
    }
}

/// Lifecycle state of a [`Batch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Created but not yet started.
    Pending,
    /// Map and/or reduce steps are in flight.
    Running,
    /// Every step finished and a result was produced.
    Completed,
    /// The batch could not produce a result.
    Failed,
    /// Cancelled before completion.
    Cancelled,
}

impl BatchStatus {
    /// Returns `true` when no further state transition will occur.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// One named prompt in a batch's prompt-set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPrompt {
    /// Identifies the prompt within its batch (used to label items).
    pub name: String,
    /// Where the prompt text comes from.
    pub source: PromptSource,
}

impl BatchPrompt {
    /// Builds a prompt from a literal body.
    #[must_use]
    pub fn inline(name: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: PromptSource::Inline { body: body.into() },
        }
    }

    /// Builds a prompt that references a stored template.
    #[must_use]
    pub fn template(name: impl Into<String>, id: TemplateId) -> Self {
        Self {
            name: name.into(),
            source: PromptSource::Template { id },
        }
    }
}

/// Where a prompt's text comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptSource {
    /// A literal prompt body.
    Inline {
        /// The prompt text.
        body: String,
    },
    /// A reference to a stored [`PromptTemplate`](super::PromptTemplate).
    Template {
        /// Template identifier.
        id: TemplateId,
    },
}

/// One target of a batch: a project plus its template variable bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTarget {
    /// Project to run against.
    pub project_id: ProjectId,
    /// Values bound to the prompt's `{{variable}}` placeholders.
    pub variables: HashMap<String, String>,
    /// Which of the project's modules to fan out over. `None` = every module
    /// of the project. Ignored when [`Batch::global`] is set.
    #[serde(default)]
    pub module_ids: Option<Vec<ModuleId>>,
}

impl BatchTarget {
    /// A target with no variable bindings.
    #[must_use]
    pub fn project(project_id: ProjectId) -> Self {
        Self {
            project_id,
            variables: HashMap::new(),
            module_ids: None,
        }
    }
}

/// Selects and configures the reduce-step strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationSpec {
    /// Strategy id, resolved against the aggregator registry
    /// (e.g. `concat`, `reviewer`, `schema_merge`).
    pub strategy: String,
    /// Strategy-specific configuration, interpreted by the chosen strategy.
    pub config: serde_json::Value,
}

impl Default for AggregationSpec {
    fn default() -> Self {
        Self {
            strategy: "concat".to_string(),
            config: serde_json::Value::Null,
        }
    }
}

/// One materialized member of a batch: the job for a single `(prompt, target)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    /// Stable identifier.
    pub id: BatchItemId,
    /// Owning batch.
    pub batch_id: BatchId,
    /// The job created for this item, once scheduled.
    pub job_id: Option<JobId>,
    /// Which of the batch's prompts this item runs.
    pub prompt_name: String,
    /// The target this item covers.
    pub target: BatchTarget,
    /// The module this item is scoped to. `None` = whole repo (global batch or
    /// a target whose project has no modules).
    #[serde(default)]
    pub module_id: Option<ModuleId>,
}

impl BatchItem {
    /// Creates an unscheduled item with a fresh id.
    #[must_use]
    pub fn new(
        batch_id: BatchId,
        prompt_name: impl Into<String>,
        target: BatchTarget,
        module_id: Option<ModuleId>,
    ) -> Self {
        Self {
            id: BatchItemId::new(),
            batch_id,
            job_id: None,
            prompt_name: prompt_name.into(),
            target,
            module_id,
        }
    }
}

/// Fluent constructor for a runnable [`Batch`] (the Builder pattern).
///
/// ```
/// # use chatur_core::model::BatchBuilder;
/// # use chatur_core::ids::ProjectId;
/// let batch = BatchBuilder::new("review")
///     .prompt("bugs", "Find logic bugs.")
///     .prompt("perf", "Find performance issues.")
///     .target_project(ProjectId::new())
///     .strategy("reviewer")
///     .build()
///     .unwrap();
/// assert_eq!(batch.item_count(), 2);
/// ```
#[derive(Debug)]
pub struct BatchBuilder {
    batch: Batch,
}

impl BatchBuilder {
    /// Starts a builder for a batch named `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            batch: Batch::new(name),
        }
    }

    /// Appends an inline prompt to the prompt-set.
    #[must_use]
    pub fn prompt(mut self, name: impl Into<String>, body: impl Into<String>) -> Self {
        self.batch.prompts.push(BatchPrompt::inline(name, body));
        self
    }

    /// Appends a prepared [`BatchPrompt`] (inline or template-backed).
    #[must_use]
    pub fn prompt_source(mut self, prompt: BatchPrompt) -> Self {
        self.batch.prompts.push(prompt);
        self
    }

    /// Appends a target project with no variable bindings.
    #[must_use]
    pub fn target_project(mut self, project_id: ProjectId) -> Self {
        self.batch.targets.push(BatchTarget::project(project_id));
        self
    }

    /// Appends a target with explicit variable bindings.
    #[must_use]
    pub fn target(mut self, target: BatchTarget) -> Self {
        self.batch.targets.push(target);
        self
    }

    /// Sets the reduce-step strategy id (default `concat`).
    #[must_use]
    pub fn strategy(mut self, strategy: impl Into<String>) -> Self {
        self.batch.aggregation.strategy = strategy.into();
        self
    }

    /// Replaces the whole aggregation spec, including strategy config.
    #[must_use]
    pub fn aggregation(mut self, spec: AggregationSpec) -> Self {
        self.batch.aggregation = spec;
        self
    }

    /// Sets the JSON schema each item output is asked to conform to.
    #[must_use]
    pub fn output_schema(mut self, schema: serde_json::Value) -> Self {
        self.batch.output_schema = Some(schema);
        self
    }

    /// Enable ChromaDB MCP context for every mapped (and reviewer) job.
    #[must_use]
    pub fn use_chromadb(mut self, enabled: bool) -> Self {
        self.batch.use_chromadb = enabled;
        self
    }

    /// Enable PR/diff mode: prefix every job's prompt with `git diff <branch>`
    /// run in the target project (module-scoped where applicable).
    #[must_use]
    pub fn diff_branch(mut self, branch: impl Into<String>) -> Self {
        self.batch.diff_branch = Some(branch.into());
        self
    }

    /// Finalizes the batch.
    ///
    /// # Errors
    /// Returns [`CoreError::Invalid`] if the batch has no prompts or no
    /// targets — either leaves nothing to run.
    pub fn build(self) -> Result<Batch> {
        if self.batch.prompts.is_empty() {
            return Err(CoreError::Invalid(
                "a batch needs at least one prompt".into(),
            ));
        }
        if self.batch.targets.is_empty() {
            return Err(CoreError::Invalid(
                "a batch needs at least one target project".into(),
            ));
        }
        Ok(self.batch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_materializes_the_prompt_target_product() {
        let batch = BatchBuilder::new("b")
            .prompt("a", "prompt a")
            .prompt("b", "prompt b")
            .target_project(ProjectId::new())
            .target_project(ProjectId::new())
            .build()
            .unwrap();
        assert_eq!(batch.item_count(), 4);
        // No known modules → one whole-repo item per (prompt, target).
        assert_eq!(batch.materialize(&HashMap::new()).len(), 4);
    }

    #[test]
    fn materialize_fans_out_over_modules() {
        let project = ProjectId::new();
        let m1 = ModuleId::new();
        let m2 = ModuleId::new();
        let m3 = ModuleId::new();
        let batch = BatchBuilder::new("b")
            .prompt("a", "x")
            .prompt("b", "y")
            .target_project(project)
            .build()
            .unwrap();
        let modules = HashMap::from([(project, vec![m1, m2, m3])]);

        // 2 prompts × 1 target × 3 modules = 6 items, all module-scoped.
        let items = batch.materialize(&modules);
        assert_eq!(items.len(), 6);
        assert!(items.iter().all(|i| i.module_id.is_some()));
    }

    #[test]
    fn global_batch_skips_module_fanout() {
        let project = ProjectId::new();
        let mut batch = BatchBuilder::new("b")
            .prompt("a", "x")
            .prompt("b", "y")
            .target_project(project)
            .build()
            .unwrap();
        batch.global = true;
        let modules = HashMap::from([(project, vec![ModuleId::new(), ModuleId::new()])]);

        // Global → 2 prompts × 1 target, no module fanout, module_id None.
        let items = batch.materialize(&modules);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.module_id.is_none()));
    }

    #[test]
    fn target_module_subset_overrides_project_modules() {
        let project = ProjectId::new();
        let m1 = ModuleId::new();
        let m2 = ModuleId::new();
        let m3 = ModuleId::new();
        let mut batch = BatchBuilder::new("b")
            .prompt("a", "x")
            .target_project(project)
            .build()
            .unwrap();
        // Pin the single target to just m2.
        batch.targets[0].module_ids = Some(vec![m2]);
        let modules = HashMap::from([(project, vec![m1, m2, m3])]);

        let items = batch.materialize(&modules);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].module_id, Some(m2));
    }

    #[test]
    fn default_one_module_project_yields_one_item_per_pair() {
        // Mirrors Project::new seeding exactly one "root" module.
        let project = ProjectId::new();
        let root = ModuleId::new();
        let batch = BatchBuilder::new("b")
            .prompt("a", "x")
            .target_project(project)
            .build()
            .unwrap();
        let modules = HashMap::from([(project, vec![root])]);
        let items = batch.materialize(&modules);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].module_id, Some(root));
    }

    #[test]
    fn builder_rejects_an_empty_batch() {
        assert!(matches!(
            BatchBuilder::new("b").build(),
            Err(CoreError::Invalid(_))
        ));
        assert!(matches!(
            BatchBuilder::new("b").prompt("a", "x").build(),
            Err(CoreError::Invalid(_))
        ));
    }
}
