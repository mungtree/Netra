<script>
  import Icon from '$lib/Icon.svelte';
  import { TASK_PRESETS } from '$lib/tasks.js';

  // `project` — the selected project (or null); `onRun(preset)` — runs a batch.
  let { project = null, onRun } = $props();

  const ready = $derived(!!project);

  function run(preset) {
    if (ready) onRun(preset);
  }
</script>

<div class="wizard-head">
  <h2><span class="step">02</span>Run a task batch</h2>
  <span class="hint">
    {ready
      ? `each card runs a prompt-set over ${project.name}`
      : 'select a project to enable batches'}
  </span>
</div>

<div class="task-grid">
  {#each TASK_PRESETS as task (task.id)}
    <button
      class="task-card"
      class:featured={task.featured}
      class:disabled={!ready}
      disabled={!ready}
      onclick={() => run(task)}
      title={ready
        ? `Run ${task.prompts.length} prompts on ${project.name}`
        : 'Select a project first'}
    >
      <div class="tc-icon"><Icon name={task.icon} size={16} /></div>
      <div class="tc-title">{task.title}</div>
      <div class="tc-desc">{task.desc}</div>
      <div class="tc-meta">
        <span>{task.prompts.length} prompts</span>
        <span>{task.strategy}</span>
      </div>
    </button>
  {/each}
</div>
