//! Tauri commands — one thin wrapper per [`Chatur`] operation.
//!
//! Every command takes the managed [`Chatur`] state and returns
//! `Result<_, String>`: domain errors are stringified so they cross the IPC
//! boundary as plain messages the front-end can display.

use chatur_api::Chatur;
use chatur_core::ids::{JobId, ProjectId};
use chatur_core::model::{Job, Project};
use tauri::State;

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
