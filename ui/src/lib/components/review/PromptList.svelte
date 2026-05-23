<script>
  import { detectOutputType, formatDuration, formatTokens } from '$lib/reviewFormat.js';

  let { prompts, activeId, onPick } = $props();
</script>

<div class="prompt-list-pane">
  <div class="plp-head">
    <span class="t">Prompts</span>
    <span class="c">{prompts.length}</span>
    <div class="plp-head-spacer"></div>
  </div>
  <div class="plp-list">
    {#each prompts as p, i (p.id)}
      {@const type = detectOutputType(p.output)}
      {@const count = type === 'structured' ? (p.output.findings?.length ?? 0) : 0}
      {@const isText = type === 'text'}
      <button
        type="button"
        class="prompt-item"
        class:active={p.id === activeId}
        class:reviewer={p.isReviewer}
        onclick={() => onPick(p.id)}
      >
        <div class="pi-num">{p.isReviewer ? '★' : String(i).padStart(2, '0')}</div>
        <div class="pi-body">
          <div class="pi-name">{p.name}</div>
          <div class="pi-meta">
            <span>{formatDuration(p.startedAt, p.finishedAt) || '—'}</span>
            <span>·</span>
            <span>{formatTokens(p.tokens) || '—'}</span>
            <span>·</span>
            {#if p.status === 'running' || p.status === 'queued'}
              <span class="findings-badge running">{p.status}</span>
            {:else if p.status === 'failed'}
              <span class="findings-badge fail">failed</span>
            {:else if isText}
              <span class="findings-badge text">text</span>
            {:else}
              <span class="findings-badge" class:zero={count === 0} class:some={count > 0}>
                {count === 0 ? 'clean' : `${count} finding${count === 1 ? '' : 's'}`}
              </span>
            {/if}
          </div>
        </div>
      </button>
    {/each}
  </div>
</div>

<style>
  .prompt-list-pane {
    width: 300px;
    flex-shrink: 0;
    border-right: 1px solid var(--border);
    background: var(--bg-panel);
    display: flex; flex-direction: column;
    min-height: 0;
  }
  .plp-head {
    padding: 12px 14px 8px;
    display: flex; align-items: center;
    border-bottom: 1px solid var(--border);
  }
  .plp-head .t {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .plp-head .c {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-dim);
    margin-left: 6px;
  }
  .plp-head-spacer { flex: 1; }
  .plp-list { flex: 1; overflow-y: auto; padding: 4px 0; }

  .prompt-item {
    width: 100%;
    padding: 10px 14px;
    cursor: pointer;
    border-left: 2px solid transparent;
    border-top: none; border-right: none; border-bottom: none;
    background: transparent;
    color: inherit;
    display: flex; align-items: flex-start; gap: 10px;
    text-align: left;
  }
  .prompt-item:hover { background: var(--bg-elev); }
  .prompt-item.active {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .prompt-item.reviewer {
    background: linear-gradient(90deg, var(--accent-bg) 0%, transparent 80%);
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .prompt-item.reviewer .pi-num {
    color: var(--accent);
    font-size: 14px;
    line-height: 1;
  }
  .prompt-item.reviewer .pi-name {
    font-weight: 600;
    color: var(--text);
  }
  .pi-num {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-dim);
    margin-top: 2px;
    width: 18px;
    flex-shrink: 0;
  }
  .prompt-item.active .pi-num { color: var(--accent); }
  .pi-body { flex: 1; min-width: 0; }
  .pi-name {
    font-size: 12.5px;
    color: var(--text);
    line-height: 1.35;
    margin-bottom: 4px;
    word-break: break-word;
  }
  .pi-meta {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-dim);
    display: flex; gap: 10px; align-items: center;
  }
  .findings-badge {
    display: inline-flex; align-items: center;
    padding: 1px 5px;
    border-radius: 2px;
    font-weight: 600;
  }
  .findings-badge.zero { color: var(--success); background: rgba(34,197,94,0.10); }
  .findings-badge.some { color: var(--accent); background: var(--accent-bg); }
  .findings-badge.text { color: var(--text-muted); background: var(--bg-elev); }
  .findings-badge.running { color: var(--accent); background: var(--accent-bg); }
  .findings-badge.fail { color: var(--sev-high); background: rgba(239,68,68,0.10); }
</style>
