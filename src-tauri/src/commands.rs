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
use chatur_api::{
    BatchTargetSpec, Chatur, ChaturConfig, ModelConfig, PlannerRuntimeConfig, ResumeSummary,
    ToolsMode,
};
use chatur_core::ids::{BatchId, JobId, ModuleId, ProjectId};
use chatur_core::model::{Batch, BatchItem, Job, Module, Project};
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
///
/// `global` (default `false`) skips module fanout: one job per `(prompt,
/// target)` against the whole repo. `target_modules`, when present, runs
/// parallel to `project_ids` — each entry selects module ids for that target
/// (an empty / missing entry means all modules). Ignored when `global`.
#[tauri::command]
pub async fn create_batch(
    chatur: State<'_, Chatur>,
    name: String,
    prompts: Vec<String>,
    project_ids: Vec<String>,
    strategy: String,
    use_chromadb: Option<bool>,
    global: Option<bool>,
    target_modules: Option<Vec<Vec<String>>>,
    diff_branch: Option<String>,
) -> Result<String, String> {
    let projects = parse_project_ids(&project_ids)?;
    let targets = projects
        .into_iter()
        .enumerate()
        .map(|(i, project_id)| {
            let module_ids = match target_modules.as_ref().and_then(|tm| tm.get(i)) {
                Some(ids) if !ids.is_empty() => Some(parse_module_ids(ids)?),
                _ => None,
            };
            Ok(BatchTargetSpec {
                project_id,
                module_ids,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    chatur
        .create_batch_full(
            name,
            prompts,
            targets,
            strategy,
            use_chromadb.unwrap_or(false),
            global.unwrap_or(false),
            diff_branch.filter(|b| !b.is_empty()),
        )
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

/// Lists local git branches for a project, for the PR/diff-mode branch picker.
///
/// Runs `git branch --format=%(refname:short)` in the project's working dir.
/// Returns an empty list (not an error) when the project isn't a git repo.
#[tauri::command]
pub async fn list_git_branches(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<Vec<String>, String> {
    let pid = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    let project = chatur.get_project(pid).await.map_err(|e| e.to_string())?;
    let out = tokio::process::Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(&project.root_path)
        .output()
        .await
        .map_err(|e| format!("failed to run git branch: {e}"))?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Parses a list of module id strings into typed ids.
fn parse_module_ids(ids: &[String]) -> Result<Vec<ModuleId>, String> {
    ids.iter()
        .map(|id| id.parse::<ModuleId>().map_err(|e| e.to_string()))
        .collect()
}

/// Infers a set of modules for a project with a read-only agent. The proposal
/// is returned for the UI to reconcile and is **not** persisted.
#[tauri::command]
pub async fn infer_project_modules(
    chatur: State<'_, Chatur>,
    project_id: String,
) -> Result<Vec<Module>, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur
        .infer_project_modules(id)
        .await
        .map_err(|e| e.to_string())
}

/// Replaces a project's module list (empty normalizes to the default `root`).
#[tauri::command]
pub async fn update_project_modules(
    chatur: State<'_, Chatur>,
    project_id: String,
    modules: Vec<Module>,
) -> Result<(), String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur
        .update_project_modules(id, modules)
        .await
        .map_err(|e| e.to_string())
}

/// The durable-queue rehydration summary captured at startup, for the resume
/// banner.
#[tauri::command]
pub async fn resume_summary(chatur: State<'_, Chatur>) -> Result<ResumeSummary, String> {
    Ok(chatur.resume_summary())
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
    /// Global model selection. Empty strings mean "unset" (the UI shows the
    /// combobox placeholder).
    pub default_provider: String,
    pub default_model: String,
    pub default_base_url: String,
    pub planner_enabled: bool,
    pub planner_endpoint: String,
}

/// Returns the active configuration as a DTO.
#[tauri::command]
pub async fn get_config(chatur: State<'_, Chatur>) -> Result<ConfigDto, String> {
    let cfg = chatur.config();
    let (default_provider, default_model, default_base_url) = match cfg.default_model.as_ref() {
        Some(m) => (
            m.provider.clone(),
            m.model.clone(),
            m.base_url.clone().unwrap_or_default(),
        ),
        None => (String::new(), String::new(), String::new()),
    };
    Ok(ConfigDto {
        global_max: cfg.concurrency.global_max,
        per_project_max: cfg.concurrency.per_project_max,
        pi_binary: cfg.pi_binary.to_string_lossy().into_owned(),
        tools_mode: tools_mode_to_str(cfg.agent.tools).to_string(),
        system_prompt_append: cfg.agent.system_prompt_append.clone().unwrap_or_default(),
        timeout_enabled: cfg.timeout.enabled,
        timeout_secs: cfg.timeout.secs,
        default_provider,
        default_model,
        default_base_url,
        planner_enabled: cfg.planner.enabled,
        planner_endpoint: cfg.planner.endpoint.clone(),
    })
}

/// Persists updated settings to `chatur.toml` and restarts the planner sidecar
/// so the new model takes effect immediately.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn save_config(
    chatur: State<'_, Chatur>,
    global_max: usize,
    per_project_max: usize,
    pi_binary: String,
    tools_mode: String,
    system_prompt_append: String,
    timeout_enabled: bool,
    timeout_secs: u64,
    default_provider: String,
    default_model: String,
    default_base_url: String,
    planner_enabled: bool,
    planner_endpoint: String,
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

    let provider = default_provider.trim();
    let model = default_model.trim();
    config.default_model = if provider.is_empty() || model.is_empty() {
        None
    } else {
        let url = default_base_url.trim();
        Some(ModelConfig {
            provider: provider.to_string(),
            model: model.to_string(),
            base_url: if url.is_empty() {
                None
            } else {
                Some(url.to_string())
            },
        })
    };

    config.planner.enabled = planner_enabled;
    let endpoint = planner_endpoint.trim();
    if !endpoint.is_empty() {
        config.planner.endpoint = endpoint.to_string();
    }

    config.save("chatur.toml").map_err(|e| e.to_string())?;

    // Restart the planner sidecar so the new model/URL apply without a relaunch.
    let runtime_cfg = PlannerRuntimeConfig {
        planner: config.planner.clone(),
        default_model: config.default_model.clone(),
        sidecar_dir: PathBuf::from("planner"),
        python: None,
    };
    if let Err(e) = chatur.planner_supervisor().apply_config(&runtime_cfg).await {
        tracing::warn!("planner restart failed: {e}");
    }
    Ok(())
}

/// One row of `~/.pi/agent/models.json` flattened for the model picker.
#[derive(serde::Serialize)]
pub struct PiModelOption {
    pub provider: String,
    pub model_id: String,
    pub display_name: String,
    pub base_url: String,
}

/// Reads `~/.pi/agent/models.json` and flattens every provider/model into a
/// pickable list. Returns an empty list if the file is missing or unreadable
/// (UI falls back to free-form input).
#[tauri::command]
pub async fn list_pi_models() -> Result<Vec<PiModelOption>, String> {
    let Some(home) = dirs::home_dir() else {
        return Ok(Vec::new());
    };
    let path = home.join(".pi").join("agent").join("models.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return Err(format!("parse {}: {e}", path.display())),
    };
    let mut out = Vec::new();
    let Some(providers) = value.get("providers").and_then(|p| p.as_object()) else {
        return Ok(out);
    };
    for (provider_name, provider_def) in providers {
        let base_url = provider_def
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let Some(models) = provider_def.get("models").and_then(|m| m.as_array()) else {
            continue;
        };
        for m in models {
            let Some(id) = m.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let name = m
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string();
            out.push(PiModelOption {
                provider: provider_name.clone(),
                model_id: id.to_string(),
                display_name: name,
                base_url: base_url.clone(),
            });
        }
    }
    Ok(out)
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
