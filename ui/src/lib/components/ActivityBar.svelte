<script>
  import Icon from '$lib/Icon.svelte';
  import { store } from '$lib/store.svelte.js';

  const items = [
    { id: 'projects', icon: 'folder' },
  ];
</script>

<div class="activitybar">
  {#each items as item (item.id)}
    <button
      class="ab-btn"
      class:active={store.activeView === item.id}
      title={item.id}
      onclick={() => (store.activeView = item.id)}
    >
      <Icon name={item.icon} />
    </button>
  {/each}
  <button
    class="ab-btn"
    class:active={store.activeView === 'modules'}
    title="Modules"
    onclick={() => (store.activeView = 'modules')}
  >
    <Icon name="layers" />
  </button>
  <button
    class="ab-btn"
    class:active={store.activeView === 'overview'}
    title="Projects & Modules"
    onclick={() => (store.activeView = 'overview')}
  >
    <Icon name="package" />
  </button>
  <button
    class="ab-btn"
    class:active={store.activeView === 'prompts'}
    title="prompts"
    onclick={() => (store.activeView = 'prompts')}
  >
    <Icon name="prompt" />
  </button>
  <button
    class="ab-btn"
    class:active={store.activeView === 'history'}
    title="history"
    onclick={() => (store.activeView = 'history')}
  >
    <Icon name="clock" />
  </button>
  <button class="ab-btn" title="activity" disabled>
    <Icon name="activity" />
  </button>
  <button
    class="ab-btn"
    class:active={store.activeView === 'chromadb'}
    title="ChromaDB"
    onclick={() => (store.activeView = 'chromadb')}
  >
    <Icon name="database" />
  </button>
  <div class="ab-spacer"></div>
  <button
    class="ab-btn"
    class:active={store.activeView === 'settings'}
    title="Settings"
    onclick={() => (store.activeView = 'settings')}
  >
    <Icon name="settings" />
  </button>
</div>
