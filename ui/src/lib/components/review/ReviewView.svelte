<script>
  import { onMount } from 'svelte';

  import RecentRunsSidebar from './RecentRunsSidebar.svelte';
  import ReviewHeader from './ReviewHeader.svelte';
  import PromptList from './PromptList.svelte';
  import ResultPane from './ResultPane.svelte';

  import {
    store,
    loadBatchDetail,
    selectBatch,
    rerunBatch,
  } from '$lib/store.svelte.js';

  // Default selection: pick the newest batch on first entry.
  $effect(() => {
    if (!store.selectedBatchId && store.batches.length > 0) {
      selectBatch(store.batches[0].id);
    } else if (store.selectedBatchId && !store.batchDetails[store.selectedBatchId]) {
      loadBatchDetail(store.selectedBatchId);
    }
  });

  const selectedBatch = $derived(
    store.batches.find((b) => b.id === store.selectedBatchId) ?? null,
  );

  const detail = $derived(
    store.selectedBatchId ? (store.batchDetails[store.selectedBatchId] ?? null) : null,
  );

  /** Synthetic row representing the aggregated reviewer output. */
  const reviewerRow = $derived.by(() => {
    if (!selectedBatch) return null;
    const result = selectedBatch.result;
    const status = selectedBatch.status;
    const usage = result?.usage;
    const tokens = usage ? (usage.input_tokens || 0) + (usage.output_tokens || 0) : 0;
    let output = '';
    if (result?.structured && Array.isArray(result.structured.findings)) {
      output = result.structured;
    } else if (result?.summary) {
      output = result.summary;
    }
    return {
      id: '__reviewer__',
      name: `Reviewer · ${selectedBatch.aggregation?.strategy ?? 'aggregate'}`,
      status,
      startedAt: selectedBatch.created_at,
      finishedAt: selectedBatch.updated_at,
      // Batches don't carry their own started/finished timestamps; the
      // created→updated span is the closest stand-in for the reviewer row.
      tokens,
      output,
      outputMissing: !result,
      isReviewer: true,
    };
  });

  const promptRows = $derived(
    (detail?.items ?? []).map((item) => {
      const job = item.job_id ? detail.jobs[item.job_id] : null;
      const output = job?.output;
      const usage = output?.usage;
      const tokens = usage ? (usage.input_tokens || 0) + (usage.output_tokens || 0) : 0;
      let promptOutput;
      if (!job) promptOutput = '';
      else if (output?.structured && Array.isArray(output.structured.findings)) {
        promptOutput = output.structured;
      } else {
        promptOutput = output?.text ?? '';
      }
      return {
        id: item.id,
        name: item.prompt_name,
        status: job?.status ?? 'queued',
        // Prefer the authoritative run timestamps; fall back to the broader
        // created→updated window for jobs persisted before those fields existed.
        startedAt: job?.started_at ?? job?.created_at,
        finishedAt: job?.finished_at ?? job?.updated_at,
        queuedAt: job?.created_at,
        tokens,
        output: promptOutput,
        outputMissing: !job?.output,
      };
    }),
  );

  /** Reviewer summary first, then per-prompt rows. */
  const prompts = $derived(
    reviewerRow ? [reviewerRow, ...promptRows] : promptRows,
  );

  let activePromptId = $state(null);

  // Reset / default the active prompt when the batch changes.
  $effect(() => {
    void store.selectedBatchId;
    activePromptId = null;
  });
  $effect(() => {
    if (activePromptId && prompts.some((p) => p.id === activePromptId)) return;
    // Reviewer row is always prompts[0] when present — surface it first.
    activePromptId = prompts[0]?.id ?? null;
  });

  const activePrompt = $derived(
    prompts.find((p) => p.id === activePromptId) ?? null,
  );

  function projectName(batch) {
    const projectId = batch?.targets?.[0]?.project_id;
    return store.projects.find((p) => p.id === projectId)?.name ?? '—';
  }

  /**
   * Effective wall-clock span of the batch: earliest job start → latest job
   * finish. Falls back to batch.created_at / updated_at when the new fields
   * aren't populated (older jobs or none started yet).
   */
  const batchSpan = $derived.by(() => {
    const jobs = Object.values(detail?.jobs ?? {});
    if (jobs.length === 0) return null;
    let minStart = Infinity;
    let maxFinish = -Infinity;
    for (const j of jobs) {
      const s = j.started_at ? Date.parse(j.started_at) : NaN;
      const f = j.finished_at ? Date.parse(j.finished_at) : NaN;
      if (Number.isFinite(s) && s < minStart) minStart = s;
      if (Number.isFinite(f) && f > maxFinish) maxFinish = f;
    }
    const startedAt = Number.isFinite(minStart)
      ? new Date(minStart).toISOString()
      : (selectedBatch?.created_at ?? null);
    const finishedAt = Number.isFinite(maxFinish)
      ? new Date(maxFinish).toISOString()
      : (selectedBatch?.updated_at ?? null);
    return { startedAt, finishedAt };
  });

  // Pull the model from the first job in the batch, if available.
  const modelLabel = $derived(() => {
    const job = Object.values(detail?.jobs ?? {})[0];
    const ref = job?.model;
    if (!ref) return '';
    if (typeof ref === 'string') return ref;
    return ref.name ?? ref.id ?? '';
  });

  onMount(() => {
    if (store.selectedBatchId) loadBatchDetail(store.selectedBatchId);
  });
</script>

<RecentRunsSidebar
  batches={store.batches}
  selectedId={store.selectedBatchId}
  {projectName}
  onPick={selectBatch}
/>

<div class="main">
  <ReviewHeader
    batch={selectedBatch}
    projectName={selectedBatch ? projectName(selectedBatch) : '—'}
    model={modelLabel()}
    span={batchSpan}
    onRerun={rerunBatch}
  />
  <div class="review-body">
    {#if selectedBatch}
      <PromptList
        {prompts}
        activeId={activePromptId}
        onPick={(id) => (activePromptId = id)}
      />
      <ResultPane prompt={activePrompt} />
    {:else}
      <div class="empty">Select a run from the sidebar.</div>
    {/if}
  </div>
</div>

<style>
  .main {
    flex: 1;
    min-width: 0;
    background: var(--bg);
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .review-body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }
  .empty {
    flex: 1;
    display: flex; align-items: center; justify-content: center;
    color: var(--text-dim);
    font-size: 13px;
  }
</style>
