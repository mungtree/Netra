<script>
  import { onMount } from 'svelte';

  import Icon from '$lib/Icon.svelte';
  import Titlebar from '$lib/components/Titlebar.svelte';
  import ActivityBar from '$lib/components/ActivityBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import MainHeader from '$lib/components/MainHeader.svelte';
  import TaskGrid from '$lib/components/TaskGrid.svelte';
  import LastRun from '$lib/components/LastRun.svelte';
  import QueuePanel from '$lib/components/QueuePanel.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';

  import {
    store,
    refresh,
    startEvents,
    addProject,
    queueJob,
    cancelJob,
    runTaskBatch,
    select,
    clearError,
  } from '$lib/store.svelte.js';

  let prompt = $state('');

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
    await queueJob(store.selectedId, prompt.trim());
    prompt = '';
  }

  onMount(() => {
    refresh();
    startEvents();
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

  <div class="body">
    <ActivityBar />
    <Sidebar
      projects={projectViews}
      selectedId={store.selectedId}
      onSelect={select}
      onAdd={addProject}
    />

    <div class="main">
      <MainHeader project={selectedProject} />
      <div class="main-scroll">
        <div class="wizard-head">
          <h2><span class="step">01</span>Queue a job</h2>
          <span class="hint">runs one pi agent turn on the selected project</span>
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

        <TaskGrid project={selectedProject} onRun={runTaskBatch} />

        <div class="wizard-head">
          <h2><span class="step">03</span>Last run</h2>
          <span class="hint">aggregated output of the most recent batch</span>
        </div>
        <LastRun batch={latestBatch} />

        <div class="wizard-head" style="margin-top: 26px;">
          <h2><span class="step">04</span>Activity</h2>
        </div>
        <div class="run-block">
          <div class="feed">
            {#each store.events.slice(0, 14) as event, i (i)}
              <div class="feed-item">
                <span class="at">{event.at}</span>
                <span class="kind">{event.kind}</span>
              </div>
            {:else}
              <div class="q-empty">No activity yet.</div>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <QueuePanel {running} {pending} {done} onCancel={cancelJob} />
  </div>

  <StatusBar running={running.length} queued={pending.length} done={done.length} />
</div>
