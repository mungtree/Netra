<script>
  import FindingCard from './FindingCard.svelte';

  let { report } = $props();

  const KINDS = ['bug', 'vulnerability', 'idea', 'change', 'fix', 'suggestion', 'other'];
  const SEVERITIES = ['critical', 'high', 'medium', 'low', 'info'];
  const SEV_ORDER = Object.fromEntries(SEVERITIES.map((s, i) => [s, i]));

  let kindFilter = $state('all');
  let sevFilter = $state('all');

  const filtered = $derived(
    (report?.findings ?? [])
      .filter((f) => kindFilter === 'all' || f.kind === kindFilter)
      .filter((f) => sevFilter === 'all' || f.severity === sevFilter)
      .slice()
      .sort((a, b) => (SEV_ORDER[a.severity] ?? 99) - (SEV_ORDER[b.severity] ?? 99)),
  );

  const kindCounts = $derived(() => {
    const c = Object.fromEntries(KINDS.map((k) => [k, 0]));
    for (const f of report?.findings ?? []) {
      if (c[f.kind] != null) c[f.kind] += 1;
    }
    return c;
  });
</script>

{#if !report || !report.findings}
  <div class="empty">No structured findings on this batch.</div>
{:else}
  {#if report.summary}
    <div class="summary">
      <div class="summary-label">Summary</div>
      <p>{report.summary}</p>
    </div>
  {/if}

  <div class="filters">
    <label>
      Kind
      <select bind:value={kindFilter}>
        <option value="all">all ({report.findings.length})</option>
        {#each KINDS as k}
          <option value={k}>{k} ({kindCounts()[k]})</option>
        {/each}
      </select>
    </label>
    <label>
      Severity
      <select bind:value={sevFilter}>
        <option value="all">all</option>
        {#each SEVERITIES as s}
          <option value={s}>{s}</option>
        {/each}
      </select>
    </label>
    <span class="count">{filtered.length} shown</span>
  </div>

  <div class="list">
    {#each filtered as finding, i (i)}
      <FindingCard {finding} />
    {:else}
      <div class="empty">Nothing matches the current filters.</div>
    {/each}
  </div>
{/if}

<style>
  .summary {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-elev);
    padding: 12px 14px;
    margin-bottom: 14px;
  }
  .summary-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    margin-bottom: 4px;
  }
  .summary p { margin: 0; font-size: 13px; line-height: 1.55; white-space: pre-wrap; }
  .filters {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 10px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .filters label { display: inline-flex; align-items: center; gap: 6px; }
  .filters select {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 3px 6px;
    font-size: 12px;
  }
  .count { margin-left: auto; font-family: var(--font-mono); }
  .empty {
    color: var(--text-dim);
    padding: 16px;
    text-align: center;
    border: 1px dashed var(--border);
    border-radius: 6px;
  }
</style>
