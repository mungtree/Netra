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

/**
 * Subscribes to the engine's `DomainEvent` stream.
 * @param {(event: object) => void} handler
 * @returns {Promise<() => void>} an unsubscribe function
 */
export const subscribeEvents = (handler) =>
  listen('chatur://event', (msg) => handler(msg.payload));
