<script>
  import Icon from '$lib/Icon.svelte';
  import { store, dismissResume } from '$lib/store.svelte.js';

  const r = $derived(store.resume);

  function review() {
    store.activeView = 'projects';
    dismissResume();
  }
</script>

{#if r}
  <div class="resume-banner">
    <span class="ic"><Icon name="refresh" size={14} /></span>
    <div class="msg">
      Resumed <b>{r.resumed} queued job{r.resumed !== 1 ? 's' : ''}</b> from your last session
      {#if r.discarded > 0}
        <span class="meta">{r.discarded} cancelled (module deleted or too many retries)</span>
      {/if}
    </div>
    <button class="btn-banner" onclick={review}>Review</button>
    <button class="dismiss" onclick={dismissResume} aria-label="dismiss">
      <Icon name="x" size={12} />
    </button>
  </div>
{/if}
