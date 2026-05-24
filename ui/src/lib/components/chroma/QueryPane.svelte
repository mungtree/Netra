<script>
  import { store } from '$lib/store.svelte.js';
  import { chromaQuery } from '$lib/api.js';

  let projectId = $state('');
  let query = $state('');
  let n = $state(10);
  let busy = $state(false);
  let error = $state('');
  let hits = $state([]);
  let expanded = $state(new Set());

  const indexedSet = $derived(new Set(
    (store.chromaCollections ?? [])
      .map((c) => (c.name.startsWith('chatur_') ? c.name.slice(7) : null))
      .filter(Boolean),
  ));

  const projects = $derived(store.projects);
  const serverRunning = $derived(store.chroma?.server?.state === 'running');

  async function run() {
    if (!projectId || !query.trim()) return;
    busy = true;
    error = '';
    hits = [];
    expanded = new Set();
    try {
      hits = await chromaQuery(projectId, query.trim(), Number(n) || 10);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function toggleExpanded(id) {
    const s = new Set(expanded);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    expanded = s;
  }

  function snippet(doc, full) {
    if (full) return doc;
    const lines = doc.split('\n').slice(0, 3).join('\n');
    return lines + (doc.length > lines.length ? ' …' : '');
  }

  function fmtDistance(d) {
    if (typeof d !== 'number') return '';
    return d.toFixed(3);
  }
</script>

<div class="qp">
  {#if !serverRunning}
    <div class="hint">Start the ChromaDB server first (Server tab).</div>
  {/if}

  <div class="form">
    <label>
      Project
      <select bind:value={projectId} disabled={!serverRunning}>
        <option value="" disabled>— pick a project —</option>
        {#each projects as p (p.id)}
          <option value={p.id}>
            {p.name}
            {indexedSet.has(p.id) ? '' : ' (not indexed)'}
          </option>
        {/each}
      </select>
    </label>

    <label class="wide">
      Query
      <textarea
        rows="2"
        bind:value={query}
        placeholder="e.g. how does the sqlite migration runner work?"
        disabled={!serverRunning}
      ></textarea>
    </label>

    <label class="n">
      n
      <input type="number" min="1" max="100" bind:value={n} />
    </label>

    <button
      onclick={run}
      disabled={busy || !serverRunning || !projectId || !query.trim()}
    >
      {busy ? 'Querying…' : 'Search'}
    </button>
  </div>

  {#if error}<div class="err">{error}</div>{/if}

  {#if hits.length > 0}
    <table>
      <thead>
        <tr>
          <th>#</th>
          <th>dist</th>
          <th>location</th>
          <th>snippet</th>
        </tr>
      </thead>
      <tbody>
        {#each hits as hit, i (hit.id)}
          <tr>
            <td class="idx">{i + 1}</td>
            <td class="dist">{fmtDistance(hit.distance)}</td>
            <td class="loc">
              {#if hit.path}
                <code>{hit.path}</code>
                {#if hit.line_start}
                  <span class="lines">:{hit.line_start}{hit.line_end ? `-${hit.line_end}` : ''}</span>
                {/if}
              {:else}
                <span class="hint">—</span>
              {/if}
            </td>
            <td class="snip" onclick={() => toggleExpanded(hit.id)}>
              <pre>{snippet(hit.document, expanded.has(hit.id))}</pre>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else if !busy && !error}
    <div class="hint">No results yet. Pick a project, type a query, hit Search.</div>
  {/if}
</div>

<style>
  .qp { max-width: 960px; display: flex; flex-direction: column; gap: 14px; }
  .form { display: grid; grid-template-columns: 1fr 2fr 80px auto; gap: 10px; align-items: end; }
  .form .wide { grid-column: span 1; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11px; opacity: 0.8; }
  select, input, textarea {
    background: #1a1a1a; color: #d4d4d4; border: 1px solid #333;
    padding: 4px 6px; font-size: 12px; border-radius: 3px;
  }
  textarea { font-family: inherit; resize: vertical; }
  button {
    background: #2a2a2a; color: #d4d4d4; border: 1px solid #444;
    padding: 6px 14px; border-radius: 3px; cursor: pointer; font-size: 12px;
    height: 30px;
  }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
  .err { color: #f88; font-size: 12px; }
  .hint { opacity: 0.55; font-size: 11px; }
  table { width: 100%; border-collapse: collapse; font-size: 12px; }
  th, td { text-align: left; padding: 6px 8px; border-bottom: 1px solid #2a2a2a; vertical-align: top; }
  th { font-weight: 500; opacity: 0.7; }
  .idx { width: 30px; opacity: 0.6; font-family: monospace; }
  .dist { width: 60px; font-family: monospace; }
  .loc code { font-family: monospace; font-size: 11px; }
  .lines { opacity: 0.6; font-family: monospace; font-size: 11px; }
  .snip { cursor: pointer; }
  .snip pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    background: #181818;
    padding: 6px 8px;
    font-size: 11px;
    border-radius: 3px;
    max-width: 480px;
  }
</style>
