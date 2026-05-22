<script>
  import Icon from '$lib/Icon.svelte';
  import { TASK_PRESETS } from '$lib/tasks.js';
</script>

<div class="wizard-head">
  <h2><span class="step">02</span>Run a task batch</h2>
  <span class="hint">batch presets — wired up in P5</span>
</div>

<div class="task-grid">
  {#each TASK_PRESETS as task (task.id)}
    <button
      class="task-card disabled"
      class:featured={task.featured}
      disabled
      title="Batch presets arrive with P5"
    >
      <span class="tc-soon">P5</span>
      <div class="tc-icon"><Icon name={task.icon} size={16} /></div>
      <div class="tc-title">{task.title}</div>
      <div class="tc-desc">{task.desc}</div>
      <div class="tc-meta"><span>prompt-set</span><span>batch</span></div>
    </button>
  {/each}
</div>
