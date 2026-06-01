//! A higher-level, stateful agent session.

use async_trait::async_trait;

use crate::Result;
use crate::model::AgentOutput;

/// A managed conversation with one agent.
///
/// Where [`AgentTransport`](super::AgentTransport) is event-stream oriented, an
/// `AgentSession` runs a prompt to completion and yields a collected
/// [`AgentOutput`]. Sessions are pooled and reused by `netra-agent`.
#[async_trait]
pub trait AgentSession: Send + Sync {
    /// The `pi` session id, once established.
    fn session_ref(&self) -> Option<&str>;

    /// Runs a prompt to completion, collecting the full output.
    async fn run(&self, prompt: &str) -> Result<AgentOutput>;

    /// Interrupts the in-flight turn.
    async fn interrupt(&self) -> Result<()>;

    /// Closes the session, releasing process resources.
    async fn close(&self) -> Result<()>;
}
