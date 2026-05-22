# Mini ChatUR — Architecture & Build Plan

A Rust + Tauri desktop app that orchestrates `pi` coding-agent processes against
local code projects: queue queries, build and run prompt batches, aggregate
batched outputs (map → reduce via a reviewer agent), and log everything.

## 1. Stack

| Concern        | Choice |
|----------------|--------|
| Language       | Rust 1.93, `tokio` async runtime |
| Desktop shell  | Tauri v2 |
| Frontend       | SvelteKit + Vite |
| Persistence    | SQLite via `sqlx` (async, compile-checked queries, migrations) |
| Serialization  | `serde` / `serde_json` |
| Logging        | `tracing` + `tracing-subscriber` + `tracing-appender` |
| Errors         | `thiserror` per crate; `anyhow` only at binary edges |
| Agent          | `pi` v0.74.2 CLI in `--mode rpc` (NDJSON over stdio) |

## 2. Two internal components (per requirement)

1. **Headless API / library** — a Cargo workspace of pure-Rust crates with no
   Tauri dependency. Fully usable on its own (proven by `chatur-cli`).
2. **Tauri shell** — a thin command layer + SvelteKit UI over the library.

## 3. Workspace layout

```
mini-chatur/
├── Cargo.toml                  # workspace manifest
├── crates/
│   ├── chatur-core/            # domain model + traits, zero I/O
│   ├── chatur-agent/           # pi process mgmt + RPC transport
│   ├── chatur-engine/          # queue, scheduler, batch orchestration, aggregation
│   ├── chatur-store/           # SQLite persistence + log sinks
│   ├── chatur-api/             # facade — the public headless library surface
│   └── chatur-cli/             # headless binary; proves library works sans UI
├── src-tauri/                  # Tauri v2 shell, thin commands over chatur-api
└── ui/                         # SvelteKit frontend
```

**Dependency direction (enforced, one-way):**
`core` ← `agent`, `store`, `engine` ← `api` ← `{cli, src-tauri}`.
`chatur-core` depends on no internal crate. Every crate boundary crosses a
**trait**, so any layer is swappable and mockable.

## 4. Crate responsibilities

### chatur-core — domain + interfaces
Pure types and traits. No process spawning, no DB, no tokio I/O.

Types: `Project`, `Job`, `JobStatus` (`Queued|Running|Completed|Failed|Cancelled`),
`Batch`, `BatchItem`, `PromptTemplate`, `ToolPolicy` (`ReadOnly|Allowlist|Full`),
`AgentEvent` (normalized pi stream: `TurnStart|AssistantText|ToolCall|Usage|TurnEnd|Error`),
`AgentOutput { text, structured: Option<Value>, usage }`, `AggregatedResult`.

Traits (the interfaces):
- `AgentTransport` — send prompt, return event stream. Abstracts pi RPC.
- `AgentSession` — lifecycle: spawn / prompt / interrupt / resume / shutdown.
- `ProjectRepo`, `JobRepo`, `BatchRepo`, `TemplateRepo` — Repository pattern.
- `JobQueue` — enqueue / dequeue / peek / reorder / cancel.
- `Aggregator` — `Vec<AgentOutput>` → `AggregatedResult` (Strategy pattern).
- `OutputSink` — receive job output (Observer): file logger, DB, future webhook.
- `EventBus` — publish/subscribe domain + agent events (`tokio::broadcast`).
- `Clock`, `IdGenerator` — injected for deterministic tests.

### chatur-agent — pi process management
- `PiProcess` — spawn `pi --mode rpc`, own the child, stdin writer + stdout
  reader task.
- JSON Lines framing — **strict LF (`\n`) only**; use `BufRead::read_until(b'\n')`.
  Do NOT use a Node-readline-style splitter (it also breaks on U+2028/U+2029,
  which are valid inside JSON strings — per pi docs).
- `RpcTransport` impl `AgentTransport`:
  - Requests `{"type":"<cmd>","id":"<corr-id>",...}`. Run a turn with
    `{"type":"prompt","message":"<text>"}` — text field is `message`.
  - Map pi events (`agent_start`, `turn_start`, `message_start/update/end`,
    `tool_execution_start/update/end`, `turn_end`, `agent_end`, `queue_update`,
    `auto_retry_*`) → normalized `AgentEvent`.
  - Correlate `{"type":"response",...}` to requests by `id`.
  - Handle `extension_ui_request` — auto-respond (select/confirm/input/editor)
    or surface to UI; ack fire-and-forget (notify/setStatus/setWidget).
  - Version/capability detection adapter; pin pi v0.74.2.
- Useful pi commands wired through: `abort`, `steer`, `follow_up`,
  `new_session`, `switch_session`, `fork`, `get_session_stats`,
  `get_last_assistant_text`, `set_model`, `set_thinking_level`, `compact`.
- `AgentPool` — Factory + pool of sessions; global + per-project concurrency
  caps, idle eviction, restart-on-crash with backoff.
- Session reuse via pi `--session` / `--continue` / `new_session` for follow-ups.
- `MockTransport` — scripted events for tests.

### chatur-engine — scheduling, batches, aggregation
- `Scheduler` + worker pool — pull from `JobQueue`, honor concurrency + rate
  limits.
- `JobRunner` — run one job: acquire session, send prompt, stream events →
  sinks + bus, parse output, persist. Retry w/ exponential backoff
  (transient vs permanent), cancellation via `CancellationToken`.
- `BatchExecutor`:
  - **Map** — expand `prompt-set × targets` → N jobs (the prompt-set is loaded
    from a JSON file via `PromptLibrary`, see §6.5); each job asked for
    **structured output** matching the batch's JSON schema; parse + validate
    (repair-or-retry on malformed output).
  - **Reduce** — chosen `Aggregator` combines results.
- Aggregation strategies, registered in an extensible registry:
  - `ConcatAggregator` — raw concatenation.
  - `ReviewerAggregator` — feeds all outputs to a reviewer pi agent
    ("consolidate, dedupe, rank") → single consolidated result.
  - `SchemaMergeAggregator` — merge structured JSON arrays, dedupe by key.
  - `VoteAggregator` / `MapReduceAggregator` — future.

### chatur-store — persistence + logging
- SQLite schema + `sqlx` migrations: `projects`, `jobs`, `batches`,
  `batch_items`, `templates`, `runs`, `events`, `usage`.
- Repo impls of the `chatur-core` repository traits.
- `FileLogSink` impl `OutputSink` — per-job log `logs/<date>/<job-id>.log`
  plus a JSONL event log; rotation.
- `PromptLibrary` — loads and saves prompts and **prompt-sets** as plain JSON
  files under a `prompts/` directory (see §6.5). The JSON files are the
  portable source of truth; the `templates` table mirrors them for querying.
- Report export — render a run to Markdown / JSON.

### chatur-api — facade library
- `Chatur` struct — wires store + agent pool + engine; the single public entry
  point of the headless library.
- Async methods: `add_project`, `queue_job`, `create_batch`, `run_batch`,
  `cancel`, `list_jobs`, `subscribe_events`, …
- `ChaturConfig` from TOML — paths, `pi` binary path, concurrency, default models.

### chatur-cli — headless binary
Thin CLI over `chatur-api` (`chatur queue`, `chatur batch run`, `chatur logs`).
Proves the library is fully usable without Tauri; doubles as an integration
test harness.

### src-tauri — Tauri shell
- `#[tauri::command]` wrappers, each calling one `chatur-api` method.
- `tauri::State<Chatur>` holds the library instance.
- Bridge `EventBus` → Tauri events to stream live job/agent progress to the UI.

### ui — SvelteKit
A desktop shell ported from the `example-ui/` React prototype (see §10): an
IDE-style layout — titlebar · activity bar · projects sidebar · main wizard
(task grid + last-run findings) · live queue panel · status bar. Dark theme,
orange accent. Later views: Batch builder · Run/batch detail · Logs · Settings.

## 5. Design patterns applied
Repository (persistence) · Strategy (aggregators) · Factory + Object Pool
(agent sessions) · Observer (`OutputSink`, `EventBus`) · Facade (`chatur-api`) ·
Command (jobs) · Builder (batch construction) · Adapter (pi RPC version
shims) · Dependency injection (`Clock`, `IdGenerator`, traits everywhere).

## 6. Extra features (beyond the four stated goals)
1. **Sandboxed edits** — run agent in a git worktree/branch; capture diff;
   approve / reject / apply.
2. **Dry-run / read-only mode** — `--no-tools` or read-only tool allowlist for
   analysis-only batches.
3. **Multi-model comparison** — same prompt across models, side-by-side.
4. **Cost & token tracking** — capture pi usage events; per-job/batch/project
   rollups with budget caps.
5. **Prompt library (JSON files)** — prompts and *prompt-sets* are stored as
   plain JSON files under a `prompts/` directory: human-readable,
   version-controllable, diffable, and shareable. A **prompt-set** file holds
   many prompts (each with name, body, variables, optional output schema) and
   is the direct input to a batch's map step — point a batch at a prompt-set
   file and every prompt in it fans out across the targets. The `chatur-store`
   `PromptLibrary` loads/saves these files and mirrors imported prompts into
   the SQLite `templates` table (which carries variables + a version number)
   for fast querying. JSON files remain the source of truth.
6. **Session resume** — continue a prior pi session for follow-ups.
7. **Scheduling** — cron-like recurring batches.
8. **Desktop notifications** on batch completion.
9. **Plugin/extension system** — dynamic registry for custom `Aggregator` and
   `OutputSink`; later WASM/dylib plugins.
10. **Run history & reproducibility** — store prompt + model + config snapshot;
    one-click re-run.
11. **Project profiles** — per-project default model, tool policy, appended
    system prompt.
12. **Interrupt / steering** — pause a running job, inject guidance.

## 7. Cross-cutting quality
- `tracing` everywhere; file + console subscribers.
- Tests: unit (`MockTransport`, in-memory repos), trait-level contract tests,
  integration (`chatur-cli` against real `pi`).
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
- Cargo feature flags keep crates/extras optional.

## 8. Build phases / milestones
- **P0 Scaffold** — workspace, 6 crates, CI, `chatur-core` types + traits,
  TOML config loading.
- **P1 Agent transport** — reverse-engineer pi RPC, `PiProcess`,
  `RpcTransport`, `MockTransport`, `AgentPool`.
- **P2 Store** — SQLite schema, migrations, repo impls, `FileLogSink`.
- **P3 Engine (single jobs)** — `JobQueue`, `Scheduler`, `JobRunner`, retry,
  cancel, `EventBus`.
- **P4 chatur-api + cli** — facade + headless CLI; end-to-end single job works.
- **P5 Batches** ✅ — `BatchExecutor` (prompts × targets → jobs → reduce),
  `AggregatorRegistry` with `Concat` + `SchemaMerge`, agent-backed `Reviewer`
  reduce step, `chatur batch` CLI, Tauri batch commands, wired `TaskGrid` +
  `LastRun` UI.
- **P6 Tauri shell** — commands, event streaming, state.
- **P7 UI** — SvelteKit desktop shell ported from `example-ui/` — see §10.
- **P8 Extras** — sandboxed edits, cost tracking, comparison, plugins,
  scheduling.

## 9. Known risk
`pi` RPC mode protocol is documented at https://pi.dev/docs/latest/rpc and
captured in project memory. The earlier flaky error
(`Cannot read properties of undefined (reading 'startsWith')`) was a wrong
request field — the prompt text field is `message`, not `prompt`. Residual
risk: protocol is version-tied. P1 must pin pi v0.74.2 and build a defensive
protocol adapter with capability detection, plus an `extension_ui_request`
handler so agent extensions cannot stall a headless run.

## 10. UI integration — `example-ui/` → SvelteKit shell (P7)

`example-ui/` is a React (CDN) prototype of the desired desktop look: an
IDE-style layout (titlebar · activity bar · projects sidebar · main wizard ·
queue panel · status bar), dark `#08090a` background, orange `#ff7a1a` accent,
Geist + JetBrains Mono. The real front-end is **SvelteKit** (decision kept from
§1); the prototype's design is ported into it.

### Decisions
- **Framework:** stay on SvelteKit. `example-ui/styles.css` is reused almost
  verbatim (framework-agnostic); the React JSX components are rewritten as
  Svelte 5 components.
- **Scope of P7:** build the *entire* visual shell now. Wire every panel the
  current backend supports to real data; render not-yet-built features as
  honest empty/placeholder states — never fake data.
- **Task cards = batch presets.** The 8 task cards (Find Bugs, Find
  Vulnerabilities, Refactor, …) are **prompt-sets**: each is a JSON file in
  `prompts/` (see §6.5) that a batch runs over the project. Until P5 lands the
  `BatchExecutor`, the cards render disabled with a "needs batches (P5)" tip.

### Steps
1. **Design system** — port `example-ui/styles.css` to `ui/src/app.css`
   (tokens + component styles); import it once in `+layout.svelte`. Bundle the
   Geist and JetBrains Mono fonts as local assets (no CDN dependency at
   runtime); the `--font-*` stacks already carry fallbacks.
2. **Icons** — port `example-ui/icons.jsx` to `ui/src/lib/Icon.svelte` (a
   `name` + `size` prop renders the matching inline SVG).
3. **Component port** — recreate each prototype component as a Svelte 5
   component in `ui/src/lib/components/`: `Titlebar`, `ActivityBar`,
   `Sidebar`, `MainHeader`, `TaskGrid`, `LastRun`, `QueuePanel`, `StatusBar`.
4. **API layer** — `ui/src/lib/api.js`: one thin async wrapper per Tauri
   command, plus `subscribeEvents()` over `listen('chatur://event')`. This is
   the only seam between UI and backend; components never call `invoke`
   directly.
5. **State store** — `ui/src/lib/store.svelte.js`: holds projects, jobs,
   live events, and the selected project; loaded on mount and refreshed on
   every `chatur://event`.
6. **Wire what the backend supports today:**
   - *Sidebar* ← `list_projects`; per-project status dot + job count derived
     from job statuses; "add project" dialog ← `add_project`.
   - *Queue panel* ← jobs grouped Running / Pending / Completed across
     projects; cancel ← `cancel_job`; live updates from `chatur://event`.
   - *Status bar* ← derived running/queued/done counts; model name from
     `ChaturConfig`.
   - *Activity feed* ← the `chatur://event` stream.
7. **Placeholder until P5** — `TaskGrid` (batch presets) and `LastRun`
   (findings with severity) render empty/disabled states; no mock data.
8. **Backend surface** — Tauri commands stay 1:1 with `chatur-api`. If
   per-project N+1 job fetches prove heavy, add a read-only
   `Chatur::project_summaries()` (project + counts) and one matching command.
9. **Build** — `tauri.conf.json` already builds/serves `ui/`; run the shell
   with `cargo tauri dev` from `src-tauri/` (see `docs/tauri.md`).

### Later UI phases (post-P5)
- `TaskGrid` cards create + run batches via `BatchExecutor`; card metadata
  (prompt count, ETA) comes from the prompt-set file.
- `LastRun` findings come from structured batch output consolidated by the
  `ReviewerAggregator`; severity from the output schema.
- Queue progress bars driven by per-batch-item events.
- Cost/token panel and local-model system stats (GPU, tok/s) — needs the P8
  cost-tracking feature and a model-stats probe.
- Command palette (⌘K) and task shortcuts (⌘1–8).
