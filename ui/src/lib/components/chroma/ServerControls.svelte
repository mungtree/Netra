<script>
  import { store, refreshChromaStatus } from '$lib/store.svelte.js';
  import {
    chromaInstall,
    chromaStart,
    chromaStop,
    chromaRestart,
    getLogPath,
  } from '$lib/api.js';

  let busy = $state(false);
  let error = $state('');
  let logPath = $state('');
  let copied = $state(null);
  let expandedError = $state(null);

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

  async function showLogPath() {
    try {
      logPath = await getLogPath();
    } catch (e) {
      logPath = String(e);
    }
  }

  async function copy(text, key) {
    try {
      await navigator.clipboard.writeText(text);
      copied = key;
      setTimeout(() => {
        if (copied === key) copied = null;
      }, 1500);
    } catch {
      /* clipboard blocked */
    }
  }

  function fmtPath(p) {
    if (!p) return '';
    const s = String(p);
    return s.length > 80 ? `…${s.slice(-78)}` : s;
  }

  const state = $derived(store.chroma?.server?.state ?? 'stopped');
  const installed = $derived(store.chroma?.installed ?? false);
  const mcpReg = $derived(store.chroma?.mcp_registered ?? false);

  const idx = $derived(store.chromaIndexState);
  const pct = $derived(
    idx.filesTotal && idx.filesTotal > 0
      ? Math.min(100, Math.round((idx.filesDone / idx.filesTotal) * 100))
      : null,
  );
  const eta = $derived.by(() => {
    if (!idx.running || !idx.startedAt || !idx.filesTotal || idx.filesDone < 2) {
      return null;
    }
    const elapsed = (Date.now() - idx.startedAt) / 1000;
    const rate = idx.filesDone / elapsed;
    if (rate <= 0) return null;
    const remaining = (idx.filesTotal - idx.filesDone) / rate;
    if (!isFinite(remaining) || remaining <= 0) return null;
    if (remaining < 60) return `${Math.round(remaining)}s`;
    return `${Math.round(remaining / 60)}m`;
  });
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
    <div><strong>Install location:</strong> <code>~/.netra/chroma-venv</code></div>
    <div><strong>Data:</strong> <code>{store.chroma?.config?.data_dir ?? '~/.netra/chroma-data'}</code></div>
    <div><strong>Endpoint:</strong>
      <code>http://{store.chroma?.config?.host}:{store.chroma?.config?.port}</code>
    </div>
  </div>

  <h3>Indexing progress</h3>
  {#if idx.running || idx.finishedAt}
    <div class="progress-block">
      <div class="progress-line">
        <span class="status-pill" data-state={idx.running ? 'starting' : (idx.errors.length ? 'error' : 'running')}>
          {idx.running ? 'indexing' : (idx.errors.length ? 'finished with errors' : 'done')}
        </span>
        <span class="meta">
          {idx.filesDone}{idx.filesTotal != null ? ` / ${idx.filesTotal}` : ''} files
          · {idx.chunks} chunks
          {#if idx.skipped > 0}· {idx.skipped} skipped{/if}
          {#if eta}· ~{eta} left{/if}
        </span>
      </div>
      {#if pct != null}
        <div class="bar"><div class="bar-fill" style="width: {pct}%"></div></div>
      {/if}
      {#if idx.lastFile}
        <div class="current" title={String(idx.lastFile)}>{fmtPath(idx.lastFile)}</div>
      {/if}
    </div>
  {:else}
    <div class="muted">No active indexing.</div>
  {/if}

  {#if idx.errors.length}
    <div class="error-block">
      <h4>Errors ({idx.errors.length})</h4>
      {#each idx.errors as err, i (i)}
        <div class="error-item">
          <div class="error-head">
            <strong>{err.stage}</strong>
            <span class="msg">{err.message}</span>
            {#if err.stderr}
              <button class="link" onclick={() => (expandedError = expandedError === i ? null : i)}>
                {expandedError === i ? 'hide' : 'stderr'}
              </button>
              <button class="link" onclick={() => copy(err.stderr, `err-${i}`)}>
                {copied === `err-${i}` ? 'copied' : 'copy'}
              </button>
            {/if}
          </div>
          {#if expandedError === i && err.stderr}
            <pre class="stderr">{err.stderr}</pre>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if idx.warnings.length}
    <details class="warning-block">
      <summary>Warnings ({idx.warnings.length})</summary>
      {#each idx.warnings.slice(-50) as w, i (i)}
        <div class="warning-item">
          {#if w.path}<code>{fmtPath(w.path)}</code> — {/if}{w.message}
        </div>
      {/each}
    </details>
  {/if}

  <div class="log-row">
    <button class="link" onclick={showLogPath}>Show log folder</button>
    {#if logPath}
      <code class="log-path">{logPath}</code>
      <button class="link" onclick={() => copy(logPath, 'log')}>
        {copied === 'log' ? 'copied' : 'copy'}
      </button>
    {/if}
  </div>
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
  .muted { opacity: 0.6; font-size: 12px; }
  .progress-block { display: flex; flex-direction: column; gap: 6px; }
  .progress-line { display: flex; align-items: center; gap: 10px; font-size: 12px; }
  .bar {
    background: #181818;
    border: 1px solid #2a2a2a;
    border-radius: 3px;
    height: 8px;
    overflow: hidden;
  }
  .bar-fill {
    background: #2a7d2a;
    height: 100%;
    transition: width 200ms linear;
  }
  .current {
    font-family: monospace;
    font-size: 11px;
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .error-block, .warning-block {
    background: #181818;
    border: 1px solid #2a2a2a;
    border-radius: 3px;
    padding: 8px 10px;
    font-size: 12px;
    max-height: 280px;
    overflow: auto;
  }
  .error-block h4 { margin: 0 0 6px; color: #f08080; font-size: 12px; }
  .error-item { padding: 4px 0; border-top: 1px solid #222; }
  .error-item:first-of-type { border-top: none; }
  .error-head { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .error-head strong { color: #f08080; }
  .error-head .msg { flex: 1; min-width: 0; word-break: break-word; }
  .stderr {
    background: #0e0e0e;
    border: 1px solid #2a2a2a;
    padding: 6px 8px;
    margin: 4px 0 0;
    font-size: 11px;
    max-height: 160px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .warning-block summary { cursor: pointer; color: #f0a040; }
  .warning-item { padding: 2px 0; opacity: 0.85; }
  .warning-item code { font-size: 11px; }
  .link {
    background: none;
    border: none;
    color: #6aa9ff;
    cursor: pointer;
    font-size: 11px;
    padding: 0;
    text-decoration: underline;
  }
  .log-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 12px; }
  .log-path { font-family: monospace; font-size: 11px; opacity: 0.8; }
</style>
