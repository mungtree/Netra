<script>
  import { store, refreshChromaStatus } from '$lib/store.svelte.js';
  import {
    chromaUpdateSettings,
    chromaSetEnabled,
    chromaSetEmbeddingModel,
    chromaDropAndReindex,
  } from '$lib/api.js';

  const MODEL_PRESETS = [
    { id: 'default', label: 'Default — all-MiniLM-L6-v2 (ONNX, 384d, bundled)' },
    { id: 'jina-code', label: 'jina-embeddings-v2-base-code (768d, ~300MB, code-tuned)' },
    { id: 'coderank', label: 'nomic-ai/CodeRankEmbed (768d, ~550MB, code search)' },
    { id: 'sfr-code', label: 'Salesforce SFR-Embedding-Code-400M_R (1024d, ~1.5GB)' },
    { id: 'bge-code', label: 'BAAI/bge-code-v1 (1536d, ~6GB)' },
    { id: 'custom', label: 'Custom HuggingFace model id…' },
  ];

  let host = $state('');
  let port = $state(8765);
  let dataDir = $state('');
  let autoStart = $state(true);
  let maxBytes = $state(1048576);
  let extraGlobs = $state('');
  let embeddingModel = $state('default');
  let embeddingModelCustom = $state('');
  let saved = $state(false);
  let error = $state('');
  let reindexing = $state(false);

  $effect(() => {
    const c = store.chroma?.config;
    if (!c) return;
    host = c.host;
    port = c.port;
    dataDir = String(c.data_dir);
    autoStart = c.auto_start;
    maxBytes = c.max_file_size_bytes;
    extraGlobs = (c.extra_ignore_globs ?? []).join('\n');
    embeddingModel = c.embedding_model ?? 'default';
    embeddingModelCustom = c.embedding_model_custom ?? '';
  });

  async function save() {
    error = '';
    try {
      await chromaUpdateSettings({
        enabled: store.chroma?.enabled ?? false,
        host,
        port: Number(port),
        data_dir: dataDir,
        auto_start: autoStart,
        max_file_size_bytes: Number(maxBytes),
        extra_ignore_globs: extraGlobs
          .split('\n')
          .map((s) => s.trim())
          .filter(Boolean),
        embedding_model: embeddingModel,
        embedding_model_custom:
          embeddingModel === 'custom' ? embeddingModelCustom.trim() || null : null,
      });

      // Embedding-model change is a separate command because it may require
      // dropping + rebuilding collections (vector dims change). When the
      // model is unchanged the backend returns requires_reindex = false.
      const change = await chromaSetEmbeddingModel(
        embeddingModel,
        embeddingModel === 'custom' ? embeddingModelCustom.trim() || null : null,
      );
      if (change.requiresReindex && change.affectedProjectIds.length > 0) {
        const ok = window.confirm(
          `Embedding model changed (${change.previousModel} → ${change.newModel}).\n\n` +
            `Existing vectors use the old model and will fail queries. ` +
            `Drop and rebuild ${change.affectedCollections.length} collection(s) now?\n\n` +
            change.affectedCollections.join('\n'),
        );
        if (ok) {
          reindexing = true;
          try {
            await chromaDropAndReindex(change.affectedProjectIds);
          } finally {
            reindexing = false;
          }
        }
      }

      saved = true;
      setTimeout(() => (saved = false), 2000);
      await refreshChromaStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleEnabled(e) {
    try {
      await chromaSetEnabled(e.currentTarget.checked);
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (err) {
      error = String(err);
    }
  }
</script>

<div class="settings">
  <div class="field">
    <label>
      <input
        type="checkbox"
        checked={store.chroma?.enabled ?? false}
        onchange={toggleEnabled}
      />
      Enable ChromaDB integration
    </label>
    <p class="desc">
      Master switch. Persisted to <code>netra.toml</code>. Takes effect after
      restart — disabled by default so the rest of the app is unaffected.
    </p>
  </div>

  <div class="field">
    <label>Host <input bind:value={host} /></label>
    <label>Port <input type="number" bind:value={port} /></label>
  </div>

  <div class="field">
    <label class="wide">
      Data directory
      <input bind:value={dataDir} />
    </label>
  </div>

  <div class="field">
    <label>
      <input type="checkbox" bind:checked={autoStart} />
      Auto-start server when NETRA launches
    </label>
  </div>

  <div class="field">
    <label>
      Max file size (bytes)
      <input type="number" bind:value={maxBytes} />
    </label>
  </div>

  <div class="field wide">
    <label>
      Embedding model
      <select bind:value={embeddingModel}>
        {#each MODEL_PRESETS as preset (preset.id)}
          <option value={preset.id}>{preset.label}</option>
        {/each}
      </select>
    </label>
    {#if embeddingModel === 'custom'}
      <label>
        HuggingFace model id
        <input
          bind:value={embeddingModelCustom}
          placeholder="e.g. BAAI/bge-small-en-v1.5"
        />
      </label>
    {/if}
    <p class="desc">
      Code-tuned models give better recall on "where is X implemented" queries
      but download model weights on first use. Switching models requires
      dropping and re-indexing every project collection (vector dimensions
      differ) — you'll be prompted on save.
    </p>
  </div>

  <div class="field wide">
    <label>Extra ignore globs (one per line)</label>
    <textarea rows="6" bind:value={extraGlobs}
      placeholder={`*.log\nvendor/**\nfixtures/**/*.json`}></textarea>
    <p class="desc">
      Added on top of <code>.gitignore</code> and the built-in binary
      blacklist (png/zip/exe/onnx/…).
    </p>
  </div>

  <div class="actions">
    <button onclick={save} disabled={reindexing}>Save settings</button>
    {#if reindexing}<span class="ok">Reindexing…</span>{/if}
    {#if saved}<span class="ok">Saved</span>{/if}
    {#if error}<span class="err">{error}</span>{/if}
  </div>
</div>

<style>
  .settings { max-width: 640px; display: flex; flex-direction: column; gap: 14px; font-size: 12px; }
  .field { display: flex; gap: 10px; align-items: center; }
  .field.wide { flex-direction: column; align-items: stretch; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 11px; opacity: 0.8; }
  input, textarea {
    background: #1a1a1a; color: #d4d4d4; border: 1px solid #333;
    padding: 4px 6px; font-size: 12px; border-radius: 3px;
  }
  textarea { font-family: monospace; }
  .desc { margin: 4px 0 0; font-size: 11px; opacity: 0.6; }
  .actions { display: flex; gap: 10px; align-items: center; }
  button {
    background: #2a2a2a; color: #d4d4d4; border: 1px solid #444;
    padding: 5px 14px; border-radius: 3px; cursor: pointer; font-size: 12px;
  }
  .ok { color: #6c6; font-size: 11px; }
  .err { color: #f88; font-size: 11px; }
  
</style>
