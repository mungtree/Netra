//! Crossplatform install of `uv` and a managed Python venv with `chromadb`
//! + `chroma-mcp` pre-installed.
//!
//! Layout under `~/.chatur/`:
//! - `uv/`              — uv install root (only when we install it ourselves)
//! - `chroma-venv/`     — managed Python venv

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::ChromaError;

/// Status of the bootstrap (uv + venv) on disk.
#[derive(Debug, Clone)]
pub struct BootstrapStatus {
    pub uv_path: Option<PathBuf>,
    pub venv_dir: PathBuf,
    pub venv_ready: bool,
    pub chromadb_installed: bool,
}

#[must_use]
pub fn venv_dir() -> PathBuf {
    crate::chatur_runtime_dir().join("chroma-venv")
}

#[must_use]
pub fn venv_python(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

/// Path where the embedded helper script is written on first use.
#[must_use]
pub fn helper_script_path() -> PathBuf {
    crate::chatur_runtime_dir().join("chroma_helper.py")
}

/// Contents of the helper baked into the binary at compile time.
const HELPER_SOURCE: &str = include_str!("python_helper.py");
const QUERY_HELPER_SOURCE: &str = include_str!("chroma_query_helper.py");

#[must_use]
pub fn query_helper_script_path() -> PathBuf {
    crate::chatur_runtime_dir().join("chroma_query_helper.py")
}

fn ensure_script(path: PathBuf, source: &str) -> Result<PathBuf, ChromaError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ChromaError::Io(parent.to_path_buf(), e))?;
    }
    let needs_write = match std::fs::read_to_string(&path) {
        Ok(existing) => existing != source,
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&path, source).map_err(|e| ChromaError::Io(path.clone(), e))?;
    }
    Ok(path)
}

/// Write the index helper to disk if missing or stale. Idempotent.
pub fn ensure_helper() -> Result<PathBuf, ChromaError> {
    ensure_script(helper_script_path(), HELPER_SOURCE)
}

/// Write the query helper to disk if missing or stale. Idempotent.
pub fn ensure_query_helper() -> Result<PathBuf, ChromaError> {
    ensure_script(query_helper_script_path(), QUERY_HELPER_SOURCE)
}

#[must_use]
pub fn venv_uvx(venv: &Path) -> PathBuf {
    // uvx may not be inside the venv; we use it from the global uv install.
    let _ = venv;
    PathBuf::new()
}

/// Inspect what's already on disk. Does NOT install anything.
pub fn inspect() -> BootstrapStatus {
    let venv = venv_dir();
    let python = venv_python(&venv);
    let venv_ready = python.exists();
    // Cheap heuristic: chromadb installed iff venv has a `chromadb` dir under
    // the site-packages tree. We accept a false-negative cost and just check
    // for any chromadb-* metadata directory.
    let chromadb_installed = if venv_ready {
        let site_packages_glob = if cfg!(windows) {
            venv.join("Lib").join("site-packages")
        } else {
            // pick first python3.* dir
            let lib = venv.join("lib");
            std::fs::read_dir(&lib)
                .ok()
                .and_then(|mut it| it.find_map(|e| e.ok().map(|e| e.path())))
                .map(|p| p.join("site-packages"))
                .unwrap_or_else(|| lib.join("site-packages"))
        };
        site_packages_glob.join("chromadb").exists()
    } else {
        false
    };
    BootstrapStatus {
        uv_path: which::which("uv").ok(),
        venv_dir: venv,
        venv_ready,
        chromadb_installed,
    }
}

/// Locate `uv`. If missing, install it via the official script (Linux/macOS)
/// or PowerShell installer (Windows).
pub async fn ensure_uv() -> Result<PathBuf, ChromaError> {
    if let Ok(p) = which::which("uv") {
        return Ok(p);
    }
    install_uv().await?;
    which::which("uv").map_err(|e| ChromaError::UvUnavailable(e.to_string()))
}

async fn install_uv() -> Result<(), ChromaError> {
    let status = if cfg!(windows) {
        Command::new("powershell")
            .args([
                "-ExecutionPolicy",
                "ByPass",
                "-c",
                "irm https://astral.sh/uv/install.ps1 | iex",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    } else {
        // Single-process pipe via `sh -c` since the installer is curl|sh.
        Command::new("sh")
            .args([
                "-c",
                "curl -LsSf https://astral.sh/uv/install.sh | sh",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await
    };
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(ChromaError::UvUnavailable(format!(
            "uv installer exited with status {s}"
        ))),
        Err(e) => Err(ChromaError::UvUnavailable(e.to_string())),
    }
}

/// Create the managed venv (idempotent) and install `chromadb` + `chroma-mcp`.
///
/// Streams stdout/stderr to the calling process — callers that want to
/// capture progress can redirect with `tracing` subscribers or wrap this
/// function in a task that pipes output to an event bus.
pub async fn ensure_venv() -> Result<PathBuf, ChromaError> {
    let uv = ensure_uv().await?;
    let venv = venv_dir();
    if let Some(parent) = venv.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ChromaError::Io(parent.to_path_buf(), e))?;
    }
    if !venv_python(&venv).exists() {
        run(&uv, &["venv", venv.to_string_lossy().as_ref(), "--python", "3.11"]).await?;
    }
    // Always run pip install — uv detects existing installs and is fast.
    let python = venv_python(&venv);
    run(
        &uv,
        &[
            "pip",
            "install",
            "--python",
            python.to_string_lossy().as_ref(),
            "chromadb",
            "chroma-mcp",
            "sentence-transformers",
        ],
    )
    .await?;
    Ok(venv)
}

async fn run(bin: &Path, args: &[&str]) -> Result<(), ChromaError> {
    let status = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .map_err(|e| ChromaError::Bootstrap(format!("spawn {}: {}", bin.display(), e)))?;
    if !status.success() {
        return Err(ChromaError::Bootstrap(format!(
            "{} {:?} exited with {}",
            bin.display(),
            args,
            status
        )));
    }
    Ok(())
}
