# Modules + Durable Queue Plan

## Context

Mini-chatur fans batch jobs out as `prompts × targets`. When a "target" is a large codebase, each agent has to grep the entire tree, which inflates context and blurs scope. This plan adds an explicit **module** layer between project and job so the batch fanout becomes `prompts × targets × modules`. A user can either let an AI agent infer modules from the repo or define them by hand in the Tauri UI; a per-batch **global** switch skips module fanout entirely.

Second goal: the in-memory `InMemoryJobQueue` loses every queued job on restart. Rehydrate the queue from SQLite on startup so an app restart resumes work instead of orphaning it.

User decisions (locked in):
- Scope enforcement: **prompt hint + module path passed in `AgentSpec`** — agents keep full repo access.
- Module shape: **`{name, description, root_subdir}`** — single subdir per module.
- Queue persistence: **reuse existing SQLite `jobs` table**, no new infra.
- Module storage: **embed `modules: Vec<Module>` in `Project`** — no schema migration.

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
Resolver consumes these (below).

## Module Inference Agent

New file: `crates/chatur-api/src/modules.rs`.

`async fn infer_modules(project: &Project, agent: &dyn AgentRunner) -> Result<Vec<Module>>`:
1. Walk `project.root_path` two levels deep (reuse any tree helper if present; otherwise `std::fs::read_dir`) and build a compact directory listing with file counts.
2. Build a system prompt: "You are a codebase splitter. Given this directory listing, return JSON `{modules:[{name, description, root_subdir}]}` for large, generic modules (frontend / backend / engine / shared). Prefer 3–8 modules. Never split below top-level dirs unless a top-level dir clearly contains multiple separable products."
3. Use existing `chatur-agent` runner with a one-shot prompt + `--no-tools` (read-only) at `cwd = project.root_path`. Parse JSON from agent output (it already returns structured text per `AgentOutput`).
4. Validate every `root_subdir` exists under `project.root_path`. Drop invalid entries. Assign fresh `ModuleId`s.

Tauri command (`src-tauri/src/commands.rs`):
```rust
#[tauri::command]
async fn infer_project_modules(project_id: ProjectId, state: State<'_, AppState>) -> Result<Vec<Module>>
```
Returns the proposed modules to the UI without saving — UI shows a diff, user accepts / edits / saves via the existing project-update path.

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

Concurrency note: the API layer and scheduler both clone the queue handle today; the SQLite-backed version stays cheaply cloneable (it holds an `Arc<SqlitePool>` + `Arc<Notify>`).

## Frontend (SvelteKit + Tauri)

New component `ui/src/lib/components/ModulesEditor.svelte`:
- Lives in project settings (extend `SettingsPane.svelte` or add a route — pick whichever the existing project-detail view uses; check `+page.svelte` routing).
- Lists `project.modules` rows: name, description, root_subdir (folder-picker constrained to under `project.root_path`).
- Buttons: **Add module**, **Delete**, **Infer with AI** (calls `infer_project_modules`, shows proposed modules in a diff view, user accepts to overwrite).
- A project with zero modules auto-shows the single default "root" module — never empty.

Batch creation UI (find in `ui/src/routes/+page.svelte` and `TaskGrid.svelte`):
- Add a **Global** toggle (Svelte `<input type="checkbox" bind:checked={batch.global}>`).
- When non-global, show a per-target module multi-select (default: all modules selected).
- Show projected job count = `prompts × Σ modules_per_target` (or `× targets` when global). Reuse / extend `Batch::item_count` semantics for the preview.

API bridge (`ui/src/lib/api.js`): add `inferProjectModules(projectId)`, `updateProjectModules(projectId, modules)` (the second can reuse the existing project-update command if one exists; otherwise add a Tauri command that calls `ProjectRepo::update`).

## Tauri Commands to Add (`src-tauri/src/commands.rs`)

```rust
infer_project_modules(project_id) -> Vec<Module>
update_project_modules(project_id, modules: Vec<Module>) -> ()
```
Existing `queue_job` and batch-creation commands need to accept the new `global` / `module_ids` fields — extend their request DTOs.

## Verification

1. `cargo test -p chatur-core` — new `Batch::materialize` fanout tests (global on/off, module subset, default-1-module project).
2. `cargo test -p chatur-engine` — port the existing `queue.rs` tests to `SqliteJobQueue` using an in-memory SQLite (`sqlite::memory:` already used in `chatur-store` tests).
3. `cargo test -p chatur-api` — unit test on `ProjectSpecResolver` asserting the module prompt block appears when `job.module_name = Some`, and is absent when `None`.
4. Manual e2e (`cargo tauri dev`):
   - Add a project, see 1 default module.
   - Press **Infer with AI**, confirm proposed modules show up; save them.
   - Create a batch with 2 prompts, 1 project (3 modules), global OFF → expect 6 jobs in the queue panel.
   - Same batch with global ON → 2 jobs.
   - While 5 jobs are queued, kill the app (`Ctrl+C` the tauri dev process), restart, confirm queued jobs still appear and start running. Confirm the running-at-shutdown job re-enters Queued.
5. Inspect one running job's spawned `pi` invocation (logs at `tracing` target `chatur::agent`) to verify the system-prompt append contains the module block and the cwd matches `project.root_path`.

## Files Touched (summary)

- `crates/chatur-core/src/{ids.rs, model/project.rs, model/batch.rs, model/job.rs}`
- `crates/chatur-api/src/{modules.rs (new), resolver.rs, chatur.rs}`
- `crates/chatur-engine/src/queue.rs` (rewrite as SQLite-backed)
- `crates/chatur-store/src/repo/job.rs` (extend with queue queries if not present)
- `src-tauri/src/commands.rs`
- `ui/src/lib/{api.js, components/ModulesEditor.svelte (new), components/SettingsPane.svelte, components/TaskGrid.svelte}` and `ui/src/routes/+page.svelte`

No SQL migration. JSON `#[serde(default)]` everywhere keeps old data forward-compatible.
