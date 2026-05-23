<script>
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { getBatch } from '$lib/api.js';
  import FindingsList from '$lib/components/FindingsList.svelte';
  import Icon from '$lib/Icon.svelte';

  const batchId = $derived(page.params.batchId);

  let batch = $state(null);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      batch = await getBatch(batchId);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  const report = $derived(batch?.result?.structured ?? null);
  const isStructured = $derived(
    batch?.aggregation?.strategy === 'structured_reviewer',
  );
</script>

<div class="findings-page">
  <header class="fp-head">
    <a class="back" href="/">
      <Icon name="chevron" size={12} />
      Back
    </a>
    <h1>{batch?.name ?? 'Findings'}</h1>
    {#if batch}
      <span class="fp-meta">
        <span class="k">strategy</span>{batch.aggregation.strategy}
        <span class="k">status</span>{batch.status}
      </span>
    {/if}
    <button class="refresh" onclick={load} title="Refresh">
      <Icon name="refresh" size={14} />
    </button>
  </header>

  {#if loading}
    <div class="state">Loading batch…</div>
  {:else if error}
    <div class="state err">Failed to load: {error}</div>
  {:else if !isStructured}
    <div class="state">
      This batch used the <code>{batch?.aggregation?.strategy}</code> strategy,
      which does not produce structured findings. Re-run with the
      <code>structured_reviewer</code> strategy to see a typed report here.
    </div>
    {#if batch?.result?.summary}
      <pre class="raw">{batch.result.summary}</pre>
    {/if}
  {:else if !report}
    <div class="state">
      No structured report available yet — the batch may still be running, or
      the reviewer agent returned non-JSON output.
    </div>
    {#if batch?.result?.summary}
      <pre class="raw">{batch.result.summary}</pre>
    {/if}
  {:else}
    <FindingsList {report} />
  {/if}
</div>

<style>
  .findings-page {
    max-width: 1000px;
    margin: 0 auto;
    padding: 22px 24px;
  }
  .fp-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }
  .fp-head h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-dim);
    text-decoration: none;
    font-size: 12px;
  }
  .back :global(svg) { transform: rotate(180deg); }
  .back:hover { color: var(--text); }
  .fp-meta {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    display: inline-flex;
    gap: 10px;
  }
  .fp-meta .k {
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-right: 4px;
  }
  .refresh {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 4px;
    padding: 4px 6px;
    cursor: pointer;
  }
  .refresh:hover { color: var(--text); }
  .state {
    color: var(--text-dim);
    border: 1px dashed var(--border);
    border-radius: 6px;
    padding: 18px;
    margin-bottom: 12px;
    font-size: 13px;
    line-height: 1.55;
  }
  .state.err { color: var(--sev-high, #ef4444); border-color: rgba(239,68,68,0.35); }
  .raw {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--bg-elev);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    white-space: pre-wrap;
  }
</style>
