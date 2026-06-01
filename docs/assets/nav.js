/* ============================================================
   NETRA code-wiki — navigation.

   THIS IS THE SINGLE SOURCE OF TRUTH FOR THE SITE TREE.
   To add a page: create the .html file (copy _TEMPLATE.html) and
   add one entry to the SITE array below. Nothing else.

   Every page must set, on its <body> tag:
     data-root  — relative path to the docs/ root ("." , ".." , "../..")
     data-page  — this page's href relative to docs/ root
   See docs/AGENTS.md for the full workflow.
   ============================================================ */

const SITE = [
  {
    title: 'Start here',
    pages: [
      { label: 'Home',                 href: 'index.html' },
      { label: 'Overview (ELI5)',      href: 'concepts/overview.html' },
      { label: 'Getting started',      href: 'guides/getting-started.html' },
      { label: 'Glossary',             href: 'concepts/glossary.html' },
    ],
  },
  {
    title: 'Understand the system',
    pages: [
      { label: 'Architecture',         href: 'concepts/architecture.html' },
      { label: 'Data flow',            href: 'concepts/data-flow.html' },
      { label: 'Design patterns',      href: 'concepts/design-patterns.html' },
      { label: 'The pi RPC protocol',  href: 'concepts/pi-protocol.html' },
      { label: 'Cross-module map',     href: 'concepts/cross-module-map.html' },
      { label: 'Architecture diagram', href: 'concepts/architecture-diagram.html' },
    ],
  },
  {
    title: 'Guides',
    pages: [
      { label: 'CLI (netra)',         href: 'guides/cli.html' },
      { label: 'Desktop shell (Tauri)',href: 'guides/tauri.html' },
    ],
  },
  {
    title: 'Reference · netra-core',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-core/index.html' },
      { label: 'lib.rs',               href: 'reference/netra-core/lib.html', depth: 1 },
      { label: 'error.rs',             href: 'reference/netra-core/error.html', depth: 1 },
      { label: 'ids.rs',               href: 'reference/netra-core/ids.html', depth: 1 },
      { label: 'model/mod.rs',         href: 'reference/netra-core/model-mod.html', depth: 1 },
      { label: 'model/agent.rs',       href: 'reference/netra-core/model-agent.html', depth: 1 },
      { label: 'model/job.rs',         href: 'reference/netra-core/model-job.html', depth: 1 },
      { label: 'model/project.rs',     href: 'reference/netra-core/model-project.html', depth: 1 },
      { label: 'model/batch.rs',       href: 'reference/netra-core/model-batch.html', depth: 1 },
      { label: 'model/aggregate.rs',   href: 'reference/netra-core/model-aggregate.html', depth: 1 },
      { label: 'model/template.rs',    href: 'reference/netra-core/model-template.html', depth: 1 },
      { label: 'traits/mod.rs',        href: 'reference/netra-core/traits-mod.html', depth: 1 },
      { label: 'traits/transport.rs',  href: 'reference/netra-core/traits-transport.html', depth: 1 },
      { label: 'traits/repo.rs',       href: 'reference/netra-core/traits-repo.html', depth: 1 },
      { label: 'traits/queue.rs',      href: 'reference/netra-core/traits-queue.html', depth: 1 },
      { label: 'traits/session.rs',    href: 'reference/netra-core/traits-session.html', depth: 1 },
      { label: 'traits/bus.rs',        href: 'reference/netra-core/traits-bus.html', depth: 1 },
      { label: 'traits/sink.rs',       href: 'reference/netra-core/traits-sink.html', depth: 1 },
      { label: 'traits/aggregator.rs', href: 'reference/netra-core/traits-aggregator.html', depth: 1 },
      { label: 'traits/support.rs',    href: 'reference/netra-core/traits-support.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · netra-agent',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-agent/index.html' },
      { label: 'lib.rs',               href: 'reference/netra-agent/lib.html', depth: 1 },
      { label: 'spec.rs',              href: 'reference/netra-agent/spec.html', depth: 1 },
      { label: 'protocol.rs',          href: 'reference/netra-agent/protocol.html', depth: 1 },
      { label: 'transport.rs',         href: 'reference/netra-agent/transport.html', depth: 1 },
      { label: 'session.rs',           href: 'reference/netra-agent/session.html', depth: 1 },
      { label: 'pool.rs',              href: 'reference/netra-agent/pool.html', depth: 1 },
      { label: 'mock.rs',              href: 'reference/netra-agent/mock.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · netra-engine',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-engine/index.html' },
      { label: 'lib.rs',               href: 'reference/netra-engine/lib.html', depth: 1 },
      { label: 'retry.rs',             href: 'reference/netra-engine/retry.html', depth: 1 },
      { label: 'bus.rs',               href: 'reference/netra-engine/bus.html', depth: 1 },
      { label: 'queue.rs',             href: 'reference/netra-engine/queue.html', depth: 1 },
      { label: 'aggregate.rs',         href: 'reference/netra-engine/aggregate.html', depth: 1 },
      { label: 'batch.rs',             href: 'reference/netra-engine/batch.html', depth: 1 },
      { label: 'scheduler.rs',         href: 'reference/netra-engine/scheduler.html', depth: 1 },
      { label: 'runner.rs',            href: 'reference/netra-engine/runner.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · netra-store',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-store/index.html' },
      { label: 'lib.rs',               href: 'reference/netra-store/lib.html', depth: 1 },
      { label: 'db.rs',                href: 'reference/netra-store/db.html', depth: 1 },
      { label: 'sink.rs',              href: 'reference/netra-store/sink.html', depth: 1 },
      { label: 'repo/mod.rs',          href: 'reference/netra-store/repo-mod.html', depth: 1 },
      { label: 'repo/project.rs',      href: 'reference/netra-store/repo-project.html', depth: 1 },
      { label: 'repo/job.rs',          href: 'reference/netra-store/repo-job.html', depth: 1 },
      { label: 'repo/batch.rs',        href: 'reference/netra-store/repo-batch.html', depth: 1 },
      { label: 'repo/template.rs',     href: 'reference/netra-store/repo-template.html', depth: 1 },
      { label: 'migrations/0001_init', href: 'reference/netra-store/migration-0001.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · netra-api',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-api/index.html' },
      { label: 'lib.rs',               href: 'reference/netra-api/lib.html', depth: 1 },
      { label: 'netra.rs',            href: 'reference/netra-api/netra.html', depth: 1 },
      { label: 'resolver.rs',          href: 'reference/netra-api/resolver.html', depth: 1 },
      { label: 'config.rs',            href: 'reference/netra-api/config.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · netra-cli',
    pages: [
      { label: 'crate overview',       href: 'reference/netra-cli/index.html' },
      { label: 'main.rs',              href: 'reference/netra-cli/main.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · src-tauri',
    pages: [
      { label: 'crate overview',       href: 'reference/src-tauri/index.html' },
      { label: 'main.rs',              href: 'reference/src-tauri/main.html', depth: 1 },
      { label: 'lib.rs',               href: 'reference/src-tauri/lib.html', depth: 1 },
      { label: 'commands.rs',          href: 'reference/src-tauri/commands.html', depth: 1 },
    ],
  },
  {
    title: 'Reference · ui (SvelteKit)',
    pages: [
      { label: 'crate overview',       href: 'reference/ui/index.html' },
      { label: 'lib/api.js',           href: 'reference/ui/api.html', depth: 1 },
      { label: 'lib/store.svelte.js',  href: 'reference/ui/store.html', depth: 1 },
      { label: 'lib/tasks.js',         href: 'reference/ui/tasks.html', depth: 1 },
      { label: 'lib/Icon.svelte',      href: 'reference/ui/icon.html', depth: 1 },
      { label: 'routes/+page.svelte',  href: 'reference/ui/page.html', depth: 1 },
      { label: 'routes/+layout.*',     href: 'reference/ui/layout.html', depth: 1 },
      { label: 'Titlebar.svelte',      href: 'reference/ui/titlebar.html', depth: 1 },
      { label: 'ActivityBar.svelte',   href: 'reference/ui/activitybar.html', depth: 1 },
      { label: 'Sidebar.svelte',       href: 'reference/ui/sidebar.html', depth: 1 },
      { label: 'MainHeader.svelte',    href: 'reference/ui/mainheader.html', depth: 1 },
      { label: 'TaskGrid.svelte',      href: 'reference/ui/taskgrid.html', depth: 1 },
      { label: 'LastRun.svelte',       href: 'reference/ui/lastrun.html', depth: 1 },
      { label: 'OutputPane.svelte',    href: 'reference/ui/outputpane.html', depth: 1 },
      { label: 'QueuePanel.svelte',    href: 'reference/ui/queuepanel.html', depth: 1 },
      { label: 'StatusBar.svelte',     href: 'reference/ui/statusbar.html', depth: 1 },
    ],
  },
  {
    title: 'Maintaining',
    pages: [
      { label: 'Maintenance prompts',  href: 'prompts.html' },
      { label: 'AGENTS.md (raw)',      href: 'AGENTS.md' },
    ],
  },
];

(function buildSidebar() {
  const body = document.body;
  const root = (body.dataset.root || '.').replace(/\/$/, '');
  const current = body.dataset.page || '';
  const rel = (href) => root + '/' + href;

  const sidebar = document.getElementById('sidebar');
  if (!sidebar) return;

  let html = '';
  html += '<a class="nav-brand" href="' + rel('index.html') + '">' +
            '<span class="nav-brand-name">NETRA</span><br>' +
            '<span class="nav-brand-sub">Code Wiki</span></a>';
  html += '<div class="nav-filter-wrap">' +
            '<input class="nav-filter" type="text" placeholder="Filter pages…" ' +
            'aria-label="Filter pages">' +
          '</div>';

  for (const section of SITE) {
    html += '<div class="nav-section">';
    html += '<div class="nav-section-title">' + section.title + '</div>';
    for (const page of section.pages) {
      const cls = ['nav-link'];
      if (page.depth === 1) cls.push('depth-1');
      if (page.href === current) cls.push('current');
      html += '<a class="' + cls.join(' ') + '" href="' + rel(page.href) + '">' +
              page.label + '</a>';
    }
    html += '</div>';
  }
  html += '<div class="nav-empty">No pages match.</div>';
  sidebar.innerHTML = html;

  /* client-side filter */
  const filter = sidebar.querySelector('.nav-filter');
  const links = Array.from(sidebar.querySelectorAll('.nav-link'));
  const sections = Array.from(sidebar.querySelectorAll('.nav-section'));
  const empty = sidebar.querySelector('.nav-empty');

  filter.addEventListener('input', () => {
    const q = filter.value.trim().toLowerCase();
    let anyVisible = false;
    for (const link of links) {
      const match = !q || link.textContent.toLowerCase().includes(q);
      link.style.display = match ? '' : 'none';
      if (match) anyVisible = true;
    }
    for (const section of sections) {
      const visibleLinks = section.querySelectorAll(
        '.nav-link:not([style*="display: none"])');
      section.style.display = visibleLinks.length ? '' : 'none';
    }
    empty.style.display = anyVisible ? 'none' : 'block';
  });

  /* keep the current page in view */
  const cur = sidebar.querySelector('.nav-link.current');
  if (cur) cur.scrollIntoView({ block: 'center' });
})();
