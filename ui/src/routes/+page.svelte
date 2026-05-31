<script>
  import { onMount } from 'svelte';

  import Icon from '$lib/Icon.svelte';
  import Titlebar from '$lib/components/Titlebar.svelte';
  import ActivityBar from '$lib/components/ActivityBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import MainHeader from '$lib/components/MainHeader.svelte';
  import TaskGrid from '$lib/components/TaskGrid.svelte';
  import LastRun from '$lib/components/LastRun.svelte';
  import OutputPane from '$lib/components/OutputPane.svelte';
  import QueuePanel from '$lib/components/QueuePanel.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import SettingsPane from '$lib/components/SettingsPane.svelte';
  import ReviewView from '$lib/components/review/ReviewView.svelte';
  import PromptEditorView from '$lib/components/prompts/PromptEditorView.svelte';
  import ChromaPane from '$lib/components/chroma/ChromaPane.svelte';
  import ModulesPane from '$lib/components/modules/ModulesPane.svelte';
  import ProjectsOverview from '$lib/components/modules/ProjectsOverview.svelte';
  import ResumeBanner from '$lib/components/modules/ResumeBanner.svelte';
  import BatchBuilder from '$lib/components/modules/BatchBuilder.svelte';

  import {
    store,
    refresh,
    startEvents,
    loadSettings,
    loadResume,
    addProject,
    queueJob,
    cancelJob,
    deleteJob,
    clearCompletedJobs,
    runTaskBatch,
    select,
    clearError,
    refreshChromaStatus,
    startChromaEvents,
    dismissChromaToast,
    startNotifications,
    dismissNotification,
  } from '$lib/store.svelte.js';

  let prompt = $state('');
  let useChromadb = $state(false);
  let showBatchBuilder = $state(false);

  const projectName = (id) =>
    store.projects.find((p) => p.id === id)?.name ?? '—';

  // Projects enriched with a derived status dot and job count.
  const projectViews = $derived(
    store.projects.map((project) => {
      const jobs = store.jobs.filter((j) => j.project_id === project.id);
      let status = 'idle';
      if (jobs.some((j) => j.status === 'running')) status = 'run';
      else if (jobs.some((j) => j.status === 'failed')) status = 'err';
      else if (jobs.some((j) => j.status === 'completed')) status = 'done';
      return { ...project, status, count: jobs.length };
    }),
  );

  const selectedProject = $derived(
    store.projects.find((p) => p.id === store.selectedId) ?? null,
  );

  // The most recent batch — store keeps `batches` sorted newest-first.
  const latestBatch = $derived(store.batches[0] ?? null);

  // Live agent output streams, one entry per job.
  const agents = $derived(Object.values(store.agents));

  // Queue groups, across every project, tagged with their project name.
  const withName = (job) => ({ ...job, projectName: projectName(job.project_id) });
  const running = $derived(
    store.jobs.filter((j) => j.status === 'running').map(withName),
  );
  const pending = $derived(
    store.jobs.filter((j) => j.status === 'queued').map(withName),
  );
  const done = $derived(
    store.jobs
      .filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status))
      .map(withName),
  );

  async function submitJob() {
    if (!store.selectedId || !prompt.trim()) return;
    await queueJob(store.selectedId, prompt.trim(), useChromadb);
    prompt = '';
  }

  const chromaRunning = $derived(
    store.chroma?.server?.state === 'running',
  );

  onMount(() => {
    refresh();
    startEvents();
    loadSettings();
    refreshChromaStatus();
    startChromaEvents();
    startNotifications();
    loadResume();
  });
</script>

<div class="app">
  <Titlebar />

  {#if store.error}
    <div class="errbar">
      <span>{store.error}</span>
      <button onclick={clearError}>dismiss</button>
    </div>
  {/if}

  {#if store.chromaDegradedToasts.length}
    <div class="toasts">
      {#each store.chromaDegradedToasts as t (t.id)}
        <div class="toast">
          <strong>ChromaDB unavailable</strong>
          <span>{t.reason}</span>
          <button onclick={() => dismissChromaToast(t.id)}>×</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if store.notifications.length}
    <div class="toasts">
      {#each store.notifications as n (n.id)}
        <div class="toast toast-{n.level}">
          <strong>{n.source}</strong>
          <span>{n.message}</span>
          <button onclick={() => dismissNotification(n.id)}>×</button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="body">
    <ActivityBar />
    {#if store.activeView === 'history'}
      <ReviewView />
    {:else if store.activeView === 'prompts'}
      <PromptEditorView />
    {:else if store.activeView === 'chromadb'}
      <ChromaPane />
    {:else if store.activeView === 'overview'}
      <ProjectsOverview />
    {:else}
      <Sidebar
        projects={projectViews}
        selectedId={store.selectedId}
        onSelect={select}
        onAdd={addProject}
      />

      {#if store.activeView === 'settings'}
        <SettingsPane />
      {:else if store.activeView === 'modules'}
        <ModulesPane />
      {:else}
      <div class="main">
        <ResumeBanner />
        <MainHeader project={selectedProject} />
        <div class="main-scroll">
          <div class="wizard-head">
            <h2><span class="step">01</span>Queue a job</h2>
            <span class="hint">runs one pi agent turn on the selected project</span>
            <button
              class="newbatch-btn"
              onclick={() => (showBatchBuilder = true)}
              title="Create a module-aware batch across projects"
            >
              <Icon name="layers" size={12} />New batch
            </button>
          </div>
          <div class="page-chroma">
            <label
              class="chroma-toggle"
              title={chromaRunning
                ? 'Tell the agent it can use ChromaDB for semantic search (applies to manual prompts and task batches)'
                : 'ChromaDB server is not running (manage it in the ChromaDB pane)'}
            >
              <input
                type="checkbox"
                bind:checked={useChromadb}
                disabled={!chromaRunning}
              />
              Use ChromaDB
            </label>
          </div>

          <div class="quickjob">
            <div class="qj-head">Prompt</div>
            <div class="qj-sub">
              {selectedProject
                ? `target · ${selectedProject.name}`
                : 'select a project first'}
            </div>
            <textarea
              bind:value={prompt}
              placeholder="Ask the agent to do something…"
              disabled={!store.selectedId}
            ></textarea>
            <div class="qj-foot">
              <button
                class="btn"
                onclick={submitJob}
                disabled={!store.selectedId || !prompt.trim()}
              >
                Queue job
              </button>
            </div>
          </div>

          <TaskGrid project={selectedProject} onRun={(preset) => runTaskBatch(preset, useChromadb)} />

          <div class="wizard-head">
            <h2><span class="step">03</span>Last run</h2>
            <span class="hint">aggregated output of the most recent batch</span>
          </div>
          <LastRun batch={latestBatch} />

          <div class="wizard-head" style="margin-top: 26px;">
            <h2><span class="step">04</span>Agent output</h2>
            <span class="hint">live thinking, tool calls, and answers per agent</span>
          </div>
          <OutputPane {agents} />
        </div>
      </div>

      <QueuePanel
        {running}
        {pending}
        {done}
        onCancel={cancelJob}
        onDelete={deleteJob}
        onClearCompleted={clearCompletedJobs}
      />
      {/if}
    {/if}
  </div>

  <StatusBar running={running.length} queued={pending.length} done={done.length} />

  {#if showBatchBuilder}
    <BatchBuilder onClose={() => (showBatchBuilder = false)} />
  {/if}
</div>

<style>
  .page-chroma {
    display: flex;
    justify-content: flex-end;
    margin: 6px 0 10px;
  }
  .chroma-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    opacity: 0.9;
    cursor: pointer;
  }
  .chroma-toggle input { margin: 0; }
  .chroma-toggle input:disabled ~ * { opacity: 0.5; }
  .newbatch-btn {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    padding: 5px 10px;
    border: 1px solid var(--accent-border);
    background: var(--bg);
    color: var(--text);
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
  }
  .newbatch-btn:hover { background: var(--bg-elev); border-color: var(--accent); }
</style>
