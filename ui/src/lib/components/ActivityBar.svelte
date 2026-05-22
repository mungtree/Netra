<script>
  import Icon from '$lib/Icon.svelte';

  // Only the Projects view is implemented today; the rest are placeholders.
  const items = [
    { id: 'projects', icon: 'folder', active: true },
    { id: 'library', icon: 'library', active: false },
    { id: 'history', icon: 'clock', active: false },
    { id: 'activity', icon: 'activity', active: false },
  ];
</script>

<div class="activitybar">
  {#each items as item (item.id)}
    <button class="ab-btn" class:active={item.active} title={item.id} disabled={!item.active}>
      <Icon name={item.icon} />
    </button>
  {/each}
  <div class="ab-spacer"></div>
  <button class="ab-btn" title="Settings" disabled>
    <Icon name="settings" />
  </button>
</div>
