//! Tauri commands — one thin wrapper per [`Chatur`] operation.
//!
//! Every command takes the managed [`Chatur`] state and returns
//! `Result<_, String>`: domain errors are stringified so they cross the IPC
//! boundary as plain messages the front-end can display.

use std::path::PathBuf;

use chatur_api::{Chatur, ChaturConfig, ToolsMode};
use chatur_core::ids::{BatchId, JobId, ProjectId};
use chatur_core::model::{Batch, BatchItem, Job, Project};
use tauri::State;

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
#[tauri::command]
pub async fn queue_job(
    chatur: State<'_, Chatur>,
    project_id: String,
    prompt: String,
) -> Result<String, String> {
    let id = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    chatur
        .queue_job(id, prompt)
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
) -> Result<String, String> {
    let projects = parse_project_ids(&project_ids)?;
    chatur
        .create_batch(name, prompts, projects, strategy)
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
