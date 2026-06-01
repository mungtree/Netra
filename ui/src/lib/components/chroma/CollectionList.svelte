<script>
  import { store, refreshChromaStatus } from '$lib/store.svelte.js';
  import {
    chromaIndexProject,
    chromaDeleteCollection,
    chromaCollectionFiles,
  } from '$lib/api.js';

  let busyId = $state('');
  let error = $state('');
  let expanded = $state(''); // project id whose file list is shown
  let files = $state([]);
  let filter = $state('');

  function projectNameForCollection(name) {
    const id = name.startsWith('netra_') ? name.slice('netra_'.length) : null;
    if (!id) return name;
    const p = store.projects.find((p) => p.id === id);
    return p ? p.name : id;
  }

  async function reindex(projectId) {
    busyId = projectId;
    error = '';
    try {
      await chromaIndexProject(projectId);
      await refreshChromaStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busyId = '';
    }
  }

  async function remove(projectId) {
    if (!confirm(`Delete chroma collection for project ${projectId}?`)) return;
    busyId = projectId;
    try {
      await chromaDeleteCollection(projectId);
      await refreshChromaStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busyId = '';
    }
  }

  async function toggleFiles(projectId) {
    if (expanded === projectId) {
      expanded = '';
      files = [];
      return;
    }
    expanded = projectId;
    files = [];
    try {
      files = await chromaCollectionFiles(projectId);
    } catch (e) {
      error = String(e);
    }
  }

  const filteredFiles = $derived(
    filter ? files.filter((f) => f.path.includes(filter)) : files,
  );

  // Rows: union of indexed collections + every project (so the user can index
  // a project that has no collection yet).
  const rows = $derived.by(() => {
    const collMap = new Map();
    for (const c of store.chromaCollections) {
      const id = c.name.startsWith('netra_') ? c.name.slice(7) : null;
      if (id) collMap.set(id, c);
    }
    const out = [];
    for (const p of store.projects) {
      out.push({
        projectId: p.id,
        name: p.name,
        collection: collMap.get(p.id) ?? null,
      });
    }
    return out;
  });
</script>

<div class="cl">
  {#if !store.chroma?.server || store.chroma.server.state !== 'running'}
    <div class="hint">Start the server to manage collections.</div>
  {/if}
  {#if error}<div class="err">{error}</div>{/if}

  <table>
    <thead>
      <tr>
        <th>Project</th>
        <th>Collection</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {#each rows as row (row.projectId)}
        <tr>
          <td>
            <div class="pname">{row.name}</div>
            <div class="pid">{row.projectId}</div>
          </td>
          <td>
            {#if row.collection}
              <code>{row.collection.name}</code>
              <button
                class="link"
                onclick={() => toggleFiles(row.projectId)}
              >
                {expanded === row.projectId ? 'hide files' : 'view files'}
              </button>
            {:else}
              <span class="hint">— not indexed —</span>
            {/if}
          </td>
          <td class="actions">
            <button
              disabled={busyId === row.projectId
                || store.chroma?.server?.state !== 'running'}
              onclick={() => reindex(row.projectId)}
            >
              {row.collection ? 'Re-index' : 'Index'}
            </button>
            {#if row.collection}
              <button
                disabled={busyId === row.projectId}
                onclick={() => remove(row.projectId)}
              >Delete</button>
            {/if}
          </td>
        </tr>
        {#if expanded === row.projectId}
          <tr class="files-row">
            <td colspan="3">
              <input
                placeholder="filter paths…"
                bind:value={filter}
              />
              <div class="files">
                {#each filteredFiles as f (f.path)}
                  <div class="file-row">
                    <span class="path">{f.path}</span>
                    <span class="meta">{f.chunk_count} chunks · {f.size_bytes}B</span>
                  </div>
                {:else}
                  <div class="hint">No files indexed yet.</div>
                {/each}
              </div>
            </td>
          </tr>
        {/if}
      {/each}
    </tbody>
  </table>
</div>

<style>
  .cl { max-width: 920px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th, td { text-align: left; padding: 6px 8px; border-bottom: 1px solid #2a2a2a; vertical-align: top; }
  th { font-weight: 500; opacity: 0.7; }
  .pname { font-weight: 500; }
  .pid { font-family: monospace; font-size: 10px; opacity: 0.5; }
  .hint { opacity: 0.5; font-size: 11px; }
  .err { color: #f08080; padding: 8px 0; font-size: 12px; }
  .actions { display: flex; gap: 4px; }
  button {
    background: #2a2a2a; color: #d4d4d4; border: 1px solid #444;
    padding: 3px 10px; font-size: 11px; border-radius: 3px; cursor: pointer;
  }
  button.link { background: none; border: none; color: #4a9eff; padding: 0 4px; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
  .files-row td { background: #181818; }
  .files { max-height: 320px; overflow: auto; margin-top: 6px; }
  .file-row { display: flex; justify-content: space-between; padding: 2px 6px; }
  .file-row:nth-child(odd) { background: #1d1d1d; }
  .path { font-family: monospace; font-size: 11px; }
  .meta { opacity: 0.6; font-size: 10px; }
  input { background: #1a1a1a; color: #d4d4d4; border: 1px solid #333; padding: 3px 6px; font-size: 11px; width: 240px; }
</style>
