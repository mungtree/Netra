<script>
  import Icon from '$lib/Icon.svelte';
  import {
    countSeverities,
    formatDuration,
    formatTokens,
  } from '$lib/reviewFormat.js';

  let { batch, projectName, model, onRerun } = $props();

  const findings = $derived(batch?.result?.structured?.findings ?? []);
  const severities = $derived(countSeverities(findings));
  const sevs = $derived(
    [
      { k: 'crit', l: 'critical', v: severities.critical },
      { k: 'high', l: 'high', v: severities.high },
      { k: 'med', l: 'medium', v: severities.medium },
      { k: 'low', l: 'low', v: severities.low },
      { k: 'info', l: 'info', v: severities.info },
    ].filter((s) => s.v > 0),
  );

  const tokens = $derived(() => {
    const u = batch?.result?.usage;
    if (!u) return null;
    return formatTokens((u.input_tokens || 0) + (u.output_tokens || 0));
  });

  const duration = $derived(
    batch ? formatDuration(batch.created_at, batch.updated_at) : '',
  );

  const idShort = $derived(batch ? batch.id.slice(0, 8) : '');
  const itemCount = $derived(
    batch ? batch.prompts.length * batch.targets.length : 0,
  );
</script>

<div class="review-header">
  <div class="rh-left">
    <div class="rh-title">
      <span class="ic"><Icon name="library" size={16} /></span>
      {batch?.name ?? 'Run'}
      <span class="id">{idShort}</span>
    </div>
    <div class="rh-meta">
      <span><span class="k">project</span><span class="v">{projectName}</span></span>
      <span><span class="k">strategy</span><span class="v">{batch?.aggregation?.strategy ?? '—'}</span></span>
      <span><span class="k">status</span><span class="v">{batch?.status ?? '—'}</span></span>
      {#if duration}
        <span><span class="k">duration</span><span class="v">{duration}</span></span>
      {/if}
      <span><span class="k">items</span><span class="v">{itemCount}</span></span>
      {#if tokens()}
        <span><span class="k">tokens</span><span class="v">{tokens()}</span></span>
      {/if}
      {#if model}
        <span><span class="k">model</span><span class="v">{model}</span></span>
      {/if}
    </div>
    {#if sevs.length}
      <div class="sev-summary">
        {#each sevs as s (s.k)}
          <span class="sev-chip {s.k}">
            <span class="ct">{s.v}</span>
            {s.l}
          </span>
        {/each}
      </div>
    {/if}
  </div>
  <div class="rh-actions">
    <button class="btn ghost" onclick={() => onRerun(batch.id)} disabled={!batch}>
      <Icon name="refresh" size={13} />Re-run
    </button>
  </div>
</div>

<style>
  .review-header {
    padding: 16px 22px 14px;
    border-bottom: 1px solid var(--border);
    display: flex; align-items: flex-start; gap: 20px;
  }
  .rh-left { flex: 1; min-width: 0; }
  .rh-title {
    display: flex; align-items: center; gap: 10px;
    font-size: 18px;
    font-weight: 500;
    color: var(--text);
    letter-spacing: -0.015em;
    margin-bottom: 6px;
  }
  .rh-title .ic {
    width: 28px; height: 28px;
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--accent-bg);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-sm);
    color: var(--accent);
  }
  .rh-title .id {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    border: 1px solid var(--border);
    background: var(--bg-elev);
    padding: 2px 7px;
    border-radius: 3px;
    margin-left: 4px;
  }
  .rh-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    display: flex; flex-wrap: wrap; gap: 18px;
  }
  .rh-meta .k { color: var(--text-dim); margin-right: 6px; }
  .rh-meta .v { color: var(--text); }

  .rh-actions {
    display: flex; align-items: center; gap: 6px;
    flex-shrink: 0;
  }

  .sev-summary {
    display: flex; gap: 8px; flex-wrap: wrap;
    margin-top: 10px;
  }
  .sev-chip {
    display: inline-flex; align-items: center; gap: 6px;
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.06em;
    padding: 3px 8px;
    border-radius: 3px;
    border: 1px solid;
  }
  .sev-chip .ct { font-weight: 600; }
  .sev-chip.crit { color: var(--sev-crit); border-color: var(--sev-crit-bd); background: var(--sev-crit-bg); }
  .sev-chip.high { color: var(--sev-high); border-color: rgba(239,68,68,0.25); background: rgba(239,68,68,0.08); }
  .sev-chip.med  { color: var(--sev-med);  border-color: rgba(245,158,11,0.25); background: rgba(245,158,11,0.08); }
  .sev-chip.low  { color: var(--sev-low);  border-color: rgba(59,130,246,0.25); background: rgba(59,130,246,0.08); }
  .sev-chip.info { color: var(--sev-info); border-color: rgba(107,114,128,0.25); background: rgba(107,114,128,0.08); }
</style>
