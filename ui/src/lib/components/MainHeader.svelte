<script>
  import Icon from '$lib/Icon.svelte';

  let { project } = $props();

  /** Renders a `ToolPolicy` (string variant or `{ allowlist: [...] }`). */
  function toolLabel(policy) {
    if (typeof policy === 'string') return policy.replace('_', ' ');
    if (policy && Array.isArray(policy.allowlist)) {
      return `allowlist · ${policy.allowlist.length}`;
    }
    return '—';
  }
</script>

<div class="main-header">
  <div class="breadcrumb">
    <Icon name="folder" size={13} />
    <span class="cur">{project ? project.name : 'No project selected'}</span>
  </div>
  <div class="main-header-spacer"></div>
  {#if project}
    <div class="main-header-meta">
      <span><span class="k">path</span><span class="v">{project.root_path}</span></span>
      <span><span class="k">tools</span><span class="v">{toolLabel(project.tool_policy)}</span></span>
    </div>
  {/if}
</div>
