<script>
  import Icon from '$lib/Icon.svelte';
  import { store, modulesOf, runModuleBatch } from '$lib/store.svelte.js';
  import { TASK_PRESETS, composePrompts } from '$lib/tasks.js';

  let { onClose } = $props();

  const presets = $derived([...TASK_PRESETS, ...store.customPresets]);

  // Selection state.
  let selectedPresetIds = $state(new Set());
  let selectedProjectIds = $state(
    new Set(store.selectedId ? [store.selectedId] : []),
  );
  let globalMode = $state(false);
  // Per-project excluded module ids (default: nothing excluded = all selected).
  let excluded = $state({}); // { [projectId]: Set<moduleId> }

  const selectedPresets = $derived(
    presets.filter((p) => selectedPresetIds.has(p.id)),
  );
  const targets = $derived(
    store.projects.filter((p) => selectedProjectIds.has(p.id)),
  );

  // Fully-composed prompt bodies across every selected preset.
  const composed = $derived(
    selectedPresets.flatMap((p) => composePrompts(p, store.settings.stopRules)),
  );
  const promptN = $derived(composed.length);

  function modulesFor(projectId) {
    return modulesOf(projectId);
  }
  function selectedModuleIds(projectId) {
    const ex = excluded[projectId] ?? new Set();
    return modulesFor(projectId)
      .map((m) => m.id)
      .filter((id) => !ex.has(id));
  }
  const modN = $derived(
    targets.reduce((n, t) => n + selectedModuleIds(t.id).length, 0),
  );
  const jobs = $derived(
    globalMode
      ? promptN * targets.length
      : targets.reduce((n, t) => n + promptN * selectedModuleIds(t.id).length, 0),
  );

  function togglePreset(id) {
    const next = new Set(selectedPresetIds);
    next.has(id) ? next.delete(id) : next.add(id);
    selectedPresetIds = next;
  }
  function toggleProject(id) {
    const next = new Set(selectedProjectIds);
    next.has(id) ? next.delete(id) : next.add(id);
    selectedProjectIds = next;
  }
  function toggleModule(projectId, moduleId) {
    const ex = new Set(excluded[projectId] ?? []);
    ex.has(moduleId) ? ex.delete(moduleId) : ex.add(moduleId);
    excluded = { ...excluded, [projectId]: ex };
  }
  const moduleActive = (projectId, moduleId) =>
    !(excluded[projectId] ?? new Set()).has(moduleId);

  const canQueue = $derived(jobs > 0 && targets.length > 0 && promptN > 0);

  async function queue() {
    if (!canQueue) return;
    const projectIds = targets.map((t) => t.id);
    const name = selectedPresets.map((p) => p.title).join(' + ') || 'Batch';
    const strategy = selectedPresets[0]?.strategy ?? 'concat';
    // [] for a target means "all modules" on the backend.
    const targetModules = globalMode
      ? null
      : projectIds.map((id) => {
          const sel = selectedModuleIds(id);
          return sel.length === modulesFor(id).length ? [] : sel;
        });
    await runModuleBatch({
      name,
      prompts: composed,
      projectIds,
      strategy,
      global: globalMode,
      targetModules,
    });
    onClose?.();
  }
</script>

<div
  class="batch-overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose?.();
  }}
>
  <div class="batch-modal">
    <div class="batch-head">
      <Icon name="layers" size={14} />
      <div>
        <div class="title">New batch</div>
        <div class="sub">{promptN} prompt{promptN !== 1 ? 's' : ''} · {targets.length} target{targets.length !== 1 ? 's' : ''}</div>
      </div>
      <button class="x" onclick={() => onClose?.()}><Icon name="x" size={14} /></button>
    </div>

    <div class="batch-body">
      <!-- Prompts -->
      <div class="batch-section">
        <div class="sec-head">
          <h4>Prompts</h4>
          <span class="hint">{selectedPresets.length} selected · {promptN} total invocations</span>
        </div>
        <div style="display:flex; gap:6px; flex-wrap:wrap;">
          {#each presets as p (p.id)}
            <button
              class="modchip sel"
              class:active={selectedPresetIds.has(p.id)}
              onclick={() => togglePreset(p.id)}
            >
              {p.title} <span style="color:var(--text-dim)">· {p.prompts.length}</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Targets -->
      <div class="batch-section">
        <div class="sec-head">
          <h4>Targets</h4>
          <span class="hint">{targets.length} project{targets.length !== 1 ? 's' : ''} selected</span>
        </div>
        <div style="display:flex; gap:6px; flex-wrap:wrap;">
          {#each store.projects as t (t.id)}
            <button
              class="modchip sel"
              class:active={selectedProjectIds.has(t.id)}
              onclick={() => toggleProject(t.id)}
            >
              {t.name} <span style="color:var(--text-dim)">· {modulesFor(t.id).length} mod</span>
            </button>
          {/each}
        </div>
      </div>

      <!-- Global toggle -->
      <button class="global-row" class:on={globalMode} onclick={() => (globalMode = !globalMode)} style="width:100%; text-align:left;">
        <Icon name="package" size={16} />
        <div class="body">
          <div class="title">Global (skip modules)</div>
          <div class="desc">
            {#if globalMode}
              On — each target is scanned in full. Fanout collapses to <code>prompts × targets</code>.
            {:else}
              Off — each agent gets a single module's scope. Fanout is <code>prompts × targets × modules</code>.
            {/if}
          </div>
        </div>
        <div class="toggle" class:on={globalMode}></div>
      </button>

      <!-- Per-target module picker -->
      <div class="batch-section" class:disabled={globalMode}>
        <div class="sec-head">
          <h4>Per-target modules</h4>
          <span class="hint">
            {globalMode ? 'ignored while Global is on' : 'tap a chip to exclude · default = all modules selected'}
          </span>
          {#if globalMode}<span class="disabled-tag">SKIPPED</span>{/if}
        </div>
        {#each targets as t (t.id)}
          {@const mods = modulesFor(t.id)}
          {@const sel = selectedModuleIds(t.id)}
          <div class="target-row">
            <span class="stat-dot"></span>
            <div class="pname">
              <span>{t.name}</span>
              <span class="path">{t.root_path}</span>
            </div>
            <div class="modchips" style="display:flex; gap:5px; flex-wrap:wrap;">
              {#each mods as m (m.id)}
                <button
                  class="modchip sel"
                  class:active={moduleActive(t.id, m.id)}
                  onclick={() => toggleModule(t.id, m.id)}
                  disabled={globalMode}
                >
                  {m.name}
                  {#if moduleActive(t.id, m.id)}<Icon name="x" size={9} />{/if}
                </button>
              {/each}
            </div>
            <div class="modcount-mini">
              <b>{sel.length}</b><span style="color:var(--text-dim)">/{mods.length}</span>
              <div style="font-size:9px; color:var(--text-dim)">selected</div>
            </div>
          </div>
        {:else}
          <div class="hint" style="padding:8px 0;">Select at least one target.</div>
        {/each}
      </div>
    </div>

    <div class="batch-foot">
      <div class="job-preview">
        <span class="n">{jobs}</span>jobs will be queued
        <div class="formula">
          = <b>{promptN}</b> prompts × <b>{targets.length}</b> {targets.length === 1 ? 'project' : 'projects'}
          {#if !globalMode}× <b>{modN}</b> module{modN !== 1 ? 's' : ''}{/if}
        </div>
      </div>
      <div class="spacer"></div>
      <button class="btn" onclick={() => onClose?.()}>Cancel</button>
      <button class="btn-go" onclick={queue} disabled={!canQueue}>
        <Icon name="play" size={11} />Queue {jobs} job{jobs === 1 ? '' : 's'}
      </button>
    </div>
  </div>
</div>
