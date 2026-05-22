//! Normalized agent interaction types.
//!
//! `pi`'s raw RPC event stream is mapped onto [`AgentEvent`] by `chatur-agent`,
//! so the rest of the workspace never sees provider-specific shapes.

use serde::{Deserialize, Serialize};

/// A single normalized event emitted while an agent turn runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A new turn has begun.
    TurnStart,
    /// Assistant produced visible text (possibly a streamed delta).
    AssistantText {
        /// The text fragment.
        text: String,
    },
    /// Assistant produced reasoning/thinking output.
    Thinking {
        /// The thinking fragment.
        text: String,
    },
    /// The agent invoked a tool.
    ToolCall {
        /// Tool name, e.g. `bash`, `edit`.
        name: String,
        /// Raw tool arguments.
        args: serde_json::Value,
    },
    /// A tool finished.
    ToolResult {
        /// Tool name.
        name: String,
        /// Whether the tool reported an error.
        is_error: bool,
    },
    /// Token usage / cost reported by the provider.
    Usage(TokenUsage),
    /// The turn finished.
    TurnEnd,
    /// The agent reported an error.
    Error {
        /// Error message.
        message: String,
    },
}

/// Token counts and optional cost for one turn or aggregated run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Prompt/input tokens consumed.
    pub input_tokens: u64,
    /// Completion/output tokens produced.
    pub output_tokens: u64,
    /// Provider-reported cost in USD, when available.
    pub cost_usd: Option<f64>,
}

impl std::ops::Add for TokenUsage {
    type Output = Self;

    /// Adds two usage records field-wise.
    fn add(self, other: Self) -> Self {
        Self {
            input_tokens: self.input_tokens + other.input_tokens,
            output_tokens: self.output_tokens + other.output_tokens,
            cost_usd: match (self.cost_usd, other.cost_usd) {
                (Some(a), Some(b)) => Some(a + b),
                (a, b) => a.or(b),
            },
        }
    }
}

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

/// The final result of one completed agent job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Full assistant text.
    pub text: String,
    /// Parsed structured output, when the job requested a defined schema.
    pub structured: Option<serde_json::Value>,
    /// Total usage for the job.
    pub usage: TokenUsage,
}
