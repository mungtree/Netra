<script>
  import Icon from '$lib/Icon.svelte';

  let { current = [], proposal = [], onApply, onCancel } = $props();

  const sub = (m) => String(m.root_subdir ?? '');

  // Reconcile proposal against current, matching by module name (ids are fresh
  // on every inference run, so they can't be used to align rows).
  const rows = $derived.by(() => {
    const curByName = new Map(current.map((m) => [m.name, m]));
    const propByName = new Map(proposal.map((m) => [m.name, m]));
    const out = [];
    for (const p of proposal) {
      const c = curByName.get(p.name);
      if (!c) out.push({ kind: 'added', after: p });
      else if (c.description !== p.description || sub(c) !== sub(p))
        out.push({ kind: 'changed', before: c, after: p });
      else out.push({ kind: 'kept', before: c, after: p });
    }
    for (const c of current) {
      if (!propByName.has(c.name)) out.push({ kind: 'removed', before: c });
    }
    return out;
  });

  // Per-row selection. added/changed default on; removed default off (keep);
  // kept rows have no toggle. Re-seed whenever the row set changes.
  let selected = $state([]);
  $effect(() => {
    selected = rows.map((r) => r.kind === 'added' || r.kind === 'changed');
  });

  const counts = $derived(
    rows.reduce((acc, r) => {
      acc[r.kind] = (acc[r.kind] || 0) + 1;
      return acc;
    }, {}),
  );
  const accepted = $derived(
    rows.filter((r, i) => selected[i] && r.kind !== 'kept').length,
  );

  function marker(kind) {
    return { added: '+', removed: '−', changed: '~' }[kind] ?? '·';
  }

  function toggle(i) {
    if (rows[i].kind === 'kept') return;
    selected[i] = !selected[i];
  }
  const selectAll = () => (selected = rows.map((r) => r.kind !== 'kept'));
  const rejectAll = () => (selected = rows.map(() => false));

  // Build the final module list honoring each row's selection.
  function apply() {
    const out = [];
    rows.forEach((r, i) => {
      const on = selected[i];
      switch (r.kind) {
        case 'kept':
          out.push(r.before);
          break;
        case 'changed':
          out.push(on ? r.after : r.before);
          break;
        case 'added':
          if (on) out.push(r.after);
          break;
        case 'removed':
          if (!on) out.push(r.before); // unselected = keep the module
          break;
      }
    });
    onApply?.(out);
  }
</script>

<div class="infer-head">
  <span class="ic"><Icon name="sparkles" size={14} /></span>
  <span class="title">Proposal</span>
  <span class="by">{proposal.length} module{proposal.length !== 1 ? 's' : ''} proposed</span>
  <div class="legend">
    <span><span class="dot add"></span>added</span>
    <span><span class="dot chg"></span>changed</span>
    <span><span class="dot rem"></span>removed</span>
    <span><span class="dot kep"></span>kept</span>
  </div>
</div>

<div class="infer-toolbar">
  <span class="count">
    <span class="accepted">{accepted}</span> of
    {(counts.added || 0) + (counts.changed || 0) + (counts.removed || 0)} changes selected
    <span class="sep">·</span>{counts.added || 0} new
    <span class="sep">·</span>{counts.changed || 0} changed
    <span class="sep">·</span>{counts.removed || 0} removed
    <span class="sep">·</span>{counts.kept || 0} unchanged
  </span>
  <div class="spacer"></div>
  <button class="btn-mini" onclick={rejectAll}>Reject all</button>
  <button class="btn-mini" onclick={selectAll}>Select all</button>
  <button class="btn-accept-all" onclick={apply}>
    <Icon name="check" size={11} /> Apply
  </button>
</div>

<div class="diff-list">
  {#each rows as r, i (i)}
    {@const data = r.kind === 'removed' ? r.before : r.after}
    {@const before = r.before ?? {}}
    {@const after = r.after ?? {}}
    <div class="diff-row {r.kind}">
      <button
        class="check"
        class:checked={selected[i] && r.kind !== 'removed'}
        onclick={() => toggle(i)}
        aria-label="toggle"
      >
        {#if selected[i] && r.kind !== 'removed'}<Icon name="check" size={10} />{/if}
      </button>
      <span class="marker">{marker(r.kind)}</span>
      <div class="name">
        {#if r.kind === 'changed' && before.name !== after.name}
          <div class="diff-change">
            <span class="from txt">{before.name}</span>
            <span class="to txt mono">{after.name}</span>
          </div>
        {:else}
          <span class="mono">{data.name}</span>
        {/if}
      </div>
      <div class="desc">
        {#if r.kind === 'changed' && before.description !== after.description}
          <div class="diff-change">
            <span class="from txt">{before.description}</span>
            <span class="to txt">{after.description}</span>
          </div>
        {:else}
          <span>{data.description}</span>
        {/if}
      </div>
      <div class="path">
        {#if r.kind === 'changed' && sub(before) !== sub(after)}
          <div class="diff-change">
            <span class="from">{sub(before) || '.'}</span>
            <span class="to">{sub(after) || '.'}</span>
          </div>
        {:else}
          <span>{sub(data) || '.'}</span>
        {/if}
      </div>
      <div class="row-actions">
        {#if r.kind !== 'kept'}
          <button
            class="btn-icon"
            title={r.kind === 'removed' ? 'Keep this module' : 'Reject'}
            onclick={() => toggle(i)}
          >
            <Icon name={r.kind === 'removed' ? 'refresh' : 'x'} size={11} />
          </button>
        {/if}
      </div>
    </div>
  {/each}
</div>

<div style="margin-top:12px; display:flex; gap:10px;">
  <button class="btn" onclick={() => onCancel?.()}>Cancel</button>
  <span style="font-size:11px; color:var(--text-dim); font-family:var(--font-mono); align-self:center;">
    tip · removed modules are detached, not deleted from disk
  </span>
</div>
