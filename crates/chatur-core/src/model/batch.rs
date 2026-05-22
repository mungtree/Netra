//! Batches: a series of prompts fanned out across targets, then aggregated.
//!
//! A [`Batch`] is the map/reduce unit of work. Its **map** step expands the
//! cartesian product [`prompts`](Batch::prompts) × [`targets`](Batch::targets)
//! into one [`BatchItem`] (and, once run, one job) per pair; its **reduce**
//! step combines every per-item output via [`aggregation`](Batch::aggregation).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BatchId, BatchItemId, JobId, ProjectId, TemplateId};
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
            status: BatchStatus::Pending,
            result: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// How many `(prompt, target)` pairs this batch fans out into.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.prompts.len() * self.targets.len()
    }

    /// Expands the `prompts × targets` product into one [`BatchItem`] per pair.
    #[must_use]
    pub fn materialize(&self) -> Vec<BatchItem> {
        let mut items = Vec::with_capacity(self.item_count());
        for prompt in &self.prompts {
            for target in &self.targets {
                items.push(BatchItem::new(self.id, prompt.name.clone(), target.clone()));
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
}

impl BatchTarget {
    /// A target with no variable bindings.
    #[must_use]
    pub fn project(project_id: ProjectId) -> Self {
        Self {
            project_id,
            variables: HashMap::new(),
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
}

impl BatchItem {
    /// Creates an unscheduled item with a fresh id.
    #[must_use]
    pub fn new(batch_id: BatchId, prompt_name: impl Into<String>, target: BatchTarget) -> Self {
        Self {
            id: BatchItemId::new(),
            batch_id,
            job_id: None,
            prompt_name: prompt_name.into(),
            target,
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

    /// Finalizes the batch.
    ///
    /// # Errors
    /// Returns [`CoreError::Invalid`] if the batch has no prompts or no
    /// targets — either leaves nothing to run.
    pub fn build(self) -> Result<Batch> {
        if self.batch.prompts.is_empty() {
            return Err(CoreError::Invalid("a batch needs at least one prompt".into()));
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
        assert_eq!(batch.materialize().len(), 4);
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
