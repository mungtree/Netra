# AGENTS.md

Rust + Tauri v2 desktop app. SvelteKit UI. Orchestrates `pi` CLI agent in RPC mode (NDJSON over stdio) against local code projects. SQLite via `sqlx`.

## Where to look

### Rust workspace (`crates/`)
- `chatur-core/` — domain types + traits. No I/O.
  - `src/model/` — `agent, aggregate, batch, finding, job, project, template`. `project.rs` holds `Module` (a named repo subset; every project seeds a root module).
  - `src/traits/` — `aggregator, bus, queue, repo, session, sink, support, transport`
  - `src/ids.rs` — typed ids incl. `ModuleId`. `src/error.rs` — `CoreError`.
- `chatur-agent/` — `pi` process lifecycle. `protocol.rs` (NDJSON), `transport.rs` (stdio), `session.rs`, `pool.rs`, `spec.rs`, `mock.rs`.
- `chatur-store/` — SQLite persistence. `db.rs`, `sink.rs`, `repo/{batch,job,project,template}.rs`.
- `chatur-engine/` — orchestration. `runner.rs`, `scheduler.rs`, `queue.rs`, `batch.rs` (`prompts × targets × modules` fanout), `aggregate.rs`, `bus.rs`, `retry.rs`, `planner.rs` (`StructuredPlanner` trait + `OutlinesHttpPlanner` client to the planner sidecar).
- `chatur-chroma/` — optional ChromaDB integration (compiled under the `chromadb` feature). `bootstrap.rs` (uv setup), `indexer.rs`, `client.rs`, `server.rs`, `query.rs`, `mcp.rs` (MCP server pi can call), `prompt.rs`, `handle.rs`, `ignore_rules.rs`, `win.rs`. Python helpers: `*_cli.py`, `*_helper.py`.
- `chatur-api/` — facade exposed to Tauri. `chatur.rs` (main API), `config.rs`, `resolver.rs`, `modules.rs` (module CRUD/inference), `planner_supervisor.rs` (spawns/monitors the Python sidecar), `notify.rs`.
- `chatur-cli/` — headless binary. `main.rs`.

### Python planner sidecar (`planner/`)
`chatur_planner/` — FastAPI app (`server.py`) wrapping `outlines` for JSON-Schema-constrained output over the llama.cpp endpoint; `settings.py` (env config, `CHATUR_PLANNER_PORT` default 8899). Driven by `chatur-engine`'s `OutlinesHttpPlanner`; lifecycle managed by `chatur-api`'s `planner_supervisor`.

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
- Full architecture: `docs/concepts/` (architecture, data-flow, design-patterns, glossary). Guides: `docs/guides/`. Open `docs/index.html`.
