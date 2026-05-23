<script>
  import Icon from '$lib/Icon.svelte';
  import { TASK_PRESETS } from '$lib/tasks.js';
  import { store, addCustomPreset, removeCustomPreset } from '$lib/store.svelte.js';
  import { serializeBatch, parseBatch } from '$lib/batchIo.js';

  // `project` — the selected project (or null); `onRun(preset)` — runs a batch.
  let { project = null, onRun } = $props();

  const ready = $derived(!!project);
  const allPresets = $derived([...TASK_PRESETS, ...store.customPresets]);

  let importError = $state('');
  /** @type {HTMLInputElement | null} */
  let fileInput = null;

  function run(preset) {
    if (ready) onRun(preset);
  }

  /** Triggers a browser-style download of `text` as `filename`. */
  function downloadText(filename, text) {
    const blob = new Blob([text], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  /** Exports every preset (built-in + custom) as a single JSON file each. */
  function exportAll() {
    for (const preset of allPresets) {
      const safe = (preset.title || preset.id).replace(/[^a-z0-9]+/gi, '-').toLowerCase();
      downloadText(`${safe}.batch.json`, serializeBatch(preset));
    }
  }

  /** Exports just one preset. */
  function exportOne(preset) {
    const safe = (preset.title || preset.id).replace(/[^a-z0-9]+/gi, '-').toLowerCase();
    downloadText(`${safe}.batch.json`, serializeBatch(preset));
  }

  function openImport() {
    importError = '';
    fileInput?.click();
  }

  async function onImportFiles(event) {
    importError = '';
    const files = Array.from(event.target.files ?? []);
    for (const file of files) {
      const text = await file.text();
      const result = parseBatch(text);
      if (!result.ok) {
        importError = `${file.name}: ${result.error}`;
        continue;
      }
      addCustomPreset(result.preset);
    }
    event.target.value = '';
  }
</script>

<div class="wizard-head">
  <h2><span class="step">02</span>Run a task batch</h2>
  <span class="hint">
    {ready
      ? `each card runs a prompt-set over ${project.name}`
      : 'select a project to enable batches'}
  </span>
  <div class="tg-actions">
    <button class="btn-mini" onclick={openImport} title="Import batch JSON">
      <Icon name="upload" size={11} />Import
    </button>
    <button class="btn-mini" onclick={exportAll} title="Export every preset">
      <Icon name="download" size={11} />Export all
    </button>
    <input
      bind:this={fileInput}
      type="file"
      accept="application/json,.json"
      multiple
      style="display:none"
      onchange={onImportFiles}
    />
  </div>
</div>

{#if importError}
  <div class="import-err">{importError}</div>
{/if}

<div class="task-grid">
  {#each allPresets as task (task.id)}
    <div class="tc-wrap">
      <button
        class="task-card"
        class:featured={task.featured}
        class:disabled={!ready}
        disabled={!ready}
        onclick={() => run(task)}
        title={ready
          ? `Run ${task.prompts.length} prompts on ${project.name}`
          : 'Select a project first'}
      >
        <div class="tc-icon"><Icon name={task.icon} size={16} /></div>
        <div class="tc-title">
          {task.title}
          {#if task.custom}<span class="tc-badge">custom</span>{/if}
        </div>
        <div class="tc-desc">{task.desc}</div>
        <div class="tc-meta">
          <span>{task.prompts.length} prompts</span>
          <span>{task.strategy}</span>
        </div>
      </button>
      <div class="tc-tools">
        <button
          class="tc-tool"
          title="Export this preset"
          onclick={() => exportOne(task)}
        >
          <Icon name="download" size={11} />
        </button>
        {#if task.custom}
          <button
            class="tc-tool danger"
            title="Remove imported preset"
            onclick={() => removeCustomPreset(task.id)}
          >
            <Icon name="trash" size={11} />
          </button>
        {/if}
      </div>
    </div>
  {/each}
</div>

<style>
  .wizard-head { display: flex; align-items: center; gap: 12px; }
  .tg-actions { margin-left: auto; display: inline-flex; gap: 6px; }
  .btn-mini {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 3px 8px;
    font-size: 11px;
    cursor: pointer;
  }
  .btn-mini:hover { color: var(--text); }
  .import-err {
    margin: 6px 0 10px;
    color: var(--sev-high, #ef4444);
    font-size: 12px;
  }
  .tc-wrap { position: relative; }
  .tc-tools {
    position: absolute;
    top: 6px;
    right: 6px;
    display: none;
    gap: 4px;
  }
  .tc-wrap:hover .tc-tools { display: inline-flex; }
  .tc-tool {
    background: var(--bg-elev);
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 4px;
    cursor: pointer;
  }
  .tc-tool:hover { color: var(--text); }
  .tc-tool.danger:hover { color: var(--sev-high, #ef4444); }
  .tc-badge {
    margin-left: 6px;
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 4px;
  }
</style>
