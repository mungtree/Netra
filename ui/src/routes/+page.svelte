<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';

  let projects = $state([]);
  let jobs = $state([]);
  let events = $state([]);
  let selected = $state('');
  let error = $state('');

  let newName = $state('');
  let newPath = $state('');
  let prompt = $state('');

  const selectedProject = $derived(projects.find((p) => p.id === selected));

  function fail(e) {
    error = String(e);
  }

  async function refreshProjects() {
    try {
      projects = await invoke('list_projects');
      if (!selected && projects.length > 0) selected = projects[0].id;
    } catch (e) {
      fail(e);
    }
  }

  async function refreshJobs() {
    if (!selected) {
      jobs = [];
      return;
    }
    try {
      jobs = await invoke('list_jobs', { projectId: selected });
    } catch (e) {
      fail(e);
    }
  }

  async function addProject() {
    if (!newName.trim() || !newPath.trim()) return;
    try {
      await invoke('add_project', { name: newName.trim(), path: newPath.trim() });
      newName = '';
      newPath = '';
      await refreshProjects();
    } catch (e) {
      fail(e);
    }
  }

  async function queueJob() {
    if (!selected || !prompt.trim()) return;
    try {
      await invoke('queue_job', { projectId: selected, prompt: prompt.trim() });
      prompt = '';
      await refreshJobs();
    } catch (e) {
      fail(e);
    }
  }

  async function cancelJob(id) {
    try {
      await invoke('cancel_job', { jobId: id });
      await refreshJobs();
    } catch (e) {
      fail(e);
    }
  }

  onMount(() => {
    refreshProjects();
    const unlisten = listen('chatur://event', (msg) => {
      events = [{ at: new Date().toLocaleTimeString(), ...msg.payload }, ...events].slice(0, 80);
      refreshJobs();
    });
    return () => {
      unlisten.then((off) => off());
    };
  });

  // Reload jobs whenever the selected project changes.
  $effect(() => {
    selected;
    refreshJobs();
  });
</script>

<main>
  <header>
    <h1>Mini ChatUR</h1>
    <span class="sub">queue &amp; batch pi agent jobs</span>
  </header>

  {#if error}
    <div class="error" role="alert">
      {error}
      <button onclick={() => (error = '')}>dismiss</button>
    </div>
  {/if}

  <div class="grid">
    <section class="panel">
      <h2>Projects</h2>
      <ul class="list">
        {#each projects as project (project.id)}
          <li>
            <button
              class:active={project.id === selected}
              onclick={() => (selected = project.id)}
            >
              <strong>{project.name}</strong>
              <small>{project.root_path}</small>
            </button>
          </li>
        {:else}
          <li class="empty">No projects yet.</li>
        {/each}
      </ul>
      <div class="form">
        <input placeholder="name" bind:value={newName} />
        <input placeholder="/path/to/repo" bind:value={newPath} />
        <button onclick={addProject}>Add project</button>
      </div>
    </section>

    <section class="panel">
      <h2>
        Jobs
        {#if selectedProject}<span class="sub">· {selectedProject.name}</span>{/if}
      </h2>

      <div class="form">
        <input
          placeholder="prompt for the agent…"
          bind:value={prompt}
          onkeydown={(e) => e.key === 'Enter' && queueJob()}
          disabled={!selected}
        />
        <button onclick={queueJob} disabled={!selected}>Queue</button>
      </div>

      <ul class="list">
        {#each jobs as job (job.id)}
          <li class="job">
            <span class="badge {job.status}">{job.status}</span>
            <span class="prompt">{job.prompt}</span>
            {#if job.status === 'queued' || job.status === 'running'}
              <button class="ghost" onclick={() => cancelJob(job.id)}>cancel</button>
            {/if}
            {#if job.output}
              <pre class="output">{job.output.text}</pre>
            {/if}
          </li>
        {:else}
          <li class="empty">No jobs for this project.</li>
        {/each}
      </ul>
    </section>

    <section class="panel">
      <h2>Live events</h2>
      <ul class="events">
        {#each events as event, i (i)}
          <li><span class="at">{event.at}</span> {event.kind}</li>
        {:else}
          <li class="empty">Waiting for activity…</li>
        {/each}
      </ul>
    </section>
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: ui-sans-serif, system-ui, sans-serif;
    background: #14161c;
    color: #e6e8ee;
  }
  main {
    padding: 1.25rem 1.5rem;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    margin-bottom: 1rem;
  }
  h1 {
    font-size: 1.3rem;
    margin: 0;
  }
  h2 {
    font-size: 0.95rem;
    margin: 0 0 0.6rem;
  }
  .sub {
    color: #8b90a0;
    font-size: 0.85rem;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.4fr 0.9fr;
    gap: 1rem;
  }
  .panel {
    background: #1c1f29;
    border: 1px solid #2a2e3b;
    border-radius: 10px;
    padding: 0.9rem;
  }
  .list,
  .events {
    list-style: none;
    margin: 0 0 0.8rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .list button {
    width: 100%;
    text-align: left;
    background: #232734;
    border: 1px solid transparent;
    color: inherit;
    border-radius: 8px;
    padding: 0.5rem 0.6rem;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .list button.active {
    border-color: #5b8cff;
  }
  .list small {
    color: #8b90a0;
  }
  .job {
    background: #232734;
    border-radius: 8px;
    padding: 0.5rem 0.6rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }
  .prompt {
    flex: 1;
    min-width: 8rem;
  }
  .badge {
    font-size: 0.7rem;
    text-transform: uppercase;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    background: #3a3f52;
  }
  .badge.completed {
    background: #1f5f3f;
  }
  .badge.failed {
    background: #6e2a2a;
  }
  .badge.running {
    background: #2a4d6e;
  }
  .badge.cancelled {
    background: #5a4a1f;
  }
  .output {
    flex-basis: 100%;
    margin: 0.3rem 0 0;
    background: #14161c;
    padding: 0.5rem;
    border-radius: 6px;
    white-space: pre-wrap;
    font-size: 0.8rem;
  }
  .form {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }
  input {
    flex: 1;
    min-width: 6rem;
    background: #14161c;
    border: 1px solid #2a2e3b;
    color: inherit;
    border-radius: 6px;
    padding: 0.45rem 0.55rem;
  }
  button {
    background: #5b8cff;
    border: none;
    color: #fff;
    border-radius: 6px;
    padding: 0.45rem 0.7rem;
    cursor: pointer;
  }
  button.ghost {
    background: transparent;
    border: 1px solid #3a3f52;
    color: #b9bdca;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
  }
  .events {
    font-size: 0.8rem;
    max-height: 24rem;
    overflow: auto;
  }
  .events .at {
    color: #8b90a0;
    margin-right: 0.4rem;
  }
  .empty {
    color: #8b90a0;
    font-size: 0.85rem;
  }
  .error {
    background: #6e2a2a;
    padding: 0.5rem 0.7rem;
    border-radius: 8px;
    margin-bottom: 0.8rem;
    display: flex;
    justify-content: space-between;
  }
  .error button {
    background: transparent;
    border: 1px solid #ffffff55;
  }
</style>
