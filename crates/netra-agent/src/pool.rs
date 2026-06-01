//! [`AgentPool`] — bounds concurrent agent processes; [`TransportFactory`].

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

use netra_core::ids::ProjectId;
use netra_core::traits::AgentTransport;
use netra_core::{CoreError, Result};

use crate::session::PiSession;
use crate::spec::AgentSpec;
use crate::transport::RpcTransport;

/// Creates [`AgentTransport`]s on demand (the Factory pattern).
///
/// The real implementation spawns `pi`; tests substitute
/// [`MockTransportFactory`](crate::MockTransportFactory).
#[async_trait]
pub trait TransportFactory: Send + Sync {
    /// Creates a transport for the given spec.
    async fn create(&self, spec: &AgentSpec) -> Result<Arc<dyn AgentTransport>>;
}

/// The production [`TransportFactory`]: spawns `pi --mode rpc` processes.
#[derive(Debug, Clone, Copy, Default)]
pub struct PiTransportFactory;

#[async_trait]
impl TransportFactory for PiTransportFactory {
    async fn create(&self, spec: &AgentSpec) -> Result<Arc<dyn AgentTransport>> {
        Ok(Arc::new(RpcTransport::spawn(spec).await?))
    }
}

/// Bounds how many agent processes run at once — globally and per project.
///
/// P1 creates a fresh transport per [`acquire`](Self::acquire) and tears it
/// down on release; transport reuse/caching is a later optimization.
pub struct AgentPool {
    factory: Arc<dyn TransportFactory>,
    global: Arc<Semaphore>,
    per_project_max: usize,
    per_project: Mutex<HashMap<ProjectId, Arc<Semaphore>>>,
}

impl AgentPool {
    /// Creates a pool. Both limits are clamped to a minimum of 1.
    #[must_use]
    pub fn new(
        factory: Arc<dyn TransportFactory>,
        global_max: usize,
        per_project_max: usize,
    ) -> Self {
        Self {
            factory,
            global: Arc::new(Semaphore::new(global_max.max(1))),
            per_project_max: per_project_max.max(1),
            per_project: Mutex::new(HashMap::new()),
        }
    }

    /// Acquires an agent for `project`, blocking until both the global and the
    /// per-project concurrency budgets allow it.
    ///
    /// # Errors
    /// Returns an error if the pool is closed or the transport fails to spawn.
    pub async fn acquire(&self, project: ProjectId, spec: AgentSpec) -> Result<AgentLease> {
        let project_sem = {
            let mut map = self.per_project.lock().await;
            map.entry(project)
                .or_insert_with(|| Arc::new(Semaphore::new(self.per_project_max)))
                .clone()
        };

        let global_permit = self
            .global
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| CoreError::Other("agent pool is closed".to_string()))?;
        let project_permit = project_sem
            .acquire_owned()
            .await
            .map_err(|_| CoreError::Other("agent pool is closed".to_string()))?;

        let transport = self.factory.create(&spec).await?;

        Ok(AgentLease {
            transport,
            _global: global_permit,
            _project: project_permit,
        })
    }
}

/// A checked-out agent. Concurrency permits are released when it is dropped.
pub struct AgentLease {
    transport: Arc<dyn AgentTransport>,
    _global: OwnedSemaphorePermit,
    _project: OwnedSemaphorePermit,
}

impl AgentLease {
    /// The leased transport.
    #[must_use]
    pub fn transport(&self) -> Arc<dyn AgentTransport> {
        self.transport.clone()
    }

    /// A [`PiSession`] over the leased transport.
    #[must_use]
    pub fn session(&self) -> PiSession {
        PiSession::new(self.transport.clone())
    }

    /// Shuts the transport down and releases the lease.
    ///
    /// Calling this is optional — [`Drop`] also shuts the transport down — but
    /// `release` lets the caller await and observe shutdown errors.
    ///
    /// # Errors
    /// Returns any error reported by the transport's shutdown.
    pub async fn release(self) -> Result<()> {
        self.transport.shutdown().await
    }
}

impl Drop for AgentLease {
    fn drop(&mut self) {
        // Best-effort async shutdown; the semaphore permits are freed
        // synchronously as this struct's fields drop.
        let transport = self.transport.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = transport.shutdown().await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::mock::MockTransportFactory;
    use netra_core::traits::AgentSession;

    fn spec() -> AgentSpec {
        AgentSpec::new("pi", "/tmp")
    }

    #[tokio::test]
    async fn acquire_produces_a_usable_session() {
        let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 2, 2);
        let lease = pool.acquire(ProjectId::new(), spec()).await.unwrap();
        let output = lease.session().run("hi").await.unwrap();
        assert_eq!(output.text, "ok");
    }

    #[tokio::test]
    async fn global_limit_blocks_then_frees_on_drop() {
        let factory = Arc::new(MockTransportFactory::default());
        let pool = AgentPool::new(factory, 2, 8);
        let project = ProjectId::new();

        let lease_a = pool.acquire(project, spec()).await.unwrap();
        let _lease_b = pool.acquire(project, spec()).await.unwrap();

        // Third acquire must block — the global budget (2) is exhausted.
        let blocked =
            tokio::time::timeout(Duration::from_millis(100), pool.acquire(project, spec())).await;
        assert!(blocked.is_err(), "third acquire should block while full");

        // Releasing one lease frees a permit; the next acquire succeeds.
        drop(lease_a);
        let unblocked =
            tokio::time::timeout(Duration::from_millis(500), pool.acquire(project, spec())).await;
        assert!(unblocked.is_ok(), "acquire should proceed after a release");
    }

    #[tokio::test]
    async fn per_project_limit_is_independent_across_projects() {
        let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 8, 1);
        let project_a = ProjectId::new();
        let project_b = ProjectId::new();

        let _a = pool.acquire(project_a, spec()).await.unwrap();
        // project_a is at its per-project cap of 1, but project_b is free.
        let b =
            tokio::time::timeout(Duration::from_millis(200), pool.acquire(project_b, spec())).await;
        assert!(b.is_ok(), "a different project must not be blocked");
    }
}
