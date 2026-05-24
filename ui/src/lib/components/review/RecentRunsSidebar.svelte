<script>
  import Icon from '$lib/Icon.svelte';
  import { countSeverities, relativeTime } from '$lib/reviewFormat.js';
  import { deleteBatch, clearAllBatches } from '$lib/store.svelte.js';

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

  async function onDelete(e, batch) {
    e.stopPropagation();
    if (!confirm(`Delete run "${batch.name}"? This cannot be undone.`)) return;
    await deleteBatch(batch.id);
  }

  async function onClearAll() {
    const n = batches.length;
    if (n === 0) return;
    if (!confirm(`Delete all ${n} runs? This cannot be undone.`)) return;
    await clearAllBatches();
  }
</script>

<div class="sidebar">
  <div class="sb-header">
    <div class="sb-title">Recent Runs</div>
    <button
      class="rrs-clear"
      onclick={onClearAll}
      disabled={batches.length === 0}
      title="Delete every run"
    >
      <Icon name="trash" size={11} />
      Clear all
    </button>
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
        <span
          class="rrs-trash"
          role="button"
          tabindex="0"
          aria-label="Delete run"
          title="Delete run"
          onclick={(e) => onDelete(e, batch)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') onDelete(e, batch);
          }}
        >
          <Icon name="trash" size={11} />
        </span>
      </button>
    {:else}
      <div class="q-empty">No runs yet — queue a task batch first.</div>
    {/each}
  </div>
</div>

<style>
  .sidebar { display: flex; flex-direction: column; height: 100%; }
  .proj-item { width: 100%; text-align: left; padding-right: 34px; }
  .sb-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
