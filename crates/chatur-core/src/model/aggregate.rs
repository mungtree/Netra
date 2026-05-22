//! The consolidated output of a batch's reduce step.

use serde::{Deserialize, Serialize};

use crate::model::TokenUsage;

/// The result of combining many [`AgentOutput`](super::AgentOutput)s via an
/// [`Aggregator`](crate::traits::Aggregator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResult {
    /// Human-readable consolidated text (e.g. the reviewer agent's summary).
    pub summary: String,
    /// Consolidated structured output, when the strategy produces one.
    pub structured: Option<serde_json::Value>,
    /// How many source outputs were combined.
    pub source_count: usize,
    /// Combined token usage across the sources and any reduce-step agent call.
    pub usage: TokenUsage,
}
