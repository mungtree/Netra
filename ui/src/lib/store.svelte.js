// Application state — a single reactive store shared by every component.
//
// Loaded on mount and refreshed on every `chatur://event` from the engine.

import {
  listProjects,
  listJobs,
  listBatches,
  addProject as apiAddProject,
  queueJob as apiQueueJob,
  cancelJob as apiCancelJob,
  createBatch as apiCreateBatch,
  runBatch as apiRunBatch,
  subscribeEvents,
} from './api.js';

export const store = $state({
  /** @type {Array} registered projects */
  projects: [],
  /** @type {Array} every job across all projects */
  jobs: [],
  /** @type {Array} every batch, newest first */
  batches: [],
  /** @type {Array} recent domain events, newest first */
  events: [],
  /** @type {string|null} currently selected project id */
  selectedId: null,
  /** @type {string} last error message, '' when clear */
  error: '',
  /** @type {boolean} true once the first load has completed */
  ready: false,
});

/** Reloads projects and all their jobs from the backend. */
export async function refresh() {
  try {
    const projects = await listProjects();

    if (store.selectedId && !projects.some((p) => p.id === store.selectedId)) {
      store.selectedId = null;
    }
    if (!store.selectedId && projects.length > 0) {
      store.selectedId = projects[0].id;
    }

    const jobs = [];
    for (const project of projects) {
      jobs.push(...(await listJobs(project.id)));
    }

    const batches = await listBatches();
    batches.sort((a, b) => b.created_at.localeCompare(a.created_at));

    store.projects = projects;
    store.jobs = jobs;
    store.batches = batches;
    store.ready = true;
  } catch (e) {
    store.error = String(e);
  }
}

export async function addProject(name, path) {
  try {
    await apiAddProject(name, path);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

export async function queueJob(projectId, prompt) {
  try {
    await apiQueueJob(projectId, prompt);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

export async function cancelJob(jobId) {
  try {
    await apiCancelJob(jobId);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

/**
 * Runs a task preset as a batch over the selected project: a series of prompts
 * fanned into jobs, their outputs aggregated by the preset's strategy.
 * @param {{title: string, prompts: string[], strategy: string}} preset
 */
export async function runTaskBatch(preset) {
  if (!store.selectedId) return;
  try {
    const id = await apiCreateBatch(
      preset.title,
      preset.prompts,
      [store.selectedId],
      preset.strategy,
    );
    await apiRunBatch(id);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

export function select(id) {
  store.selectedId = id;
}

export function clearError() {
  store.error = '';
}

let listening = false;

/** Subscribes to the engine event stream; refreshes state on each event. */
export async function startEvents() {
  if (listening) return;
  listening = true;
  await subscribeEvents((event) => {
    store.events = [
      { at: new Date().toLocaleTimeString(), ...event },
      ...store.events,
    ].slice(0, 120);
    refresh();
  });
}
