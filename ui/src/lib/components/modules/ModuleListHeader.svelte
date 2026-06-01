<script>
  import Icon from '$lib/Icon.svelte';

  let {
    count = 0,
    onAdd,
    onInfer,
    onExport,
    onImport,
    inferDisabled = false,
  } = $props();
</script>

<div class="mod-toolbar">
  <div class="left">
    <strong>{count}</strong> module{count !== 1 ? 's' : ''} · fanout =
    <strong>prompts × targets × modules</strong>
  </div>
  <div class="spacer"></div>
  {#if onImport}
    <button class="btn" onclick={onImport} title="Import modules from JSON">
      <Icon name="upload" size={12} />Import
    </button>
  {/if}
  {#if onExport}
    <button class="btn" onclick={onExport} title="Export modules to JSON">
      <Icon name="download" size={12} />Export
    </button>
  {/if}
  <button class="btn" onclick={onAdd}>
    <Icon name="plus" size={12} />Add module<span class="kbd">⌘N</span>
  </button>
  {#if onInfer}
    <button class="btn-infer" onclick={onInfer} disabled={inferDisabled}>
      <span class="ic"><Icon name="sparkles" size={13} /></span>
      Infer with AI
    </button>
  {/if}
</div>
