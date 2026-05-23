// Application state — a single reactive store shared by every component.
//
// Project / job / batch lists are loaded on mount and refreshed (debounced) on
// lifecycle events. Agent turn output is streamed live, per job, into
// `store.agents` — never via a backend round-trip.

import {
  listProjects,
  listJobs,
  listBatches,
  getJob,
  addProject as apiAddProject,
  queueJob as apiQueueJob,
  cancelJob as apiCancelJob,
  createBatch as apiCreateBatch,
  runBatch as apiRunBatch,
  subscribeEvents,
  getConfig,
  saveConfig as apiSaveConfig,
} from './api.js';

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
  /** @type {'projects'|'settings'} active view in the ActivityBar */
  activeView: 'projects',
  /** @type {{globalMax: number, perProjectMax: number, piBinary: string, toolsMode: 'read'|'read_bash'|'full', systemPromptAppend: string}} */
  settings: {
    globalMax: 4,
    perProjectMax: 2,
    piBinary: 'pi',
    toolsMode: 'read',
    systemPromptAppend: '',
  },
  /** @type {boolean} true for a moment after a successful settings save */
  settingsSaved: false,
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

export async function loadSettings() {
  try {
    const cfg = await getConfig();
    store.settings = {
      globalMax: cfg.global_max,
      perProjectMax: cfg.per_project_max,
      piBinary: cfg.pi_binary,
      toolsMode: cfg.tools_mode ?? 'read',
      systemPromptAppend: cfg.system_prompt_append ?? '',
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
    );
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
  };
  store.agents[jobId] = agent;

  // Fill metadata without blocking the event stream.
  getJob(jobId)
    .then((job) => {
      const a = store.agents[jobId];
      if (!a) return;
      a.prompt = job.prompt ?? '';
      a.projectName = projectName(job.project_id);
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
  // The persisted job carries the complete text — recover from any dropped
  // stream tokens by replacing the coalesced text line with it.
  getJob(jobId)
    .then((job) => {
      const a = store.agents[jobId];
      if (!a || !job.output?.text) return;
      const nonText = a.lines.filter((l) => l.type !== 'text');
      a.lines = [...nonText, { type: 'text', text: job.output.text }];
    })
    .catch(() => {});
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
        break;
      case 'job_failed':
        finishAgent(event.job_id, 'failed');
        scheduleRefresh();
        break;
      case 'job_queued':
      case 'batch_started':
      case 'batch_completed':
      case 'batch_failed':
        scheduleRefresh();
        break;
      default:
        scheduleRefresh();
        break;
    }
  });
}
