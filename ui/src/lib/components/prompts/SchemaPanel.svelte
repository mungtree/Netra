<script>
  import Icon from '$lib/Icon.svelte';

  let { preset, readOnly = false, update } = $props();

  // Only `schema_merge` consumes a user-supplied schema. `structured_reviewer`
  // has its built-in finding shape — no schema needed.
  const needsSchema = $derived(preset.strategy === 'schema_merge');

  let open = $state(!!preset.output_schema || needsSchema);
  let text = $state(
    preset.output_schema ? JSON.stringify(preset.output_schema, null, 2) : '',
  );
  let err = $state('');

  // Reset textarea when switching presets.
  let lastId = preset.id;
  $effect(() => {
    if (preset.id !== lastId) {
      lastId = preset.id;
      text = preset.output_schema
        ? JSON.stringify(preset.output_schema, null, 2)
        : '';
      err = '';
    }
  });

  function onText(v) {
    text = v;
    if (!v.trim()) {
      err = '';
      update({ output_schema: null });
      return;
    }
    try {
      const parsed = JSON.parse(v);
      err = '';
      update({ output_schema: parsed });
    } catch (e) {
      err = e.message;
    }
  }
</script>

{#if needsSchema}
<div class="pe-schema" class:open>
  <button class="pe-schema-head" type="button" onclick={() => (open = !open)}>
    <Icon
      name="layers"
      size={13}
    />
    <span class="ttl">Output schema</span>
    <span class="meta">
      {preset.output_schema
        ? 'set · appended to every prompt'
        : 'required by schema_merge · not set'}
    </span>
    <div class="spacer"></div>
    <span class="chev" class:flip={open}><Icon name="chevron" size={14} /></span>
  </button>
  {#if open}
    <div class="pe-schema-body">
      <textarea
        class="pe-schema-textarea"
        class:err={!!err}
        value={text}
        oninput={(e) => onText(e.currentTarget.value)}
        placeholder={'{ "type": "array", "items": { ... } }'}
        spellcheck="false"
        disabled={readOnly}
      ></textarea>
      {#if err}
        <div class="pe-schema-err">JSON error: {err}</div>
      {/if}
    </div>
  {/if}
</div>
{/if}
