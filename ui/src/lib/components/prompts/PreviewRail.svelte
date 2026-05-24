<script>
  import Icon from '$lib/Icon.svelte';
  import { serializeBatch } from '$lib/batchIo.js';
  import { STOP_CONDITIONS } from '$lib/prompts/promptsData.js';

  let { preset, onToast } = $props();

  let tab = $state('json');

  const json = $derived(serializeBatch(preset));

  const cond = $derived(
    STOP_CONDITIONS.find((s) => s.id === preset.stopConditionId) ?? STOP_CONDITIONS[0],
  );
  const stopText = $derived(
    cond.id === 'custom' ? preset.customStopText ?? '' : cond.text,
  );
  const hasStop = $derived(cond.id !== 'default' && stopText.trim() !== '');

  const schemaText = $derived(
    preset.strategy === 'schema_merge' && preset.output_schema
      ? JSON.stringify(preset.output_schema, null, 2)
      : '',
  );
  const hasSchema = $derived(schemaText !== '');

  const highlighted = $derived(highlightJSON(json));

  function highlightJSON(text) {
    const esc = text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    return esc.replace(
      /("(?:\\.|[^"\\])*"\s*:?)|\b(true|false|null)\b|\b(-?\d+(?:\.\d+)?)\b/g,
      (m, str, bool, num) => {
        if (str) {
          const isKey = /:$/.test(str);
          const body = isKey ? str.slice(0, -1) : str;
          return `<span class="${isKey ? 'k' : 's'}">${body}</span>${isKey ? '<span class="p">:</span>' : ''}`;
        }
        if (bool) return `<span class="b">${bool}</span>`;
        if (num) return `<span class="n">${num}</span>`;
        return m;
      },
    );
  }

  function copy() {
    navigator.clipboard.writeText(json).then(
      () => onToast?.('Copied JSON to clipboard'),
      () => onToast?.('Copy failed', true),
    );
  }

  function download() {
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    const safe = (preset.title || 'batch')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-');
    a.href = url;
    a.download = `${safe}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    onToast?.(`Downloaded ${a.download}`);
  }
</script>

<div class="pe-preview">
  <div class="pe-preview-head">
    <span class="ttl">Output</span>
    <div class="spacer"></div>
    <div class="pe-preview-tabs">
      <button class:active={tab === 'json'} onclick={() => (tab = 'json')}>JSON</button>
      <button class:active={tab === 'resolved'} onclick={() => (tab = 'resolved')}>RESOLVED</button>
    </div>
  </div>
  <div class="pe-preview-actions">
    <button class="btn" onclick={copy}><Icon name="copy" size={12} />Copy</button>
    <button class="btn btn-primary" onclick={download}>
      <Icon name="download" size={12} />Export
    </button>
  </div>
  <div class="pe-preview-body">
    {#if tab === 'json'}
      <!-- eslint-disable-next-line svelte/no-at-html-tags -->
      <pre class="pe-json">{@html highlighted}</pre>
    {:else}
      <div class="pe-resolved">
        {#each preset.prompts as p, i (i)}
          {@const apply = hasStop && (preset.appendToAll || i === preset.prompts.length - 1)}
          {@const extra =
            (apply ? stopText.length + 22 : 0) + (hasSchema ? schemaText.length + 22 : 0)}
          <div class="pe-resolved-item">
            <div class="hdr">
              <span class="num">{String(i + 1).padStart(2, '0')}</span>
              <span>·</span>
              <span>{p.length + extra} chars</span>
              {#if hasSchema}
                <span>·</span>
                <span class="plus-stop">+ schema</span>
              {/if}
              {#if apply}
                <span>·</span>
                <span class="plus-stop">+ stop</span>
              {/if}
            </div>
            <div class="body">
              {#if p}
                {p}
              {:else}
                <span class="empty">(empty)</span>
              {/if}
              {#if hasSchema}
                <div class="appended">
                  <span class="lbl">— Output schema —</span>
                  {schemaText}
                </div>
              {/if}
              {#if apply}
                <div class="appended">
                  <span class="lbl">— Stop condition —</span>
                  {stopText}
                </div>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
