<script>
  import ScopePill from './ScopePill.svelte';
  import ModuleListHeader from './ModuleListHeader.svelte';
  import ModRow from './ModRow.svelte';
  import EmptyHint from './EmptyHint.svelte';
  import InferLoading from './InferLoading.svelte';
  import InferDiff from './InferDiff.svelte';
  import Icon from '$lib/Icon.svelte';
  import {
    store,
    modulesOf,
    saveModules,
    inferModules,
    cancelInfer,
    exportModulesToFile,
    importModulesFromFile,
  } from '$lib/store.svelte.js';

  const project = $derived(
    store.projects.find((p) => p.id === store.selectedId) ?? null,
  );
  const modules = $derived(project ? modulesOf(project.id) : []);
  const editor = $derived(store.modulesEditor);

  // A project is "default-only" when it has a single whole-repo root module.
  const isDefaultOnly = $derived(
    modules.length === 1 && /^root$/i.test(modules[0].name),
  );

  // Inline-edit / add state (id of the module being edited, or 'new').
  let editingId = $state(null);

  // Reference shown in the "Import / Export JSON format" disclosure. Mirrors the
  // `netra.modules/v1` shape that modules_from_json / modules_to_json use.
  const schemaExample = `{
  "format": "netra.modules/v1",
  "modules": [
    {
      "name": "Authentication Module",
      "description": "Handles user login, session management, and JWT validation.",
      "root_subdir": "src/auth"
    },
    {
      "name": "Billing Module",
      "description": "Processes subscription payments and manages invoicing records.",
      "root_subdir": "src/billing"
    }
  ]
}`;

  function freshId() {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) return crypto.randomUUID();
    return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  }

  function startAdd() {
    editingId = 'new';
  }

  async function saveNew(mod) {
    await saveModules(project.id, [...modules, { ...mod, id: freshId() }]);
    editingId = null;
  }

  async function saveEdit(mod) {
    await saveModules(
      project.id,
      modules.map((m) => (m.id === mod.id ? { ...mod } : m)),
    );
    editingId = null;
  }

  async function duplicate(mod) {
    await saveModules(project.id, [
      ...modules,
      { ...mod, id: freshId(), name: `${mod.name}-copy` },
    ]);
  }

  async function remove(mod) {
    await saveModules(
      project.id,
      modules.filter((m) => m.id !== mod.id),
    );
  }

  // Overlap detection: a module whose subdir is a path-parent of another's.
  function norm(p) {
    return String(p ?? '').replace(/^\.?\/*/, '').replace(/\/+$/, '');
  }
  function warnFor(mod) {
    const a = norm(mod.root_subdir);
    const children = modules.filter((m) => {
      if (m.id === mod.id) return false;
      const b = norm(m.root_subdir);
      return a !== '' && b !== a && (b === a || b.startsWith(`${a}/`));
    });
    if (a !== '' && children.length > 0) {
      const names = children.map((c) => `<code>${c.name}</code>`).join(', ');
      return {
        kind: 'warn',
        body: `<b>Overlaps with ${names}.</b> <em>This subdir is a parent scope — jobs will scan the same files multiple times.</em>`,
      };
    }
    return null;
  }
</script>

{#if !project}
  <div class="settings-pane">
    <div class="settings-head">
      <div><h2>Modules</h2><div class="sub">Select a project in the sidebar to manage its modules.</div></div>
    </div>
  </div>
{:else if editor.paneState === 'inferLoading'}
  <div class="settings-pane">
    <div class="settings-head">
      <div>
        <h2>Modules · Inferring…</h2>
        <div class="sub">
          Netra is scanning the project layout. Your current modules stay
          untouched — you'll review a proposal before anything saves.
        </div>
      </div>
      <div class="settings-head-actions"><ScopePill {project} /></div>
    </div>
    <div class="settings-body">
      <InferLoading onCancel={cancelInfer} />
    </div>
  </div>
{:else if editor.paneState === 'inferDiff'}
  <div class="settings-pane">
    <div class="settings-head">
      <div>
        <h2>Modules · Review proposal</h2>
        <div class="sub">
          Cherry-pick the rows you want — nothing saves until you click
          <b style="color:var(--text)">Apply</b>.
        </div>
      </div>
      <div class="settings-head-actions"><ScopePill {project} /></div>
    </div>
    <div class="settings-body">
      <InferDiff
        current={modules}
        proposal={editor.proposal ?? []}
        onApply={(mods) => saveModules(project.id, mods)}
        onCancel={cancelInfer}
      />
    </div>
  </div>
{:else}
  <div class="settings-pane">
    <div class="settings-head">
      <div>
        <h2>Modules</h2>
        <div class="sub">
          Split a project into named subdirectory scopes so each agent gets a
          tighter context window. Fanout = prompts × targets × modules.
        </div>
      </div>
      <div class="settings-head-actions"><ScopePill {project} /></div>
    </div>
    <div class="settings-body">
      {#if editor.error}
        <div class="callout-error">
          <span class="ic"><Icon name="info" size={14} /></span>
          <div><b>Could not infer modules.</b> {editor.error}</div>
        </div>
      {/if}

      <ModuleListHeader
        count={modules.length}
        onAdd={startAdd}
        onInfer={() => inferModules(project.id)}
        onExport={() => exportModulesToFile(project.id)}
        onImport={() => importModulesFromFile(project.id)}
      />

      <div class="mod-list">
        {#each modules as mod (mod.id)}
          <ModRow
            {mod}
            isDefault={isDefaultOnly && /^root$/i.test(mod.name)}
            editing={editingId === mod.id}
            warn={warnFor(mod)}
            onEdit={() => (editingId = mod.id)}
            onDuplicate={() => duplicate(mod)}
            onDelete={() => remove(mod)}
            onSave={saveEdit}
            onCancel={() => (editingId = null)}
          />
        {/each}

        {#if editingId === 'new'}
          <ModRow
            mod={{ id: 'new', name: '', description: '', root_subdir: '' }}
            editing
            isNew
            onSave={saveNew}
            onCancel={() => (editingId = null)}
          />
        {:else}
          <button class="mod-add-row" onclick={startAdd}>
            <span class="ic"><Icon name="plus" size={13} /></span>
            <span>Add module… <span class="kbd" style="margin-left:6px">⌘N</span></span>
          </button>
        {/if}
      </div>

      {#if isDefaultOnly}
        <EmptyHint onAdd={startAdd} onInfer={() => inferModules(project.id)} />
      {/if}

      <details class="schema-ref">
        <summary>
          <Icon name="info" size={13} />
          Import / Export JSON format
        </summary>
        <p class="schema-ref-note">
          Import expects a <code>netra.modules/v1</code> file. Each module needs
          a <code>name</code>, a <code>description</code>, and a
          <code>root_subdir</code> (a path relative to the repo root;
          <code>""</code> means the whole repo). Module ids are assigned on
          import, so leave them out. Entries whose <code>root_subdir</code> does
          not exist are skipped.
        </p>
        <pre class="schema-ref-code">{schemaExample}</pre>
      </details>
    </div>
  </div>
{/if}
