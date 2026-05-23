<script>
  import Icon from '$lib/Icon.svelte';
  import { SEV_SHORT, KIND_LABEL, highlightInlineCode } from '$lib/reviewFormat.js';

  let { finding } = $props();

  const sevShort = $derived(SEV_SHORT[finding.severity] || 'info');
  const kindLabel = $derived(
    KIND_LABEL[finding.kind] || (finding.kind ?? 'other').toUpperCase(),
  );
  const fixHtml = $derived(
    finding.suggested_fix ? highlightInlineCode(finding.suggested_fix) : '',
  );
</script>

<div class="finding-card {sevShort}">
  <div class="fc-head">
    <div class="fc-badges">
      <span class="sev {sevShort}">{(finding.severity ?? 'info').toUpperCase()}</span>
      <span class="kind-badge">{kindLabel}</span>
    </div>
    <div class="fc-title-wrap">
      <div class="fc-title">{finding.title}</div>
      <div class="fc-loc" class:no-loc={!finding.location}>
        <Icon name="folder" size={11} />
        {#if finding.location}
          <span class="file">{finding.location}</span>
        {:else}
          <span>no location reported</span>
        {/if}
      </div>
    </div>
  </div>
  <div class="fc-body">
    <div class="fc-desc">{finding.description}</div>
    {#if finding.suggested_fix}
      <div class="fc-fix">
        <div class="label"><Icon name="wand" size={11} />Suggested fix</div>
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        <div class="text">{@html fixHtml}</div>
      </div>
    {/if}
    {#if finding.tags && finding.tags.length}
      <div class="fc-tags">
        {#each finding.tags as t (t)}<span class="fc-tag">{t}</span>{/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .finding-card {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    margin-bottom: 10px;
    overflow: hidden;
    position: relative;
  }
  .finding-card::before {
    content: '';
    position: absolute; left: 0; top: 0; bottom: 0;
    width: 3px;
  }
  .finding-card.crit::before { background: var(--sev-crit); }
  .finding-card.high::before { background: var(--sev-high); }
  .finding-card.med::before  { background: var(--sev-med); }
  .finding-card.low::before  { background: var(--sev-low); }
  .finding-card.info::before { background: var(--sev-info); }

  .fc-head {
    padding: 12px 16px 8px;
    display: flex; align-items: flex-start; gap: 12px;
  }
  .fc-badges {
    display: flex; flex-direction: column; gap: 5px;
    align-items: flex-start;
    flex-shrink: 0;
    width: 86px;
  }
  .sev {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.1em;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    width: fit-content;
  }
  .sev.crit { color: var(--sev-crit); background: var(--sev-crit-bg); border: 1px solid var(--sev-crit-bd); }
  .sev.high { color: var(--sev-high); background: rgba(239,68,68,0.10); border: 1px solid rgba(239,68,68,0.25); }
  .sev.med  { color: var(--sev-med);  background: rgba(245,158,11,0.10); border: 1px solid rgba(245,158,11,0.25); }
  .sev.low  { color: var(--sev-low);  background: rgba(59,130,246,0.10); border: 1px solid rgba(59,130,246,0.25); }
  .sev.info { color: var(--sev-info); background: rgba(107,114,128,0.10); border: 1px solid rgba(107,114,128,0.25); }
  .kind-badge {
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 2px 6px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    border-radius: 3px;
  }
  .fc-title-wrap { flex: 1; min-width: 0; }
  .fc-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
    margin-bottom: 5px;
    line-height: 1.35;
  }
  .fc-loc {
    font-family: var(--font-mono);
    font-size: 11.5px;
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--text-muted);
  }
  .fc-loc .file { color: var(--accent-soft); word-break: break-all; }
  .fc-loc.no-loc { color: var(--text-dim); font-style: italic; }

  .fc-body { padding: 0 16px 14px; }
  .fc-desc {
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.6;
    margin-bottom: 12px;
    text-wrap: pretty;
    white-space: pre-wrap;
  }
  .fc-fix {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 10px 12px;
    margin-bottom: 12px;
  }
  .fc-fix .label {
    display: inline-flex; align-items: center; gap: 6px;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--success);
    margin-bottom: 6px;
  }
  .fc-fix .text {
    font-size: 12.5px;
    color: var(--text);
    line-height: 1.55;
    font-family: var(--font-mono);
    white-space: pre-wrap;
  }
  .fc-fix .text :global(code) {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
    color: var(--accent-soft);
    font-size: 11.5px;
  }
  .fc-tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .fc-tag {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-muted);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 7px;
  }
  .fc-tag::before { content: '#'; color: var(--text-dim); margin-right: 1px; }
</style>
