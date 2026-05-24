# Mini ChatUR

Desktop app for running `pi` coding-agent jobs against local code projects — queue prompts, run batches, aggregate outputs, and review findings, all without leaving your machine.

**Why it exists:** `pi` is powerful but single-threaded and fire-and-forget. Mini ChatUR wraps it with a persistent queue, batch orchestration (map → reduce via a reviewer agent), SQLite job history, and a UI so you can throw many prompts at a codebase and get a rolled-up report back.

---

## Prerequisites

| What | Requirement |
|------|-------------|
| Core library + CLI | Rust 1.93+ (edition 2024) |
| Running agent jobs | `pi` v0.74.2 on `PATH`, reachable model server |
| Desktop app | Above + Tauri system libs (`webkit2gtk` etc.), Tauri CLI, Node 20+ |
| ChromaDB (optional) | Auto-bootstraps via `uv` on first enable — see [docs/guides/chromadb.md](docs/guides/chromadb.md) |

Install the Tauri CLI once:
```bash
cargo install tauri-cli --version "^2"
```

---

## Build

### Library + CLI (no system deps)

```bash
# from repo root
cargo build --workspace            # all 6 library crates
cargo test  --workspace            # test suite (no pi or model server needed)

cargo build -p chatur-cli --release
# binary: target/release/chatur
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

Both the CLI and the desktop app read `chatur.toml` from the working directory. All fields optional.

```toml
pi_binary = "pi"
data_dir  = ".chatur/data"     # SQLite DB
log_dir   = ".chatur/logs"     # per-job log files

[concurrency]
global_max      = 4
per_project_max = 2
```

Local model setup: `pi` is configured in `~/.pi/agent/config.json` + `models.json`. Dev default is a llama.cpp OpenAI-compatible server at `http://127.0.0.1:8888/v1` running `qwen3.6-35b-a3b`.

---

## Quick start (CLI)

```bash
# register a project
target/release/chatur project add myrepo /path/to/code

# queue a job and wait for output
target/release/chatur run <project-id> summarize the architecture

# run a batch (multiple prompts → aggregated report)
target/release/chatur batch run <project-id> \
  -p "list public API surface" \
  -p "find error handling gaps"
```

---

## Architecture at a glance

```
crates/
  chatur-core    # domain types + traits, zero I/O
  chatur-agent   # pi RPC process management (NDJSON over stdio)
  chatur-store   # SQLite persistence
  chatur-engine  # queue, scheduler, batch orchestration
  chatur-api     # public headless library facade
  chatur-cli     # standalone binary

src-tauri/       # Tauri v2 shell — thin IPC commands over chatur-api
ui/              # SvelteKit frontend
```

Dependency direction (enforced): `core` ← `agent/store/engine` ← `api` ← `cli / src-tauri`.

Full docs: open `docs/index.html` in a browser (static HTML, no build step).

---

## Project structure guide for agents

See `AGENTS.md` — maps each crate and UI directory to its responsibility.
