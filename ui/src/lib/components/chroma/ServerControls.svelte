<script>
  import { store, refreshChromaStatus } from '$lib/store.svelte.js';
  import {
    chromaInstall,
    chromaStart,
    chromaStop,
    chromaRestart,
  } from '$lib/api.js';

  let busy = $state(false);
  let error = $state('');

  async function wrap(fn) {
    busy = true;
    error = '';
    try {
      await fn();
      await refreshChromaStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const state = $derived(store.chroma?.server?.state ?? 'stopped');
  const installed = $derived(store.chroma?.installed ?? false);
  const mcpReg = $derived(store.chroma?.mcp_registered ?? false);

  const recentEvents = $derived(store.chromaIndexEvents.slice(-20).reverse());
</script>

<div class="server-controls">
  <div class="status-row">
    <span class="status-pill" data-state={state}>{state}</span>
    {#if store.chroma?.server?.state === 'running'}
      <span class="meta">
        pid {store.chroma.server.pid} · port {store.chroma.server.port}
      </span>
    {/if}
    {#if !installed}
      <span class="warn">venv not installed</span>
    {/if}
    {#if !mcpReg}
      <span class="warn">pi MCP entry missing</span>
    {/if}
  </div>

  <div class="btn-row">
    <button disabled={busy} onclick={() => wrap(chromaInstall)}>
      {installed ? 'Reinstall' : 'Install'}
    </button>
    <button
      disabled={busy || !installed || state === 'running' || state === 'starting'}
      onclick={() => wrap(chromaStart)}
    >Start</button>
    <button
      disabled={busy || state !== 'running'}
      onclick={() => wrap(chromaStop)}
    >Stop</button>
    <button
      disabled={busy || !installed}
      onclick={() => wrap(chromaRestart)}
    >Restart</button>
    <button disabled={busy} onclick={refreshChromaStatus}>Refresh</button>
  </div>

  {#if error}
    <div class="err">{error}</div>
  {/if}

  <div class="info">
    <div><strong>Install location:</strong> <code>~/.chatur/chroma-venv</code></div>
    <div><strong>Data:</strong> <code>{store.chroma?.config?.data_dir ?? '~/.chatur/chroma-data'}</code></div>
    <div><strong>Endpoint:</strong>
      <code>http://{store.chroma?.config?.host}:{store.chroma?.config?.port}</code>
    </div>
  </div>

  <h3>Activity</h3>
  <pre class="events">
{#each recentEvents as ev, i (i)}{JSON.stringify(ev)}
{/each}
  </pre>
</div>

<style>
  .server-controls { display: flex; flex-direction: column; gap: 14px; max-width: 720px; }
  .status-row { display: flex; align-items: center; gap: 10px; font-size: 12px; }
  .status-pill {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 11px;
    text-transform: uppercase;
    background: #555;
    color: white;
  }
  .status-pill[data-state="running"] { background: #2a7d2a; }
  .status-pill[data-state="starting"] { background: #b08820; }
  .status-pill[data-state="error"] { background: #a83232; }
  .status-pill[data-state="stopped"] { background: #555; }
  .meta { opacity: 0.7; font-family: monospace; font-size: 11px; }
  .warn { color: #f0a040; font-size: 11px; }
  .btn-row { display: flex; gap: 6px; flex-wrap: wrap; }
  .btn-row button {
    padding: 4px 12px; font-size: 12px;
    background: #2a2a2a; color: #d4d4d4;
    border: 1px solid #444; border-radius: 3px; cursor: pointer;
  }
  .btn-row button:disabled { opacity: 0.4; cursor: not-allowed; }
  .err { color: #f08080; font-size: 12px; }
  .info { font-size: 12px; opacity: 0.8; display: flex; flex-direction: column; gap: 2px; }
  .events {
    background: #181818;
    padding: 8px;
    font-size: 11px;
    max-height: 260px;
    overflow: auto;
    border: 1px solid #2a2a2a;
    border-radius: 3px;
  }
</style>
