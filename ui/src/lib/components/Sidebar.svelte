<script>
  import Icon from '$lib/Icon.svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  let { projects, selectedId, onSelect, onAdd, onDelete } = $props();

  let adding = $state(false);
  let name = $state('');
  let path = $state('');
  let confirmId = $state(null);

  function requestDelete(project) {
    confirmId = project.id;
  }

  function confirmDelete(project) {
    confirmId = null;
    onDelete(project.id);
  }

  async function browse() {
    const picked = await open({ directory: true, multiple: false, title: 'Select project folder' });
    if (!picked) return;
    path = picked;
    if (!name.trim()) name = picked.split(/[/\\]/).filter(Boolean).pop() || '';
  }

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
      <div class="path-row">
        <input placeholder="/path/to/repo" bind:value={path} />
        <button class="browse" title="Browse for folder" onclick={browse}>
          <Icon name="folder" size={14} />
        </button>
      </div>
      <div class="row">
        <button class="btn" onclick={submit}>Add</button>
        <button class="btn ghost" onclick={() => (adding = false)}>Cancel</button>
      </div>
    </div>
  {/if}

  <div class="proj-list">
    {#each projects as project (project.id)}
      <div class="proj-row" class:active={project.id === selectedId}>
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
        {#if confirmId === project.id}
          <div class="proj-confirm">
            <button class="del-yes" title="Confirm delete" onclick={() => confirmDelete(project)}>
              <Icon name="check" size={13} />
            </button>
            <button class="del-no" title="Cancel" onclick={() => (confirmId = null)}>
              <Icon name="x" size={13} />
            </button>
          </div>
        {:else}
          <button class="proj-del" title="Delete project" onclick={() => requestDelete(project)}>
            <Icon name="trash" size={13} />
          </button>
        {/if}
      </div>
    {:else}
      <div class="q-empty">No projects yet — add one above.</div>
    {/each}
  </div>

  <div class="sb-footer">
    <div class="model-chip">
      <Icon name="cpu" size={12} />
      <span><span class="auto">AUTO</span> · <span class="mname"></span></span>
    </div>
  </div>
</div>
