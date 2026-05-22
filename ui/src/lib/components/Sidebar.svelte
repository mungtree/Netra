<script>
  import Icon from '$lib/Icon.svelte';

  let { projects, selectedId, onSelect, onAdd } = $props();

  let adding = $state(false);
  let name = $state('');
  let path = $state('');

  function submit() {
    const n = name.trim();
    const p = path.trim();
    if (!n || !p) return;
    onAdd(n, p);
    name = '';
    path = '';
    adding = false;
  }
</script>

<div class="sidebar">
  <div class="sb-header">
    <div class="sb-title">Projects</div>
    <button class="sb-add" title="Add project" onclick={() => (adding = !adding)}>
      <Icon name="plus" size={14} />
    </button>
  </div>

  {#if adding}
    <div class="add-form">
      <input placeholder="name" bind:value={name} />
      <input placeholder="/path/to/repo" bind:value={path} />
      <div class="row">
        <button class="btn" onclick={submit}>Add</button>
        <button class="btn ghost" onclick={() => (adding = false)}>Cancel</button>
      </div>
    </div>
  {/if}

  <div class="proj-list">
    {#each projects as project (project.id)}
      <button
        class="proj-item"
        class:active={project.id === selectedId}
        onclick={() => onSelect(project.id)}
      >
        <span class="proj-status {project.status}"></span>
        <div class="proj-info">
          <div class="proj-name">{project.name}</div>
          <div class="proj-path">{project.root_path}</div>
        </div>
        <div class="proj-count">{project.count || ''}</div>
      </button>
    {:else}
      <div class="q-empty">No projects yet — add one above.</div>
    {/each}
  </div>

  <div class="sb-footer">
    <div class="model-chip">
      <Icon name="cpu" size={12} />
      <span><span class="auto">AUTO</span> · <span class="mname">qwen3.6-35b-a3b</span></span>
    </div>
  </div>
</div>
