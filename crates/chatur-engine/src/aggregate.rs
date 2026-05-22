//! Reduce-step aggregators and the registry that selects them.
//!
//! Each [`Aggregator`] is a Strategy: [`AggregatorRegistry`] keeps them keyed
//! by [`Aggregator::id`], and [`BatchExecutor`](crate::BatchExecutor) looks one
//! up per batch from its [`AggregationSpec`](chatur_core::model::AggregationSpec).
//!
//! Two pure strategies live here — [`ConcatAggregator`] and
//! [`SchemaMergeAggregator`]. The `reviewer` strategy needs a live agent and is
//! handled directly by the executor, so it is not a registry entry.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use chatur_core::Result;
use chatur_core::model::{AgentOutput, AggregatedResult, TokenUsage};
use chatur_core::traits::Aggregator;

/// A keyed set of reduce strategies.
///
/// Construct with [`with_defaults`](Self::with_defaults) for the built-ins, then
/// [`register`](Self::register) any extras (the plugin seam from `PLAN.md` §6).
#[derive(Clone, Default)]
pub struct AggregatorRegistry {
    strategies: HashMap<String, Arc<dyn Aggregator>>,
}

impl AggregatorRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry pre-loaded with the built-in pure strategies.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(ConcatAggregator));
        registry.register(Arc::new(SchemaMergeAggregator));
        registry
    }

    /// Adds (or replaces) a strategy, keyed by its [`Aggregator::id`].
    pub fn register(&mut self, aggregator: Arc<dyn Aggregator>) {
        self.strategies
            .insert(aggregator.id().to_string(), aggregator);
    }

    /// Looks up a strategy by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<Arc<dyn Aggregator>> {
        self.strategies.get(id).cloned()
    }

    /// The ids of every registered strategy.
    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }
}

/// Sums the token usage of every output.
fn total_usage(outputs: &[AgentOutput]) -> TokenUsage {
    outputs.iter().fold(TokenUsage::default(), |mut acc, o| {
        acc += o.usage;
        acc
    })
}

/// Concatenates every output verbatim under a numbered header.
///
/// The simplest reduce step: no agent call, no parsing — useful as a default
/// and as a fallback when richer strategies are not configured.
pub struct ConcatAggregator;

#[async_trait]
impl Aggregator for ConcatAggregator {
    fn id(&self) -> &str {
        "concat"
    }

    fn description(&self) -> &str {
        "Concatenates every per-item output verbatim."
    }

    async fn aggregate(
        &self,
        outputs: Vec<AgentOutput>,
        _config: &serde_json::Value,
    ) -> Result<AggregatedResult> {
        let usage = total_usage(&outputs);
        let summary = outputs
            .iter()
            .enumerate()
            .map(|(i, o)| format!("=== Output {} ===\n{}", i + 1, o.text))
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(AggregatedResult {
            summary,
            structured: None,
            source_count: outputs.len(),
            usage,
        })
    }
}

/// Merges the structured JSON of every output into one array.
///
/// Each output's `structured` value contributes its elements (arrays are
/// flattened, scalars/objects pushed whole). Duplicates are dropped: by the
/// field named in `config.dedup_key` when present, otherwise by deep equality.
pub struct SchemaMergeAggregator;

#[async_trait]
impl Aggregator for SchemaMergeAggregator {
    fn id(&self) -> &str {
        "schema_merge"
    }

    fn description(&self) -> &str {
        "Merges every item's structured JSON into one deduplicated array."
    }

    async fn aggregate(
        &self,
        outputs: Vec<AgentOutput>,
        config: &serde_json::Value,
    ) -> Result<AggregatedResult> {
        let usage = total_usage(&outputs);
        let source_count = outputs.len();
        let dedup_key = config.get("dedup_key").and_then(serde_json::Value::as_str);

        let mut merged: Vec<serde_json::Value> = Vec::new();
        for output in outputs {
            let Some(value) = output.structured else {
                continue;
            };
            match value {
                serde_json::Value::Array(items) => {
                    for item in items {
                        push_unique(&mut merged, item, dedup_key);
                    }
                }
                other => push_unique(&mut merged, other, dedup_key),
            }
        }

        let structured = serde_json::Value::Array(merged);
        let summary = serde_json::to_string_pretty(&structured)?;
        Ok(AggregatedResult {
            summary,
            structured: Some(structured),
            source_count,
            usage,
        })
    }
}

/// Pushes `value` onto `merged` unless an equivalent entry is already present.
fn push_unique(merged: &mut Vec<serde_json::Value>, value: serde_json::Value, dedup_key: Option<&str>) {
    let duplicate = match dedup_key {
        Some(key) => {
            let incoming = value.get(key);
            incoming.is_some() && merged.iter().any(|existing| existing.get(key) == incoming)
        }
        None => merged.contains(&value),
    };
    if !duplicate {
        merged.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn output(text: &str, structured: Option<serde_json::Value>) -> AgentOutput {
        AgentOutput {
            text: text.to_string(),
            structured,
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                cost_usd: None,
            },
        }
    }

    #[tokio::test]
    async fn concat_joins_outputs_and_sums_usage() {
        let result = ConcatAggregator
            .aggregate(
                vec![output("first", None), output("second", None)],
                &serde_json::Value::Null,
            )
            .await
            .unwrap();
        assert_eq!(result.source_count, 2);
        assert!(result.summary.contains("first"));
        assert!(result.summary.contains("second"));
        assert_eq!(result.usage.input_tokens, 20);
        assert_eq!(result.usage.output_tokens, 10);
    }

    #[tokio::test]
    async fn schema_merge_flattens_and_dedupes_by_key() {
        let result = SchemaMergeAggregator
            .aggregate(
                vec![
                    output("", Some(json!([{ "id": 1 }, { "id": 2 }]))),
                    output("", Some(json!([{ "id": 2 }, { "id": 3 }]))),
                ],
                &json!({ "dedup_key": "id" }),
            )
            .await
            .unwrap();
        let merged = result.structured.unwrap();
        assert_eq!(merged.as_array().unwrap().len(), 3);
    }

    #[test]
    fn registry_carries_the_built_ins() {
        let registry = AggregatorRegistry::with_defaults();
        assert!(registry.get("concat").is_some());
        assert!(registry.get("schema_merge").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
