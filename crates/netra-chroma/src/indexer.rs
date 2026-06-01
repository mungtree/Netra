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
    pub errors: u64,
}

/// Streaming progress event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IndexProgress {
    Started {
        project_id: String,
        root: PathBuf,
        total_candidates: Option<u64>,
    },
    File {
        path: PathBuf,
        chunks: usize,
        files_done: u64,
        files_total: Option<u64>,
    },
    Skipped {
        path: PathBuf,
        reason: String,
    },
    BatchUpserted {
        batch_size: usize,
        chunks_total: u64,
    },
    Warning {
        path: Option<PathBuf>,
        message: String,
    },
    Error {
        stage: String,
        message: String,
        stderr: Option<String>,
    },
    Finished {
        stats: IndexStats,
    },
}

const CHUNK_SIZE: usize = 800;
const CHUNK_OVERLAP: usize = 100;
const UPSERT_BATCH: usize = 64;

/// Failure detail from a single upsert batch — separated from `ChromaError`
/// so the caller can emit a structured progress event before deciding
/// whether to continue.
#[derive(Debug, Clone)]
pub struct UpsertFailure {
    pub stage: &'static str,
    pub message: String,
    pub stderr: Option<String>,
}

async fn upsert_via_helper(
    cfg: &ChromaConfig,
    collection: &str,
    ids: &[String],
    documents: &[String],
    metadatas: &[serde_json::Value],
) -> Result<(), UpsertFailure> {
    let python = venv_python(&venv_dir());
    if !python.exists() {
        return Err(UpsertFailure {
            stage: "venv",
            message: "venv not bootstrapped; install ChromaDB first".into(),
            stderr: None,
        });
    }
    let helper = ensure_helper().map_err(|e| UpsertFailure {
        stage: "helper",
        message: format!("ensure helper: {e}"),
        stderr: None,
    })?;
    let payload = serde_json::to_vec(&serde_json::json!({
        "ids": ids,
        "documents": documents,
        "metadatas": metadatas,
    }))
    .map_err(|e| UpsertFailure {
        stage: "serialize",
        message: format!("serialize payload: {e}"),
        stderr: None,
    })?;

    let mut cmd = Command::new(&python);
    cmd.arg(&helper)
        .arg(&cfg.host)
        .arg(cfg.port.to_string())
        .arg(collection)
        .arg(cfg.resolved_model())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    crate::win::no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| {
        let msg = if e.kind() == std::io::ErrorKind::NotFound {
            "python missing from chroma venv — reinstall ChromaDB".into()
        } else {
            format!("spawn helper: {e}")
        };
        UpsertFailure {
            stage: "spawn",
            message: msg,
            stderr: None,
        }
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(&payload).await {
            return Err(UpsertFailure {
                stage: "stdin",
                message: format!("helper stdin: {e}"),
                stderr: None,
            });
        }
        if let Err(e) = stdin.shutdown().await {
            return Err(UpsertFailure {
                stage: "stdin",
                message: format!("helper stdin close: {e}"),
                stderr: None,
            });
        }
    }
    let out = child.wait_with_output().await.map_err(|e| UpsertFailure {
        stage: "wait",
        message: format!("helper wait: {e}"),
        stderr: None,
    })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        tracing::debug!(target: "netra::chroma::indexer", "helper stderr: {}", stderr);
        return Err(UpsertFailure {
            stage: "upsert",
            message: format!("helper exited {}", out.status),
            stderr: Some(truncate_stderr(&stderr)),
        });
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.trim().is_empty() {
        tracing::debug!(target: "netra::chroma::indexer", "helper stderr (ok): {}", stderr);
    }
    Ok(())
}

fn truncate_stderr(s: &str) -> String {
    const MAX: usize = 2048;
    if s.len() <= MAX {
        s.to_string()
    } else {
        let mut cut = MAX;
        while cut > 0 && !s.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}…[truncated]", &s[..cut])
    }
}

/// Cheap pre-walk to estimate the candidate file count so the progress UI
/// has a denominator. Returns `None` if the walk fails. This intentionally
/// counts every file that survives the ignore rules; the real indexer may
/// still skip some for size or non-utf8 reasons.
fn count_candidates(root: &Path, extra: &[String]) -> Option<u64> {
    let mut n: u64 = 0;
    for entry in walker(root, extra).build().flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            n += 1;
        }
    }
    Some(n)
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

    let total_candidates = count_candidates(root, &cfg.extra_ignore_globs);

    if let Some(tx) = &progress {
        let _ = tx
            .send(IndexProgress::Started {
                project_id: project_id.to_string(),
                root: root.to_path_buf(),
                total_candidates,
            })
            .await;
    }

    let mut stats = IndexStats::default();
    let mut batch_ids: Vec<String> = Vec::with_capacity(UPSERT_BATCH);
    let mut batch_docs: Vec<String> = Vec::with_capacity(UPSERT_BATCH);
    let mut batch_meta: Vec<serde_json::Value> = Vec::with_capacity(UPSERT_BATCH);

    async fn flush_batch(
        cfg: &ChromaConfig,
        name: &str,
        ids: &mut Vec<String>,
        docs: &mut Vec<String>,
        meta: &mut Vec<serde_json::Value>,
        stats: &mut IndexStats,
        progress: &Option<mpsc::Sender<IndexProgress>>,
    ) {
        if ids.is_empty() {
            return;
        }
        let batch_size = ids.len();
        match upsert_via_helper(cfg, name, ids, docs, meta).await {
            Ok(()) => {
                stats.chunks_upserted += batch_size as u64;
                if let Some(tx) = progress {
                    let _ = tx
                        .send(IndexProgress::BatchUpserted {
                            batch_size,
                            chunks_total: stats.chunks_upserted,
                        })
                        .await;
                }
            }
            Err(f) => {
                stats.errors += 1;
                tracing::error!(
                    target: "netra::chroma::indexer",
                    stage = %f.stage,
                    "upsert batch failed: {} ({} chunks dropped)",
                    f.message,
                    batch_size
                );
                if let Some(s) = &f.stderr {
                    tracing::error!(target: "netra::chroma::indexer", "stderr: {}", s);
                }
                if let Some(tx) = progress {
                    let _ = tx
                        .send(IndexProgress::Error {
                            stage: f.stage.to_string(),
                            message: format!("{} ({} chunks dropped)", f.message, batch_size),
                            stderr: f.stderr,
                        })
                        .await;
                }
            }
        }
        ids.clear();
        docs.clear();
        meta.clear();
    }

    for entry in walker(root, &cfg.extra_ignore_globs).build() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                if let Some(tx) = &progress {
                    let _ = tx
                        .send(IndexProgress::Warning {
                            path: None,
                            message: format!("walk error: {e}"),
                        })
                        .await;
                }
                tracing::warn!(target: "netra::chroma::indexer", "walk error: {e}");
                continue;
            }
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
            Err(e) => {
                stats.files_skipped += 1;
                let reason = match e.kind() {
                    std::io::ErrorKind::InvalidData => "non-utf8".to_string(),
                    _ => format!("read error: {e}"),
                };
                if let Some(tx) = &progress {
                    let _ = tx
                        .send(IndexProgress::Skipped {
                            path: path.to_path_buf(),
                            reason,
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
                flush_batch(
                    cfg,
                    &name,
                    &mut batch_ids,
                    &mut batch_docs,
                    &mut batch_meta,
                    &mut stats,
                    &progress,
                )
                .await;
            }
        }
        stats.files_indexed += 1;
        if let Some(tx) = &progress {
            let _ = tx
                .send(IndexProgress::File {
                    path: path.to_path_buf(),
                    chunks: chunks.len(),
                    files_done: stats.files_indexed,
                    files_total: total_candidates,
                })
                .await;
        }
    }
    flush_batch(
        cfg,
        &name,
        &mut batch_ids,
        &mut batch_docs,
        &mut batch_meta,
        &mut stats,
        &progress,
    )
    .await;
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
