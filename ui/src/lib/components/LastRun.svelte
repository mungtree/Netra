<script>
  import Icon from '$lib/Icon.svelte';
  import { openReview } from '$lib/store.svelte.js';

  // `batch` — the most recent batch, or null when none have run yet.
  let { batch = null } = $props();

  const promptCount = $derived(batch ? batch.prompts.length : 0);
  const targetCount = $derived(batch ? batch.targets.length : 0);
  const itemCount = $derived(promptCount * targetCount);

  const tokens = $derived(() => {
    const u = batch?.result?.usage;
    if (!u) return null;
    return u.input_tokens + u.output_tokens;
  });

  // True when the reduce step produced a structured findings report —
  // surfaces the "view findings" link.
  const hasFindings = $derived(
    batch?.aggregation?.strategy === 'structured_reviewer' &&
      !!batch?.result?.structured?.findings,
  );
</script>

{#if !batch}
  <div class="empty-state">
    <span class="es-icon"><Icon name="inbox" size={22} /></span>
    <span>Run a task batch above — the aggregated result appears here.</span>
  </div>
{:else}
  <div class="run-block">
    <div class="run-block-head">
      <span class="rb-title">
        <span class="ic"><Icon name="library" size={14} /></span>
        {batch.name}
        <span class="lr-status" data-status={batch.status}>{batch.status}</span>
      </span>
      <span class="rb-meta">
        <span><span class="k">prompts</span>{promptCount}</span>
        <span><span class="k">items</span>{itemCount}</span>
        <span><span class="k">reduce</span>{batch.aggregation.strategy}</span>
        {#if tokens()}
          <span><span class="k">tokens</span>{tokens()}</span>
        {/if}
      </span>
    </div>

    <div class="lr-body">
      {#if batch.status === 'completed' && batch.result}
        <div class="lr-sub">
          Consolidated from {batch.result.source_count} output{batch.result
            .source_count === 1
            ? ''
            : 's'}
          {#if hasFindings}
            <button class="lr-link" type="button" onclick={() => openReview(batch.id)}>
              View findings →
            </button>
          {/if}
        </div>
        <pre class="lr-summary">{batch.result.summary}</pre>
      {:else if batch.status === 'failed'}
        <div class="lr-note err">This batch failed to produce a result.</div>
      {:else if batch.status === 'cancelled'}
        <div class="lr-note">This batch was cancelled.</div>
      {:else}
        <div class="lr-note">
          Running {itemCount} job{itemCount === 1 ? '' : 's'} — results aggregate on
          completion.
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .lr-status {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 3px;
    color: var(--text-muted);
    border: 1px solid var(--border);
  }
  .lr-status[data-status='running'] {
    color: var(--accent);
    border-color: var(--accent-border);
  }
  .lr-status[data-status='completed'] {
    color: var(--sev-low);
    border-color: rgba(59, 130, 246, 0.25);
  }
  .lr-status[data-status='failed'] {
    color: var(--sev-high);
    border-color: rgba(239, 68, 68, 0.25);
  }
  .lr-body {
    padding: 14px 16px;
  }
  .lr-sub {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    margin-bottom: 10px;
  }
  .lr-summary {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 340px;
    overflow-y: auto;
  }
  .lr-note {
    font-size: 12px;
    color: var(--text-muted);
  }
  .lr-note.err {
    color: var(--sev-high);
  }
  .lr-link {
    margin-left: 10px;
    color: var(--accent, #3b82f6);
    text-decoration: none;
    font-size: 11px;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    font-family: inherit;
  }
  .lr-link:hover { text-decoration: underline; }
</style>
