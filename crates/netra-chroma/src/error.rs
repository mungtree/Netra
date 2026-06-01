use std::path::PathBuf;

/// Errors raised by the ChromaDB integration.
#[derive(Debug, thiserror::Error)]
pub enum ChromaError {
    #[error("uv binary not found on PATH and auto-install failed: {0}")]
    UvUnavailable(String),

    #[error("failed to bootstrap chroma venv: {0}")]
    Bootstrap(String),

    #[error("chroma server failed to start: {0}")]
    ServerStart(String),

    #[error("chroma server health check timed out after {0:?}")]
    HealthTimeout(std::time::Duration),

    #[error("chroma server is not running")]
    NotRunning,

    #[error("chroma server already running")]
    AlreadyRunning,

    #[error("io error at {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("pi config error: {0}")]
    PiConfig(String),

    #[error("indexer error: {0}")]
    Indexer(String),

    #[error("query helper failed at {stage}: {message}")]
    Query {
        stage: String,
        message: String,
        stderr: Option<String>,
    },

    #[error("{0}")]
    Other(String),
}

impl ChromaError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
