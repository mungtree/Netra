//! [`FileLogSink`] — an [`OutputSink`] that writes per-job event logs.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use chrono::Utc;
use tokio::fs::{File, OpenOptions, create_dir_all};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::Mutex;

use chatur_core::ids::JobId;
use chatur_core::model::AgentEvent;
use chatur_core::traits::OutputSink;
use chatur_core::{CoreError, Result};

/// Writes each job's events as JSON Lines to `<root>/<date>/<job-id>.jsonl`.
///
/// One file handle is kept open per active job and flushed after every event
/// (logs must survive a crash); [`flush`](OutputSink::flush) closes it.
pub struct FileLogSink {
    root: PathBuf,
    writers: Mutex<HashMap<JobId, BufWriter<File>>>,
}

impl FileLogSink {
    /// Creates a sink rooted at `root`; date sub-directories are made on write.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            writers: Mutex::new(HashMap::new()),
        }
    }

    /// The log path for `job_id` under today's date directory.
    fn log_path(&self, job_id: JobId) -> PathBuf {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        self.root.join(date).join(format!("{job_id}.jsonl"))
    }
}

/// Maps a filesystem error to a [`CoreError::Storage`].
fn io_err(error: std::io::Error) -> CoreError {
    CoreError::Storage(format!("log I/O error: {error}"))
}

#[async_trait]
impl OutputSink for FileLogSink {
    fn id(&self) -> &str {
        "file-log"
    }

    // `entry().or_insert_with` cannot host the fallible async file-open below,
    // so the contains/insert split is intentional.
    #[allow(clippy::map_entry)]
    async fn on_event(&self, job_id: JobId, event: &AgentEvent) -> Result<()> {
        let mut writers = self.writers.lock().await;

        if !writers.contains_key(&job_id) {
            let path = self.log_path(job_id);
            if let Some(parent) = path.parent() {
                create_dir_all(parent).await.map_err(io_err)?;
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(io_err)?;
            writers.insert(job_id, BufWriter::new(file));
        }

        let writer = writers
            .get_mut(&job_id)
            .expect("writer was just inserted if missing");

        let record = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
        });
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');

        writer.write_all(line.as_bytes()).await.map_err(io_err)?;
        writer.flush().await.map_err(io_err)?;
        Ok(())
    }

    async fn flush(&self, job_id: JobId) -> Result<()> {
        if let Some(mut writer) = self.writers.lock().await.remove(&job_id) {
            writer.flush().await.map_err(io_err)?;
        }
        Ok(())
    }
}
