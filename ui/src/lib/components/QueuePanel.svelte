<script>
  import Icon from '$lib/Icon.svelte';

  let { running = [], pending = [], done = [], onCancel } = $props();

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

    <div class="q-group">Completed<span class="line"></span></div>
    {#each done as job (job.id)}
      <div class="q-item done" class:failed={job.status === 'failed'}>
        <div class="q-item-head">
          <span class="q-ic"><Icon name={doneIcon(job.status)} size={13} /></span>
          <span class="q-name">{job.prompt}</span>
          <span class="q-badge {job.status}">{job.status}</span>
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
