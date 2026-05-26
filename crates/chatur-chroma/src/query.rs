//! Manual / programmatic query against a chroma collection.
//!
//! Mirrors the index path: spawns a small Python helper inside the managed
//! venv so we reuse chromadb's DefaultEmbeddingFunction (the same one used
//! at index time). The helper computes embeddings client-side and POSTs the
//! query to the running chroma server.

use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::bootstrap::{ensure_query_helper, venv_dir, venv_python};
use crate::error::ChromaError;
use crate::ChromaConfig;

/// One ranked hit returned by [`query_collection`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHit {
    pub id: String,
    pub distance: f32,
    pub document: String,
    /// Convenience accessors lifted out of `metadata` for the UI.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub line_start: Option<u64>,
    #[serde(default)]
    pub line_end: Option<u64>,
    /// Full metadata blob (everything we set at index time).
    pub metadata: serde_json::Value,
}

#[derive(Deserialize)]
struct HelperResponse {
    hits: Vec<HelperHit>,
}

#[derive(Deserialize)]
struct HelperHit {
    id: String,
    distance: f32,
    document: String,
    #[serde(default)]
    metadata: serde_json::Value,
}

/// Query `collection_name` with `query_text`. `n_results` caps the response.
pub async fn query_collection(
    cfg: &ChromaConfig,
    collection_name: &str,
    query_text: &str,
    n_results: u32,
) -> Result<Vec<QueryHit>, ChromaError> {
    let python = venv_python(&venv_dir());
    if !python.exists() {
        return Err(ChromaError::Query {
            stage: "venv".into(),
            message: "venv not bootstrapped; install ChromaDB first".into(),
            stderr: None,
        });
    }
    let helper = ensure_query_helper().map_err(|e| ChromaError::Query {
        stage: "helper".into(),
        message: format!("ensure query helper: {e}"),
        stderr: None,
    })?;

    let payload = serde_json::to_vec(&serde_json::json!({
        "query_texts": [query_text],
        "n_results": n_results,
    }))
    .map_err(|e| ChromaError::Query {
        stage: "serialize".into(),
        message: format!("serialize payload: {e}"),
        stderr: None,
    })?;

    let mut cmd = Command::new(&python);
    cmd.arg(&helper)
        .arg(&cfg.host)
        .arg(cfg.port.to_string())
        .arg(collection_name)
        .arg(cfg.resolved_model())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::win::no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| {
        let msg = if e.kind() == std::io::ErrorKind::NotFound {
            "python missing from chroma venv — reinstall ChromaDB".to_string()
        } else {
            format!("spawn query helper: {e}")
        };
        tracing::error!(target: "chatur::chroma::query", "{msg}");
        ChromaError::Query {
            stage: "spawn".into(),
            message: msg,
            stderr: None,
        }
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(&payload).await {
            tracing::error!(target: "chatur::chroma::query", "query helper stdin: {e}");
            return Err(ChromaError::Query {
                stage: "stdin".into(),
                message: format!("query helper stdin: {e}"),
                stderr: None,
            });
        }
        if let Err(e) = stdin.shutdown().await {
            tracing::error!(target: "chatur::chroma::query", "query helper stdin close: {e}");
            return Err(ChromaError::Query {
                stage: "stdin".into(),
                message: format!("query helper stdin close: {e}"),
                stderr: None,
            });
        }
    }
    let out = child.wait_with_output().await.map_err(|e| {
        tracing::error!(target: "chatur::chroma::query", "query helper wait: {e}");
        ChromaError::Query {
            stage: "wait".into(),
            message: format!("query helper wait: {e}"),
            stderr: None,
        }
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        tracing::error!(
            target: "chatur::chroma::query",
            "query helper exited {}: {}",
            out.status,
            stderr
        );
        return Err(ChromaError::Query {
            stage: "exec".into(),
            message: format!("query helper exited {}", out.status),
            stderr: Some(stderr),
        });
    }

    let resp: HelperResponse =
        serde_json::from_slice(&out.stdout).map_err(|e| ChromaError::Query {
            stage: "decode".into(),
            message: format!("decode helper response: {e}"),
            stderr: Some(String::from_utf8_lossy(&out.stderr).to_string()),
        })?;
    Ok(resp
        .hits
        .into_iter()
        .map(|h| {
            let path = h
                .metadata
                .get("path")
                .and_then(|v| v.as_str())
                .map(String::from);
            let line_start = h.metadata.get("line_start").and_then(|v| v.as_u64());
            let line_end = h.metadata.get("line_end").and_then(|v| v.as_u64());
            QueryHit {
                id: h.id,
                distance: h.distance,
                document: h.document,
                path,
                line_start,
                line_end,
                metadata: h.metadata,
            }
        })
        .collect())
}
