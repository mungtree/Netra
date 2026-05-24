use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::client::ChromaClient;
use crate::error::ChromaError;
use crate::server::{Collection, ServerProcess};
use crate::ChromaConfig;

/// High-level status of the chroma server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum ChromaStatus {
    /// Bootstrap (uv/venv install) has not been completed.
    NotInstalled,
    /// Server is not running.
    Stopped,
    /// Server is starting up; health check has not yet succeeded.
    Starting,
    /// Server is up and responding to health checks.
    Running {
        pid: u32,
        port: u16,
    },
    /// Last operation failed.
    Error {
        message: String,
    },
}

/// Owns the chroma server child process (if any) and the HTTP client used to
/// talk to it. Cheaply clonable via `Arc`.
#[derive(Clone)]
pub struct ChromaHandle {
    inner: Arc<HandleInner>,
}

struct HandleInner {
    config: RwLock<ChromaConfig>,
    server: Mutex<Option<ServerProcess>>,
    status: RwLock<ChromaStatus>,
    client: ChromaClient,
}

impl ChromaHandle {
    /// Construct a new handle. Does NOT start the server.
    #[must_use]
    pub fn new(config: ChromaConfig) -> Self {
        let client = ChromaClient::new(config.base_url());
        Self {
            inner: Arc::new(HandleInner {
                config: RwLock::new(config),
                server: Mutex::new(None),
                status: RwLock::new(ChromaStatus::Stopped),
                client,
            }),
        }
    }

    pub async fn config(&self) -> ChromaConfig {
        self.inner.config.read().await.clone()
    }

    pub async fn set_config(&self, cfg: ChromaConfig) {
        let url = cfg.base_url();
        *self.inner.config.write().await = cfg;
        self.inner.client.set_base_url(url).await;
    }

    pub async fn status(&self) -> ChromaStatus {
        self.inner.status.read().await.clone()
    }

    pub async fn set_status(&self, s: ChromaStatus) {
        *self.inner.status.write().await = s;
    }

    pub(crate) async fn take_server(&self) -> Option<ServerProcess> {
        self.inner.server.lock().await.take()
    }

    pub(crate) async fn store_server(&self, s: ServerProcess) {
        *self.inner.server.lock().await = Some(s);
    }

    pub(crate) async fn has_server(&self) -> bool {
        self.inner.server.lock().await.is_some()
    }

    /// Returns the HTTP client for the server. Caller must check `status()`
    /// before assuming requests will succeed.
    #[must_use]
    pub fn client(&self) -> &ChromaClient {
        &self.inner.client
    }

    /// Convenience: whether the server is currently in `Running` state.
    pub async fn is_running(&self) -> bool {
        matches!(self.status().await, ChromaStatus::Running { .. })
    }

    /// List collections via the server.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, ChromaError> {
        if !self.is_running().await {
            return Err(ChromaError::NotRunning);
        }
        self.inner.client.list_collections().await
    }

    /// Delete a collection by name.
    pub async fn delete_collection(&self, name: &str) -> Result<(), ChromaError> {
        if !self.is_running().await {
            return Err(ChromaError::NotRunning);
        }
        self.inner.client.delete_collection(name).await
    }
}
