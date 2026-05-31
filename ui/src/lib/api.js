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
export const queueJob = (projectId, prompt, useChromadb = false) =>
  invoke('queue_job', { projectId, prompt, useChromadb });

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
export const createBatch = (
  name,
  prompts,
  projectIds,
  strategy,
  useChromadb = false,
  global = false,
  targetModules = null,
  diffBranch = null,
) =>
  invoke('create_batch', {
    name,
    prompts,
    projectIds,
    strategy,
    useChromadb,
    global,
    targetModules,
    diffBranch,
  });

/** Lists local git branches for a project (for PR/diff-mode branch picker). */
export const listGitBranches = (projectId) =>
  invoke('list_git_branches', { projectId });

/** Convenience: create + run in one step. */
export const runBatchNow = async (name, prompts, projectIds, strategy, useChromadb = false) => {
  const id = await createBatch(name, prompts, projectIds, strategy, useChromadb);
  await invoke('run_batch', { batchId: id });
  return id;
};

// ─────────────────────────── Modules ───────────────────────────

/**
 * Runs a read-only agent that proposes a module split for a project.
 * The proposal is NOT persisted — the UI diffs it and lets the user apply.
 * @returns {Promise<Array>} proposed `Module`s ({id,name,description,root_subdir})
 */
export const inferProjectModules = (projectId) =>
  invoke('infer_project_modules', { projectId });

/** Replaces a project's module list (empty normalizes to the default `root`). */
export const updateProjectModules = (projectId, modules) =>
  invoke('update_project_modules', { projectId, modules });

/** The durable-queue rehydration summary captured at startup. */
export const resumeSummary = () => invoke('resume_summary');

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

// ─────────────────────────── ChromaDB ───────────────────────────

export const chromaStatus = () => invoke('chroma_status');
export const chromaInstall = () => invoke('chroma_install');
export const chromaStart = () => invoke('chroma_start');
export const chromaStop = () => invoke('chroma_stop');
export const chromaRestart = () => invoke('chroma_restart');
export const chromaListCollections = () => invoke('chroma_list_collections');
export const chromaCollectionFiles = (projectId) =>
  invoke('chroma_collection_files', { projectId });
export const chromaDeleteCollection = (projectId) =>
  invoke('chroma_delete_collection', { projectId });
export const chromaIndexProject = (projectId) =>
  invoke('chroma_index_project', { projectId });
export const chromaUpdateSettings = (config) =>
  invoke('chroma_update_settings', { config });
export const chromaSetEnabled = (enabled) =>
  invoke('chroma_set_enabled', { enabled });
export const chromaSetEmbeddingModel = (model, custom = null) =>
  invoke('chroma_set_embedding_model', { model, custom });
export const chromaDropAndReindex = (projectIds) =>
  invoke('chroma_drop_and_reindex', { projectIds });
export const chromaQuery = (projectId, query, nResults = 10) =>
  invoke('chroma_query', { projectId, query, nResults });

/** Subscribe to chroma-specific events (install + index progress). */
export const subscribeChromaEvents = (handler) =>
  listen('chatur://chroma', (msg) => handler(msg.payload));

/** Subscribe to global UI notifications (toasts). Payload: {level, source, message}. */
export const subscribeNotifications = (handler) =>
  listen('chatur://notification', (msg) => handler(msg.payload));

/** Returns the directory where the app writes log files. */
export const getLogPath = () => invoke('get_log_path');

/** Returns the active configuration (concurrency limits, pi binary, agent). */
export const getConfig = () => invoke('get_config');

/**
 * Persists updated settings to chatur.toml. Restarts the planner sidecar so
 * the new model takes effect immediately; other engine values apply on next
 * app restart.
 */
export const saveConfig = ({
  globalMax,
  perProjectMax,
  piBinary,
  toolsMode,
  systemPromptAppend,
  timeoutEnabled,
  timeoutSecs,
  defaultProvider,
  defaultModel,
  defaultBaseUrl,
  plannerEnabled,
  plannerEndpoint,
}) =>
  invoke('save_config', {
    globalMax,
    perProjectMax,
    piBinary,
    toolsMode,
    systemPromptAppend,
    timeoutEnabled,
    timeoutSecs,
    defaultProvider,
    defaultModel,
    defaultBaseUrl,
    plannerEnabled,
    plannerEndpoint,
  });

/** Reads `~/.pi/agent/models.json` and returns one entry per provider/model. */
export const listPiModels = () => invoke('list_pi_models');
