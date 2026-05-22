//! Batches: one prompt fanned out across many targets, then aggregated.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ids::{BatchId, BatchItemId, JobId, ProjectId, TemplateId};

/// A map/reduce unit of work: run [`template`](Self::template) against every
/// [`target`](Self::targets), then combine the outputs with
/// [`aggregation`](Self::aggregation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    /// Stable identifier.
    pub id: BatchId,
    /// Human-readable name.
    pub name: String,
    /// The prompt run against each target (the "map" step).
    pub template: PromptSource,
    /// The targets to fan out over.
    pub targets: Vec<BatchTarget>,
    /// How per-target outputs are combined (the "reduce" step).
    pub aggregation: AggregationSpec,
    /// Optional JSON schema each target output must conform to.
    pub output_schema: Option<serde_json::Value>,
}

/// Where a batch's prompt text comes from.
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
    /// Values bound to the template's `{{variable}}` placeholders.
    pub variables: HashMap<String, String>,
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

/// One materialized member of a batch: the job for a single target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    /// Stable identifier.
    pub id: BatchItemId,
    /// Owning batch.
    pub batch_id: BatchId,
    /// The job created for this item, once scheduled.
    pub job_id: Option<JobId>,
    /// The target this item covers.
    pub target: BatchTarget,
}
