//! [`PiSession`] — a prompt-to-completion wrapper over an [`AgentTransport`].

use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;

use netra_core::model::{AgentEvent, AgentOutput, TokenUsage};
use netra_core::traits::{AgentSession, AgentTransport, PromptRequest};
use netra_core::{CoreError, Result};

/// A stateful conversation with one agent.
///
/// Where [`AgentTransport`] is event-stream oriented, `PiSession::run` drains
/// the stream of a turn and collects an [`AgentOutput`].
pub struct PiSession {
    transport: Arc<dyn AgentTransport>,
    session_ref: Option<String>,
}

impl PiSession {
    /// Wraps a transport in a session.
    #[must_use]
    pub fn new(transport: Arc<dyn AgentTransport>) -> Self {
        Self {
            transport,
            session_ref: None,
        }
    }
}

#[async_trait]
impl AgentSession for PiSession {
    fn session_ref(&self) -> Option<&str> {
        self.session_ref.as_deref()
    }

    async fn run(&self, prompt: &str) -> Result<AgentOutput> {
        let mut stream = self
            .transport
            .send_prompt(PromptRequest::new(prompt))
            .await?;

        let mut text = String::new();
        let mut usage = TokenUsage::default();
        let mut failure: Option<String> = None;

        while let Some(event) = stream.next().await {
            match event {
                AgentEvent::AssistantText { text: delta } => text.push_str(&delta),
                AgentEvent::Usage(reported) => usage += reported,
                AgentEvent::Error { message } => failure = Some(message),
                // Thinking, tool, and turn markers do not affect the output.
                _ => {}
            }
        }

        if let Some(message) = failure {
            return Err(CoreError::Agent(message));
        }

        // If the agent was asked for a defined output, the text may itself be
        // JSON. Parse opportunistically — `None` simply means "plain text".
        let structured = serde_json::from_str(text.trim()).ok();

        Ok(AgentOutput {
            text,
            structured,
            usage,
        })
    }

    async fn interrupt(&self) -> Result<()> {
        self.transport.abort().await
    }

    async fn close(&self) -> Result<()> {
        self.transport.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockTransport;

    #[tokio::test]
    async fn run_collects_assistant_text() {
        let transport = Arc::new(MockTransport::replying("hello world"));
        let session = PiSession::new(transport);
        let output = session.run("hi").await.unwrap();
        assert_eq!(output.text, "hello world");
        assert!(output.structured.is_none());
    }

    #[tokio::test]
    async fn run_parses_structured_json_output() {
        let transport = Arc::new(MockTransport::replying(r#"{"score": 7}"#));
        let session = PiSession::new(transport);
        let output = session.run("rate it").await.unwrap();
        assert_eq!(output.structured.unwrap()["score"], 7);
    }

    #[tokio::test]
    async fn run_surfaces_agent_errors() {
        let transport = Arc::new(MockTransport::new(vec![
            AgentEvent::TurnStart,
            AgentEvent::Error {
                message: "model unavailable".to_string(),
            },
        ]));
        let session = PiSession::new(transport);
        let err = session.run("hi").await.unwrap_err();
        assert!(matches!(err, CoreError::Agent(_)));
    }
}
