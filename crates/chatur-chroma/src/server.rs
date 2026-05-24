//! Chroma server process lifecycle.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

use crate::bootstrap::{venv_dir, venv_python};
use crate::error::ChromaError;
use crate::handle::{ChromaHandle, ChromaStatus};

fn venv_bin(venv: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join(format!("{name}.exe"))
    } else {
        venv.join("bin").join(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
}

/// Returned by status queries for richer per-collection info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub file_count: u64,
    pub chunk_count: u64,
    pub last_indexed_at: Option<String>,
}

#[allow(dead_code)]
pub(crate) struct ServerProcess {
    pub child: Child,
    pub pid: u32,
    pub port: u16,
    pub started_at: Instant,
    pub data_dir: PathBuf,
}

/// Start the chroma server using the managed venv. Blocks until either:
/// - the heartbeat endpoint responds 200 (success), or
/// - the timeout expires (error, child is killed).
pub async fn start(handle: &ChromaHandle) -> Result<(), ChromaError> {
    if handle.has_server().await {
        return Err(ChromaError::AlreadyRunning);
    }
    let cfg = handle.config().await;
    handle.set_status(ChromaStatus::Starting).await;

    let venv = venv_dir();
    let python = venv_python(&venv);
    if !python.exists() {
        handle
            .set_status(ChromaStatus::Error {
                message: "venv not bootstrapped; run install first".into(),
            })
            .await;
        return Err(ChromaError::ServerStart(
            "venv not bootstrapped; call bootstrap::ensure_venv() first".into(),
        ));
    }
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| ChromaError::Io(cfg.data_dir.clone(), e))?;

    let port_str = cfg.port.to_string();
    // chroma 0.5+ ships a `chroma` console script in the venv. Prefer it; fall
    // back to `python -m chromadb.cli.cli` if the script isn't there.
    let chroma_bin = venv_bin(&venv, "chroma");
    let mut cmd = if chroma_bin.exists() {
        let mut c = Command::new(&chroma_bin);
        c.args([
            "run",
            "--path",
            cfg.data_dir.to_string_lossy().as_ref(),
            "--host",
            &cfg.host,
            "--port",
            &port_str,
        ]);
        c
    } else {
        let mut c = Command::new(&python);
        c.args([
            "-m",
            "chromadb.cli.cli",
            "run",
            "--path",
            cfg.data_dir.to_string_lossy().as_ref(),
            "--host",
            &cfg.host,
            "--port",
            &port_str,
        ]);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd
        .spawn()
        .map_err(|e| ChromaError::ServerStart(e.to_string()))?;
    let pid = child.id().unwrap_or(0);

    // Drain stdout/stderr to tracing so a failed startup is debuggable.
    if let Some(out) = child.stdout.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!(target: "chroma_server", "{line}");
            }
        });
    }
    let stderr_buf: std::sync::Arc<tokio::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    if let Some(err) = child.stderr.take() {
        let buf = stderr_buf.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::warn!(target: "chroma_server", "{line}");
                let mut g = buf.lock().await;
                g.push(line);
                if g.len() > 200 {
                    let drain = g.len() - 200;
                    g.drain(0..drain);
                }
            }
        });
    }

    // Heartbeat poll up to 30s.
    let deadline = Instant::now() + Duration::from_secs(30);
    let client = handle.client();
    let mut ok = false;
    while Instant::now() < deadline {
        if let Ok(true) = client.heartbeat().await {
            ok = true;
            break;
        }
        if let Ok(Some(status)) = child.try_wait() {
            // Give the stderr-drain task a moment to flush.
            tokio::time::sleep(Duration::from_millis(100)).await;
            let tail = stderr_buf.lock().await.join("\n");
            let msg = if tail.is_empty() {
                format!("chroma exited with {status} during startup (no stderr)")
            } else {
                format!("chroma exited with {status} during startup:\n{tail}")
            };
            handle
                .set_status(ChromaStatus::Error {
                    message: msg.clone(),
                })
                .await;
            return Err(ChromaError::ServerStart(msg));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    if !ok {
        let _ = child.start_kill();
        let _ = child.wait().await;
        handle
            .set_status(ChromaStatus::Error {
                message: "heartbeat timeout".into(),
            })
            .await;
        return Err(ChromaError::HealthTimeout(Duration::from_secs(30)));
    }

    handle
        .store_server(ServerProcess {
            child,
            pid,
            port: cfg.port,
            started_at: Instant::now(),
            data_dir: cfg.data_dir.clone(),
        })
        .await;
    handle
        .set_status(ChromaStatus::Running { pid, port: cfg.port })
        .await;
    Ok(())
}

/// Gracefully stop the chroma server. SIGTERM, wait 5s, then SIGKILL.
pub async fn stop(handle: &ChromaHandle) -> Result<(), ChromaError> {
    let Some(mut srv) = handle.take_server().await else {
        return Err(ChromaError::NotRunning);
    };
    let _ = srv.child.start_kill();
    // wait briefly for clean shutdown
    let wait = tokio::time::timeout(Duration::from_secs(5), srv.child.wait()).await;
    if wait.is_err() {
        let _ = srv.child.kill().await;
    }
    handle.set_status(ChromaStatus::Stopped).await;
    Ok(())
}
