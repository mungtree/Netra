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
Views: Project list · Job queue (live, reorderable) · Batch builder
(template + target picker + strategy) · Run/batch detail (live event stream,
per-item outputs, aggregated result) · Template library · Logs viewer ·
Settings.

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
- **P5 Batches** — `BatchExecutor`, structured output, `Concat` + `Reviewer`
  aggregators.
- **P6 Tauri shell** — commands, event streaming, state.
- **P7 UI** — SvelteKit views.
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
