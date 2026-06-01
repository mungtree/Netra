//! [`FileLogSink`] — an [`OutputSink`] that writes per-job event logs.
//!
//! Two non-obvious behaviours:
//!
//! - **`Thinking` events are dropped from disk.** They dominate file size and
//!   the UI sink already streams them to the front-end; the disk log keeps the
//!   useful signal (assistant text, tool calls, results, errors, usage).
//! - **Size-based rotation.** Each per-job file is capped at `max_bytes`; when
//!   exceeded the active file is rolled to `.1`, `.1`→`.2`, …, and any file
//!   beyond `max_files` is dropped. This bounds disk use for runs that go long.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::Utc;
use tokio::fs::{File, OpenOptions, create_dir_all, remove_file, rename};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;

use netra_core::ids::JobId;
use netra_core::model::AgentEvent;
use netra_core::traits::OutputSink;
use netra_core::{CoreError, Result};

/// Default cap per per-job log file before rotation.
const DEFAULT_MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Default number of rolled-over files to keep alongside the active one.
const DEFAULT_MAX_FILES: u32 = 5;

/// One open per-job writer plus its running byte count.
struct JobWriter {
    writer: BufWriter<File>,
    bytes: u64,
}

/// Writes each job's events as JSON Lines to `<root>/<date>/<job-id>.jsonl`
/// with size-based rotation and a `Thinking`-event filter (see [module docs]).
///
/// One file handle is kept open per active job and flushed after every event
/// (logs must survive a crash); [`flush`](OutputSink::flush) closes it.
///
/// [module docs]: self
pub struct FileLogSink {
    root: PathBuf,
    writers: Mutex<HashMap<JobId, JobWriter>>,
    max_bytes: u64,
    max_files: u32,
}

impl FileLogSink {
    /// Creates a sink rooted at `root` with default rotation
    /// (`5 MiB` per file, last `5` rolls kept).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_rotation(root, DEFAULT_MAX_BYTES, DEFAULT_MAX_FILES)
    }

    /// Creates a sink with explicit rotation tuning.
    ///
    /// `max_bytes` is the per-file cap; `max_files` is the number of rolled
    /// files to keep (e.g. `5` keeps `.1`..`.5` plus the active file).
    #[must_use]
    pub fn with_rotation(root: impl Into<PathBuf>, max_bytes: u64, max_files: u32) -> Self {
        Self {
            root: root.into(),
            writers: Mutex::new(HashMap::new()),
            max_bytes: max_bytes.max(1),
            max_files,
        }
    }

    /// The active log path for `job_id` under today's date directory.
    fn log_path(&self, job_id: JobId) -> PathBuf {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        self.root.join(date).join(format!("{job_id}.jsonl"))
    }
}

/// Maps a filesystem error to a [`CoreError::Storage`].
fn io_err(error: std::io::Error) -> CoreError {
    CoreError::Storage(format!("log I/O error: {error}"))
}

/// Rolls `path.jsonl` → `path.jsonl.1`, `.1` → `.2`, …, dropping anything
/// beyond `max_files`.
async fn rotate(path: &Path, max_files: u32) -> Result<()> {
    if max_files == 0 {
        let _ = remove_file(path).await;
        return Ok(());
    }
    // Drop the oldest if it exists.
    let oldest = rolled_path(path, max_files);
    let _ = remove_file(&oldest).await;
    // Shift every roll one slot older.
    for n in (1..max_files).rev() {
        let from = rolled_path(path, n);
        let to = rolled_path(path, n + 1);
        if tokio::fs::metadata(&from).await.is_ok() {
            rename(&from, &to).await.map_err(io_err)?;
        }
    }
    // Move the active file into the `.1` slot.
    if tokio::fs::metadata(path).await.is_ok() {
        rename(path, rolled_path(path, 1)).await.map_err(io_err)?;
    }
    Ok(())
}

/// Returns the path with `.{n}` appended to the active log path.
fn rolled_path(path: &Path, n: u32) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(format!(".{n}"));
    PathBuf::from(s)
}

#[async_trait]
impl OutputSink for FileLogSink {
    fn id(&self) -> &str {
        "file-log"
    }

    async fn on_event(&self, job_id: JobId, event: &AgentEvent) -> Result<()> {
        // `Thinking` is high-volume and recoverable from the live UI stream —
        // it is intentionally excluded from disk logs.
        if matches!(event, AgentEvent::Thinking { .. }) {
            return Ok(());
        }

        let mut writers = self.writers.lock().await;
        let path = self.log_path(job_id);

        if !writers.contains_key(&job_id) {
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await.map_err(io_err)?;
            }
            let bytes = tokio::fs::metadata(&path)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(io_err)?;
            writers.insert(
                job_id,
                JobWriter {
                    writer: BufWriter::new(file),
                    bytes,
                },
            );
        }

        let record = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
        });
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        let line_len = line.len() as u64;

        let needs_rotate = {
            let entry = writers.get(&job_id).expect("writer present");
            entry.bytes + line_len > self.max_bytes && entry.bytes > 0
        };

        if needs_rotate {
            // Close the current writer, rotate the file, then reopen.
            if let Some(mut entry) = writers.remove(&job_id) {
                entry.writer.flush().await.map_err(io_err)?;
            }
            rotate(&path, self.max_files).await?;
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await.map_err(io_err)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(io_err)?;
            writers.insert(
                job_id,
                JobWriter {
                    writer: BufWriter::new(file),
                    bytes: 0,
                },
            );
        }

        let entry = writers
            .get_mut(&job_id)
            .expect("writer was just inserted if missing");
        entry.writer.write_all(line.as_bytes()).await.map_err(io_err)?;
        entry.writer.flush().await.map_err(io_err)?;
        entry.bytes += line_len;
        Ok(())
    }

    async fn flush(&self, job_id: JobId) -> Result<()> {
        if let Some(mut entry) = self.writers.lock().await.remove(&job_id) {
            entry.writer.flush().await.map_err(io_err)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use netra_core::model::AgentEvent;
    use tempfile::TempDir;

    /// Lists every regular file under `root`, one level deep into date subdirs.
    fn list_log_files(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(read) = std::fs::read_dir(root) {
            for entry in read.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(inner) = std::fs::read_dir(&path) {
                        for e in inner.flatten() {
                            if e.path().is_file() {
                                out.push(e.path());
                            }
                        }
                    }
                } else if path.is_file() {
                    out.push(path);
                }
            }
        }
        out
    }

    #[tokio::test]
    async fn drops_thinking_events_from_disk() {
        let dir = TempDir::new().unwrap();
        let sink = FileLogSink::with_rotation(dir.path(), 1024 * 1024, 5);
        let job = JobId::new();
        sink.on_event(job, &AgentEvent::Thinking { text: "x".repeat(100) })
            .await
            .unwrap();
        sink.on_event(
            job,
            &AgentEvent::AssistantText {
                text: "hello".into(),
            },
        )
        .await
        .unwrap();
        sink.flush(job).await.unwrap();

        let files = list_log_files(dir.path());
        let file = files.first().expect("one log file");
        let body = std::fs::read_to_string(file).unwrap();
        assert!(!body.contains("\"thinking\""), "thinking line present: {body}");
        assert!(body.contains("assistant_text"));
    }

    #[tokio::test]
    async fn rotates_when_max_bytes_exceeded() {
        let dir = TempDir::new().unwrap();
        // 200-byte cap to force rotation quickly; keep 3 rolls.
        let sink = FileLogSink::with_rotation(dir.path(), 200, 3);
        let job = JobId::new();
        let big = AgentEvent::AssistantText {
            text: "x".repeat(150),
        };
        for _ in 0..6 {
            sink.on_event(job, &big).await.unwrap();
        }
        sink.flush(job).await.unwrap();

        let names: Vec<String> = list_log_files(dir.path())
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n.ends_with(".jsonl")));
        assert!(names.iter().any(|n| n.ends_with(".jsonl.1")));
        // `max_files = 3` => `.1`, `.2`, `.3` allowed; never `.4`.
        assert!(!names.iter().any(|n| n.ends_with(".jsonl.4")));
    }
}
