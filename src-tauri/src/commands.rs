//! Tauri commands — one thin wrapper per [`Chatur`] operation.
//!
//! Every command takes the managed [`Chatur`] state and returns
//! `Result<_, String>`: domain errors are stringified so they cross the IPC
//! boundary as plain messages the front-end can display.

use std::path::PathBuf;

use chatur_api::chatur_chroma::{
    self, bootstrap, indexer, mcp, query as chroma_query_mod, server as chroma_server, ChromaConfig,
    ChromaStatus, IndexStats, IndexedFile, QueryHit,
};
use chatur_api::{Chatur, ChaturConfig, ToolsMode};
use chatur_core::ids::{BatchId, JobId, ProjectId};
use chatur_core::model::{Batch, BatchItem, Job, Project};
use tauri::{AppHandle, Emitter, State};

/// Parses a list of id strings into typed ids, stringifying any parse error.
fn parse_project_ids(ids: &[String]) -> Result<Vec<ProjectId>, String> {
    ids.iter()
        .map(|id| id.parse::<ProjectId>().map_err(|e| e.to_string()))
        .collect()
}

/// Registers a project and returns its id.
#[tauri::command]
pub async fn add_project(
    chatur: State<'_, Chatur>,
    name: String,
    path: String,
) -> Result<String, String> {
    chatur
        .add_project(name, path)
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

/// Lists every registered project.
#[tauri::command]
pub async fn list_projects(chatur: State<'_, Chatur>) -> Result<Vec<Project>, String> {
    chatur.list_projects().await.map_err(|e| e.to_string())
}

/// Fetches one project by id.
#[tauri::command]
pub async fn get_project(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<Project, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur.get_project(id).await.map_err(|e| e.to_string())
}

/// Queues a job against a project and returns the job id.
///
/// `use_chromadb` is optional (defaults to `false` for callers on the old
/// signature). When `true` AND the ChromaDB integration is enabled and the
/// server is running, the agent will be told it has access to the chroma MCP.
#[tauri::command]
pub async fn queue_job(
    chatur: State<'_, Chatur>,
    project_id: String,
    prompt: String,
    use_chromadb: Option<bool>,
) -> Result<String, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur
        .queue_job_with_options(id, prompt, use_chromadb.unwrap_or(false))
        .await
        .map(|job_id| job_id.to_string())
        .map_err(|e| e.to_string())
}

/// Lists every job belonging to a project.
#[tauri::command]
pub async fn list_jobs(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<Vec<Job>, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur.list_jobs(id).await.map_err(|e| e.to_string())
}

/// Fetches one job by id.
#[tauri::command]
pub async fn get_job(chatur: State<'_, Chatur>, job_id: String) -> Result<Job, String> {
    let id = job_id.parse::<JobId>().map_err(|e| e.to_string())?;
    chatur.get_job(id).await.map_err(|e| e.to_string())
}

/// Cancels a running or queued job.
#[tauri::command]
pub async fn cancel_job(chatur: State<'_, Chatur>, job_id: String) -> Result<(), String> {
    let id = job_id.parse::<JobId>().map_err(|e| e.to_string())?;
    chatur.cancel_job(id).await.map_err(|e| e.to_string())
}

/// Hard-deletes a terminal (completed / failed / cancelled) job.
#[tauri::command]
pub async fn delete_job(chatur: State<'_, Chatur>, job_id: String) -> Result<(), String> {
    let id = job_id.parse::<JobId>().map_err(|e| e.to_string())?;
    chatur.delete_job(id).await.map_err(|e| e.to_string())
}

/// Hard-deletes a batch and its items.
#[tauri::command]
pub async fn delete_batch(chatur: State<'_, Chatur>, batch_id: String) -> Result<(), String> {
    let id = batch_id.parse::<BatchId>().map_err(|e| e.to_string())?;
    chatur.delete_batch(id).await.map_err(|e| e.to_string())
}

/// Removes every completed/failed/cancelled job for a project.
/// Returns the number of rows deleted.
#[tauri::command]
pub async fn clear_completed_jobs(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<u64, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur
        .clear_completed_jobs(id)
        .await
        .map_err(|e| e.to_string())
}

/// Creates a batch — a series of prompts run across one or more projects — and
/// returns its id. The batch is persisted but not started.
#[tauri::command]
pub async fn create_batch(
    chatur: State<'_, Chatur>,
    name: String,
    prompts: Vec<String>,
    project_ids: Vec<String>,
    strategy: String,
    use_chromadb: Option<bool>,
) -> Result<String, String> {
    let projects = parse_project_ids(&project_ids)?;
    chatur
        .create_batch_with_options(
            name,
            prompts,
            projects,
            strategy,
            use_chromadb.unwrap_or(false),
        )
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

/// Starts a batch running in the background.
#[tauri::command]
pub async fn run_batch(chatur: State<'_, Chatur>, batch_id: String) -> Result<(), String> {
    let id = batch_id.parse::<BatchId>().map_err(|e| e.to_string())?;
    chatur.run_batch(id).await.map_err(|e| e.to_string())
}

/// Lists every batch.
#[tauri::command]
pub async fn list_batches(chatur: State<'_, Chatur>) -> Result<Vec<Batch>, String> {
    chatur.list_batches().await.map_err(|e| e.to_string())
}

/// Fetches one batch, including its aggregated result once complete.
#[tauri::command]
pub async fn get_batch(chatur: State<'_, Chatur>, batch_id: String) -> Result<Batch, String> {
    let id = batch_id.parse::<BatchId>().map_err(|e| e.to_string())?;
    chatur.get_batch(id).await.map_err(|e| e.to_string())
}

/// Lists the items (one per `prompt × target`) of a batch.
#[tauri::command]
pub async fn batch_items(
    chatur: State<'_, Chatur>,
    batch_id: String,
) -> Result<Vec<BatchItem>, String> {
    let id = batch_id.parse::<BatchId>().map_err(|e| e.to_string())?;
    chatur.batch_items(id).await.map_err(|e| e.to_string())
}

/// Current configuration values exposed to the settings UI.
#[derive(serde::Serialize)]
pub struct ConfigDto {
    pub global_max: usize,
    pub per_project_max: usize,
    pub pi_binary: String,
    /// One of `"read"`, `"read_bash"`, `"full"`.
    pub tools_mode: String,
    pub system_prompt_append: String,
    pub timeout_enabled: bool,
    pub timeout_secs: u64,
}

/// Returns the active configuration as a DTO.
#[tauri::command]
pub async fn get_config(chatur: State<'_, Chatur>) -> Result<ConfigDto, String> {
    let cfg = chatur.config();
    Ok(ConfigDto {
        global_max: cfg.concurrency.global_max,
        per_project_max: cfg.concurrency.per_project_max,
        pi_binary: cfg.pi_binary.to_string_lossy().into_owned(),
        tools_mode: tools_mode_to_str(cfg.agent.tools).to_string(),
        system_prompt_append: cfg.agent.system_prompt_append.clone().unwrap_or_default(),
        timeout_enabled: cfg.timeout.enabled,
        timeout_secs: cfg.timeout.secs,
    })
}

/// Persists updated settings to `chatur.toml`.
///
/// The running engine keeps the old values until the app restarts.
#[tauri::command]
pub async fn save_config(
    global_max: usize,
    per_project_max: usize,
    pi_binary: String,
    tools_mode: String,
    system_prompt_append: String,
    timeout_enabled: bool,
    timeout_secs: u64,
) -> Result<(), String> {
    let mut config =
        ChaturConfig::load_or_default("chatur.toml").map_err(|e| e.to_string())?;
    config.concurrency.global_max = global_max.max(1);
    config.concurrency.per_project_max = per_project_max.max(1);
    config.pi_binary = PathBuf::from(pi_binary);
    config.agent.tools = parse_tools_mode(&tools_mode)?;
    let trimmed = system_prompt_append.trim();
    config.agent.system_prompt_append = if trimmed.is_empty() {
        None
    } else {
        Some(system_prompt_append)
    };
    config.timeout.enabled = timeout_enabled;
    config.timeout.secs = timeout_secs.max(1);
    config.save("chatur.toml").map_err(|e| e.to_string())
}

fn tools_mode_to_str(mode: ToolsMode) -> &'static str {
    match mode {
        ToolsMode::Read => "read",
        ToolsMode::ReadBash => "read_bash",
        ToolsMode::Full => "full",
    }
}

fn parse_tools_mode(s: &str) -> Result<ToolsMode, String> {
    match s {
        "read" => Ok(ToolsMode::Read),
        "read_bash" => Ok(ToolsMode::ReadBash),
        "full" => Ok(ToolsMode::Full),
        other => Err(format!("unknown tools_mode: {other}")),
    }
}

// ──────────────────────────── ChromaDB commands ────────────────────────────
//
// Every chroma_* command first checks `chatur.chroma()` is `Some`. When the
// integration is disabled (default), the command returns a structured
// `not_enabled` status / error so the UI can stay graceful.

fn chroma_disabled<T>() -> Result<T, String> {
    Err("ChromaDB integration disabled. Enable [chromadb] in chatur.toml.".to_string())
}

/// Snapshot of bootstrap + server state.
#[derive(serde::Serialize)]
pub struct ChromaStatusDto {
    pub enabled: bool,
    pub installed: bool,
    pub mcp_registered: bool,
    pub server: ChromaStatus,
    pub config: ChromaConfig,
}

#[tauri::command]
pub async fn chroma_status(chatur: State<'_, Chatur>) -> Result<ChromaStatusDto, String> {
    let Some(h) = chatur.chroma() else {
        // Even when disabled, expose the default config so the UI can show
        // the section greyed-out with sensible placeholder values.
        let cfg = ChromaConfig::default();
        return Ok(ChromaStatusDto {
            enabled: false,
            installed: bootstrap::inspect().chromadb_installed,
            mcp_registered: mcp::is_registered(),
            server: ChromaStatus::Stopped,
            config: cfg,
        });
    };
    Ok(ChromaStatusDto {
        enabled: true,
        installed: bootstrap::inspect().chromadb_installed,
        mcp_registered: mcp::is_registered(),
        server: h.status().await,
        config: h.config().await,
    })
}

#[tauri::command]
pub async fn chroma_install(app: AppHandle) -> Result<(), String> {
    let _ = app.emit("chatur://chroma", serde_json::json!({ "kind": "install_started" }));
    bootstrap::ensure_venv()
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(
        "chatur://chroma",
        serde_json::json!({ "kind": "install_finished" }),
    );
    Ok(())
}

#[tauri::command]
pub async fn chroma_start(chatur: State<'_, Chatur>) -> Result<(), String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let cfg = h.config().await;
    mcp::register_pi_mcp(&cfg.host, cfg.port).map_err(|e| e.to_string())?;
    chroma_server::start(h).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_stop(chatur: State<'_, Chatur>) -> Result<(), String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    chroma_server::stop(h).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_restart(chatur: State<'_, Chatur>) -> Result<(), String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let _ = chroma_server::stop(h).await;
    chroma_server::start(h).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_list_collections(
    chatur: State<'_, Chatur>,
) -> Result<Vec<chatur_chroma::server::Collection>, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    h.list_collections().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_collection_files(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<Vec<IndexedFile>, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let name = ChromaConfig::collection_name(&project_id);
    let collections = h.list_collections().await.map_err(|e| e.to_string())?;
    let Some(c) = collections.into_iter().find(|c| c.name == name) else {
        return Ok(Vec::new());
    };
    h.client()
        .collection_files(&c.id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_delete_collection(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<(), String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let name = ChromaConfig::collection_name(&project_id);
    h.delete_collection(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_index_project(
    chatur: State<'_, Chatur>,
    app: AppHandle,
    project_id: String,
) -> Result<IndexStats, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let pid: ProjectId = project_id.parse().map_err(|e| format!("{e}"))?;
    let project = chatur.get_project(pid).await.map_err(|e| e.to_string())?;
    let cfg = h.config().await;
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let app2 = app.clone();
    tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let _ = app2.emit("chatur://chroma", &ev);
        }
    });
    indexer::index_project(&project_id, &project.root_path, &cfg, h.client(), Some(tx))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn chroma_update_settings(
    chatur: State<'_, Chatur>,
    config: ChromaConfig,
) -> Result<(), String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    h.set_config(config.clone()).await;
    // Persist to chatur.toml so it survives restart.
    let mut full = ChaturConfig::load_or_default("chatur.toml").map_err(|e| e.to_string())?;
    full.chromadb = config;
    full.save("chatur.toml").map_err(|e| e.to_string())
}

/// Manual semantic-search query against a project's collection. Returns
/// up to `n_results` ranked hits (lower `distance` = closer match).
#[tauri::command]
pub async fn chroma_query(
    chatur: State<'_, Chatur>,
    project_id: String,
    query: String,
    n_results: Option<u32>,
) -> Result<Vec<QueryHit>, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    if !h.is_running().await {
        return Err("ChromaDB server is not running.".into());
    }
    let cfg = h.config().await;
    let name = ChromaConfig::collection_name(&project_id);
    chroma_query_mod::query_collection(&cfg, &name, &query, n_results.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

/// Result of [`chroma_set_embedding_model`]: tells the UI whether the
/// caller now needs to drop + reindex any existing project collections
/// because the vector dimensions changed.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingModelChange {
    pub requires_reindex: bool,
    pub previous_model: String,
    pub new_model: String,
    /// `chatur_*` collections currently present on the server. Empty when
    /// nothing needs to be rebuilt (server down or no project collections).
    pub affected_collections: Vec<String>,
    pub affected_project_ids: Vec<String>,
}

/// Persist a new embedding model selection. Returns metadata so the UI can
/// prompt the user to drop + reindex affected collections.
#[tauri::command]
pub async fn chroma_set_embedding_model(
    chatur: State<'_, Chatur>,
    model: String,
    custom: Option<String>,
) -> Result<EmbeddingModelChange, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    let mut cfg = h.config().await;
    let previous = cfg.resolved_model();
    cfg.embedding_model = model.clone();
    cfg.embedding_model_custom = custom.clone();
    let new_resolved = cfg.resolved_model();
    let changed = previous != new_resolved;

    h.set_config(cfg.clone()).await;
    let mut full = ChaturConfig::load_or_default("chatur.toml").map_err(|e| e.to_string())?;
    full.chromadb = cfg;
    full.save("chatur.toml").map_err(|e| e.to_string())?;

    let mut affected_collections = Vec::new();
    let mut affected_project_ids = Vec::new();
    if changed && h.is_running().await {
        if let Ok(cols) = h.list_collections().await {
            for c in cols {
                if let Some(rest) = c.name.strip_prefix("chatur_") {
                    affected_collections.push(c.name.clone());
                    affected_project_ids.push(rest.to_string());
                }
            }
        }
    }

    Ok(EmbeddingModelChange {
        requires_reindex: changed && !affected_collections.is_empty(),
        previous_model: previous,
        new_model: new_resolved,
        affected_collections,
        affected_project_ids,
    })
}

/// Drop the listed project collections and re-index each from scratch. Used
/// after switching embedding models (vector dimensions change so old vectors
/// are unusable). Streams the usual `chatur://chroma` progress events.
#[tauri::command]
pub async fn chroma_drop_and_reindex(
    chatur: State<'_, Chatur>,
    app: AppHandle,
    project_ids: Vec<String>,
) -> Result<Vec<IndexStats>, String> {
    let Some(h) = chatur.chroma() else {
        return chroma_disabled();
    };
    if !h.is_running().await {
        return Err("ChromaDB server is not running.".into());
    }
    let cfg = h.config().await;

    let mut out = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        let name = ChromaConfig::collection_name(&project_id);
        // Best-effort drop — ignore "not found" so the caller can pass ids
        // whose collection was already cleaned up.
        let _ = h.delete_collection(&name).await;

        let pid: ProjectId = project_id.parse().map_err(|e| format!("{e}"))?;
        let project = chatur.get_project(pid).await.map_err(|e| e.to_string())?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(64);
        let app2 = app.clone();
        tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let _ = app2.emit("chatur://chroma", &ev);
            }
        });
        let stats = indexer::index_project(
            &project_id,
            &project.root_path,
            &cfg,
            h.client(),
            Some(tx),
        )
        .await
        .map_err(|e| e.to_string())?;
        out.push(stats);
    }
    Ok(out)
}

#[tauri::command]
pub async fn chroma_set_enabled(enabled: bool) -> Result<(), String> {
    // Toggles the master switch. Takes effect after restart (the runtime
    // handle is wired during Chatur::start). Persisted to chatur.toml.
    let mut full = ChaturConfig::load_or_default("chatur.toml").map_err(|e| e.to_string())?;
    full.chromadb.enabled = enabled;
    full.save("chatur.toml").map_err(|e| e.to_string())
}

/// Returns the directory where the app writes rolling daily log files.
/// The UI exposes this as an "Open log folder" link so users can attach
/// logs to bug reports.
#[tauri::command]
pub fn get_log_path() -> Result<String, String> {
    Ok(crate::log_dir().to_string_lossy().to_string())
}
