// Application state — a single reactive store shared by every component.
//
// Project / job / batch lists are loaded on mount and refreshed (debounced) on
// lifecycle events. Agent turn output is streamed live, per job, into
// `store.agents` — never via a backend round-trip.

import {
  listProjects,
  listJobs,
  listBatches,
  batchItems as apiBatchItems,
  getBatch as apiGetBatch,
  runBatch as apiRunBatchById,
  getJob,
  addProject as apiAddProject,
  queueJob as apiQueueJob,
  cancelJob as apiCancelJob,
  deleteJob as apiDeleteJob,
  deleteBatch as apiDeleteBatch,
  clearCompletedJobs as apiClearCompletedJobs,
  createBatch as apiCreateBatch,
  runBatch as apiRunBatch,
  subscribeEvents,
  getConfig,
  saveConfig as apiSaveConfig,
  chromaStatus as apiChromaStatus,
  chromaListCollections as apiChromaListCollections,
  subscribeChromaEvents,
} from './api.js';

import { DEFAULT_STOP_RULES, composePrompts } from './tasks.js';
import { normalizePreset } from './prompts/promptsData.js';
import { toEpochMs } from './time.js';

const CUSTOM_PRESETS_KEY = 'chatur.customPresets.v1';
const STOP_RULES_KEY = 'chatur.stopRules.v1';

function loadStopRules() {
  if (typeof localStorage === 'undefined') return DEFAULT_STOP_RULES;
  try {
    const raw = localStorage.getItem(STOP_RULES_KEY);
    return raw == null ? DEFAULT_STOP_RULES : raw;
  } catch {
    return DEFAULT_STOP_RULES;
  }
}

function persistStopRules(value) {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(STOP_RULES_KEY, value);
  } catch {
    /* quota or disabled storage — silently skip */
  }
}

/** Reads any previously imported custom presets from localStorage. */
function loadCustomPresets() {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(CUSTOM_PRESETS_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed)
      ? parsed.map((p) => normalizePreset({ ...p, builtin: false }))
      : [];
  } catch {
    return [];
  }
}

function shortId() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID().slice(0, 8);
  }
  return Math.random().toString(36).slice(2, 10);
}

/** Persists custom presets so they survive a reload. */
function persistCustomPresets(list) {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(CUSTOM_PRESETS_KEY, JSON.stringify(list));
  } catch {
    /* quota or disabled storage — silently skip */
  }
}

// Keep memory bounded on long sessions.
const MAX_LINES_PER_AGENT = 500;
const MAX_AGENTS = 12;

export const store = $state({
  /** @type {Array} registered projects */
  projects: [],
  /** @type {Array} every job across all projects */
  jobs: [],
  /** @type {Array} every batch, newest first */
  batches: [],
  /** @type {Record<string, object>} live agent output, keyed by job id */
  agents: {},
  /** @type {string|null} currently selected project id */
  selectedId: null,
  /** @type {string} last error message, '' when clear */
  error: '',
  /** @type {boolean} true once the first load has completed */
  ready: false,
  /** @type {'projects'|'history'|'settings'|'prompts'} active view in the ActivityBar */
  activeView: 'projects',
  /** @type {string|null} currently selected batch id (history view) */
  selectedBatchId: null,
  /**
   * @type {Record<string, { items: Array, jobs: Record<string, object>, loadedAt: number }>}
   * per-batch detail cache: items + a jobs map keyed by job id.
   */
  batchDetails: {},
  /** @type {{globalMax: number, perProjectMax: number, piBinary: string, toolsMode: 'read'|'read_bash'|'full', systemPromptAppend: string, stopRules: string, timeoutEnabled: boolean, timeoutSecs: number}} */
  settings: {
    globalMax: 4,
    perProjectMax: 2,
    piBinary: 'pi',
    toolsMode: 'read',
    systemPromptAppend: '',
    /** Appended to every scoped preset prompt to cap scope for small models. */
    stopRules: loadStopRules(),
    timeoutEnabled: true,
    timeoutSecs: 300,
  },
  /** @type {boolean} true for a moment after a successful settings save */
  settingsSaved: false,
  /** @type {Array} imported/custom task presets (persisted to localStorage) */
  customPresets: loadCustomPresets(),
  /**
   * ChromaDB integration state. `null` until the first status fetch.
   * Shape mirrors the backend's `ChromaStatusDto`:
   * `{ enabled, installed, mcp_registered, server: { state, ... }, config }`.
   */
  chroma: null,
  /** @type {Array<{id:string,name:string}>} collections in the chroma server */
  chromaCollections: [],
  /** @type {Array} streaming index-progress events for the latest run */
  chromaIndexEvents: [],
  /**
   * Aggregated state of the *current* indexing run, reset on every
   * `started` event. Lets the UI render a progress bar and error list
   * without re-walking the event log on every change.
   */
  chromaIndexState: {
    running: false,
    projectId: null,
    root: null,
    filesDone: 0,
    filesTotal: null,
    chunks: 0,
    skipped: 0,
    lastFile: null,
    startedAt: null,
    finishedAt: null,
    /** @type {Array<{stage:string,message:string,stderr?:string,at:number}>} */
    errors: [],
    /** @type {Array<{path:string|null,message:string,at:number}>} */
    warnings: [],
  },
  /**
   * Transient toasts for chroma degradation (job ran without ChromaDB
   * even though the user asked for it). Each entry auto-dismisses after
   * a few seconds via the UI layer.
   */
  chromaDegradedToasts: [],
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

export async function queueJob(projectId, prompt, useChromadb = false) {
  try {
    await apiQueueJob(projectId, prompt, useChromadb);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

export async function refreshChromaStatus() {
  try {
    store.chroma = await apiChromaStatus();
    if (store.chroma?.server?.state === 'running') {
      try {
        store.chromaCollections = await apiChromaListCollections();
      } catch {
        store.chromaCollections = [];
      }
    } else {
      store.chromaCollections = [];
    }
  } catch (e) {
    store.error = String(e);
  }
}

function applyChromaEvent(ev) {
  const s = store.chromaIndexState;
  const now = Date.now();
  switch (ev?.kind) {
    case 'started':
      store.chromaIndexState = {
        running: true,
        projectId: ev.project_id ?? null,
        root: ev.root ?? null,
        filesDone: 0,
        filesTotal: ev.total_candidates ?? null,
        chunks: 0,
        skipped: 0,
        lastFile: null,
        startedAt: now,
        finishedAt: null,
        errors: [],
        warnings: [],
      };
      break;
    case 'file':
      s.filesDone = ev.files_done ?? s.filesDone + 1;
      if (ev.files_total != null) s.filesTotal = ev.files_total;
      s.lastFile = ev.path ?? null;
      break;
    case 'skipped':
      s.skipped += 1;
      s.warnings.push({
        path: ev.path ?? null,
        message: `skipped: ${ev.reason ?? 'unknown'}`,
        at: now,
      });
      break;
    case 'batch_upserted':
      s.chunks = ev.chunks_total ?? s.chunks + (ev.batch_size ?? 0);
      break;
    case 'warning':
      s.warnings.push({
        path: ev.path ?? null,
        message: ev.message ?? '',
        at: now,
      });
      break;
    case 'error':
      s.errors.push({
        stage: ev.stage ?? 'unknown',
        message: ev.message ?? '',
        stderr: ev.stderr ?? null,
        at: now,
      });
      break;
    case 'finished':
      s.running = false;
      s.finishedAt = now;
      if (ev.stats) {
        s.chunks = ev.stats.chunks_upserted ?? s.chunks;
        s.filesDone = ev.stats.files_indexed ?? s.filesDone;
        s.skipped = ev.stats.files_skipped ?? s.skipped;
      }
      break;
    case 'install_started':
    case 'install_finished':
    default:
      break;
  }
}

let _chromaEventsStarted = false;
export async function startChromaEvents() {
  if (_chromaEventsStarted) return;
  _chromaEventsStarted = true;
  await subscribeChromaEvents((ev) => {
    // Keep latest 500 events; refresh status on terminal kinds.
    const list = store.chromaIndexEvents;
    list.push(ev);
    if (list.length > 500) list.splice(0, list.length - 500);
    applyChromaEvent(ev);
    if (ev?.kind === 'install_finished' || ev?.kind === 'finished') {
      refreshChromaStatus();
    }
  });
}

/** Dismiss a chroma-degraded toast by its id. */
export function dismissChromaToast(id) {
  store.chromaDegradedToasts = store.chromaDegradedToasts.filter(
    (t) => t.id !== id,
  );
}

function pushDegradedToast(jobId, reason) {
  const id = `${jobId}-${Date.now()}`;
  store.chromaDegradedToasts = [
    ...store.chromaDegradedToasts,
    { id, jobId, reason, at: Date.now() },
  ];
  setTimeout(() => dismissChromaToast(id), 8000);
}

export async function cancelJob(jobId) {
  try {
    await apiCancelJob(jobId);
    await refresh();
  } catch (e) {
    store.error = String(e);
  }
}

export async function deleteJob(jobId) {
  try {
    await apiDeleteJob(jobId);
    store.jobs = store.jobs.filter((j) => j.id !== jobId);
    delete store.agents[jobId];
  } catch (e) {
    store.error = String(e);
  }
}

export async function deleteBatch(batchId) {
  try {
    await apiDeleteBatch(batchId);
    store.batches = store.batches.filter((b) => b.id !== batchId);
    store.jobs = store.jobs.filter((j) => j.batch_id !== batchId);
  } catch (e) {
    store.error = String(e);
  }
}

export async function clearCompletedJobs() {
  if (!store.selectedId) return;
  try {
    await apiClearCompletedJobs(store.selectedId);
    store.jobs = store.jobs.filter(
      (j) =>
        j.project_id !== store.selectedId ||
        !['completed', 'failed', 'cancelled'].includes(j.status),
    );
  } catch (e) {
    store.error = String(e);
  }
}

/** Adds an imported preset, dedupes by id, and persists. */
export function addCustomPreset(preset) {
  const normalized = normalizePreset({ ...preset, builtin: false });
  store.customPresets = [
    ...store.customPresets.filter((p) => p.id !== normalized.id),
    normalized,
  ];
  persistCustomPresets(store.customPresets);
  return normalized;
}

/** Removes a custom preset by id and persists. */
export function removeCustomPreset(id) {
  store.customPresets = store.customPresets.filter((p) => p.id !== id);
  persistCustomPresets(store.customPresets);
}

/** Patches a custom preset in place; ignored for built-ins. Returns the new preset. */
export function updateCustomPreset(id, patch) {
  let updated = null;
  store.customPresets = store.customPresets.map((p) => {
    if (p.id !== id) return p;
    updated = normalizePreset({ ...p, ...patch, builtin: false });
    return updated;
  });
  persistCustomPresets(store.customPresets);
  return updated;
}

/** Creates a blank custom preset, persists, and returns it. */
export function createBlankPreset() {
  const fresh = normalizePreset({
    id: `custom-${shortId()}`,
    icon: 'bookmark',
    title: 'Untitled batch',
    strategy: 'concat',
    stopConditionId: 'default',
    customStopText: '',
    appendToAll: false,
    output_schema: null,
    prompts: ['Write your first prompt here…'],
    builtin: false,
  });
  store.customPresets = [...store.customPresets, fresh];
  persistCustomPresets(store.customPresets);
  return fresh;
}

/** Duplicates any preset (built-in or custom) into a new editable custom one. */
export function duplicatePreset(preset) {
  const dup = normalizePreset({
    ...preset,
    id: `custom-${shortId()}`,
    title: `${preset.title ?? 'Untitled batch'} (copy)`,
    builtin: false,
  });
  store.customPresets = [...store.customPresets, dup];
  persistCustomPresets(store.customPresets);
  return dup;
}

/** Deletes every batch (run history) and clears related selection. */
export async function clearAllBatches() {
  const ids = store.batches.map((b) => b.id);
  for (const id of ids) {
    try {
      await apiDeleteBatch(id);
    } catch (e) {
      store.error = String(e);
    }
  }
  store.batches = [];
  store.batchDetails = {};
  store.selectedBatchId = null;
  // Stale per-batch jobs get cleared on the next refresh; force one now.
  await refresh();
}

/** Saves the current stop-rules text to localStorage. */
export function saveStopRules() {
  persistStopRules(store.settings.stopRules);
}

/** Restores the built-in default stop rules. */
export function resetStopRules() {
  store.settings.stopRules = DEFAULT_STOP_RULES;
  persistStopRules(DEFAULT_STOP_RULES);
}

/**
 * Runs a task preset as a batch over the selected project: a series of prompts
 * fanned into jobs, their outputs aggregated by the preset's strategy.
 * @param {{title: string, prompts: string[], strategy: string}} preset
 */
export async function runTaskBatch(preset, useChromadb = false) {
  if (!store.selectedId) return;
  try {
    const prompts = composePrompts(preset, store.settings.stopRules);
    const id = await apiCreateBatch(
      preset.title,
      prompts,
      [store.selectedId],
      preset.strategy,
      useChromadb,
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

/** Opens the history view focused on a batch. */
export function openReview(batchId) {
  store.selectedBatchId = batchId;
  store.activeView = 'history';
  loadBatchDetail(batchId);
}

/** Selects a batch within the already-open history view. */
export function selectBatch(batchId) {
  store.selectedBatchId = batchId;
  loadBatchDetail(batchId);
}

/**
 * Loads (or refreshes) per-prompt detail for a batch:
 * items + the job behind each item.
 */
export async function loadBatchDetail(batchId) {
  if (!batchId) return;
  try {
    const [batch, items] = await Promise.all([
      apiGetBatch(batchId),
      apiBatchItems(batchId),
    ]);

    const jobs = {};
    await Promise.all(
      items.map(async (item) => {
        if (!item.job_id) return;
        try {
          jobs[item.job_id] = await getJob(item.job_id);
        } catch {
          /* missing/in-flight job — skip */
        }
      }),
    );

    // Replace the cached batch entry too so the header reflects fresh result.
    const idx = store.batches.findIndex((b) => b.id === batchId);
    if (idx >= 0) store.batches[idx] = batch;

    store.batchDetails[batchId] = { items, jobs, loadedAt: Date.now() };
  } catch (e) {
    store.error = String(e);
  }
}

/** Re-runs a batch by id, then refreshes its detail. */
export async function rerunBatch(batchId) {
  try {
    await apiRunBatchById(batchId);
    await loadBatchDetail(batchId);
  } catch (e) {
    store.error = String(e);
  }
}

export function clearError() {
  store.error = '';
}

export async function loadSettings() {
  try {
    const cfg = await getConfig();
    store.settings = {
      globalMax: cfg.global_max,
      perProjectMax: cfg.per_project_max,
      piBinary: cfg.pi_binary,
      toolsMode: cfg.tools_mode ?? 'read',
      systemPromptAppend: cfg.system_prompt_append ?? '',
      stopRules: store.settings.stopRules ?? loadStopRules(),
      timeoutEnabled: cfg.timeout_enabled ?? true,
      timeoutSecs: cfg.timeout_secs ?? 300,
    };
  } catch (e) {
    store.error = String(e);
  }
}

export async function saveSettings() {
  try {
    await apiSaveConfig(
      store.settings.globalMax,
      store.settings.perProjectMax,
      store.settings.piBinary,
      store.settings.toolsMode,
      store.settings.systemPromptAppend,
      store.settings.timeoutEnabled,
      store.settings.timeoutSecs,
    );
    persistStopRules(store.settings.stopRules);
    store.settingsSaved = true;
    setTimeout(() => {
      store.settingsSaved = false;
    }, 3000);
  } catch (e) {
    store.error = String(e);
  }
}

// --- live event ingestion ---------------------------------------------------

// A single debounced `refresh()`: lifecycle bursts collapse to one round-trip.
let refreshTimer = null;
function scheduleRefresh() {
  if (refreshTimer) return;
  refreshTimer = setTimeout(() => {
    refreshTimer = null;
    refresh();
  }, 200);
}

/** Resolves a project's display name from the loaded project list. */
function projectName(projectId) {
  return store.projects.find((p) => p.id === projectId)?.name ?? '';
}

/**
 * Returns the agent entry for `jobId`, creating it (and lazily fetching its
 * project / prompt metadata) on first sight — no full refresh.
 */
function ensureAgent(jobId) {
  let agent = store.agents[jobId];
  if (agent) return agent;

  // Evict the oldest finished agent once we are over the cap.
  const ids = Object.keys(store.agents);
  if (ids.length >= MAX_AGENTS) {
    const stale = ids
      .map((id) => store.agents[id])
      .filter((a) => a.status !== 'running' && a.status !== 'queued')
      .sort((a, b) => a.updatedAt - b.updatedAt)[0];
    if (stale) delete store.agents[stale.jobId];
  }

  agent = {
    jobId,
    status: 'running',
    projectName: '',
    prompt: '',
    lines: [],
    updatedAt: Date.now(),
    // Local wall-clock fallback for elapsed display; replaced once the
    // authoritative backend timestamps load.
    startedAt: Date.now(),
    endedAt: null,
  };
  store.agents[jobId] = agent;

  // Fill metadata without blocking the event stream.
  getJob(jobId)
    .then((job) => {
      const a = store.agents[jobId];
      if (!a) return;
      a.prompt = job.prompt ?? '';
      a.projectName = projectName(job.project_id);
      const started = toEpochMs(job.started_at);
      if (started != null) a.startedAt = started;
      const finished = toEpochMs(job.finished_at);
      if (finished != null) a.endedAt = finished;
    })
    .catch(() => {});

  return agent;
}

/** Appends one line, coalescing consecutive same-type streaming deltas. */
function pushLine(agent, type, text, extra = {}) {
  const last = agent.lines[agent.lines.length - 1];
  if (last && last.type === type && (type === 'thinking' || type === 'text')) {
    last.text += text;
  } else {
    agent.lines.push({ type, text, ...extra });
    if (agent.lines.length > MAX_LINES_PER_AGENT) agent.lines.shift();
  }
  agent.updatedAt = Date.now();
}

/** Folds one `AgentEvent` (from a `job_progress` event) into its agent. */
function ingestAgentEvent(jobId, ev) {
  const agent = ensureAgent(jobId);
  switch (ev.kind) {
    case 'thinking':
      pushLine(agent, 'thinking', ev.text);
      break;
    case 'assistant_text':
      pushLine(agent, 'text', ev.text);
      break;
    case 'tool_call':
      pushLine(agent, 'tool', summarizeArgs(ev.args), { name: ev.name });
      break;
    case 'tool_result':
      pushLine(agent, 'tool_result', '', {
        name: ev.name,
        isError: ev.is_error,
      });
      break;
    case 'turn_start':
      pushLine(agent, 'turn', 'turn started');
      break;
    case 'turn_end':
      pushLine(agent, 'turn', 'turn ended');
      break;
    case 'error':
      pushLine(agent, 'error', ev.message ?? 'agent error');
      break;
    case 'prompt':
      pushLine(agent, 'prompt', ev.text ?? 'Prompt');
      break;
    // `usage` carries no display text.
    default:
      break;
  }
}

/** Compacts arbitrary tool arguments into a short one-line summary. */
function summarizeArgs(args) {
  if (args == null) return '';
  const text = typeof args === 'string' ? args : JSON.stringify(args);
  return text.length > 200 ? `${text.slice(0, 200)}…` : text;
}

/** Marks an agent terminal and replaces its text with the authoritative output. */
function finishAgent(jobId, status) {
  const agent = ensureAgent(jobId);
  agent.status = status;
  agent.updatedAt = Date.now();
  if (agent.endedAt == null) agent.endedAt = Date.now();
  // The persisted job carries the complete text — recover from any dropped
  // stream tokens by replacing the coalesced text line with it.
  getJob(jobId)
    .then((job) => {
      const a = store.agents[jobId];
      if (!a) return;
      const started = toEpochMs(job.started_at);
      if (started != null) a.startedAt = started;
      const finished = toEpochMs(job.finished_at);
      if (finished != null) a.endedAt = finished;
      if (job.output?.text) {
        const nonText = a.lines.filter((l) => l.type !== 'text');
        a.lines = [...nonText, { type: 'text', text: job.output.text }];
      }
    })
    .catch(() => {});
}

/** Reloads the batch detail that owns `jobId`, if any is currently cached. */
function refreshBatchDetailForJob(jobId) {
  for (const [batchId, detail] of Object.entries(store.batchDetails)) {
    if (detail.items.some((it) => it.job_id === jobId)) {
      loadBatchDetail(batchId);
      return;
    }
  }
}

let listening = false;

/** Subscribes to the engine event stream; routes events without per-token refresh. */
export async function startEvents() {
  if (listening) return;
  listening = true;
  await subscribeEvents((event) => {
    switch (event.kind) {
      case 'job_progress':
        ingestAgentEvent(event.job_id, event.event);
        break;
      case 'job_started':
        ensureAgent(event.job_id).status = 'running';
        scheduleRefresh();
        break;
      case 'job_completed':
        finishAgent(event.job_id, 'completed');
        scheduleRefresh();
        refreshBatchDetailForJob(event.job_id);
        break;
      case 'job_failed':
        finishAgent(event.job_id, 'failed');
        scheduleRefresh();
        refreshBatchDetailForJob(event.job_id);
        break;
      case 'job_queued':
      case 'batch_started':
      case 'batch_completed':
      case 'batch_failed':
        scheduleRefresh();
        if (event.batch_id && store.batchDetails[event.batch_id]) {
          loadBatchDetail(event.batch_id);
        }
        break;
      case 'chroma_prompt_degraded':
        pushDegradedToast(event.job_id, event.reason ?? 'ChromaDB unavailable');
        break;
      default:
        scheduleRefresh();
        break;
    }
  });
}
