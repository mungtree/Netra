<script>
  import Icon from '$lib/Icon.svelte';
  import { countSeverities, relativeTime } from '$lib/reviewFormat.js';

  let { batches, selectedId, projectName, onPick } = $props();

  let query = $state('');

  const filtered = $derived(
    batches.filter((b) => {
      if (!query.trim()) return true;
      const q = query.toLowerCase();
      return (
        b.name.toLowerCase().includes(q) ||
        projectName(b).toLowerCase().includes(q)
      );
    }),
  );

  function statusClass(b) {
    if (b.status === 'running') return 'run';
    if (b.status === 'failed' || b.status === 'cancelled') return 'err';
    const sevs = countSeverities(b.result?.structured?.findings);
    if (sevs.critical + sevs.high > 0) return 'err';
    return 'done';
  }

  function findingsCount(b) {
    const n = b.result?.structured?.findings?.length ?? 0;
    return n || '';
  }
</script>

<div class="sidebar">
  <div class="sb-header">
    <div class="sb-title">Recent Runs</div>
  </div>
  <div class="sb-search">
    <Icon name="search" size={13} />
    <input placeholder="Search runs…" bind:value={query} />
  </div>
  <div class="proj-list">
    {#each filtered as batch (batch.id)}
      <button
        type="button"
        class="proj-item"
        class:active={batch.id === selectedId}
        onclick={() => onPick(batch.id)}
      >
        <span class="proj-status {statusClass(batch)}"></span>
        <div class="proj-info">
          <div class="proj-name">{batch.name}</div>
          <div class="proj-path">
            {projectName(batch)} · {relativeTime(batch.created_at)}
          </div>
        </div>
        <div class="proj-count">{findingsCount(batch)}</div>
      </button>
    {:else}
      <div class="q-empty">No runs yet — queue a task batch first.</div>
    {/each}
  </div>
</div>

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; }
  .proj-item { width: 100%; text-align: left; }
</style>
