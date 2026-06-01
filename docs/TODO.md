# docs/ code-wiki — unfinished work

Status as of 2026-05-22. The wiki is a static HTML site (no build step).
Read `docs/AGENTS.md` before continuing — it has the page template, the
`nav.js` rule, and the style rules. Every page below already has a `nav.js`
entry, so each one 404s until its file is created.

## Done (53 files)

Infra (`style.css`, `nav.js`, `_TEMPLATE.html`), `index.html`, all `concepts/`
except `cross-module-map`, all `guides/`, `AGENTS.md`, `prompts.html`, and
reference pages for **netra-core** (19), **netra-agent** (8),
**netra-engine** (9), **netra-store** (9 of 10).

## Remaining — 29 HTML pages + cleanup

Follow the same pattern as the finished reference pages: read the source, then
write the page with the 7-section template (`AGENTS.md` §4). `data-root` for
`reference/<crate>/` pages is `../..`.

### 1. netra-store — 1 page (source already mapped)
- [x] `reference/netra-store/migration-0001.html` — source:
  `crates/netra-store/migrations/0001_init.sql`. Tables `projects`, `batches`,
  `jobs` (+ idx on status, project_id), `batch_items` (+ idx on batch_id),
  `templates`. Explain the JSON-blob storage model and the FK cascade rules
  (`jobs.project_id` CASCADE, `jobs.batch_id` SET NULL, `batch_items.batch_id`
  CASCADE).

### 2. netra-api — 5 pages
Read `crates/netra-api/src/{lib,netra,resolver,config}.rs`.
- [x] `reference/netra-api/index.html` — crate overview (the `Netra` facade).
- [ ] `reference/netra-api/lib.html`
- [ ] `reference/netra-api/netra.html` — the `Netra` struct + ~20 async
  methods (`start`, `add_project`, `queue_job`, `create_batch`, `run_batch`,
  `cancel_job`, `wait_for_job`, `subscribe_events`, …).
- [ ] `reference/netra-api/resolver.html` — `ProjectSpecResolver`
  (`SpecResolver` impl; model precedence job > project > app).
- [ ] `reference/netra-api/config.html` — `NetraConfig`, `ConcurrencyConfig`,
  `ModelConfig`, `ConfigError`.

### 3. netra-cli — 2 pages
Read `crates/netra-cli/src/main.rs`.
- [ ] `reference/netra-cli/index.html`
- [ ] `reference/netra-cli/main.html` — clap commands, `dispatch`, `batch`,
  `project` handlers.

### 4. src-tauri — 4 pages
Read `src-tauri/src/{main,lib,commands}.rs`.
- [ ] `reference/src-tauri/index.html`
- [ ] `reference/src-tauri/main.html`
- [ ] `reference/src-tauri/lib.html` — `run()`, managed `Netra` state, the
  `DomainEvent` → `netra://event` bridge.
- [ ] `reference/src-tauri/commands.html` — 12 `#[tauri::command]`s, each 1:1
  with a `Netra` method.

### 5. ui — 16 pages
Read `ui/src/lib/*`, `ui/src/routes/*`, `ui/src/lib/components/*`.
- [ ] `reference/ui/index.html` — crate overview (SvelteKit shell).
- [ ] `api.html` (`lib/api.js`), `store.html` (`lib/store.svelte.js`),
  `tasks.html` (`lib/tasks.js`), `icon.html` (`lib/Icon.svelte`),
  `page.html` (`routes/+page.svelte`), `layout.html` (`routes/+layout.*`).
- [ ] components: `titlebar`, `activitybar`, `sidebar`, `mainheader`,
  `taskgrid`, `lastrun`, `outputpane`, `queuepanel`, `statusbar`.

### 6. cross-module map — 1 page (write LAST)
- [ ] `concepts/cross-module-map.html` — the master ripple map. Write after all
  reference pages so the call chains are accurate. Cover: `queue_job` →
  `Scheduler` → `JobRunner` → `OutputSink`+`EventBus` → `netra://event` →
  `store.startEvents`; the batch map→reduce chain; the retry loop; the
  `AgentSpec::build_args` ← `ToolPolicy`/`ModelRef` mapping.

### 7. Cleanup
- [ ] `PLAN.md` — update 2 refs: `docs/cli.md` → `docs/guides/cli.html`,
  `docs/tauri.md` → `docs/guides/tauri.html`.
- [ ] Delete `docs/cli.md` and `docs/tauri.md` (content moved to `guides/`).
- [ ] Verify: open `docs/index.html`, check sidebar/filter, click through every
  `nav.js` entry — no 404s; spot-check 5 reference pages against source for
  signature/line accuracy.

## After everything is done
Delete this file.
