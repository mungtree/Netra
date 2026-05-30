# Modules + Durable Queue Plan

## Context

Mini-chatur fans batch jobs out as `prompts × targets`. When a "target" is a large codebase, each agent has to grep the entire tree, which inflates context and blurs scope. This plan adds an explicit **module** layer between project and job so the batch fanout becomes `prompts × targets × modules`. A user can either let an AI agent infer modules from the repo or define them by hand in the Tauri UI; a per-batch **global** switch skips module fanout entirely.

Second goal: the in-memory `InMemoryJobQueue` loses every queued job on restart. Rehydrate the queue from SQLite on startup so an app restart resumes work instead of orphaning it.

User decisions (locked in):
- Scope enforcement: **prompt hint + module path passed in `AgentSpec`** — agents keep full repo access.
- Module shape: **`{name, description, root_subdir}`** — single subdir per module.
- Queue persistence: **reuse existing SQLite `jobs` table**, no new infra.
- Module storage: **embed `modules: Vec<Module>` in `Project`** — no schema migration.

## UI Source of Truth — `modules-ui/` mockup

A React + CSS design mockup lives in `modules-ui/` and is the **spec for the frontend** of this feature. It is built on the project's real design tokens (`var(--accent)`, `var(--bg-panel)`, `--font-mono`, etc. from the existing `tokens.css` / `chrome.css`), so it layers onto the current IDE chrome rather than replacing it. The Frontend section below maps every mockup surface to its Svelte target.

Mockup files:
- `modules-ui/modules-editor.jsx` — the Modules editor pane (states: empty/default, populated, inline-add, validation, AI-infer loading, AI-infer diff/preview).
- `modules-ui/modules-views.jsx` — `ProjectsOverview` page, `BatchBuilder` modal (Global ON/OFF), `QueueWithModules` panel, `ResumeBanner`.
- `modules-ui/modules-shell.jsx` — shows how the panes mount inside the existing chrome (Titlebar / ActivityBar / Sidebar / Main / QueuePanel / StatusBar); also the `SettingsNav` sub-nav.
- `modules-ui/modulesData.js` — sample data; documents the field shapes the UI expects (`{id,name,desc,root,files,lines}`, batch `moduleSelection`, queue-with-modules, resume).
- `modules-ui/modules.css` — all new styles (`.mod-row`, `.diff-row`, `.batch-modal`, `.global-row`, `.toggle`, `.q-modline`, `.resume-banner`, …). Port verbatim into the Svelte app.
- `modules-ui/Modules.html` — design-canvas composition; reference only (artboard scaffolding, not shipped).

Key design decisions the mockup makes (adopt them):
- **Modules editor is its own top-level activity-bar tab** (the `layers` icon), not a Settings sub-tab. Pick a project in the sidebar, manage its modules in the main pane.
- A new **Projects & Modules overview** page (reached via an activity-bar icon).
- Batch builder gains a **Global** toggle that dims the per-target module picker and switches the live job-count formula.
- Queue rows and finding rows get a small **module badge**.
- A subtle, dismissable **resume banner** pinned to the top of the main pane on startup.

## Data Model Changes

### `chatur-core/src/model/project.rs`
Add to `Project`:
```rust
#[serde(default)]
pub modules: Vec<Module>,
```
New struct in same file:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub description: String,
    /// Path relative to project.root_path. Empty string = whole repo.
    pub root_subdir: PathBuf,
}
```
Add `ModuleId` to `chatur-core/src/ids.rs` alongside the other `*Id` newtypes. `#[serde(default)]` keeps existing JSON blobs readable — no migration needed.

`Project::new` initializes `modules` with one default module:
```rust
Module { id: ModuleId::new(), name: "root".into(), description: "Whole project".into(), root_subdir: PathBuf::new() }
```
Default-1-module rule lives here.

Note: the mockup also displays per-module `files` / `lines` counts. These are **derived/display-only** (computed by walking `root_subdir` when the pane loads) — do **not** persist them on `Module`. Expose via a lightweight `module_stats(project_id, module_id)` command if needed, or fold counts into the list response DTO.

### `chatur-core/src/model/batch.rs`
Add to `Batch`:
```rust
#[serde(default)]
pub global: bool,  // true => skip module fanout
```
Add to `BatchTarget`:
```rust
#[serde(default)]
pub module_ids: Option<Vec<ModuleId>>,  // None = all modules
```
Update `Batch::materialize` so `BatchItem` carries a `module_id: Option<ModuleId>`:
- if `batch.global` → one item per `(prompt, target)`, `module_id = None`.
- else → for each target, expand into `(prompt, target, module)` triples using `target.module_ids` if set, else every module of that target's project. `BatchItem` and `item_count` update accordingly. Tests at the bottom of `batch.rs` must be extended.

### `chatur-core/src/model/job.rs`
Add to `Job`:
```rust
#[serde(default)]
pub module_id: Option<ModuleId>,
#[serde(default)]
pub module_root: Option<PathBuf>,   // absolute, resolved at fanout time
#[serde(default)]
pub module_name: Option<String>,    // for prompt injection + UI display
```
Resolver consumes these (below). `module_name` is what the queue/finding **module badge** in the mockup renders.

## Module Inference Agent

New file: `crates/chatur-api/src/modules.rs`.

`async fn infer_modules(project: &Project, agent: &dyn AgentRunner) -> Result<Vec<Module>>`:
1. Walk `project.root_path` two levels deep (reuse any tree helper if present; otherwise `std::fs::read_dir`) and build a compact directory listing with file counts.
2. Build a system prompt: "You are a codebase splitter. Given this directory listing, return JSON `{modules:[{name, description, root_subdir}]}` for large, generic modules (frontend / backend / engine / shared). Prefer 3–8 modules. Never split below top-level dirs unless a top-level dir clearly contains multiple separable products."
3. Use existing `chatur-agent` runner with a one-shot prompt + `--no-tools` (read-only) at `cwd = project.root_path`. Parse JSON from agent output (it already returns structured text per `AgentOutput`).
4. Validate every `root_subdir` exists under `project.root_path`. Drop invalid entries. Assign fresh `ModuleId`s.

The mockup's **AI-infer diff** (`PaneInferDiff` / `inferProposal` in `modulesData.js`) wants a reconciliation against current modules, tagging each row `added | changed | removed | kept`. Do this diff **client-side**: the command returns the proposed `Vec<Module>`; the UI computes the diff buckets against `project.modules` and lets the user cherry-pick before applying. (Backend stays stateless; nothing saves until "Apply".)

Tauri command (`src-tauri/src/commands.rs`):
```rust
#[tauri::command]
async fn infer_project_modules(project_id: ProjectId, state: State<'_, AppState>) -> Result<Vec<Module>>
```
Returns the proposed modules to the UI without saving — UI shows the diff, user accepts / edits / saves via the project-update path.

## Resolver Change (prompt injection + cwd)

`crates/chatur-api/src/resolver.rs::resolve` (line 70):
- After computing `project.root_path`, if `job.module_root` is `Some(path)` use that as the `AgentSpec` cwd; else use `project.root_path` (current behavior).
- Append a new system-prompt block when `job.module_name` is `Some`:
  ```
  Scope: focus on module `{name}` rooted at `{module_root}` (relative to repo: `{root_subdir}`).
  You still have read access to the entire repo at `{project.root_path}` if cross-module context is needed, but your work should target this module.
  ```
  Push into the existing `appended` Vec before the `appended.join("\n\n")` call at line 122.

Important: keep cwd = project root if we want full-repo access for tooling, and put the module path in the prompt only. The decision was "prompt hint + path list in spec" — implement that as: cwd stays at `project.root_path` (so tools see full repo), `module_root` is exposed via a new `AgentSpec` field (e.g. `extra_env("CHATUR_MODULE_ROOT", ...)`) and via the prompt block. This avoids fighting the existing cwd semantics.

## Durable Queue

`crates/chatur-engine/src/queue.rs` — replace `InMemoryJobQueue` with `SqliteJobQueue` backed by the existing `jobs` table.

Storage plan (no migration; status enum already persists):
- Queue order: use the existing `created_at` column as the FIFO key. `reorder` writes a new `queue_position` value into the JSON `data` blob (add `#[serde(default)] pub queue_position: Option<i64>` to `Job`), and `dequeue` selects `WHERE status = 'queued' ORDER BY COALESCE(queue_position, created_at_epoch) ASC LIMIT 1`. Tie-break on `created_at`.
- `enqueue`: insert/update job row with `status='queued'` (reuse `SqliteJobRepo`).
- `dequeue`: `SELECT ... LIMIT 1` then `UPDATE status='running', started_at=now` in a single transaction. Return the deserialized `Job`.
- `cancel`: `UPDATE status='cancelled' WHERE id=? AND status='queued'`.
- `len`: `SELECT COUNT(*) WHERE status='queued'`.
- `wait_for_job` semantics preserved via the existing in-process `tokio::sync::Notify` (queue still owns one; persistence is orthogonal).

Startup rehydration (`crates/chatur-api/src/chatur.rs:73-78`, the existing `reconcile_orphan_jobs`):
- Today it marks orphaned `Running` jobs as `Cancelled`. Change to: mark them `Queued` again (reset `started_at`, increment `attempts`?) so they resume on restart. Keep `Queued` rows untouched — they're now durable.
- Add a configurable `max_attempts` cap so a job that crashes the agent repeatedly can't loop forever; on exceeding it, transition to `Failed` with an error message.
- After reconciliation, no explicit "reload queue into memory" step is needed because dequeue reads SQLite directly.
- Emit a startup summary (count resumed + count discarded-because-module-deleted) so the UI **resume banner** (`modules-ui` `ResumeBanner`) has data to show.

Concurrency note: the API layer and scheduler both clone the queue handle today; the SQLite-backed version stays cheaply cloneable (it holds an `Arc<SqlitePool>` + `Arc<Notify>`).

## Frontend (SvelteKit + Tauri) — port `modules-ui/` mockup

The UI is **SvelteKit** (`ui/src/routes/+page.svelte`, `+layout.svelte`) with Svelte 5 runes, state in `ui/src/lib/store.svelte.js`, Tauri bridge in `ui/src/lib/api.js`. Existing chrome components: `Titlebar`, `ActivityBar`, `Sidebar`, `StatusBar`, `QueuePanel`, `MainHeader`, `TaskGrid`, `SettingsPane`. The mockup reuses all of them — only the main-pane content and a few badges are new.

### Step 0 — styles
Copy `modules-ui/modules.css` into the app (e.g. `ui/src/lib/styles/modules.css`) and import it once in `+layout.svelte`. It depends only on existing token vars, so it drops in. Fix the trailing duplicate/stray braces at the tail of the file while copying.

### Mockup → Svelte component map

| Mockup (React) | Svelte target | Notes |
|---|---|---|
| `ShellActivityBar` (`layers` item) | extend `ui/src/lib/components/ActivityBar.svelte` | add a **Modules** activity tab (layers icon) + a Projects-overview entry |
| `ModulesPane` + `Pane*` states (`modules-editor.jsx`) | new `ui/src/lib/components/modules/ModulesPane.svelte` | top-level activity view; sub-components below |
| `ModRow` / `ModRowEditing` | `modules/ModRow.svelte` | read + inline-edit row; `.mod-row.editing` |
| `ModuleListHeader` / `EmptyHint` / `ScopePill` | `modules/ModuleListHeader.svelte`, `EmptyHint.svelte`, `ScopePill.svelte` | toolbar (Add / Import / Infer), default-state hint |
| `mod-warn` validation (`PaneValidation`) | inline in `ModRow.svelte` | path-not-found (err) + overlap (warn); validate against backend before queue |
| `PaneInferLoading` | `modules/InferLoading.svelte` | spinner + step list while `infer_project_modules` runs |
| `PaneInferDiff` / `DiffRow` | `modules/InferDiff.svelte` | client-side diff of proposal vs current; cherry-pick checkboxes → Apply |
| `ProjectsOverview` (`modules-views.jsx`) | new `modules/ProjectsOverview.svelte` | stats cards + project table w/ module chips + row actions |
| `BatchBuilder` modal | extend the existing batch-creation flow (find it off `+page.svelte` / `TaskGrid.svelte`); new `modules/BatchBuilder.svelte` modal | Global `.toggle`, per-target module `.modchip.sel` picker, live job-count formula |
| `QueueWithModules` | extend `ui/src/lib/components/QueuePanel.svelte` | add `.q-modline` module badge to each queue row |
| finding-row module tag | extend findings/results rendering | small `module: <name>` chip |
| `ResumeBanner` | new `modules/ResumeBanner.svelte` | pinned top of main pane; fed by queue startup summary |

### State + API
- `ui/src/lib/store.svelte.js` — add module state to the project store: current project's `modules`, the active editor pane state (`empty/populated/adding/validation/inferLoading/inferDiff`), and the in-flight infer proposal.
- `ui/src/lib/api.js` — add wrappers:
  - `inferProjectModules(projectId)` → `Vec<Module>`
  - `updateProjectModules(projectId, modules)` → reuse the existing project-update command if present; else add a Tauri command calling `ProjectRepo::update`.
  - `moduleStats(projectId, moduleId)` (optional) for the files/lines counts shown in rows.
- Batch creation command + DTO: accept the new `global` / per-target `module_ids` fields. Job-count preview computed client-side from the same formula in `BatchBuilder` (`prompts × targets` when global, else `Σ prompts × modules_per_target`).
- A project with zero modules auto-shows the single default "root" module — never empty (matches `EmptyHint` / `PaneEmpty`).

### Convert React → Svelte 5 idioms
`useState`→`$state`, `useMemo`/derived→`$derived`, `onChange`→`on:input`/`bind:value`, `className`→`class`, `checked`→`bind:checked`, component props via `$props()`. Reuse the existing `Icon.svelte` for all `<Icon name=.../>` usages in the mockup.

## Tauri Commands to Add (`src-tauri/src/commands.rs`)

```rust
infer_project_modules(project_id) -> Vec<Module>
update_project_modules(project_id, modules: Vec<Module>) -> ()
```
Register both in `src-tauri/src/lib.rs` `invoke_handler!`. Existing `queue_job`, `create_batch`, and batch-creation commands need to accept the new `global` / `module_ids` fields — extend their request DTOs.

## Verification

1. `cargo test -p chatur-core` — new `Batch::materialize` fanout tests (global on/off, module subset, default-1-module project).
2. `cargo test -p chatur-engine` — port the existing `queue.rs` tests to `SqliteJobQueue` using an in-memory SQLite (`sqlite::memory:` already used in `chatur-store` tests).
3. `cargo test -p chatur-api` — unit test on `ProjectSpecResolver` asserting the module prompt block appears when `job.module_name = Some`, and is absent when `None`.
4. Manual e2e (`cargo tauri dev`):
   - Open the **Modules** activity tab, add a project → see 1 default "root" module + empty hint.
   - **Infer with AI** → loading state → diff/preview; cherry-pick rows → Apply → modules saved.
   - Trigger validation: point a module at a non-existent path → err row; overlap a parent dir → warn row.
   - Open **Projects & Modules overview** → all projects with module counts + chips.
   - Create a batch (2 prompts, chatur w/ 3 modules), Global OFF → preview shows 6 jobs; Global ON → picker dims, preview 2 jobs.
   - Queue panel rows show the module badge; finding rows show the `module:` tag.
   - While 5 jobs queued, kill the app (`Ctrl+C` the tauri dev process), restart → queued jobs still present and start running; running-at-shutdown job re-enters Queued; **resume banner** appears with the resumed/discarded counts.
5. Inspect one running job's spawned `pi` invocation (logs at `tracing` target `chatur::agent`) to verify the system-prompt append contains the module block and the cwd matches `project.root_path`.

## Files Touched (summary)

- `crates/chatur-core/src/{ids.rs, model/project.rs, model/batch.rs, model/job.rs}`
- `crates/chatur-api/src/{modules.rs (new), resolver.rs, chatur.rs}`
- `crates/chatur-engine/src/queue.rs` (rewrite as SQLite-backed)
- `crates/chatur-store/src/repo/job.rs` (extend with queue queries if not present)
- `src-tauri/src/{commands.rs, lib.rs}`
- `ui/src/lib/styles/modules.css` (new — ported from `modules-ui/modules.css`)
- `ui/src/lib/components/modules/*` (new: `ModulesPane`, `ModRow`, `ModuleListHeader`, `EmptyHint`, `ScopePill`, `InferLoading`, `InferDiff`, `ProjectsOverview`, `BatchBuilder`, `ResumeBanner`)
- `ui/src/lib/components/{ActivityBar.svelte, QueuePanel.svelte, Sidebar.svelte}` (extend)
- `ui/src/lib/{api.js, store.svelte.js}` and `ui/src/routes/+page.svelte` / `+layout.svelte`

No SQL migration. JSON `#[serde(default)]` everywhere keeps old data forward-compatible.
