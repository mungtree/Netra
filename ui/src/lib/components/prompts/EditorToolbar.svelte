<script>
  import Icon from '$lib/Icon.svelte';

  let { preset, onDuplicate, onDelete } = $props();

  const totalChars = $derived(
    preset.prompts.reduce((s, p) => s + p.length, 0),
  );
</script>

<div class="pe-toolbar">
  <div class="meta">
    <span><span class="k">prompts</span><span class="v">{preset.prompts.length}</span></span>
    <span><span class="k">chars</span><span class="v">{totalChars.toLocaleString()}</span></span>
    <span><span class="k">~tokens</span><span class="v">{Math.ceil(totalChars / 4).toLocaleString()}</span></span>
    <span><span class="k">strategy</span><span class="v">{preset.strategy}</span></span>
  </div>
  <div class="spacer"></div>
  <button class="btn" onclick={onDuplicate}>
    <Icon name="copy" size={12} />Duplicate
  </button>
  <button
    class="btn"
    onclick={onDelete}
    disabled={preset.builtin}
    title={preset.builtin ? "Built-in batches can't be deleted" : 'Delete'}
  >
    <Icon name="trash" size={12} />Delete
  </button>
</div>
