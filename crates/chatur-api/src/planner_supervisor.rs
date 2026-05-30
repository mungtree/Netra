//! Supervisor for the Python `chatur-planner` sidecar.
//!
//! Spawns `python -m chatur_planner.server` (or `uvicorn` directly) on startup,
//! polls `/healthz` until ready, and kills the child on shutdown. The sidecar
//! inherits its model + llama.cpp URL from env vars set here, sourced from
//! [`PlannerRuntimeConfig`].

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use crate::config::{ModelConfig, PlannerConfig};
use crate::notify;

/// Subset of [`crate::config::ChaturConfig`] that the supervisor needs.
#[derive(Debug, Clone)]
pub struct PlannerRuntimeConfig {
    pub planner: PlannerConfig,
    pub default_model: Option<ModelConfig>,
    /// Working directory for the sidecar process (where `pyproject.toml` lives).
    pub sidecar_dir: PathBuf,
    /// Override the python interpreter; defaults to `python3`.
    pub python: Option<PathBuf>,
}

/// Owns the sidecar child process. Restarts on config change, kills on drop.
pub struct PlannerSupervisor {
    state: Arc<Mutex<Option<Child>>>,
}

impl PlannerSupervisor {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(None)),
        }
    }

    /// Spawns the sidecar if `cfg.planner.autostart` is true, then waits up to
    /// 5 seconds for `/healthz`. Idempotent — calling twice replaces the child.
    pub async fn start(&self, cfg: &PlannerRuntimeConfig) -> Result<(), PlannerError> {
        if !cfg.planner.enabled || !cfg.planner.autostart {
            tracing::info!("planner sidecar not auto-starting (enabled or autostart is off)");
            return Ok(());
        }
        self.kill_child().await;

        let port = endpoint_port(&cfg.planner.endpoint).unwrap_or(8899);
        let sidecar_dir = resolve_sidecar_dir(&cfg.sidecar_dir).ok_or_else(|| {
            PlannerError::Spawn(format!(
                "planner sidecar dir not found (tried `{}` and walking up from cwd)",
                cfg.sidecar_dir.display()
            ))
        })?;
        let python = match cfg
            .python
            .clone()
            .or_else(|| detect_venv_python(&sidecar_dir))
        {
            Some(p) => p,
            None => {
                notify::info(
                    "planner",
                    "Installing planner sidecar (one-time, ~30s)…",
                );
                ensure_venv(&sidecar_dir).await?;
                detect_venv_python(&sidecar_dir).ok_or_else(|| {
                    PlannerError::Spawn(format!(
                        "venv created but python not found under `{}/.venv`",
                        sidecar_dir.display()
                    ))
                })?
            }
        };

        let mut cmd = Command::new(&python);
        cmd.arg("-m").arg("chatur_planner.server");
        cmd.current_dir(&sidecar_dir);
        cmd.env("CHATUR_PLANNER_PORT", port.to_string());
        if let Some(model) = cfg.default_model.as_ref() {
            cmd.env("CHATUR_PLANNER_MODEL", &model.model);
            if let Some(url) = model.base_url.as_ref() {
                cmd.env("CHATUR_PLANNER_LLAMACPP_URL", url);
            }
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| {
            PlannerError::Spawn(format!(
                "spawn `{} -m chatur_planner.server` in `{}`: {e}",
                python.display(),
                sidecar_dir.display()
            ))
        })?;
        tracing::info!(
            port,
            python = %python.display(),
            cwd = %sidecar_dir.display(),
            "planner sidecar spawned"
        );

        // Drain stdout/stderr into tracing so failures surface in app logs.
        if let Some(out) = child.stdout.take() {
            tokio::spawn(pipe_to_tracing(out, "planner stdout"));
        }
        if let Some(err) = child.stderr.take() {
            tokio::spawn(pipe_to_tracing(err, "planner stderr"));
        }
        *self.state.lock().await = Some(child);

        match wait_for_healthz(&cfg.planner.endpoint, Duration::from_secs(15)).await {
            Ok(()) => {
                notify::info("planner", "Planner sidecar ready.");
                Ok(())
            }
            Err(e) => {
                notify::error("planner", format!("Planner sidecar failed: {e}"));
                Err(e)
            }
        }
    }

    /// Kill, then start again with the new config. Use after the user saves
    /// settings that change the model or URL.
    pub async fn apply_config(&self, cfg: &PlannerRuntimeConfig) -> Result<(), PlannerError> {
        self.start(cfg).await
    }

    /// Kill the child (if any). Called on `Chatur::shutdown`.
    pub async fn shutdown(&self) {
        self.kill_child().await;
    }

    async fn kill_child(&self) {
        let mut guard = self.state.lock().await;
        if let Some(mut child) = guard.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}

impl Default for PlannerSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Create `<sidecar>/.venv` and `pip install -e .`. Idempotent enough — pip
/// is a no-op if everything is already installed.
async fn ensure_venv(sidecar_dir: &std::path::Path) -> Result<(), PlannerError> {
    let venv = sidecar_dir.join(".venv");
    if !venv.exists() {
        let out = Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&venv)
            .current_dir(sidecar_dir)
            .output()
            .await
            .map_err(|e| PlannerError::Spawn(format!("python3 -m venv: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Err(PlannerError::Spawn(format!(
                "creating venv failed: {stderr}"
            )));
        }
    }
    let pip = if cfg!(windows) {
        venv.join("Scripts/pip.exe")
    } else {
        venv.join("bin/pip")
    };
    let out = Command::new(&pip)
        .arg("install")
        .arg("-e")
        .arg(".")
        .current_dir(sidecar_dir)
        .output()
        .await
        .map_err(|e| PlannerError::Spawn(format!("pip install: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(PlannerError::Spawn(format!(
            "pip install failed: {stderr}"
        )));
    }
    notify::info("planner", "Planner dependencies installed.");
    Ok(())
}

/// Try `path` as given; if relative and missing, walk upward from cwd looking
/// for `planner/pyproject.toml`. Lets the supervisor work from either repo
/// root or `src-tauri/`.
fn resolve_sidecar_dir(path: &std::path::Path) -> Option<PathBuf> {
    if path.join("pyproject.toml").exists() {
        return Some(path.to_path_buf());
    }
    let cwd = std::env::current_dir().ok()?;
    let name = path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("planner"));
    for ancestor in cwd.ancestors() {
        let cand = ancestor.join(name);
        if cand.join("pyproject.toml").exists() {
            return Some(cand);
        }
    }
    None
}

/// Prefer `<sidecar>/.venv/bin/python` (or `Scripts/python.exe` on Windows)
/// if it exists — avoids forcing the user to symlink onto `python3`.
fn detect_venv_python(sidecar_dir: &std::path::Path) -> Option<PathBuf> {
    let candidates = if cfg!(windows) {
        vec![sidecar_dir.join(".venv/Scripts/python.exe")]
    } else {
        vec![sidecar_dir.join(".venv/bin/python")]
    };
    candidates.into_iter().find(|p| p.exists())
}

async fn pipe_to_tracing<R>(reader: R, label: &'static str)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        tracing::info!(target: "chatur_planner", "{label}: {line}");
    }
}

fn endpoint_port(endpoint: &str) -> Option<u16> {
    let url = reqwest::Url::parse(endpoint).ok()?;
    url.port_or_known_default()
}

async fn wait_for_healthz(endpoint: &str, timeout: Duration) -> Result<(), PlannerError> {
    let url = format!("{}/healthz", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .map_err(|e| PlannerError::Health(format!("client build: {e}")))?;
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Ok(resp) = client.get(&url).send().await
            && resp.status().is_success()
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(PlannerError::Health(format!(
                "planner sidecar did not respond at {url} within {:?}",
                timeout
            )));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("failed to spawn planner sidecar: {0}")]
    Spawn(String),
    #[error("planner sidecar health check failed: {0}")]
    Health(String),
}
