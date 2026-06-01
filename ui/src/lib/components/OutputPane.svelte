<script>
  import Icon from '$lib/Icon.svelte';
  import { formatDuration } from '$lib/time.js';

  // `agents` — live agent output entries from the store.
  let { agents = [] } = $props();

  let pinnedId = $state(null); // tab the user clicked; null = auto-follow
  let showThinking = $state(true);
  let termEl = $state(null);
  let stuck = $state(true); // true while scrolled to the bottom
  let nowTick = $state(Date.now()); // re-ticks once a second while a job runs

  // Running agents first, then most-recently-updated. Capped to the newest
  // MAX_TABS so a 100-job batch can't balloon the DOM / memory.
  const MAX_TABS = 10;
  const sorted = $derived(
    [...agents]
      .sort((a, b) => {
        const ar = a.status === 'running' ? 0 : 1;
        const br = b.status === 'running' ? 0 : 1;
        if (ar !== br) return ar - br;
        return b.updatedAt - a.updatedAt;
      })
      .slice(0, MAX_TABS),
  );

  // The shown agent: the pinned tab if still present, else the top of the list.
  const active = $derived(
    sorted.find((a) => a.jobId === pinnedId) ?? sorted[0] ?? null,
  );

  const lines = $derived(
    active
      ? active.lines.filter((l) => showThinking || l.type !== 'thinking')
      : [],
  );

  const tabLabel = (agent) =>
    agent.projectName ||
    (agent.prompt ? agent.prompt.slice(0, 24) : '') ||
    agent.jobId.slice(0, 8);

  function onScroll() {
    if (!termEl) return;
    stuck = termEl.scrollHeight - termEl.scrollTop - termEl.clientHeight < 40;
  }

  // Elapsed time for the active agent: live for running jobs, frozen for
  // finished ones.
  const elapsedMs = $derived.by(() => {
    if (!active || active.startedAt == null) return null;
    const end =
      active.status === 'running' ? nowTick : (active.endedAt ?? nowTick);
    return end - active.startedAt;
  });

  // Drive the ticker only while the active agent is still running; teardown
  // clears the interval when the job finishes or the tab changes.
  $effect(() => {
    if (!active || active.status !== 'running') return;
    const id = setInterval(() => (nowTick = Date.now()), 1000);
    return () => clearInterval(id);
  });

  // Follow new output to the bottom unless the user scrolled up.
  $effect(() => {
    // Touch reactive deps so the effect re-runs as content streams in —
    // coalesced deltas grow `text` without changing line count, so sum it.
    void active?.jobId;
    let chars = 0;
    for (const line of lines) chars += line.text.length;
    void chars;
    if (stuck && termEl) {
      queueMicrotask(() => {
        if (termEl) termEl.scrollTop = termEl.scrollHeight;
      });
    }
  });
</script>

<div class="outpane">
  {#if sorted.length === 0}
    <div class="empty-state">
      <span class="es-icon"><Icon name="activity" size={22} /></span>
      <span>Agent output streams here once a job or batch runs.</span>
    </div>
  {:else}
    <div class="op-tabs">
      {#each sorted as agent (agent.jobId)}
        <button
          class="op-tab"
          class:active={active && agent.jobId === active.jobId}
          onclick={() => (pinnedId = agent.jobId)}
          title={agent.prompt}
        >
          <span class="op-dot" data-status={agent.status}></span>
          <span class="op-tab-label">{tabLabel(agent)}</span>
        </button>
      {/each}
      <span class="op-tabs-spacer"></span>
      <button
        class="op-toggle"
        class:on={showThinking}
        onclick={() => (showThinking = !showThinking)}
        title="Show or hide agent thinking"
      >
        <span class="op-check">{showThinking ? '✓' : ''}</span>
        thinking
      </button>
    </div>

    {#if active}
      <div class="op-meta">
        <span class="op-status" data-status={active.status}>{active.status}</span>
        <span class="op-prompt">{active.prompt || '—'}</span>
        {#if elapsedMs != null}
          <span
            class="op-timer"
            data-status={active.status}
            title={active.status === 'running' ? 'Running for' : 'Ran for'}
          >
            <Icon name="clock" size={11} />
            {formatDuration(elapsedMs)}
          </span>
        {/if}
      </div>

      <div class="op-term" bind:this={termEl} onscroll={onScroll}>
        {#each lines as line, i (i)}
          {#if line.type === 'thinking'}
            <div class="op-line thinking"><span class="op-gutter">┊</span>{line.text}</div>
          {:else if line.type === 'text'}
            <div class="op-line text">{line.text}</div>
          {:else if line.type === 'tool'}
            <div class="op-line tool">
              <span class="op-gutter">▸</span><span class="op-tool-name">{line.name}</span>
              <span class="op-tool-args">{line.text}</span>
            </div>
          {:else if line.type === 'tool_result'}
            <div class="op-line tool-result" class:err={line.isError}>
              <span class="op-gutter">{line.isError ? '✗' : '◂'}</span>{line.name}
              {line.isError ? 'failed' : 'ok'}
            </div>
          {:else if line.type === 'error'}
            <div class="op-line error"><span class="op-gutter">✗</span>{line.text}</div>
          {:else if line.type === 'turn'}
            <div class="op-line turn">— {line.text} —</div>
          {:else if line.type === 'prompt'}
            <div class="op-line prompt">Prompt: {line.text}</div>
          {/if}
        {:else}
          <div class="op-idle">Waiting for the agent…</div>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .outpane {
    border: 1px solid var(--border);
    background: var(--bg-panel);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .op-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 8px;
    background: var(--bg-elev);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
  }
  .op-tab {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }
  .op-tab:hover {
    background: var(--bg-hover);
  }
  .op-tab.active {
    background: var(--bg-active);
    border-color: var(--border-strong);
    color: var(--text);
  }
  .op-tab-label {
    max-width: 130px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .op-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-dim);
    flex-shrink: 0;
  }
  .op-dot[data-status='running'] {
    background: var(--accent);
    animation: op-pulse 1.4s ease-in-out infinite;
  }
  .op-dot[data-status='completed'] {
    background: var(--success);
  }
  .op-dot[data-status='failed'] {
    background: var(--sev-high);
  }
  @keyframes op-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }

  .op-tabs-spacer {
    flex: 1;
  }
  .op-toggle {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 10px;
    cursor: pointer;
    white-space: nowrap;
  }
  .op-toggle.on {
    color: var(--accent);
    border-color: var(--accent-border);
  }
  .op-check {
    width: 8px;
    display: inline-block;
  }

  .op-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .op-status {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 3px;
    color: var(--text-muted);
    border: 1px solid var(--border);
    flex-shrink: 0;
  }
  .op-status[data-status='running'] {
    color: var(--accent);
    border-color: var(--accent-border);
  }
  .op-status[data-status='completed'] {
    color: var(--success);
    border-color: rgba(34, 197, 94, 0.25);
  }
  .op-status[data-status='failed'] {
    color: var(--sev-high);
    border-color: rgba(239, 68, 68, 0.25);
  }
  .op-prompt {
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .op-timer {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--border);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .op-timer[data-status='running'] {
    color: var(--accent);
    border-color: var(--accent-border);
  }

  .op-term {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.55;
    padding: 12px 14px;
    background: var(--bg);
    max-height: 360px;
    overflow-y: auto;
  }
  .op-line {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .op-line.thinking {
    color: var(--text-dim);
    font-style: italic;
  }
  .op-line.text {
    color: var(--text);
  }
  .op-line.tool {
    color: var(--accent);
  }
  .op-line.tool-result {
    color: var(--text-muted);
  }
  .op-line.tool-result.err {
    color: var(--sev-high);
  }
  .op-line.error {
    color: var(--sev-high);
  }
  .op-line.turn {
    color: var(--text-faint);
    margin: 4px 0;
  }
  .op-line.prompt {
    color: var(--accent);
  }
  .op-gutter {
    display: inline-block;
    width: 1.4em;
    color: var(--text-faint);
  }
  .op-tool-name {
    font-weight: 600;
  }
  .op-tool-args {
    color: var(--text-dim);
    margin-left: 6px;
  }
  .op-idle {
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
