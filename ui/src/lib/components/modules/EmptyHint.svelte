<script>
  import Icon from '$lib/Icon.svelte';

  let { onAdd, onInfer } = $props();
</script>

<div class="mod-empty-hint">
  <span class="ic"><Icon name="layers" size={20} /></span>
  <div class="body">
    <b>Every project has at least one module.</b> Newly-added projects start with a
    single <code>root</code> module covering the whole repo. Split it into smaller
    scopes so agents stay focused — or let Chatur infer them.
  </div>
  <div class="actions">
    <button class="btn" onclick={onAdd}><Icon name="plus" size={12} />Add manually</button>
    <button class="btn-infer" onclick={onInfer}>
      <Icon name="sparkles" size={13} />Infer with AI
    </button>
  </div>
</div>
