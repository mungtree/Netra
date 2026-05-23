# AGENTS.md

Rust + Tauri v2 desktop app. SvelteKit UI. Orchestrates `pi` CLI agent in RPC mode (NDJSON over stdio) against local code projects. SQLite via `sqlx`.

## Where to look

### Rust workspace (`crates/`)
- `chatur-core/` — domain types + traits. No I/O.
  - `src/model/` — `agent, aggregate, batch, finding, job, project, template`
  - `src/traits/` — `aggregator, bus, queue, repo, session, sink, support, transport`
- `chatur-agent/` — `pi` process lifecycle. `protocol.rs` (NDJSON), `transport.rs` (stdio), `session.rs`, `pool.rs`, `spec.rs`, `mock.rs`.
- `chatur-store/` — SQLite persistence. `db.rs`, `sink.rs`, `repo/{batch,job,project,template}.rs`.
- `chatur-engine/` — orchestration. `runner.rs`, `scheduler.rs`, `queue.rs`, `batch.rs`, `aggregate.rs`, `bus.rs`, `retry.rs`.
- `chatur-api/` — facade exposed to Tauri. `chatur.rs` (main API), `config.rs`, `resolver.rs`.
- `chatur-cli/` — headless binary. `main.rs`.

### Tauri shell (`src-tauri/`)
Separate workspace (needs webkit). `src/commands.rs` = IPC commands → calls `chatur-api`. `src/lib.rs`, `main.rs`. Config: `tauri.conf.json`, `chatur.toml`.

### Frontend (`ui/`)
SvelteKit + Vite.
- `src/routes/+page.svelte` — single page.
- `src/lib/store.svelte.js` — reactive store ($state runes).
- `src/lib/api.js` — Tauri IPC wrapper.
- `src/lib/tasks.js`, `batchIo.js`, `reviewFormat.js` — logic helpers.
- `src/lib/components/` — top-level panes (Sidebar, ActivityBar, TaskGrid, SettingsPane, QueuePanel, OutputPane, etc.).
- `src/lib/components/review/` — review subview (PromptList, FindingCard, ResultPane, ReviewHeader, ReviewView, RecentRunsSidebar).

## Conventions
- Edition 2024, Rust 1.93. `thiserror` per crate; `anyhow` only at binary edges.
- `tracing` for logs. `serde`/`serde_json` everywhere.
- Domain types live in `chatur-core`; never add I/O there.
- Tauri commands are thin — delegate to `chatur-api`.
- Full architecture: `PLAN.md`. Deeper docs: `docs/`.
