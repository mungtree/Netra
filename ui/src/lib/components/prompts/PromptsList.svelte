<script>
  import Icon from '$lib/Icon.svelte';
  import { STOP_CONDITIONS } from '$lib/prompts/promptsData.js';

  let { preset, readOnly = false, update } = $props();

  const cond = $derived(
    STOP_CONDITIONS.find((s) => s.id === preset.stopConditionId) ?? STOP_CONDITIONS[0],
  );
  const hasStop = $derived(
    cond.id !== 'default' &&
      (cond.id !== 'custom' || (preset.customStopText ?? '').trim() !== ''),
  );

  function setPrompt(i, v) {
    const next = preset.prompts.slice();
    next[i] = v;
    update({ prompts: next });
  }
  function removePrompt(i) {
    if (preset.prompts.length <= 1) return;
    update({ prompts: preset.prompts.filter((_, idx) => idx !== i) });
  }
  function duplicatePrompt(i) {
    const next = preset.prompts.slice();
    next.splice(i + 1, 0, preset.prompts[i]);
    update({ prompts: next });
  }
  function movePrompt(i, dir) {
    const j = i + dir;
    if (j < 0 || j >= preset.prompts.length) return;
    const next = preset.prompts.slice();
    [next[i], next[j]] = [next[j], next[i]];
    update({ prompts: next });
  }
  function addPrompt() {
    update({ prompts: [...preset.prompts, ''] });
  }

  function hint(strategy) {
    if (strategy === 'reviewer') return 'first is draft · rest critique';
    if (strategy === 'schema_merge') return 'each fills a slice of schema';
    if (strategy === 'structured_reviewer') return 'each must emit schema-valid JSON';
    return 'outputs concatenated in order';
  }
</script>

<div class="pe-section-head">
  <h3>Prompts</h3>
  <span class="count">
    {preset.prompts.length} step{preset.prompts.length === 1 ? '' : 's'}
  </span>
  <div class="spacer"></div>
  <span class="hint">{hint(preset.strategy)}</span>
</div>

<div class="pe-prompts">
  {#each preset.prompts as text, i (i)}
    {@const stopApplies =
      hasStop && (preset.appendToAll || i === preset.prompts.length - 1)}
    <div class="pe-prompt">
      <div class="pe-prompt-gutter">
        <span class="pe-prompt-num">{String(i + 1).padStart(2, '0')}</span>
      </div>
      <div class="pe-prompt-main">
        <textarea
          class="pe-prompt-text"
          value={text}
          oninput={(e) => setPrompt(i, e.currentTarget.value)}
          placeholder={`Prompt ${i + 1}…`}
          spellcheck="false"
          disabled={readOnly}
        ></textarea>
        <div class="pe-prompt-foot">
          <span>{text.length} chars · ~{Math.ceil(text.length / 4)} tokens</span>
          {#if stopApplies}
            <span class="stop-tag" title="Stop condition will be appended to this prompt">
              <Icon name="stop" size={9} />
              stop appended
            </span>
          {/if}
          <div class="spacer"></div>
          <button
            title="Move up"
            onclick={() => movePrompt(i, -1)}
            disabled={readOnly || i === 0}
          >↑</button>
          <button
            title="Move down"
            onclick={() => movePrompt(i, 1)}
            disabled={readOnly || i === preset.prompts.length - 1}
          >↓</button>
          <button
            title="Duplicate"
            onclick={() => duplicatePrompt(i)}
            disabled={readOnly}
          ><Icon name="copy" size={11} /></button>
          <button
            class="danger"
            title="Delete"
            onclick={() => removePrompt(i)}
            disabled={readOnly || preset.prompts.length === 1}
          ><Icon name="trash" size={11} /></button>
        </div>
      </div>
    </div>
  {/each}
</div>

<button class="pe-add-prompt" onclick={addPrompt} disabled={readOnly}>
  <Icon name="plus" size={13} />
  Add prompt
</button>
