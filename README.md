# NETRA

Desktop app for running `pi` coding-agent jobs against local code projects — queue prompts, run batches, aggregate outputs, and review findings, all without leaving your machine.

**Why it exists:** `pi` is powerful but single-threaded and fire-and-forget. NETRA wraps it with a persistent queue, batch orchestration (map → reduce via a reviewer agent), SQLite job history, and a UI so you can throw many prompts at a codebase and get a rolled-up report back.

A batch fans out as the product `prompts × targets × modules`: one job per prompt, per target project, per **module** (a named subset of a repo — every project starts with a single root module spanning the whole repo). Outputs are then reduced by a chosen strategy (`concat`, `schema_merge`, or `reviewer` / `structured_reviewer`). The structured path routes through the Python **planner** sidecar for schema-guaranteed JSON findings, and an optional ChromaDB index gives agents semantic code search.

---

## Prerequisites

| What | Requirement |
|------|-------------|
| Core library + CLI | Rust 1.93+ (edition 2024) |
| Running agent jobs | `pi` v0.74.2 on `PATH`, reachable model server |
| Desktop app | Above + Tauri system libs (`webkit2gtk` etc.), Tauri CLI, Node 20+ |
| ChromaDB (optional) | Auto-bootstraps via `uv` on first enable — see [docs/guides/chromadb.md](docs/guides/chromadb.md) |
| Structured planner (optional) | Python ≥3.10 sidecar in `planner/` — autostarted by default; see [Planner sidecar](#planner-sidecar) |

Install the Tauri CLI once:
```bash
cargo install tauri-cli --version "^2"
```

---

## Build

### Library + CLI (no system deps)

```bash
# from repo root
cargo build --workspace            # all 7 library crates
cargo test  --workspace            # test suite (no pi or model server needed)

cargo build -p netra-cli --release
# binary: target/release/netra
```

### Desktop app

```bash
# install frontend deps once
cd ui && npm install && cd ..

# dev mode (hot-reload UI + Rust recompile on save)
cd src-tauri && cargo tauri dev

# production bundle
cd src-tauri && cargo tauri build
```

> `src-tauri` is a **separate** Cargo workspace. Run `cargo tauri` commands from inside `src-tauri/`, not the repo root.

---

## Configuration

Both the CLI and the desktop app read `netra.toml` from the working directory. All fields optional.

```toml
pi_binary = "pi"
data_dir  = ".netra/data"     # SQLite DB
log_dir   = ".netra/logs"     # per-job log files

[concurrency]
global_max      = 4
per_project_max = 2

[planner]                          # structured-output sidecar
enabled   = true
endpoint  = "http://127.0.0.1:8899"
autostart = true                   # false = start the sidecar yourself
```

Local model setup: `pi` is configured in `~/.pi/agent/config.json` + `models.json`. Dev default is a llama.cpp OpenAI-compatible server at `http://127.0.0.1:8888/v1` running `qwen3.6-35b-a3b`.

### Planner sidecar

`planner/` is a small FastAPI service (`netra-planner`) wrapping `outlines` over the same llama.cpp endpoint. It produces JSON-Schema-constrained output, guaranteeing the `structured_reviewer` reduce strategy returns conformant findings (no best-effort parsing). With `autostart = true` the app spawns it; otherwise:

```bash
cd planner && uv run uvicorn netra_planner.server:app --port 8899
```

---

## Quick start (CLI)

```bash
# register a project
target/release/netra project add myrepo /path/to/code

# queue a job and wait for output
target/release/netra run <project-id> summarize the architecture

# run a batch (multiple prompts → aggregated report)
target/release/netra batch run <project-id> \
  -p "list public API surface" \
  -p "find error handling gaps"
```

---

## Architecture at a glance

```
crates/
  netra-core    # domain types + traits, zero I/O
  netra-agent   # pi RPC process management (NDJSON over stdio)
  netra-store   # SQLite persistence
  netra-engine  # queue, scheduler, batch + module fanout, structured planner client
  netra-chroma  # optional ChromaDB index + MCP server (bootstrapped via uv)
  netra-api     # public headless library facade (+ planner sidecar supervisor)
  netra-cli     # standalone binary

planner/         # Python netra-planner sidecar (FastAPI + outlines, structured JSON)
src-tauri/       # Tauri v2 shell — thin IPC commands over netra-api
ui/              # SvelteKit frontend
```

Dependency direction (enforced): `core` ← `agent/store/engine/chroma` ← `api` ← `cli / src-tauri`.

Full docs: open `docs/index.html` in a browser (static HTML, no build step).

---

## Project structure guide for agents

See `AGENTS.md` — maps each crate and UI directory to its responsibility.
