<script>
  import Icon from '$lib/Icon.svelte';

  let { project } = $props();
</script>

<span class="scope-pill">
  <span class="ic"><Icon name="folder" size={11} /></span>
  <span>{project?.name ?? '—'}</span>
  <span class="path">· {project?.root_path ?? ''}</span>
</span>
