# Mini ChatUR Desktop Shell (Tauri)

The desktop app is a thin Tauri v2 shell over the `chatur-api` library. The Rust
side lives in `src-tauri/`; the SvelteKit front-end lives in `ui/`.

> The Tauri shell is a **separate Cargo workspace** (`src-tauri/Cargo.toml` has
> its own `[workspace]` table and is `exclude`d from the root workspace). This
> keeps the main `cargo build --workspace` and CI free of the system webkit
> dependency. Build the shell from `src-tauri/`.

## Architecture

```
ui/ (SvelteKit SPA)
   │  invoke('queue_job', …)        listen('chatur://event')
   ▼                                        ▲
src-tauri/src/commands.rs  ───────►  src-tauri/src/lib.rs
   │  one #[tauri::command] per         manages one Chatur instance;
   │  Chatur method                     forwards DomainEvents to the UI
   ▼
chatur-api  ::  Chatur  (store + agent pool + engine)
```

- `src-tauri/src/lib.rs` — starts one `Chatur` instance, registers it as Tauri
  managed state, and spawns a task that bridges the engine's `DomainEvent`
  stream to the front-end as the `chatur://event` Tauri event.
- `src-tauri/src/commands.rs` — one `#[tauri::command]` per `Chatur` operation;
  each returns `Result<_, String>`.

## Prerequisites

- The Rust toolchain (already required by the workspace).
- **Tauri system dependencies** — on Arch: `webkit2gtk-4.1`, `gtk3`,
  `libsoup3`, `librsvg` (`pacman -S webkit2gtk-4.1`). Other distros: see
  <https://tauri.app/start/prerequisites/>.
- **Tauri CLI**: `cargo install tauri-cli --version '^2'` (or it may already be
  installed — check `cargo tauri --version`).
- **Node.js + npm** (Node 20+).

## Important: run tauri commands from `src-tauri/`

Because `src-tauri` is its own Cargo workspace (excluded from the root
workspace), `cargo tauri …` **must be run from inside `src-tauri/`**. Run at the
repository root it cannot locate the app crate and aborts with
"Couldn't recognize the current folder as a Tauri project".

## First-time setup

```sh
# 1. Front-end dependencies
cd ui
npm install
cd ..

# 2. Generate application icons (the repo ships a 1×1 placeholder).
#    Provide any square PNG; this writes all required sizes into src-tauri/icons.
cd src-tauri
cargo tauri icon ../path/to/logo.png
cd ..
```

## Run in development

```sh
cd src-tauri
cargo tauri dev
```

This runs the SvelteKit dev server (`npm --prefix ../ui run dev`, port 5173) and
launches the desktop window with hot reload. The app reads `chatur.toml` from
the current working directory — see [`docs/cli.md`](./cli.md) for the config
format.

## Build a release bundle

```sh
cd src-tauri
cargo tauri build
```

Bundles land in `src-tauri/target/release/bundle/`.

## Notes

- Running agent jobs from the UI needs the same prerequisites as the CLI: `pi`
  on `PATH` and a reachable model server (see `docs/cli.md`).
- Command arguments cross the IPC boundary as **camelCase** from JavaScript and
  arrive as `snake_case` in Rust (e.g. JS `projectId` → Rust `project_id`).
- The shell and the CLI share one `chatur.toml` and one SQLite database, so
  jobs queued in one are visible in the other.
