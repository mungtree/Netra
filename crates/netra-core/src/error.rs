//! Crate-wide error type.

use thiserror::Error;

/// The single error type returned across `netra-core` interfaces.
///
/// Higher crates map their concrete failures (process I/O, SQL, RPC framing)
/// into these variants so callers depend only on a stable surface.
#[derive(Debug, Error)]
pub enum CoreError {
    /// An entity was looked up by id but does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// Caller-supplied input failed validation.
    #[error("invalid input: {0}")]
    Invalid(String),

    /// The agent process reported a failure.
    #[error("agent error: {0}")]
    Agent(String),

    /// Attempted to steer agent without a active turn
    #[error("no turn steer error: {0}")]
    SteerNoTurn(String),

    /// The transport to the agent failed (spawn, framing, I/O).
    #[error("transport error: {0}")]
    Transport(String),

    /// A persistence-layer failure.
    #[error("storage error: {0}")]
    Storage(String),

    /// (De)serialization failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// The operation was cancelled before completion.
    #[error("operation cancelled")]
    Cancelled,

    /// An error that does not fit the categories above.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the workspace.
pub type Result<T> = std::result::Result<T, CoreError>;
