<script>
  import Icon from '$lib/Icon.svelte';

  /** options: [{id, name, desc?, code?, muted?}] */
  let { options, value, onChange } = $props();

  let open = $state(false);
  /** @type {HTMLDivElement | null} */
  let rootEl = null;

  const selected = $derived(options.find((o) => o.id === value) ?? null);

  function toggle() {
    open = !open;
  }
  function pick(id) {
    onChange(id);
    open = false;
  }
  function onDocDown(e) {
    if (!open) return;
    if (rootEl && !rootEl.contains(e.target)) open = false;
  }
</script>

<svelte:document on:mousedown={onDocDown} />

<div
  bind:this={rootEl}
  class="pe-select"
  class:open
  onclick={toggle}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggle();
    }
  }}
  role="button"
  tabindex="0"
>
  <div class="left">
    <span class="name">{selected?.name ?? ''}</span>
    {#if selected?.desc}<span class="desc">{selected.desc}</span>{/if}
  </div>
  <span class="arrow"><Icon name="chevron" size={14} /></span>
  {#if open}
    <div
      class="pe-popover"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="listbox"
      tabindex="-1"
    >
      {#each options as opt (opt.id)}
        <div
          class="pe-popover-item"
          class:selected={opt.id === value}
          class:muted={opt.muted}
          onclick={() => pick(opt.id)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              pick(opt.id);
            }
          }}
          role="option"
          aria-selected={opt.id === value}
          tabindex="0"
        >
          <div style="flex:1; min-width:0;">
            <div class="name">
              {opt.name}{#if opt.code}<code>{opt.code}</code>{/if}
            </div>
            {#if opt.desc}<div class="desc">{opt.desc}</div>{/if}
          </div>
          {#if opt.id === value}
            <span class="tick"><Icon name="check" size={13} /></span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
