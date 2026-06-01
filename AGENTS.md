# AGENTS.md

Rust + Tauri v2 desktop app. SvelteKit UI. Orchestrates `pi` CLI agent in RPC mode (NDJSON over stdio) against local code projects. SQLite via `sqlx`.

## Where to look

### Rust workspace (`crates/`)
- `netra-core/` — domain types + traits. No I/O.
  - `src/model/` — `agent, aggregate, batch, finding, job, project, template`. `project.rs` holds `Module` (a named repo subset; every project seeds a root module).
  - `src/traits/` — `aggregator, bus, queue, repo, session, sink, support, transport`
  - `src/ids.rs` — typed ids incl. `ModuleId`. `src/error.rs` — `CoreError`.
- `netra-agent/` — `pi` process lifecycle. `protocol.rs` (NDJSON), `transport.rs` (stdio), `session.rs`, `pool.rs`, `spec.rs`, `mock.rs`.
- `netra-store/` — SQLite persistence. `db.rs`, `sink.rs`, `repo/{batch,job,project,template}.rs`.
- `netra-engine/` — orchestration. `runner.rs`, `scheduler.rs`, `queue.rs`, `batch.rs` (`prompts × targets × modules` fanout), `aggregate.rs`, `bus.rs`, `retry.rs`, `planner.rs` (`StructuredPlanner` trait + `OutlinesHttpPlanner` client to the planner sidecar).
- `netra-chroma/` — optional ChromaDB integration (compiled under the `chromadb` feature). `bootstrap.rs` (uv setup), `indexer.rs`, `client.rs`, `server.rs`, `query.rs`, `mcp.rs` (MCP server pi can call), `prompt.rs`, `handle.rs`, `ignore_rules.rs`, `win.rs`. Python helpers: `*_cli.py`, `*_helper.py`.
- `netra-api/` — facade exposed to Tauri. `netra.rs` (main API), `config.rs`, `resolver.rs`, `modules.rs` (module CRUD/inference), `planner_supervisor.rs` (spawns/monitors the Python sidecar), `notify.rs`.
- `netra-cli/` — headless binary. `main.rs`.

### Python planner sidecar (`planner/`)
`netra_planner/` — FastAPI app (`server.py`) wrapping `outlines` for JSON-Schema-constrained output over the llama.cpp endpoint; `settings.py` (env config, `NETRA_PLANNER_PORT` default 8899). Driven by `netra-engine`'s `OutlinesHttpPlanner`; lifecycle managed by `netra-api`'s `planner_supervisor`.

### Tauri shell (`src-tauri/`)
Separate workspace (needs webkit). `src/commands.rs` = IPC commands → calls `netra-api`. `src/lib.rs`, `main.rs`. Config: `tauri.conf.json`, `netra.toml`.

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
- Domain types live in `netra-core`; never add I/O there.
- Tauri commands are thin — delegate to `netra-api`.
- Full architecture: `docs/concepts/` (architecture, data-flow, design-patterns, glossary). Guides: `docs/guides/`. Open `docs/index.html`.
