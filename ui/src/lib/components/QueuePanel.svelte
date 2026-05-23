<script>
  import Icon from '$lib/Icon.svelte';

  let {
    running = [],
    pending = [],
    done = [],
    onCancel,
    onDelete,
    onClearCompleted,
  } = $props();

  function doneIcon(status) {
    if (status === 'completed') return 'check';
    return 'x';
  }
</script>

<div class="queue">
  <div class="queue-head">
    <div class="q-title">Queue</div>
    <div class="q-count">{running.length + pending.length} active</div>
  </div>

  <div class="queue-scroll">
    <div class="q-group">Running<span class="line"></span></div>
    {#each running as job (job.id)}
      <div class="q-item running">
        <div class="q-item-head">
          <span class="q-ic"><Icon name="activity" size={13} /></span>
          <span class="q-name">{job.prompt}</span>
        </div>
        <div class="q-item-sub"><span class="repo">{job.projectName}</span></div>
        <div class="q-progress"><div class="bar"></div></div>
        <div class="q-actions">
          <button class="btn-mini danger" onclick={() => onCancel(job.id)}>
            <Icon name="x" size={10} />Cancel
          </button>
        </div>
      </div>
    {:else}
      <div class="q-empty">Nothing running.</div>
    {/each}

    <div class="q-group">Pending · {pending.length}<span class="line"></span></div>
    {#each pending as job (job.id)}
      <div class="q-item">
        <div class="q-item-head">
          <span class="q-ic"><Icon name="clock" size={13} /></span>
          <span class="q-name">{job.prompt}</span>
        </div>
        <div class="q-item-sub"><span class="repo">{job.projectName}</span></div>
        <div class="q-actions">
          <button class="btn-mini danger" onclick={() => onCancel(job.id)}>
            <Icon name="x" size={10} />Cancel
          </button>
        </div>
      </div>
    {:else}
      <div class="q-empty">Queue is empty.</div>
    {/each}

    <div class="q-group">
      Completed · {done.length}
      <span class="line"></span>
      {#if done.length > 0 && onClearCompleted}
        <button
          class="q-clear"
          title="Remove every completed/failed/cancelled job"
          onclick={onClearCompleted}
        >
          <Icon name="eraser" size={11} />Clear
        </button>
      {/if}
    </div>
    {#each done as job (job.id)}
      <div class="q-item done" class:failed={job.status === 'failed'}>
        <div class="q-item-head">
          <span class="q-ic"><Icon name={doneIcon(job.status)} size={13} /></span>
          <span class="q-name">{job.prompt}</span>
          <span class="q-badge {job.status}">{job.status}</span>
          {#if onDelete}
            <button
              class="q-del"
              title="Delete this job"
              onclick={() => onDelete(job.id)}
            >
              <Icon name="x" size={11} />
            </button>
          {/if}
        </div>
        <div class="q-item-sub"><span class="repo">{job.projectName}</span></div>
        {#if job.output && job.output.text}
          <div class="q-output">{job.output.text}</div>
        {/if}
      </div>
    {:else}
      <div class="q-empty">No completed jobs.</div>
    {/each}
  </div>

  <div class="queue-foot">
    <div class="q-stat">
      <span class="label">Running</span>
      <span class="val run">{running.length}</span>
    </div>
    <div class="q-stat">
      <span class="label">Pending</span>
      <span class="val">{pending.length}</span>
    </div>
    <div class="q-stat">
      <span class="label">Done</span>
      <span class="val done">{done.length}</span>
    </div>
  </div>
</div>

<style>
  .q-clear {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 10.5px;
    cursor: pointer;
  }
  .q-clear:hover { color: var(--text); }
  .q-del {
    margin-left: auto;
    background: transparent;
    color: var(--text-dim);
    border: none;
    padding: 2px 4px;
    cursor: pointer;
    border-radius: 3px;
  }
  .q-del:hover { color: var(--sev-high, #ef4444); background: var(--bg-hover); }
</style>
