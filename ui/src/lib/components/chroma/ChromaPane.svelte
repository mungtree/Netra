<script>
  import { onMount } from 'svelte';
  import { store, refreshChromaStatus } from '$lib/store.svelte.js';
  import ServerControls from './ServerControls.svelte';
  import CollectionList from './CollectionList.svelte';
  import ChromaSettings from './ChromaSettings.svelte';
  import QueryPane from './QueryPane.svelte';

  let tab = $state('server');

  onMount(() => {
    refreshChromaStatus();
  });
</script>

<div class="chroma-pane">
  <header class="cp-head">
    <h1>ChromaDB</h1>
    <p class="cp-sub">
      Opt-in vector store. Lets the <code>pi</code> agent search your codebase
      semantically through the <code>chroma</code> MCP server.
    </p>
    {#if !store.chroma?.enabled}
      <div class="cp-disabled">
        Integration is <strong>disabled</strong>. Enable it in
        <em>Settings → ChromaDB</em>, then restart NETRA.
      </div>
    {/if}
  </header>

  <nav class="cp-tabs">
    <button class:active={tab === 'server'} onclick={() => (tab = 'server')}>
      Server
    </button>
    <button class:active={tab === 'collections'} onclick={() => (tab = 'collections')}>
      Collections
    </button>
    <button class:active={tab === 'query'} onclick={() => (tab = 'query')}>
      Query
    </button>
    <button class:active={tab === 'settings'} onclick={() => (tab = 'settings')}>
      Settings
    </button>
  </nav>

  <section class="cp-body">
    {#if tab === 'server'}
      <ServerControls />
    {:else if tab === 'collections'}
      <CollectionList />
    {:else if tab === 'query'}
      <QueryPane />
    {:else}
      <ChromaSettings />
    {/if}
  </section>
</div>

<style>
  .chroma-pane {
    flex: 1;
    overflow: auto;
    padding: 24px 32px;
    color: var(--fg, #d4d4d4);
  }
  .cp-head h1 { margin: 0 0 4px; font-size: 18px; }
  .cp-sub { margin: 0 0 16px; opacity: 0.7; font-size: 12px; }
  .cp-disabled {
    background: #3a2a1a;
    border: 1px solid #6a4a20;
    color: #f0c070;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 12px;
    margin-bottom: 16px;
  }
  .cp-tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid #333;
    margin-bottom: 16px;
  }
  .cp-tabs button {
    background: transparent;
    border: none;
    color: var(--fg, #d4d4d4);
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }
  .cp-tabs button.active { border-bottom-color: #4a9eff; color: #fff; }
</style>
