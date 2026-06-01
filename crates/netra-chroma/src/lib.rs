//! ChromaDB integration for NETRA (opt-in).
//!
//! This crate is only compiled when the `chromadb` feature is enabled on
//! `netra-api` (or when depended on directly, e.g. by the Tauri shell).
//! Nothing in this crate is invoked unless [`ChromaConfig::enabled`] is `true`
//! in the application's config — the host application must guard all entry
//! points behind that flag.

pub mod bootstrap;
pub mod client;
pub mod error;
pub mod handle;
pub mod ignore_rules;
pub mod indexer;
pub mod mcp;
pub mod prompt;
pub mod query;
pub mod server;
mod win;

pub use error::ChromaError;
pub use handle::{ChromaHandle, ChromaStatus};
pub use indexer::{IndexProgress, IndexStats, IndexedFile};
pub use prompt::chromadb_system_prompt;
pub use query::QueryHit;
pub use server::{Collection, CollectionInfo};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the ChromaDB integration.
///
/// Lives under `[chromadb]` in `netra.toml`. When `enabled = false` (the
/// default), **no** code in this crate is invoked — the host application must
/// check the flag before constructing a [`ChromaHandle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChromaConfig {
    /// Master switch. Default `false` — must be explicitly opted into.
    pub enabled: bool,
    /// Bind host for the chroma server.
    pub host: String,
    /// Bind port for the chroma server.
    pub port: u16,
    /// Where chroma persists its data on disk.
    pub data_dir: PathBuf,
    /// Whether the host application should start the server on launch.
    pub auto_start: bool,
    /// Skip any file whose size exceeds this many bytes.
    pub max_file_size_bytes: u64,
    /// Extra glob patterns (in addition to `.gitignore` + built-in binary
    /// blacklist) to skip during indexing. e.g. `*.log`, `vendor/**`.
    pub extra_ignore_globs: Vec<String>,
    /// Embedding model preset id. `"default"` uses chromadb's bundled ONNX
    /// `all-MiniLM-L6-v2`. Any other recognized preset (see
    /// [`ChromaConfig::resolved_model`]) selects a sentence-transformers
    /// model. `"custom"` defers to `embedding_model_custom`.
    pub embedding_model: String,
    /// Free-text HuggingFace model id, used only when
    /// `embedding_model == "custom"`.
    #[serde(default)]
    pub embedding_model_custom: Option<String>,
}

impl Default for ChromaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "127.0.0.1".to_string(),
            port: 8765,
            data_dir: default_data_dir(),
            auto_start: true,
            max_file_size_bytes: 1_048_576,
            extra_ignore_globs: Vec::new(),
            embedding_model: "default".to_string(),
            embedding_model_custom: None,
        }
    }
}

impl ChromaConfig {
    /// Returns the base URL of the chroma HTTP API.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// Returns the collection name used for a given project id.
    #[must_use]
    pub fn collection_name(project_id: &str) -> String {
        format!("netra_{project_id}")
    }

    /// Resolve `embedding_model` to the string the python helpers consume.
    /// Returns `"default"` for the built-in ONNX model, otherwise the HF
    /// model id for the SentenceTransformer backend.
    #[must_use]
    pub fn resolved_model(&self) -> String {
        match self.embedding_model.as_str() {
            "default" => "default".to_string(),
            "jina-code" => "jinaai/jina-embeddings-v2-base-code".to_string(),
            "coderank" => "nomic-ai/CodeRankEmbed".to_string(),
            "sfr-code" => "Salesforce/SFR-Embedding-Code-400M_R".to_string(),
            "bge-code" => "BAAI/bge-code-v1".to_string(),
            "custom" => self
                .embedding_model_custom
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            other => other.to_string(),
        }
    }
}

fn default_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".netra")
        .join("chroma-data")
}

/// Returns the directory under which we install uv + the chroma venv.
#[must_use]
pub fn netra_runtime_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".netra")
}
