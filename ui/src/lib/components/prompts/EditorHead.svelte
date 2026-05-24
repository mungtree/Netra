<script>
  import PESelect from './PESelect.svelte';
  import { STRATEGIES, STOP_CONDITIONS, BATCH_FORMAT } from '$lib/prompts/promptsData.js';

  let { preset, dirty, readOnly = false, focusSignal = 0, update } = $props();

  /** @type {HTMLInputElement | null} */
  let titleEl = null;

  let lastSignal = focusSignal;
  $effect(() => {
    if (focusSignal !== lastSignal) {
      lastSignal = focusSignal;
      queueMicrotask(() => {
        titleEl?.focus();
        titleEl?.select();
      });
    }
  });

  const cond = $derived(
    STOP_CONDITIONS.find((s) => s.id === preset.stopConditionId) ?? STOP_CONDITIONS[0],
  );

  const strategyOpts = STRATEGIES.map((s) => ({
    id: s.id,
    name: s.name,
    code: s.id,
    desc: s.desc,
  }));
  const stopOpts = STOP_CONDITIONS.map((s) => ({
    id: s.id,
    name: s.name,
    desc: s.desc,
    muted: s.id === 'default',
  }));
</script>

<div class="pe-editor-head">
  <div class="pe-title-row">
    <input
      bind:this={titleEl}
      class="pe-title-input"
      value={preset.title}
      oninput={(e) => update({ title: e.currentTarget.value })}
      placeholder="Untitled batch"
      spellcheck="false"
      disabled={readOnly}
    />
    {#if readOnly}
      <span class="pe-badge">BUILT-IN</span>
    {:else}
      <span class="pe-badge" class:dirty>
        {dirty ? '● UNSAVED' : 'SAVED'}
      </span>
    {/if}
    <span class="pe-badge">{BATCH_FORMAT}</span>
  </div>

  <div class="pe-controls">
    <div class="pe-field">
      <span class="pe-field-label">
        Strategy
        <span class="hint">— how prompts compose into a run</span>
      </span>
      <PESelect
        options={strategyOpts}
        value={preset.strategy}
        onChange={(v) => !readOnly && update({ strategy: v })}
      />
    </div>
    <div class="pe-field">
      <span class="pe-field-label">
        Stop condition
        <span class="hint">— appended to prompts at run time</span>
      </span>
      <PESelect
        options={stopOpts}
        value={preset.stopConditionId}
        onChange={(v) => !readOnly && update({ stopConditionId: v })}
      />
    </div>
  </div>

  <div class="pe-stop-extra">
    {#if cond.id === 'custom'}
      <div class="pe-stop-lbl">Custom halt rule</div>
      <textarea
        class="pe-custom-stop"
        value={preset.customStopText}
        oninput={(e) => update({ customStopText: e.currentTarget.value })}
        placeholder="e.g. When you have nothing more to add, reply with FINISHED on a single line."
        spellcheck="false"
        disabled={readOnly}
      ></textarea>
    {:else if cond.id === 'default'}
      <div class="pe-stop-preview">
        <span class="placeholder">
          No text is appended — the runner uses {preset.strategy}'s built-in halt rule.
        </span>
      </div>
    {:else}
      <div class="pe-stop-preview">{cond.text}</div>
    {/if}

    <div class="pe-stop-options">
      <label class="pe-toggle">
        <input
          type="checkbox"
          checked={preset.appendToAll}
          onchange={(e) => update({ appendToAll: e.currentTarget.checked })}
          disabled={readOnly || cond.id === 'default'}
        />
        <span class="sw"></span>
        <span>Append to every prompt</span>
      </label>
      <span class="pe-stop-summary">
        {cond.id === 'default'
          ? 'disabled — strategy default'
          : preset.appendToAll
            ? `${preset.prompts.length} of ${preset.prompts.length} prompts`
            : `1 of ${preset.prompts.length} prompts (last only)`}
      </span>
    </div>
  </div>
</div>
