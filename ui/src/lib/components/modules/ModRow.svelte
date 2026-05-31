<script>
  import Icon from '$lib/Icon.svelte';

  let {
    mod,
    isDefault = false,
    editing = false,
    isNew = false,
    warn = null,
    onEdit,
    onDuplicate,
    onDelete,
    onSave,
    onCancel,
  } = $props();

  // Local editable copy — only touched while `editing`.
  let name = $state(mod?.name ?? '');
  let desc = $state(mod?.description ?? '');
  let root = $state(mod?.root_subdir ?? '');

  function commit() {
    const n = name.trim();
    if (!n) return;
    onSave?.({ ...mod, name: n, description: desc.trim(), root_subdir: root.trim() });
  }

  function onKey(e) {
    if (e.key === 'Enter') commit();
    else if (e.key === 'Escape') onCancel?.();
  }
</script>

{#if editing}
  <div class="mod-row editing">
    <span class="ic"><Icon name={isNew ? 'plus' : 'code'} size={13} /></span>
    <div class="name">
      <input class="cell-input mono" bind:value={name} placeholder="name" onkeydown={onKey} />
    </div>
    <div class="desc">
      <input
        class="cell-input"
        bind:value={desc}
        placeholder="one-line description"
        onkeydown={onKey}
      />
    </div>
    <div class="path" style="display:flex; align-items:center;">
      <input
        class="cell-input mono"
        bind:value={root}
        placeholder="subdir/relative/path"
        style="flex:1"
        onkeydown={onKey}
      />
    </div>
    <div class="row-actions">
      <button class="btn-icon commit" title="Save" onclick={commit}>
        <Icon name="check" size={12} />
      </button>
      <button class="btn-icon" title="Cancel" onclick={() => onCancel?.()}>
        <Icon name="x" size={12} />
      </button>
    </div>
  </div>
{:else}
  <div class="mod-row" class:default={isDefault}>
    <span class="ic"><Icon name={isDefault ? 'package' : 'layers'} size={13} /></span>
    <div class="name"><span>{mod.name}</span></div>
    <div class="desc">{mod.description}</div>
    <div class="path">
      <Icon name="folder" size={11} />
      <span>{mod.root_subdir || '.'}</span>
    </div>
    <div class="row-actions">
      <button class="btn-icon" title="Edit" onclick={() => onEdit?.()}>
        <Icon name="code" size={12} />
      </button>
      <button class="btn-icon" title="Duplicate" onclick={() => onDuplicate?.()}>
        <Icon name="copy" size={12} />
      </button>
      <button
        class="btn-icon danger"
        title="Delete"
        disabled={isDefault}
        onclick={() => onDelete?.()}
      >
        <Icon name="trash" size={12} />
      </button>
    </div>
    {#if warn}
      <div class="mod-warn" class:err={warn.kind === 'err'}>
        <span class="ic"><Icon name="info" size={12} /></span>
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        <div class="body">{@html warn.body}</div>
      </div>
    {/if}
  </div>
{/if}
