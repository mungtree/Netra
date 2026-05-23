// The only seam between the UI and the Tauri backend.
// Each function maps to one `#[tauri::command]` in src-tauri/src/commands.rs.
// JS arguments are camelCase; Tauri delivers them to Rust as snake_case.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** @returns {Promise<Array>} every registered project */
export const listProjects = () => invoke('list_projects');

/** Registers a project, returns its id. */
export const addProject = (name, path) => invoke('add_project', { name, path });

/** Fetches one project by id. */
export const getProject = (projectId) => invoke('get_project', { projectId });

/** Queues a job, returns its id. */
export const queueJob = (projectId, prompt) =>
  invoke('queue_job', { projectId, prompt });

/** @returns {Promise<Array>} every job for a project */
export const listJobs = (projectId) => invoke('list_jobs', { projectId });

/** Fetches one job by id. */
export const getJob = (jobId) => invoke('get_job', { jobId });

/** Cancels a running or queued job. */
export const cancelJob = (jobId) => invoke('cancel_job', { jobId });

/** Hard-deletes a terminal job (completed/failed/cancelled). */
export const deleteJob = (jobId) => invoke('delete_job', { jobId });

/** Hard-deletes a batch and its items. */
export const deleteBatch = (batchId) => invoke('delete_batch', { batchId });

/** Removes every terminal job for a project. Returns the count. */
export const clearCompletedJobs = (projectId) =>
  invoke('clear_completed_jobs', { projectId });

/**
 * Creates a batch — a series of prompts run across one or more projects.
 * @param {string} name
 * @param {string[]} prompts
 * @param {string[]} projectIds
 * @param {string} strategy  reduce strategy: `concat`, `schema_merge`, `reviewer`
 * @returns {Promise<string>} the new batch id
 */
export const createBatch = (name, prompts, projectIds, strategy) =>
  invoke('create_batch', { name, prompts, projectIds, strategy });

/** Convenience: create + run in one step. */
export const runBatchNow = async (name, prompts, projectIds, strategy) => {
  const id = await createBatch(name, prompts, projectIds, strategy);
  await invoke('run_batch', { batchId: id });
  return id;
};

/** Starts a batch running in the background. */
export const runBatch = (batchId) => invoke('run_batch', { batchId });

/** @returns {Promise<Array>} every batch */
export const listBatches = () => invoke('list_batches');

/** Fetches one batch, including its aggregated result once complete. */
export const getBatch = (batchId) => invoke('get_batch', { batchId });

/** @returns {Promise<Array>} the items of a batch */
export const batchItems = (batchId) => invoke('batch_items', { batchId });

/**
 * Subscribes to the engine's `DomainEvent` stream.
 * @param {(event: object) => void} handler
 * @returns {Promise<() => void>} an unsubscribe function
 */
export const subscribeEvents = (handler) =>
  listen('chatur://event', (msg) => handler(msg.payload));

/** Returns the active configuration (concurrency limits, pi binary, agent). */
export const getConfig = () => invoke('get_config');

/**
 * Persists updated settings to chatur.toml. Takes effect on next restart.
 * @param {number} globalMax
 * @param {number} perProjectMax
 * @param {string} piBinary
 * @param {'read'|'read_bash'|'full'} toolsMode
 * @param {string} systemPromptAppend
 */
export const saveConfig = (
  globalMax,
  perProjectMax,
  piBinary,
  toolsMode,
  systemPromptAppend,
) =>
  invoke('save_config', {
    globalMax,
    perProjectMax,
    piBinary,
    toolsMode,
    systemPromptAppend,
  });
