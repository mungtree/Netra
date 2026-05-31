<script>
  import Icon from '$lib/Icon.svelte';
  import { store } from '$lib/store.svelte.js';

  let { onCancel } = $props();

  const model = $derived(store.settings.defaultModel || 'the configured agent');
</script>

<div class="infer-loading">
  <div class="spin"></div>
  <div class="body">
    <div class="title">Inferring modules with {model}</div>
    <div class="step">Reading package manifests &amp; top-level dirs…</div>
    <div class="steps">
      <div class="s done">Discovering top-level directories</div>
      <div class="s done">Reading package manifests</div>
      <div class="s cur">Naming and describing module candidates…</div>
      <div class="s">Reconciling with your current modules</div>
    </div>
  </div>
  <button class="btn" style="align-self:flex-start" onclick={() => onCancel?.()}>
    <Icon name="x" size={12} />Cancel
  </button>
</div>
