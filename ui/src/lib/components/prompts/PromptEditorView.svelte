<script>
  import { onMount } from 'svelte';

  import PresetRail from './PresetRail.svelte';
  import EditorHead from './EditorHead.svelte';
  import EditorToolbar from './EditorToolbar.svelte';
  import PromptsList from './PromptsList.svelte';
  import SchemaPanel from './SchemaPanel.svelte';
  import PreviewRail from './PreviewRail.svelte';
  import Icon from '$lib/Icon.svelte';

  import { TASK_PRESETS } from '$lib/tasks.js';
  import { normalizePreset } from '$lib/prompts/promptsData.js';
  import { parseBatch } from '$lib/batchIo.js';
  import {
    store,
    addCustomPreset,
    removeCustomPreset,
    updateCustomPreset,
    createBlankPreset,
    duplicatePreset,
  } from '$lib/store.svelte.js';

  // Built-in presets are read-only; map them through normalizePreset so the
  // editor receives the same shape it gets for custom ones.
  const builtinPresets = $derived(
    TASK_PRESETS.map((t) => normalizePreset({ ...t, builtin: true })),
  );

  const allPresets = $derived([...builtinPresets, ...store.customPresets]);

  let activeId = $state(allPresets[0]?.id ?? null);

  const active = $derived(
    allPresets.find((p) => p.id === activeId) ?? allPresets[0] ?? null,
  );

  // Snapshot of the last saved version of the active preset (custom only).
  // Built-ins are always "saved" since they're code-defined.
  let savedSnapshot = $state(active ? JSON.stringify(active) : '');
  const dirty = $derived(
    !!active && !active.builtin && JSON.stringify(active) !== savedSnapshot,
  );

  let toast = $state(/** @type {{msg: string, err: boolean} | null} */ (null));
  let focusSignal = $state(0);
  /** @type {ReturnType<typeof setTimeout> | null} */
  let toastTimer = null;

  function showToast(msg, err = false) {
    toast = { msg, err };
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 2200);
  }

  function pick(id) {
    activeId = id;
    const next = allPresets.find((p) => p.id === id);
    savedSnapshot = next ? JSON.stringify(next) : '';
  }

  // Re-sync snapshot when the active id changes (e.g. after a custom preset
  // mutation reorders the list and `active` becomes a different object).
  $effect(() => {
    if (active && active.id === activeId && !savedSnapshotMatches()) {
      // No-op — savedSnapshot is only re-armed via pick / save.
    }
  });
  function savedSnapshotMatches() {
    return active && JSON.stringify(active) === savedSnapshot;
  }

  function update(patch) {
    if (!active || active.builtin) return;
    const updated = updateCustomPreset(active.id, patch);
    if (updated) {
      // Live edits ride on the persisted store, so to keep dirty meaningful
      // we leave savedSnapshot alone — user must hit Cmd-S to "save".
    }
  }

  function save() {
    if (!active || active.builtin) return;
    savedSnapshot = JSON.stringify(active);
    showToast('Saved');
  }

  function newBatch() {
    const fresh = createBlankPreset();
    activeId = fresh.id;
    savedSnapshot = JSON.stringify(fresh);
    focusSignal += 1;
    showToast('New batch created');
  }

  function duplicate() {
    if (!active) return;
    const dup = duplicatePreset(active);
    activeId = dup.id;
    savedSnapshot = JSON.stringify(dup);
    showToast('Duplicated');
  }

  function remove() {
    if (!active || active.builtin) return;
    const idx = store.customPresets.findIndex((p) => p.id === active.id);
    removeCustomPreset(active.id);
    const remaining = [...builtinPresets, ...store.customPresets];
    const fallback =
      remaining[Math.min(builtinPresets.length + Math.max(0, idx - 1), remaining.length - 1)] ??
      remaining[0] ??
      null;
    activeId = fallback?.id ?? null;
    savedSnapshot = fallback ? JSON.stringify(fallback) : '';
    showToast('Deleted batch');
  }

  /** @type {HTMLInputElement | null} */
  let fileInput = null;

  function openImport() {
    fileInput?.click();
  }

  async function onImportFiles(event) {
    const files = Array.from(event.target.files ?? []);
    for (const file of files) {
      const text = await file.text();
      const result = parseBatch(text);
      if (!result.ok) {
        showToast(`${file.name}: ${result.error}`, true);
        continue;
      }
      const added = addCustomPreset(result.preset);
      activeId = added.id;
      savedSnapshot = JSON.stringify(added);
      showToast(`Imported "${added.title}"`);
    }
    event.target.value = '';
  }

  function onKey(e) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      save();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<div class="pe-layout">
  <PresetRail
    presets={allPresets}
    activeId={active?.id ?? null}
    onPick={pick}
    onNew={newBatch}
    onImport={openImport}
  />

  {#if active}
    <div class="pe-editor">
      <EditorHead
        preset={active}
        {dirty}
        {focusSignal}
        readOnly={active.builtin}
        update={update}
      />
      <EditorToolbar
        preset={active}
        onDuplicate={duplicate}
        onDelete={remove}
      />
      <div class="pe-body">
        <PromptsList preset={active} readOnly={active.builtin} update={update} />
        <SchemaPanel preset={active} readOnly={active.builtin} update={update} />
      </div>
    </div>

    <PreviewRail preset={active} onToast={showToast} />
  {:else}
    <div class="pe-empty">No batches — click + to create one.</div>
  {/if}

  <input
    bind:this={fileInput}
    type="file"
    accept="application/json,.json"
    multiple
    style="display:none"
    onchange={onImportFiles}
  />

  {#if toast}
    <div class="pe-toast" class:err={toast.err}>
      <Icon name={toast.err ? 'x' : 'check'} size={13} />
      {toast.msg}
    </div>
  {/if}
</div>
