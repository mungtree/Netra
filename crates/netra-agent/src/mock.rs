//! In-process test doubles for [`AgentTransport`] and [`TransportFactory`].
//!
//! Exported (not test-gated) so downstream crates can drive their own tests
//! without spawning a real `pi` process.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;

use netra_core::Result;
use netra_core::model::AgentEvent;
use netra_core::traits::{AgentTransport, PromptRequest};

use crate::pool::TransportFactory;
use crate::spec::AgentSpec;

/// An [`AgentTransport`] that replays a fixed event script for every prompt.
pub struct MockTransport {
    script: Vec<AgentEvent>,
    /// When `true`, the stream emits `script` then stays pending forever
    /// instead of closing — useful for simulating a stuck tool call.
    hang_after_script: bool,
    prompts: Mutex<Vec<String>>,
    steer_messages: Mutex<Vec<String>>,
    steers: AtomicUsize,
    aborts: AtomicUsize,
    shutdowns: AtomicUsize,
}

impl MockTransport {
    /// Builds a transport that replays `script` on each `send_prompt`.
    #[must_use]
    pub fn new(script: Vec<AgentEvent>) -> Self {
        Self {
            script,
            hang_after_script: false,
            prompts: Mutex::new(Vec::new()),
            steer_messages: Mutex::new(Vec::new()),
            steers: AtomicUsize::new(0),
            aborts: AtomicUsize::new(0),
            shutdowns: AtomicUsize::new(0),
        }
    }

    /// Like [`new`](Self::new) but the returned event stream never closes
    /// after the script is exhausted — it stays pending so the runner sees
    /// the simulated tool call as still in flight.
    #[must_use]
    pub fn pending_after(script: Vec<AgentEvent>) -> Self {
        let mut t = Self::new(script);
        t.hang_after_script = true;
        t
    }

    /// Convenience: a transport whose turn emits a single assistant message.
    #[must_use]
    pub fn replying(text: impl Into<String>) -> Self {
        Self::new(vec![
            AgentEvent::TurnStart,
            AgentEvent::AssistantText { text: text.into() },
            AgentEvent::TurnEnd,
        ])
    }

    /// The prompt messages received so far, in order.
    #[must_use]
    pub fn prompts(&self) -> Vec<String> {
        self.prompts.lock().expect("mock lock poisoned").clone()
    }

    /// How many times [`abort`](AgentTransport::abort) was called.
    #[must_use]
    pub fn abort_count(&self) -> usize {
        self.aborts.load(Ordering::SeqCst)
    }

    /// How many times [`send_steer`](AgentTransport::send_steer) was called.
    #[must_use]
    pub fn steer_count(&self) -> usize {
        self.steers.load(Ordering::SeqCst)
    }

    /// The steer messages received so far, in order.
    #[must_use]
    pub fn steer_messages(&self) -> Vec<String> {
        self.steer_messages
            .lock()
            .expect("mock lock poisoned")
            .clone()
    }

    /// How many times [`shutdown`](AgentTransport::shutdown) was called.
    #[must_use]
    pub fn shutdown_count(&self) -> usize {
        self.shutdowns.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl AgentTransport for MockTransport {
    async fn send_prompt(&self, request: PromptRequest) -> Result<BoxStream<'static, AgentEvent>> {
        self.prompts
            .lock()
            .expect("mock lock poisoned")
            .push(request.message);
        let scripted = futures::stream::iter(self.script.clone());
        if self.hang_after_script {
            Ok(scripted.chain(futures::stream::pending()).boxed())
        } else {
            Ok(scripted.boxed())
        }
    }

    async fn send_steer(&self, request: PromptRequest) -> Result<()> {
        self.steers.fetch_add(1, Ordering::SeqCst);
        self.steer_messages
            .lock()
            .expect("mock lock poisoned")
            .push(request.message);
        Ok(())
    }

    async fn abort(&self) -> Result<()> {
        self.aborts.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// A [`TransportFactory`] that hands out [`MockTransport`]s.
#[derive(Default)]
pub struct MockTransportFactory {
    created: AtomicUsize,
}

impl MockTransportFactory {
    /// How many transports this factory has created.
    #[must_use]
    pub fn created_count(&self) -> usize {
        self.created.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl TransportFactory for MockTransportFactory {
    async fn create(&self, _spec: &AgentSpec) -> Result<Arc<dyn AgentTransport>> {
        self.created.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(MockTransport::replying("ok")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_records_prompts_and_replays_script() {
        let transport = MockTransport::replying("pong");
        let mut stream = transport
            .send_prompt(PromptRequest::new("ping"))
            .await
            .unwrap();

        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }

        assert_eq!(transport.prompts(), vec!["ping".to_string()]);
        assert_eq!(events.len(), 3);
        assert!(matches!(events[1], AgentEvent::AssistantText { .. }));
    }
}
