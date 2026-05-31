<script>
  import Icon from '$lib/Icon.svelte';
  import { store, modulesOf, runModuleBatch } from '$lib/store.svelte.js';
  import { listGitBranches } from '$lib/api.js';
  import { TASK_PRESETS, composePrompts } from '$lib/tasks.js';

  // `inline` — render in-page (no overlay/modal chrome) instead of as a modal.
  let { onClose, inline = false } = $props();

  const presets = $derived([...TASK_PRESETS, ...store.customPresets]);

  // Selection state.
  let selectedPresetIds = $state(new Set());
  let selectedProjectIds = $state(
    new Set(store.selectedId ? [store.selectedId] : []),
  );
  // On a cold start `store.selectedId` is still null while projects load async,
  // so the initial seed above is empty. Seed once it arrives (one-shot, so a
  // user deselecting everything later isn't overridden).
  let seededSelection = $state(selectedProjectIds.size > 0);
  $effect(() => {
    if (!seededSelection && store.selectedId) {
      selectedProjectIds = new Set([store.selectedId]);
      seededSelection = true;
    }
  });
  let globalMode = $state(false);
  // PR/diff mode: prefix every job with `git diff <branch>` from the target.
  let diffMode = $state(false);
  let diffBranch = $state('');
  let branches = $state([]); // local branches of the first selected target
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

  const canQueue = $derived(
    jobs > 0 && targets.length > 0 && promptN > 0 && (!diffMode || !!diffBranch),
  );

  // Load the first selected target's branches when diff mode is enabled.
  $effect(() => {
    const projectId = targets[0]?.id;
    if (!diffMode || !projectId) {
      branches = [];
      return;
    }
    listGitBranches(projectId)
      .then((b) => {
        branches = b;
        if (!b.includes(diffBranch)) diffBranch = b[0] ?? '';
      })
      .catch(() => {
        branches = [];
      });
  });

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
      diffBranch: diffMode && diffBranch ? diffBranch : null,
    });
    onClose?.();
  }
</script>

{#snippet inner()}
    <div class="batch-head">
      <Icon name="layers" size={14} />
      <div>
        <div class="title">New batch</div>
        <div class="sub">{promptN} prompt{promptN !== 1 ? 's' : ''} · {targets.length} target{targets.length !== 1 ? 's' : ''}</div>
      </div>
      {#if !inline}
        <button class="x" onclick={() => onClose?.()}><Icon name="x" size={14} /></button>
      {/if}
    </div>

    <div class="batch-body">
      <!-- Prompts -->
      <div class="batch-section">
        <div class="sec-head">
          <h4>Prompts</h4>
          <span class="hint">{selectedPresets.length} selected · {promptN} total invocations</span>
        </div>
        <div class="preset-grid">
          {#each presets as p (p.id)}
            <button
              class="preset-card"
              class:active={selectedPresetIds.has(p.id)}
              onclick={() => togglePreset(p.id)}
              title={p.desc}
            >
              <div class="pc-icon"><Icon name={p.icon} size={16} /></div>
              {#if selectedPresetIds.has(p.id)}
                <div class="pc-check"><Icon name="check" size={11} /></div>
              {/if}
              <div class="pc-title">
                {p.title}
                {#if p.custom}<span class="pc-badge">custom</span>{/if}
              </div>
              <div class="pc-desc">{p.desc}</div>
              <div class="pc-meta">
                <span>{p.prompts.length} prompt{p.prompts.length === 1 ? '' : 's'}</span>
                <span>{p.strategy}</span>
              </div>
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

      <!-- Diff (PR review) toggle -->
      <button class="global-row" class:on={diffMode} onclick={() => (diffMode = !diffMode)} style="width:100%; text-align:left;">
        <Icon name="branch" size={16} />
        <div class="body">
          <div class="title">Diff mode (PR review)</div>
          <div class="desc">
            {#if diffMode}
              On — each job's prompt is prefixed with <code>git diff {diffBranch || '<branch>'}</code> run in the target.
            {:else}
              Off — agents scan the full tree. Turn on to review only changes vs a base branch.
            {/if}
          </div>
        </div>
        <div class="toggle" class:on={diffMode}></div>
      </button>

      {#if diffMode}
        <div class="batch-section">
          <div class="sec-head">
            <h4>Base branch</h4>
            <span class="hint">
              {targets.length > 0 ? `branches of ${targets[0].name}` : 'select a target first'}
            </span>
          </div>
          {#if branches.length > 0}
            <select bind:value={diffBranch} class="branch-select">
              {#each branches as b (b)}
                <option value={b}>{b}</option>
              {/each}
            </select>
            <span class="hint" style="display:block; margin-top:6px;">
              Will run <code>git diff {diffBranch}</code> per target (scoped to each module's subdir).
            </span>
          {:else}
            <div class="hint" style="padding:6px 0;">
              No branches found{targets.length === 0 ? ' — select a target.' : ' (target may not be a git repo).'}
            </div>
          {/if}
        </div>
      {/if}

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
      {#if !inline}
        <button class="btn" onclick={() => onClose?.()}>Cancel</button>
      {/if}
      <button class="btn-go" onclick={queue} disabled={!canQueue}>
        <Icon name="play" size={11} />Queue {jobs} job{jobs === 1 ? '' : 's'}
      </button>
    </div>
{/snippet}

{#if inline}
  <div class="batch-modal inline">{@render inner()}</div>
{:else}
  <div
    class="batch-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose?.();
    }}
  >
    <div class="batch-modal">{@render inner()}</div>
  </div>
{/if}
