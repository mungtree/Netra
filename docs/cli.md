# Mini ChatUR CLI

`chatur` is the headless command-line interface to Mini ChatUR. It drives the
`chatur-api` library directly — no Tauri, no UI — and is the quickest way to
queue and run `pi` agent jobs against local code projects.

## Build

From the repository root:

```sh
# Debug build — fast to compile, slower to run
cargo build -p chatur-cli

# Release build — recommended for real use
cargo build -p chatur-cli --release
```

The binary is named `chatur` (not `chatur-cli`):

- debug:   `target/debug/chatur`
- release: `target/release/chatur`

Optionally put it on your `PATH`:

```sh
cargo install --path crates/chatur-cli
# installs `chatur` into ~/.cargo/bin
```

## Configuration

`chatur` reads `chatur.toml` from the current directory by default. The file is
optional — every field has a default. Override the path with `--config`.

Example `chatur.toml`:

```toml
# Path to (or name of) the pi executable.
pi_binary = "pi"

# Where the SQLite database and runtime state live.
data_dir = ".chatur/data"

# Where per-job log files are written.
log_dir = ".chatur/logs"

[concurrency]
global_max = 4       # max agent jobs running at once, all projects
per_project_max = 2  # max agent jobs running at once for one project

# Default model, used when neither the job nor the project sets one.
[default_model]
provider = "llamacpp"
model = "qwen3.6-35b-a3b"
```

State is stored under `data_dir` (`chatur.db`) and `log_dir`. Both directories
are created automatically.

## Prerequisites for running jobs

`project` and `jobs` commands work offline. Actually *running* a job
(`queue` picked up by the scheduler, or `run`) launches a `pi` process, so you
need:

- `pi` installed and on `PATH` (or `pi_binary` set in `chatur.toml`).
- A reachable model. For local models, the model server must be up — e.g. the
  llama.cpp server for `provider = "llamacpp"`.

## Commands

Run `chatur --help` or `chatur <command> --help` for full usage.

### Register a project

```sh
chatur project add <name> <path>
# → project added: <project-id>
```

`<path>` is the project root the agent operates in. The printed `<project-id>`
is used by every other command.

### List projects

```sh
chatur project list
# <project-id>  <name>  <path>
```

### Queue a job (fire and forget)

```sh
chatur queue <project-id> <prompt...>
# → job queued: <job-id>
```

All trailing words form the prompt — no quotes needed:

```sh
chatur queue 1c15cee4-... summarize the architecture of this repo
```

The job runs in the background scheduler. Because the CLI is one-shot, it exits
right after queuing; check progress later with `chatur jobs`.

### Queue a job and wait for it

```sh
chatur run <project-id> <prompt...>
```

`run` queues the job, waits up to 10 minutes for it to finish, then prints the
status and the agent's output:

```
job <job-id> running...
status: Completed
--- output ---
<agent reply>
```

### List a project's jobs

```sh
chatur jobs <project-id>
# <job-id>  <Status>  <prompt>
```

Status is one of `Queued`, `Running`, `Completed`, `Failed`, `Cancelled`.

## Example session

```sh
cargo build -p chatur-cli --release
cd /path/to/your/code/project
PID=$(../mini-chatur/target/release/chatur project add myrepo . | awk '{print $3}')

# Make sure the local model server is running, then:
../mini-chatur/target/release/chatur run "$PID" list the top-level modules
```

## Logs

Every job's event stream is written as JSON Lines to:

```
<log_dir>/<YYYY-MM-DD>/<job-id>.jsonl
```

Inspect a run after the fact, e.g.:

```sh
cat .chatur/logs/2026-05-22/<job-id>.jsonl | jq .
```
