<script>
  import Icon from '$lib/Icon.svelte';

  let { finding } = $props();

  const sevColor = $derived(
    {
      critical: 'sev-critical',
      high: 'sev-high',
      medium: 'sev-medium',
      low: 'sev-low',
      info: 'sev-info',
    }[finding.severity] ?? 'sev-info',
  );

  const kindIcon = $derived(
    {
      bug: 'bug',
      vulnerability: 'shield',
      idea: 'bulb',
      change: 'wand',
      fix: 'check',
      suggestion: 'sparkle',
      other: 'dot',
    }[finding.kind] ?? 'dot',
  );

  let expanded = $state(false);
</script>

<div class="fc" class:expanded>
  <button class="fc-head" onclick={() => (expanded = !expanded)}>
    <span class="fc-kind"><Icon name={kindIcon} size={13} /></span>
    <span class="fc-sev {sevColor}">{finding.severity}</span>
    <span class="fc-title">{finding.title}</span>
    {#if finding.location}
      <code class="fc-loc">{finding.location}</code>
    {/if}
    <span class="fc-chevron" class:open={expanded}>
      <Icon name="chevron" size={12} />
    </span>
  </button>

  {#if expanded}
    <div class="fc-body">
      <p class="fc-desc">{finding.description}</p>
      {#if finding.suggested_fix}
        <div class="fc-fix">
          <div class="fc-fix-label">Suggested fix</div>
          <pre>{finding.suggested_fix}</pre>
        </div>
      {/if}
      {#if finding.tags && finding.tags.length}
        <div class="fc-tags">
          {#each finding.tags as tag}<span class="fc-tag">{tag}</span>{/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .fc {
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-bottom: 8px;
    background: var(--bg-elev);
  }
  .fc-head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    background: transparent;
    border: none;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }
  .fc-head:hover { background: var(--bg-hover); }
  .fc-kind { color: var(--text-dim); display: inline-flex; }
  .fc-sev {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid currentColor;
  }
  .sev-critical { color: #ef4444; }
  .sev-high     { color: #f97316; }
  .sev-medium   { color: #eab308; }
  .sev-low      { color: #3b82f6; }
  .sev-info     { color: var(--text-dim); }
  .fc-title { flex: 1; font-size: 13px; }
  .fc-loc {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    background: var(--bg);
    padding: 2px 6px;
    border-radius: 3px;
  }
  .fc-chevron { color: var(--text-dim); transition: transform 120ms; }
  .fc-chevron.open { transform: rotate(90deg); }
  .fc-body {
    padding: 4px 14px 12px 14px;
    border-top: 1px solid var(--border);
    font-size: 12.5px;
    line-height: 1.55;
  }
  .fc-desc { margin: 8px 0 10px 0; white-space: pre-wrap; }
  .fc-fix-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
    margin-bottom: 4px;
  }
  .fc-fix pre {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    white-space: pre-wrap;
    background: var(--bg);
    padding: 8px 10px;
    border-radius: 4px;
  }
  .fc-tags { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 8px; }
  .fc-tag {
    font-size: 10.5px;
    color: var(--text-dim);
    border: 1px solid var(--border);
    padding: 1px 6px;
    border-radius: 999px;
  }
</style>
