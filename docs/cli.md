# NETRA CLI

`netra` is the headless command-line interface to NETRA. It drives the
`netra-api` library directly — no Tauri, no UI — and is the quickest way to
queue and run `pi` agent jobs against local code projects.

## Build

From the repository root:

```sh
# Debug build — fast to compile, slower to run
cargo build -p netra-cli

# Release build — recommended for real use
cargo build -p netra-cli --release
```

The binary is named `netra` (not `netra-cli`):

- debug:   `target/debug/netra`
- release: `target/release/netra`

Optionally put it on your `PATH`:

```sh
cargo install --path crates/netra-cli
# installs `netra` into ~/.cargo/bin
```

## Configuration

`netra` reads `netra.toml` from the current directory by default. The file is
optional — every field has a default. Override the path with `--config`.

Example `netra.toml`:

```toml
# Path to (or name of) the pi executable.
pi_binary = "pi"

# Where the SQLite database and runtime state live.
data_dir = ".netra/data"

# Where per-job log files are written.
log_dir = ".netra/logs"

[concurrency]
global_max = 4       # max agent jobs running at once, all projects
per_project_max = 2  # max agent jobs running at once for one project

# Default model, used when neither the job nor the project sets one.
[default_model]
provider = "llamacpp"
model = "qwen3.6-35b-a3b"
```

State is stored under `data_dir` (`netra.db`) and `log_dir`. Both directories
are created automatically.

## Prerequisites for running jobs

`project` and `jobs` commands work offline. Actually *running* a job
(`queue` picked up by the scheduler, or `run`) launches a `pi` process, so you
need:

- `pi` installed and on `PATH` (or `pi_binary` set in `netra.toml`).
- A reachable model. For local models, the model server must be up — e.g. the
  llama.cpp server for `provider = "llamacpp"`.

## Commands

Run `netra --help` or `netra <command> --help` for full usage.

### Register a project

```sh
netra project add <name> <path>
# → project added: <project-id>
```

`<path>` is the project root the agent operates in. The printed `<project-id>`
is used by every other command.

### List projects

```sh
netra project list
# <project-id>  <name>  <path>
```

### Queue a job (fire and forget)

```sh
netra queue <project-id> <prompt...>
# → job queued: <job-id>
```

All trailing words form the prompt — no quotes needed:

```sh
netra queue 1c15cee4-... summarize the architecture of this repo
```

The job runs in the background scheduler. Because the CLI is one-shot, it exits
right after queuing; check progress later with `netra jobs`.

### Queue a job and wait for it

```sh
netra run <project-id> <prompt...>
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
netra jobs <project-id>
# <job-id>  <Status>  <prompt>
```

Status is one of `Queued`, `Running`, `Completed`, `Failed`, `Cancelled`.

### Run a batch (a series of prompts, aggregated)

A batch runs a *series* of prompts against a project, then combines the
per-prompt outputs into one result. Each `--prompt` (short `-p`) adds one prompt
to the series:

```sh
netra batch run <project-id> \
  -p "Find logic bugs." \
  -p "Find performance issues." \
  -p "Find missing tests." \
  --strategy reviewer
```

`batch run` creates the batch, runs every prompt as its own job through the
scheduler, waits for all of them, aggregates, and prints the result:

```
batch <batch-id> running (3 prompts)...
status: Completed
--- aggregated result (3 outputs) ---
<consolidated summary>
```

The `--strategy` flag selects the reduce step:

- `concat` (default) — concatenates every output verbatim.
- `schema_merge` — merges each output's structured JSON into one array.
- `reviewer` — runs one more agent that consolidates, dedupes, and ranks.

List and inspect batches:

```sh
netra batch list
# <batch-id>  <Status>  <name>  (<n> items)

netra batch show <batch-id>
# prints the batch and its aggregated result
```

## Example session

```sh
cargo build -p netra-cli --release
cd /path/to/your/code/project
PID=$(../netra/target/release/netra project add myrepo . | awk '{print $3}')

# Make sure the local model server is running, then:
../netra/target/release/netra run "$PID" list the top-level modules
```

## Logs

Every job's event stream is written as JSON Lines to:

```
<log_dir>/<YYYY-MM-DD>/<job-id>.jsonl
```

Inspect a run after the fact, e.g.:

```sh
cat .netra/logs/2026-05-22/<job-id>.jsonl | jq .
```
