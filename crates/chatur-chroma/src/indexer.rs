//! Walk a project directory, chunk text files, and upsert into chroma.
//!
//! v1 lets chroma compute embeddings server-side using its built-in default
//! model. A future revision can compute embeddings locally with `fastembed`
//! and pass them via the `embeddings` field of the upsert call.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::bootstrap::{ensure_helper, venv_dir, venv_python};
use crate::client::ChromaClient;
use crate::error::ChromaError;
use crate::ignore_rules::{too_large, walker};
use crate::ChromaConfig;

/// Per-file row returned by the "indexed files" UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub chunk_count: u64,
    pub size_bytes: u64,
}

/// Summary returned at the end of indexing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_seen: u64,
    pub files_indexed: u64,
    pub files_skipped: u64,
    pub chunks_upserted: u64,
}

/// Streaming progress event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IndexProgress {
    Started { project_id: String, root: PathBuf },
    File { path: PathBuf, chunks: usize },
    Skipped { path: PathBuf, reason: String },
    Finished { stats: IndexStats },
}

const CHUNK_SIZE: usize = 800;
const CHUNK_OVERLAP: usize = 100;
const UPSERT_BATCH: usize = 64;

async fn upsert_via_helper(
    cfg: &ChromaConfig,
    collection: &str,
    ids: &[String],
    documents: &[String],
    metadatas: &[serde_json::Value],
) -> Result<(), ChromaError> {
    let python = venv_python(&venv_dir());
    if !python.exists() {
        return Err(ChromaError::Indexer(
            "venv not bootstrapped; install ChromaDB first".into(),
        ));
    }
    let helper = ensure_helper()?;
    let payload = serde_json::to_vec(&serde_json::json!({
        "ids": ids,
        "documents": documents,
        "metadatas": metadatas,
    }))?;

    let mut child = Command::new(&python)
        .arg(&helper)
        .arg(&cfg.host)
        .arg(cfg.port.to_string())
        .arg(collection)
        .arg(cfg.resolved_model())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ChromaError::Indexer(format!("spawn helper: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&payload)
            .await
            .map_err(|e| ChromaError::Indexer(format!("helper stdin: {e}")))?;
        stdin
            .shutdown()
            .await
            .map_err(|e| ChromaError::Indexer(format!("helper stdin close: {e}")))?;
    }
    let out = child
        .wait_with_output()
        .await
        .map_err(|e| ChromaError::Indexer(format!("helper wait: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(ChromaError::Indexer(format!(
            "helper exited {}: {}",
            out.status, stderr
        )));
    }
    Ok(())
}

/// Index `root` into the project's collection.
pub async fn index_project(
    project_id: &str,
    root: &Path,
    cfg: &ChromaConfig,
    client: &ChromaClient,
    progress: Option<mpsc::Sender<IndexProgress>>,
) -> Result<IndexStats, ChromaError> {
    let name = ChromaConfig::collection_name(project_id);
    // Create the collection via HTTP so we get the id back, but upserts go
    // through the python helper (chroma 1.x requires client-side embeddings).
    let _ = client.ensure_collection(&name).await?;

    if let Some(tx) = &progress {
        let _ = tx
            .send(IndexProgress::Started {
                project_id: project_id.to_string(),
                root: root.to_path_buf(),
            })
            .await;
    }

    let mut stats = IndexStats::default();
    let mut batch_ids: Vec<String> = Vec::with_capacity(UPSERT_BATCH);
    let mut batch_docs: Vec<String> = Vec::with_capacity(UPSERT_BATCH);
    let mut batch_meta: Vec<serde_json::Value> = Vec::with_capacity(UPSERT_BATCH);

    for entry in walker(root, &cfg.extra_ignore_globs).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        stats.files_seen += 1;
        let path = entry.path();
        if too_large(path, cfg.max_file_size_bytes) {
            stats.files_skipped += 1;
            if let Some(tx) = &progress {
                let _ = tx
                    .send(IndexProgress::Skipped {
                        path: path.to_path_buf(),
                        reason: "too large".into(),
                    })
                    .await;
            }
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => {
                stats.files_skipped += 1;
                if let Some(tx) = &progress {
                    let _ = tx
                        .send(IndexProgress::Skipped {
                            path: path.to_path_buf(),
                            reason: "non-utf8".into(),
                        })
                        .await;
                }
                continue;
            }
        };

        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let size = text.len() as u64;
        let sha = sha256_hex(text.as_bytes());
        let chunks = chunk_text(&text);
        if chunks.is_empty() {
            continue;
        }
        for (idx, (line_start, line_end, body)) in chunks.iter().enumerate() {
            batch_ids.push(format!("{}#{}", sha, idx));
            batch_docs.push(body.clone());
            batch_meta.push(serde_json::json!({
                "path": rel,
                "chunk_idx": idx,
                "line_start": line_start,
                "line_end": line_end,
                "sha": sha,
                "size": size,
            }));
            if batch_ids.len() >= UPSERT_BATCH {
                upsert_via_helper(cfg, &name, &batch_ids, &batch_docs, &batch_meta).await?;
                stats.chunks_upserted += batch_ids.len() as u64;
                batch_ids.clear();
                batch_docs.clear();
                batch_meta.clear();
            }
        }
        stats.files_indexed += 1;
        if let Some(tx) = &progress {
            let _ = tx
                .send(IndexProgress::File {
                    path: path.to_path_buf(),
                    chunks: chunks.len(),
                })
                .await;
        }
    }
    if !batch_ids.is_empty() {
        upsert_via_helper(cfg, &name, &batch_ids, &batch_docs, &batch_meta).await?;
        stats.chunks_upserted += batch_ids.len() as u64;
    }
    if let Some(tx) = &progress {
        let _ = tx
            .send(IndexProgress::Finished {
                stats: stats.clone(),
            })
            .await;
    }
    Ok(stats)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let out = h.finalize();
    hex_encode(&out)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Returns `(line_start, line_end, chunk_text)` triples. Char-based windows;
/// line numbers derived by counting `\n` in the window.
fn chunk_text(text: &str) -> Vec<(usize, usize, String)> {
    if text.is_empty() {
        return Vec::new();
    }
    let len = text.len();
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < len {
        // forward to next char boundary if start is mid-char (overlap may land there)
        while start < len && !text.is_char_boundary(start) {
            start += 1;
        }
        if start >= len {
            break;
        }
        let mut e = (start + CHUNK_SIZE).min(len);
        while e < len && !text.is_char_boundary(e) {
            e += 1;
        }
        let slice = &text[start..e];
        let line_start = 1 + text[..start].bytes().filter(|b| *b == b'\n').count();
        let line_end = line_start + slice.bytes().filter(|b| *b == b'\n').count();
        out.push((line_start, line_end, slice.to_string()));
        if e == len {
            break;
        }
        let next = e.saturating_sub(CHUNK_OVERLAP);
        start = if next <= start { e } else { next };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_short_text_in_one() {
        let c = chunk_text("hello world");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].2, "hello world");
    }

    #[test]
    fn chunks_multibyte_at_window_edge() {
        // Multi-byte char (`…` = 3 bytes) straddling the 800-byte boundary and
        // again inside the overlap region. Previously panicked on slice.
        let mut s = String::new();
        for _ in 0..400 { s.push('a'); s.push('…'); } // ~1600 bytes, fully utf-8
        let chunks = chunk_text(&s);
        // every chunk must round-trip into a String without panic; len > 1
        assert!(chunks.len() > 1);
        for (_, _, body) in &chunks { assert!(!body.is_empty()); }
    }

    #[test]
    fn chunks_long_text_with_overlap() {
        let text = "a\n".repeat(1000);
        let c = chunk_text(&text);
        assert!(c.len() > 1);
        // line numbers monotonic
        for w in c.windows(2) {
            assert!(w[1].0 >= w[0].0);
        }
    }
}
