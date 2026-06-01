<script>
  import Icon from '$lib/Icon.svelte';
  import { store, modulesOf, inferModules } from '$lib/store.svelte.js';

  let filter = $state('');

  function statusOf(projectId) {
    const jobs = store.jobs.filter((j) => j.project_id === projectId);
    if (jobs.some((j) => j.status === 'running')) return 'run';
    if (jobs.some((j) => j.status === 'failed')) return 'err';
    if (jobs.some((j) => j.status === 'completed')) return 'done';
    return 'idle';
  }

  const rows = $derived(
    store.projects.map((p) => ({
      ...p,
      status: statusOf(p.id),
      modules: modulesOf(p.id),
    })),
  );

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.modules.some((m) => m.name.toLowerCase().includes(q)),
    );
  });

  const moduleTotal = $derived(rows.reduce((n, p) => n + p.modules.length, 0));

  function openModules(p) {
    store.selectedId = p.id;
    store.activeView = 'modules';
  }
  function inferFor(p) {
    store.selectedId = p.id;
    store.activeView = 'modules';
    inferModules(p.id);
  }
  const isDefault = (p) => p.modules.length === 1 && /^root$/i.test(p.modules[0].name);
</script>

<div class="main">
  <div class="po-head">
    <div>
      <h2>Projects &amp; Modules</h2>
      <div class="sub">
        All projects connected to Netra. Click any project to manage its
        modules in the Modules tab.
      </div>
    </div>
    <div class="actions">
      <div class="po-search">
        <Icon name="search" size={12} />
        <input placeholder="Filter projects or modules…" bind:value={filter} />
      </div>
    </div>
  </div>

  <div class="po-body">
    <div class="po-stats">
      <div class="po-stat">
        <div class="label">Projects</div>
        <div class="val">{rows.length}<span class="unit">connected</span></div>
      </div>
      <div class="po-stat">
        <div class="label">Modules</div>
        <div class="val accent">{moduleTotal}<span class="unit">across repos</span></div>
      </div>
      <div class="po-stat">
        <div class="label">Avg scope</div>
        <div class="val">
          {rows.length ? (moduleTotal / rows.length).toFixed(1) : 0}
          <span class="unit">modules / project</span>
        </div>
      </div>
      <div class="po-stat">
        <div class="label">Multi-module</div>
        <div class="val">{rows.filter((p) => !isDefault(p)).length}<span class="unit">split</span></div>
      </div>
    </div>

    <div class="po-section-head">
      <h3>All projects</h3>
      <span class="hint">{filtered.length} shown</span>
    </div>

    <div class="po-table">
      {#each filtered as p (p.id)}
        <div class="po-row">
          <span class="stat-dot {p.status}"></span>
          <button
            class="pname"
            style="text-align:left; background:none; border:none; cursor:pointer;"
            onclick={() => openModules(p)}
          >
            <span>{p.name}</span>
            <span class="ppath">{p.root_path}</span>
          </button>
          <div class="stack"></div>
          <div class="modcount">
            <span class="label">MODULES</span>
            <span style="color:{p.modules.length > 1 ? 'var(--accent)' : 'var(--text-muted)'}">
              {p.modules.length}
            </span>
            {#if isDefault(p)}
              <span style="font-size:10px; color:var(--text-dim); margin-left:6px; font-family:var(--font-mono)">default</span>
            {/if}
          </div>
          <div class="modchips">
            {#each p.modules.slice(0, 6) as m (m.id)}
              <span class="modchip" class:default={isDefault(p)}>{m.name}</span>
            {/each}
            {#if p.modules.length > 6}
              <span class="modchip muted">+{p.modules.length - 6}</span>
            {/if}
          </div>
          <div class="row-actions">
            <button class="btn-icon" title="Open modules" onclick={() => openModules(p)}>
              <Icon name="layers" size={12} />
            </button>
            <button class="btn-icon" title="Infer with AI" onclick={() => inferFor(p)}>
              <Icon name="sparkles" size={12} />
            </button>
          </div>
        </div>
      {:else}
        <div class="po-row"><div class="pname"><span>No projects yet.</span></div></div>
      {/each}
    </div>

    <div style="margin-top:14px; font-size:11px; color:var(--text-dim); font-family:var(--font-mono)">
      tip · projects with one default module fan out as prompts × targets;
      adding more modules splits each job into smaller, focused contexts.
    </div>
  </div>
</div>
