<script>
  import Icon from '$lib/Icon.svelte';

  let {
    presets,
    activeId,
    onPick,
    onNew,
    onImport,
  } = $props();

  let q = $state('');

  const filtered = $derived(
    presets.filter((p) =>
      (p.title ?? '').toLowerCase().includes(q.toLowerCase()),
    ),
  );
  const builtins = $derived(filtered.filter((p) => p.builtin));
  const customs = $derived(filtered.filter((p) => !p.builtin));
</script>

<div class="pe-rail">
  <div class="pe-rail-head">
    <div class="ttl">Batches</div>
    <div class="pe-rail-actions">
      <button class="btn-icon" title="Import JSON" onclick={onImport}>
        <Icon name="upload" size={14} />
      </button>
      <button class="btn-icon" title="New batch" onclick={onNew}>
        <Icon name="plus" size={14} />
      </button>
    </div>
  </div>
  <div class="pe-rail-search">
    <Icon name="search" size={13} />
    <input placeholder="Search batches…" bind:value={q} />
  </div>

  <div class="pe-rail-list">
    {#if builtins.length > 0}
      <div class="pe-rail-section">Built-in</div>
      {#each builtins as p (p.id)}
        <button
          type="button"
          class="pe-preset"
          class:active={p.id === activeId}
          onclick={() => onPick(p.id)}
        >
          <span class="pi-icon"><Icon name={p.icon || 'bookmark'} size={13} /></span>
          <div class="pi-body">
            <div class="pi-title">{p.title}</div>
            <div class="pi-meta">{p.prompts.length} prompts · {p.strategy}</div>
          </div>
        </button>
      {/each}
    {/if}

    {#if customs.length > 0}
      <div class="pe-rail-section" style="margin-top:8px;">Custom</div>
      {#each customs as p (p.id)}
        <button
          type="button"
          class="pe-preset"
          class:active={p.id === activeId}
          onclick={() => onPick(p.id)}
        >
          <span class="pi-icon"><Icon name={p.icon || 'bookmark'} size={13} /></span>
          <div class="pi-body">
            <div class="pi-title">
              {p.title}
              <span class="pi-tag">CUSTOM</span>
            </div>
            <div class="pi-meta">{p.prompts.length} prompts · {p.strategy}</div>
          </div>
        </button>
      {/each}
    {/if}

    {#if filtered.length === 0}
      <div class="pe-rail-empty">No batches match "{q}"</div>
    {/if}
  </div>
</div>
