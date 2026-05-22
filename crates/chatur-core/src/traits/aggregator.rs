//! The batch reduce-step strategy interface.

use async_trait::async_trait;

use crate::Result;
use crate::model::{AgentOutput, AggregatedResult};

/// A reduce strategy: combines many per-target outputs into one result.
///
/// This is the Strategy pattern — `chatur-engine` keeps a registry of
/// implementations (`concat`, `reviewer`, `schema_merge`, ...) keyed by
/// [`id`](Self::id), selected per batch via
/// [`AggregationSpec`](crate::model::AggregationSpec).
#[async_trait]
pub trait Aggregator: Send + Sync {
    /// Stable strategy id, matched against `AggregationSpec.strategy`.
    fn id(&self) -> &str;

    /// One-line description for UI listings.
    fn description(&self) -> &str;

    /// Combines `outputs` into a single [`AggregatedResult`].
    ///
    /// `config` is the strategy-specific JSON from the batch's
    /// [`AggregationSpec`](crate::model::AggregationSpec).
    async fn aggregate(
        &self,
        outputs: Vec<AgentOutput>,
        config: &serde_json::Value,
    ) -> Result<AggregatedResult>;
}
