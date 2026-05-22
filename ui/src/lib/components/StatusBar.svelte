<script>
  import Icon from '$lib/Icon.svelte';

  let { running = 0, queued = 0, done = 0 } = $props();
</script>

<div class="statusbar">
  <span class="sb-item">
    <span class="ic"><Icon name="cpu" size={11} /></span>qwen3.6-35b-a3b · local
  </span>
  <span class="statusbar-spacer"></span>
  <span class="sb-item accent">● {running} running</span>
  <span class="sb-item">{queued} queued</span>
  <span class="sb-item">{done} done</span>
  <span class="sb-item">tauri · 0.1.0</span>
</div>
