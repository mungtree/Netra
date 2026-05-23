<script>
  import Icon from '$lib/Icon.svelte';
  import FindingCard from './FindingCard.svelte';
  import {
    detectOutputType,
    renderMarkdown,
    highlightJson,
    formatDuration,
    formatTokens,
    countSeverities,
  } from '$lib/reviewFormat.js';

  let { prompt } = $props();

  let tab = $state('default');
  let filter = $state('all');

  $effect(() => {
    // Reset tabs/filters whenever the selected prompt changes.
    void prompt?.id;
    tab = 'default';
    filter = 'all';
  });

  const type = $derived(prompt ? detectOutputType(prompt.output) : 'text');
  const isText = $derived(type === 'text');
  const findings = $derived(
    !isText && prompt ? (prompt.output.findings ?? []) : [],
  );
  const sevCounts = $derived(countSeverities(findings));
  const visible = $derived(
    filter === 'all' ? findings : findings.filter((f) => f.severity === filter),
  );
  const showRaw = $derived(tab === 'raw');
  const markdownHtml = $derived(
    isText && typeof prompt?.output === 'string' ? renderMarkdown(prompt.output) : '',
  );
  const jsonHtml = $derived(
    !isText && prompt ? highlightJson(prompt.output) : '',
  );
</script>

{#if !prompt}
  <div class="result-pane empty">
    <p>Select a prompt to view its output.</p>
  </div>
{:else}
  <div class="result-pane">
    <div class="result-pane-head">
      <h3>{prompt.name}</h3>
      <div class="rph-meta">
        <span><span class="k">duration</span>{formatDuration(prompt.startedAt, prompt.finishedAt) || '—'}</span>
        <span><span class="k">tokens</span>{formatTokens(prompt.tokens) || '—'}</span>
        {#if !isText}
          <span><span class="k">findings</span>{findings.length}</span>
          <span class="type-chip structured"><Icon name="check" size={9} />structured</span>
        {:else}
          <span class="type-chip text"><Icon name="folder" size={9} />plaintext</span>
        {/if}
      </div>
      <div class="rph-spacer"></div>
      <div class="tabs">
        <button class="tab" class:active={!showRaw} onclick={() => (tab = 'default')}>
          {isText ? 'Rendered' : 'Structured'}
        </button>
        <button class="tab" class:active={showRaw} onclick={() => (tab = 'raw')}>
          {isText ? 'Raw' : 'Raw JSON'}
        </button>
      </div>
    </div>

    {#if prompt.status === 'queued' || prompt.status === 'running'}
      <div class="state">This prompt is still {prompt.status}…</div>
    {:else if prompt.status === 'failed' && !prompt.output}
      <div class="state err">This prompt failed to produce output.</div>
    {:else if isText}
      {#if prompt.outputMissing}
        <div class="state">No output recorded.</div>
      {:else}
        <div class="schema-warn">
          <span class="ic"><Icon name="bulb" size={14} /></span>
          <div class="body">
            <div class="t">Unstructured output</div>
            <div class="s">This prompt did not return JSON matching the findings schema. Displaying as plaintext — finding filters and counts are unavailable.</div>
          </div>
        </div>
        {#if showRaw}
          <pre class="raw-json plain">{prompt.output}</pre>
        {:else}
          <!-- eslint-disable-next-line svelte/no-at-html-tags -->
          <div class="text-output">{@html markdownHtml}</div>
        {/if}
      {/if}
    {:else if showRaw}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      <pre class="raw-json">{@html jsonHtml}</pre>
    {:else}
      {#if prompt.output.summary}
        <div class="summary-card">
          <span class="sc-ic"><Icon name="sparkle" size={16} /></span>
          <div class="sc-body">
            <div class="sc-label">Summary</div>
            <div class="sc-text">{prompt.output.summary}</div>
          </div>
        </div>
      {/if}

      {#if findings.length > 0}
        <div class="filter-row">
          <button
            class="filter-chip"
            class:active={filter === 'all'}
            onclick={() => (filter = 'all')}
          >
            All <span class="ct">{findings.length}</span>
          </button>
          {#each ['critical', 'high', 'medium', 'low', 'info'] as sev (sev)}
            {#if sevCounts[sev] > 0}
              <button
                class="filter-chip"
                class:active={filter === sev}
                onclick={() => (filter = sev)}
              >
                {sev} <span class="ct">{sevCounts[sev]}</span>
              </button>
            {/if}
          {/each}
          <div class="grow"></div>
          <span class="filter-chip static">
            showing <span class="ct">{visible.length}</span>
          </span>
        </div>

        <div class="findings-section-head">
          <h4>Findings</h4>
          <span class="count">{visible.length} of {findings.length}</span>
        </div>

        {#each visible as f, i (i)}
          <FindingCard finding={f} />
        {/each}
      {:else}
        <div class="no-findings">
          <div class="ic"><Icon name="check" size={20} /></div>
          <div class="t">No findings reported</div>
          <div class="s">This prompt completed cleanly — nothing actionable surfaced.</div>
        </div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .result-pane {
    flex: 1;
    overflow-y: auto;
    padding: 22px 24px 32px;
    background: var(--bg);
    min-width: 0;
  }
  .result-pane.empty {
    display: flex; align-items: center; justify-content: center;
    color: var(--text-dim);
  }
  .result-pane-head {
    display: flex; align-items: center; gap: 12px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }
  .result-pane-head h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }
  .rph-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    display: flex; gap: 14px; align-items: center;
  }
  .rph-meta .k { color: var(--text-dim); margin-right: 5px; }
  .rph-spacer { flex: 1; }

  .tabs {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-elev);
    padding: 2px;
  }
  .tabs .tab {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 3px;
    color: var(--text-muted);
  }
  .tabs .tab:hover { color: var(--text); }
  .tabs .tab.active {
    background: var(--bg);
    color: var(--accent);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .type-chip {
    display: inline-flex; align-items: center; gap: 5px;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 2px 7px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    color: var(--text-muted);
  }
  .type-chip.structured { color: var(--accent); border-color: var(--accent-border); background: var(--accent-bg); }

  .summary-card {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius);
    padding: 14px 16px;
    margin-bottom: 18px;
    display: flex; gap: 12px;
  }
  .summary-card .sc-ic { color: var(--accent); margin-top: 1px; flex-shrink: 0; }
  .summary-card .sc-body { flex: 1; min-width: 0; }
  .summary-card .sc-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: 6px;
  }
  .summary-card .sc-text {
    font-size: 13.5px;
    color: var(--text);
    line-height: 1.55;
    white-space: pre-wrap;
  }

  .filter-row {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }
  .filter-chip {
    display: inline-flex; align-items: center; gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    padding: 4px 9px;
    border: 1px solid var(--border);
    background: var(--bg-elev);
    border-radius: 3px;
    cursor: pointer;
  }
  .filter-chip:hover { color: var(--text); border-color: var(--border-strong); }
  .filter-chip.active { color: var(--accent); border-color: var(--accent-border); background: var(--accent-bg); }
  .filter-chip.static { cursor: default; }
  .filter-chip .ct { color: var(--text-dim); }
  .filter-chip.active .ct { color: var(--accent); }
  .grow { flex: 1; }

  .findings-section-head {
    display: flex; align-items: baseline; justify-content: space-between;
    margin-bottom: 10px;
  }
  .findings-section-head h4 {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .findings-section-head .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
  }

  .no-findings {
    background: var(--bg-panel);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    padding: 28px;
    text-align: center;
  }
  .no-findings .ic {
    display: inline-flex; align-items: center; justify-content: center;
    width: 40px; height: 40px;
    border-radius: 50%;
    background: rgba(34,197,94,0.12);
    color: var(--success);
    margin-bottom: 10px;
  }
  .no-findings .t { font-size: 13.5px; color: var(--text); margin-bottom: 4px; }
  .no-findings .s { font-size: 12px; color: var(--text-muted); }

  .raw-json {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    padding: 16px 18px;
    white-space: pre;
    overflow-x: auto;
    color: var(--text-muted);
  }
  .raw-json.plain { white-space: pre-wrap; }
  .raw-json :global(.k) { color: var(--accent); }
  .raw-json :global(.s) { color: #a6e3a1; }
  .raw-json :global(.n) { color: #89b4fa; }
  .raw-json :global(.b) { color: #f5c2e7; }
  .raw-json :global(.nl) { color: var(--text-dim); font-style: italic; }

  .text-output {
    background: var(--bg-panel);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 18px 22px 22px;
    color: var(--text);
    font-size: 13.5px;
    line-height: 1.65;
  }
  .text-output :global(> *:first-child) { margin-top: 0; }
  .text-output :global(> *:last-child) { margin-bottom: 0; }
  .text-output :global(h1),
  .text-output :global(h2),
  .text-output :global(h3),
  .text-output :global(h4) {
    color: var(--text);
    font-weight: 500;
    letter-spacing: -0.01em;
    margin: 22px 0 10px;
    line-height: 1.3;
  }
  .text-output :global(h1) { font-size: 18px; padding-bottom: 8px; border-bottom: 1px solid var(--border); }
  .text-output :global(h2) { font-size: 15px; }
  .text-output :global(h3) {
    font-size: 13px;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--accent);
  }
  .text-output :global(h4) { font-size: 12px; color: var(--text-muted); }
  .text-output :global(p) { margin: 0 0 12px; color: var(--text); }
  .text-output :global(ul),
  .text-output :global(ol) { margin: 0 0 14px; padding-left: 22px; }
  .text-output :global(li) { margin-bottom: 6px; color: var(--text); }
  .text-output :global(li::marker) { color: var(--accent); }
  .text-output :global(strong) { color: var(--text); font-weight: 600; }
  .text-output :global(em) { color: var(--accent-soft); font-style: italic; }
  .text-output :global(a) { color: var(--accent); text-decoration: none; }
  .text-output :global(a:hover) { text-decoration: underline; }
  .text-output :global(code) {
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent-soft);
  }
  .text-output :global(pre) {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
    margin: 0 0 14px;
    overflow-x: auto;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
    color: var(--text);
  }
  .text-output :global(pre code) {
    background: none; border: none; padding: 0; color: inherit; font-size: inherit;
  }
  .text-output :global(blockquote) {
    margin: 0 0 14px;
    padding: 10px 14px;
    border-left: 2px solid var(--accent);
    background: var(--accent-bg);
    border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
    color: var(--text-muted);
    font-style: italic;
  }
  .text-output :global(hr) {
    border: none; border-top: 1px solid var(--border); margin: 18px 0;
  }

  .schema-warn {
    display: flex; gap: 10px; align-items: flex-start;
    background: rgba(245,158,11,0.06);
    border: 1px solid rgba(245,158,11,0.20);
    border-left: 3px solid var(--sev-med);
    border-radius: var(--radius);
    padding: 11px 14px;
    margin-bottom: 14px;
    font-size: 12px;
    color: var(--text);
    line-height: 1.55;
  }
  .schema-warn .ic { color: var(--sev-med); margin-top: 1px; flex-shrink: 0; }
  .schema-warn .body { flex: 1; }
  .schema-warn .body .t { font-weight: 500; margin-bottom: 2px; }
  .schema-warn .body .s { color: var(--text-muted); }

  .state {
    color: var(--text-dim);
    border: 1px dashed var(--border);
    border-radius: 6px;
    padding: 18px;
    margin-bottom: 12px;
    font-size: 13px;
  }
  .state.err { color: var(--sev-high); border-color: rgba(239,68,68,0.35); }
</style>
