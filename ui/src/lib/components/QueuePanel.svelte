<script>
  import Icon from '$lib/Icon.svelte';
  import { formatDuration, toEpochMs } from '$lib/time.js';

  let {
    running = [],
    pending = [],
    done = [],
    onCancel,
    onDelete,
    onClearCompleted,
  } = $props();

  let nowTick = $state(Date.now());

  // A 1s ticker keeps live "running for X" and "waiting for X" labels fresh.
  // It runs whenever any job is mid-flight or queued; teardown stops it once
  // the queue is fully drained, so idle panels cost nothing.
  $effect(() => {
    if (running.length === 0 && pending.length === 0) return;
    const id = setInterval(() => (nowTick = Date.now()), 1000);
    return () => clearInterval(id);
  });

  function doneIcon(status) {
    if (status === 'completed') return 'check';
    return 'x';
  }

  /** How long a running job has been executing. */
  function runElapsed(job) {
    const started = toEpochMs(job.started_at);
    return started == null ? null : nowTick - started;
  }

  /** How long a pending job has been waiting in the queue. */
  function queueElapsed(job) {
    const created = toEpochMs(job.created_at);
    return created == null ? null : nowTick - created;
  }

  /** Final run duration for a completed/failed/cancelled job. */
  function runDuration(job) {
    const started = toEpochMs(job.started_at);
    const finished = toEpochMs(job.finished_at);
    if (started == null || finished == null) return null;
    return finished - started;
  }

  /** Final queue wait — time spent queued before execution started. */
  function queueWait(job) {
    const created = toEpochMs(job.created_at);
    const started = toEpochMs(job.started_at);
    if (created == null || started == null) return null;
    return Math.max(0, started - created);
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
          {#if runElapsed(job) != null}
            <span class="q-timer running" title="Running for">
              <Icon name="clock" size={10} />{formatDuration(runElapsed(job))}
            </span>
          {/if}
        </div>
        <div class="q-item-sub">
          <span class="repo">{job.projectName}</span>
          {#if queueWait(job) != null}
            <span class="q-wait" title="Time spent queued before starting">
              waited {formatDuration(queueWait(job))}
            </span>
          {/if}
        </div>
        {#if job.module_name}
          <div class="q-modline">
            <span class="ic"><Icon name="layers" size={10} /></span>
            <span class="modname">{job.module_name}</span>
          </div>
        {/if}
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
          {#if queueElapsed(job) != null}
            <span class="q-timer wait" title="Waiting in queue for">
              <Icon name="clock" size={10} />{formatDuration(queueElapsed(job))}
            </span>
          {/if}
        </div>
        <div class="q-item-sub"><span class="repo">{job.projectName}</span></div>
        {#if job.module_name}
          <div class="q-modline">
            <span class="ic"><Icon name="layers" size={10} /></span>
            <span class="modname">{job.module_name}</span>
          </div>
        {/if}
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
        <div class="q-item-sub">
          <span class="repo">{job.projectName}</span>
          {#if runDuration(job) != null}
            <span class="q-meta-pill" title="Run duration">
              ran {formatDuration(runDuration(job))}
            </span>
          {/if}
          {#if queueWait(job) != null}
            <span class="q-meta-pill" title="Time spent queued before starting">
              waited {formatDuration(queueWait(job))}
            </span>
          {/if}
        </div>
        {#if job.module_name}
          <div class="q-modline">
            <span class="ic"><Icon name="layers" size={10} /></span>
            <span class="modname">{job.module_name}</span>
          </div>
        {/if}
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

  .q-timer {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 10.5px;
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid var(--border);
    color: var(--text-muted);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .q-timer.running {
    color: var(--accent);
    border-color: var(--accent-border);
  }
  .q-timer.wait {
    color: var(--text-dim);
  }
  .q-wait,
  .q-meta-pill {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-dim);
    margin-left: 6px;
    font-variant-numeric: tabular-nums;
  }
</style>
