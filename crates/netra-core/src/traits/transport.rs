//! The low-level transport to a single agent process.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::Result;
use crate::model::AgentEvent;

/// A prompt sent to an agent over a transport.
#[derive(Debug, Clone)]
pub struct PromptRequest {
    /// The prompt text (the `pi` RPC `message` field).
    pub message: String,
    /// Existing `pi` session to continue; `None` starts a fresh session.
    pub session_ref: Option<String>,
}

impl PromptRequest {
    /// Builds a request that starts a new session.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            session_ref: None,
        }
    }
}

/// Drives one agent process: send a prompt, receive normalized events.
///
/// Implemented by `netra-agent`'s `RpcTransport` over `pi --mode rpc`, and by
/// `MockTransport` in tests.
#[async_trait]
pub trait AgentTransport: Send + Sync {
    /// Sends a prompt and returns the stream of events for that turn.
    async fn send_prompt(&self, request: PromptRequest) -> Result<BoxStream<'static, AgentEvent>>;

    /// Sends a steer requests to the currently running agent.
    async fn send_steer(&self, request: PromptRequest) -> Result<()>;

    /// Aborts the turn currently in flight, if any.
    async fn abort(&self) -> Result<()>;


    /// Shuts the transport (and its underlying process) down.
    async fn shutdown(&self) -> Result<()>;
}
