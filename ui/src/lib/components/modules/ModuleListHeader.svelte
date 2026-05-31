<script>
  import Icon from '$lib/Icon.svelte';

  let { count = 0, onAdd, onInfer, inferDisabled = false } = $props();
</script>

<div class="mod-toolbar">
  <div class="left">
    <strong>{count}</strong> module{count !== 1 ? 's' : ''} · fanout =
    <strong>prompts × targets × modules</strong>
  </div>
  <div class="spacer"></div>
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
