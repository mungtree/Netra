<script module>
  // Inline SVG icon set — ported from example-ui/icons.jsx.
  const PATHS = {
    folder:
      '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z"/>',
    library:
      '<path d="M4 4h4v16H4z"/><path d="M10 4h4v16h-4z"/><path d="m16 5 3.5 1L17 22l-3.5-1L16 5Z"/>',
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    settings:
      '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1A1.7 1.7 0 0 0 4.6 9a1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    bug: '<path d="M8 3 9.5 5.5"/><path d="M16 3 14.5 5.5"/><path d="M9 8h6"/><path d="M5 13H3"/><path d="M21 13h-2"/><path d="M5 19l2-2"/><path d="M19 19l-2-2"/><path d="M12 21v-8"/><rect x="7" y="8" width="10" height="11" rx="5"/>',
    shield: '<path d="M12 3 4 6v6c0 5 3.5 8 8 9 4.5-1 8-4 8-9V6l-8-3Z"/><path d="m9 12 2 2 4-4"/>',
    bulb: '<path d="M9 18h6"/><path d="M10 22h4"/><path d="M12 2a7 7 0 0 0-4 12.7c.7.6 1 1.4 1 2.3v1h6v-1c0-.9.3-1.7 1-2.3A7 7 0 0 0 12 2Z"/>',
    wand: '<path d="m15 4 1 2 2 1-2 1-1 2-1-2-2-1 2-1 1-2Z"/><path d="m4 20 9-9"/><path d="m18 8 3 3"/>',
    gauge: '<path d="M12 14 8 8"/><circle cx="12" cy="14" r="9"/><path d="M3 14a9 9 0 0 1 18 0"/>',
    book: '<path d="M4 4a2 2 0 0 1 2-2h13v18H6a2 2 0 0 0-2 2V4Z"/><path d="M4 20a2 2 0 0 1 2-2h13"/>',
    check: '<path d="m5 12 5 5 9-11"/>',
    package:
      '<path d="M21 7 12 2 3 7v10l9 5 9-5V7Z"/><path d="m3 7 9 5 9-5"/><path d="M12 22V12"/>',
    play: '<path d="M6 4v16l14-8L6 4Z"/>',
    pause: '<path d="M6 4h4v16H6z"/><path d="M14 4h4v16h-4z"/>',
    x: '<path d="M6 6l12 12M18 6 6 18"/>',
    branch:
      '<circle cx="6" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="6" r="2"/><path d="M6 8v8"/><path d="M18 8a6 6 0 0 1-6 6"/>',
    cpu: '<rect x="6" y="6" width="12" height="12" rx="1"/><rect x="9" y="9" width="6" height="6"/><path d="M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3"/>',
    activity: '<path d="M3 12h4l3-9 4 18 3-9h4"/>',
    code: '<path d="m9 8-5 4 5 4"/><path d="m15 8 5 4-5 4"/>',
    test: '<path d="M9 2v6l-5 9a3 3 0 0 0 3 5h10a3 3 0 0 0 3-5l-5-9V2"/><path d="M8 2h8"/><path d="M7 14h10"/>',
    refresh:
      '<path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/><path d="M3 21v-5h5"/>',
    more: '<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>',
    inbox:
      '<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5 5h14l3 7v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-6Z"/>',
  };
</script>

<script>
  let { name, size = 16 } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html PATHS[name] ?? ''}
</svg>
