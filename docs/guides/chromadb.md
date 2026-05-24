# ChromaDB Integration (opt-in)

Mini ChatUR can wire a local [ChromaDB](https://www.trychroma.com/) vector
store into the `pi` agent so it can semantically search a codebase before
answering. The integration is **off by default** — every install path, every
process, and every config mutation is gated on the `[chromadb] enabled = true`
switch in `chatur.toml`. With the switch off, the rest of the app is
unaffected.

This guide covers what the integration does, how to turn it on, and how the
pieces fit together.

---

## What you get

1. A managed Python venv under `~/.chatur/chroma-venv` with
   [`chromadb`](https://pypi.org/project/chromadb/) and
   [`chroma-mcp`](https://pypi.org/project/chroma-mcp/) pre-installed.
2. A local chroma HTTP server (`http://127.0.0.1:8765` by default) spawned and
   supervised by Mini ChatUR.
3. An `mcpServers.chroma` entry added to `~/.pi/agent/config.json` (kept for
   future use — pi's `--tools` allowlist currently filters MCP tools out, so
   the agent reaches chroma via the `chatur-chroma` CLI instead; see
   [How the agent uses ChromaDB](#how-the-agent-uses-chromadb)).
4. A per-project collection (`chatur_<project_id>`) populated from the project
   tree, respecting `.gitignore` plus a built-in binary blacklist plus your
   own glob list.
5. A **ChromaDB** page in the desktop UI for server lifecycle, indexing,
   collection management, and settings.
6. A "Use ChromaDB" checkbox on the quick-prompt form that signals the
   agent to use the MCP tools.

---

## Crossplatform install

The integration uses [`uv`](https://github.com/astral-sh/uv) so the same
recipe works on Linux, macOS, and Windows.

1. **First time only** — clicking *Install* in the ChromaDB pane does:
   - `which uv` → if missing, runs the official one-liner installer
     (`curl … | sh` on Linux/macOS; `irm … | iex` on Windows).
   - `uv venv ~/.chatur/chroma-venv --python 3.11`.
   - `uv pip install --python <venv> chromadb chroma-mcp`.
2. **Every launch** (when `enabled = true` and `auto_start = true`):
   - bootstraps the venv if missing (no-op when ready),
   - registers `mcpServers.chroma` in `~/.pi/agent/config.json` (first edit
     backs the file up to `config.json.chatur-bak`),
   - starts the chroma server,
   - polls `/api/v2/heartbeat` until it returns 200 (timeout 30s).

No Docker. No global Python pollution. Removing the integration is `rm -rf
~/.chatur/chroma-venv ~/.chatur/chroma-data` plus deleting the `chroma`
entry from your pi config (the *unregister* path does this for you).

---

## Configuration

In `chatur.toml`:

```toml
[chromadb]
enabled              = true     # master switch — default false
host                 = "127.0.0.1"
port                 = 8765
data_dir             = "/home/you/.chatur/chroma-data"
auto_start           = true
max_file_size_bytes  = 1048576  # skip files larger than 1 MiB
extra_ignore_globs   = ["*.log", "vendor/**"]
embedding_model      = "default" # see "Embedding model" below
embedding_model_custom = ""      # only used when embedding_model = "custom"
```

Every field has a default. A missing `[chromadb]` section is treated as
`enabled = false` so older config files keep working untouched.

You can also edit these values live in the *ChromaDB → Settings* tab; the
master switch is persisted via `chroma_set_enabled` and takes effect after a
restart (the runtime handle is constructed during `Chatur::start`).

---

## Embedding model

Chroma 1.x requires client-side embeddings, so Mini ChatUR runs a small
Python helper inside the managed venv for every upsert and every query. The
same embedding function must be used for both — switching models invalidates
existing vectors (dimensions change) so the *Settings* save flow prompts to
drop and re-index the affected collections.

Curated presets (set `embedding_model` to the preset id):

| Preset id   | HF model                                       | Dim  | Notes                                             |
|-------------|------------------------------------------------|------|---------------------------------------------------|
| `default`   | `sentence-transformers/all-MiniLM-L6-v2` (ONNX) | 384  | Bundled with chromadb. No extra download.         |
| `jina-code` | `jinaai/jina-embeddings-v2-base-code`          | 768  | Code-tuned, 8k context, ~300 MB. Good default for code. |
| `coderank`  | `nomic-ai/CodeRankEmbed`                       | 768  | Code-search SOTA-ish open weights, ~550 MB.       |
| `sfr-code`  | `Salesforce/SFR-Embedding-Code-400M_R`         | 1024 | Strong on CoIR benchmark, ~1.5 GB, slower.        |
| `bge-code`  | `BAAI/bge-code-v1`                             | 1536 | Largest preset, ~6 GB.                            |
| `custom`    | (whatever you set in `embedding_model_custom`) | —    | Any HuggingFace sentence-transformers model id.   |

Non-default presets are downloaded by `sentence-transformers` to
`~/.cache/huggingface/` on first use. The first index or query after a switch
will block while the model downloads; subsequent calls are fast.

Switching models from the UI calls `chroma_set_embedding_model`, which
returns the list of `chatur_*` collections currently on the server. The UI
then asks for confirmation before calling `chroma_drop_and_reindex` to wipe
and rebuild them.

---

## Indexing rules

Files are walked with the `ignore` crate (same logic as `ripgrep`), which
honours `.gitignore`, `.ignore`, and `.git/info/exclude` automatically.

On top of that, two more filters apply:

* **Built-in binary blacklist** — extensions skipped unconditionally:
  `png jpg jpeg gif webp svg ico bmp tiff pdf zip tar gz bz2 xz 7z rar exe
  dll so dylib bin wasm a o obj lib woff woff2 ttf otf eot mp3 mp4 mov avi
  mkv ogg wav flac webm parquet onnx safetensors pt ckpt h5 pkl class jar
  war`.
* **`extra_ignore_globs`** — your own additions. A pattern prefixed with
  `!` re-includes (e.g. `!important.png`).

Files above `max_file_size_bytes` are skipped with a `Skipped { reason: "too
large" }` progress event. Non-UTF8 files are skipped with `reason:
"non-utf8"`.

Each text file is chunked into 800-character windows with 100 characters of
overlap, line-aware. Metadata persisted per chunk: `path, chunk_idx,
line_start, line_end, sha, size`. The chunk SHA is the file SHA, so re-indexing
unchanged files produces the same upsert ids — chroma deduplicates by id.

---

## Tauri / IPC reference

JS bindings live in `ui/src/lib/api.js`. Backend wrappers in
`src-tauri/src/commands.rs`.

| Command                          | Purpose                                        |
|----------------------------------|------------------------------------------------|
| `chroma_status`                  | Return `{enabled, installed, mcp_registered, server, config}` |
| `chroma_install`                 | uv + venv bootstrap (idempotent)              |
| `chroma_start` / `_stop` / `_restart` | Server lifecycle                          |
| `chroma_list_collections`        | List collections on the server                 |
| `chroma_collection_files`        | List files indexed in a project collection     |
| `chroma_delete_collection`       | Drop a project's collection                    |
| `chroma_index_project`           | Walk + chunk + upsert; streams `chatur://chroma` events |
| `chroma_update_settings`         | Save `ChromaConfig` to `chatur.toml`           |
| `chroma_set_enabled`             | Toggle the master switch (restart required)    |
| `chroma_set_embedding_model`     | Persist `embedding_model` (+ optional custom HF id); returns whether reindex is needed and which `chatur_*` collections are affected |
| `chroma_drop_and_reindex`        | Drop the listed project collections and re-index each (streams `chatur://chroma` events) |
| `chroma_query`                   | Manual semantic-search query, returns `Vec<QueryHit>` |

The `queue_job` and `create_batch` commands accept an optional
`useChromadb: bool`. When `true`, the resolver adds a system-prompt
fragment instructing the agent to query collection
`chatur_<project_id>` before answering. When the server is not running,
the flag is a silent no-op.

The frontend listens on the `chatur://chroma` event channel for
install / index progress (`install_started`, `install_finished`,
`started`, `file`, `skipped`, `finished`).

---

## How the agent uses ChromaDB

`pi` is spawned with a strict `--tools` allowlist (`read,bash`) and that
filter applies to MCP tools too, so the `chroma-mcp` server registered in
`~/.pi/agent/config.json` is invisible to the model. To work around that we
ship a tiny CLI — `chatur-chroma` — that the agent invokes through its
existing `bash` tool.

When you queue a job (or batch) with **Use ChromaDB** ticked AND the server
is running, Mini ChatUR:

1. Writes `~/.chatur/chatur_chroma_cli.py` + the `~/.chatur/bin/chatur-chroma`
   shim (idempotent — content-diff check).
2. Sets these env vars on the spawned `pi` process:
   `CHATUR_CHROMA_HOST`, `CHATUR_CHROMA_PORT`, `CHATUR_CHROMA_MODEL`,
   `CHATUR_CHROMA_COLLECTION`.
3. Appends a system-prompt fragment naming the collection, the absolute
   shim path, and the subcommand surface.

### `chatur-chroma` CLI

| Subcommand | Purpose |
|---|---|
| `query --query <text> [--n 10] [--where '<json>']` | **primary** semantic search |
| `peek [--n 5]` | sample documents without a query |
| `list` | list collections on the server |
| `info` | collection count + metadata |
| `get [--ids id1,id2] [--where '<json>']` | fetch by id / metadata filter |

`--collection` defaults to `CHATUR_CHROMA_COLLECTION`. Add `--json` to any
subcommand for machine-readable output. Default output is one line per hit:

```
0.317  src/db/migrate.rs:42-78  applies pending migrations …
```

The full prompt text lives in `crates/chatur-chroma/src/prompt.rs` —
`chromadb_system_prompt(collection_name, shim_path)`. The resolver
(`crates/chatur-api/src/resolver.rs`) appends it on every job whose
`use_chromadb` flag is set and whose chroma server is running.

The CLI shares its embedding-function code with the indexer + query helpers,
so query embeddings always match the index embeddings exactly.

### Recommended workflow (the one we tell the agent)

1. Run 2–4 `chatur-chroma query` calls with different short phrasings of the
   intent — e.g. `--query "sqlite migration runner"`, then
   `--query "applies pending migrations"`. Use `--n 10` for exploratory
   questions, smaller for targeted ones.
2. Read each hit's path with the normal `read` tool — chunks are ~800 chars
   and may be truncated.
3. Only fall back to directory listing / grep if `query` returns nothing
   relevant after TWO distinct phrasings.

### Sample prompts that exploit chroma

```
"Where is request authentication handled? Use chatur-chroma query first,
 then read the top hits."

"Summarise the SQLite schema and its migrations — run chatur-chroma query
 with 'migration' and 'CREATE TABLE' before reading."

"Find every place that calls run_reviewer; chatur-chroma query with
 'reviewer prompt' and 'review aggregator', then verify with read."
```

### Manual query — the Query tab

If the agent is misbehaving, sanity-check your index by hand. Open the
**ChromaDB** pane → **Query** tab:

- Pick a project. Disabled projects in the dropdown have no collection yet.
- Type a natural-language query. `n` defaults to 10.
- Results show `distance | path:line_start-line_end | snippet`. Lower
  distance = closer match. Click a snippet to expand.

Backed by `chroma_query` Tauri command → `chatur_chroma::query::query_collection`
→ the same Python helper pattern indexing uses (chroma 1.x requires
client-side embeddings; we reuse chromadb's `DefaultEmbeddingFunction` so
query embeddings match the index embeddings exactly).

---

## Removing the integration

1. *Settings → ChromaDB → Enable* off (or set `enabled = false` in
   `chatur.toml`).
2. *Server → Stop* — kills the chroma child process.
3. Edit `~/.pi/agent/config.json` and delete the `mcpServers.chroma` entry
   (restore from `config.json.chatur-bak` if you prefer).
4. Optional cleanup: `rm -rf ~/.chatur/chroma-venv ~/.chatur/chroma-data`.

After step 1, restarting Mini ChatUR runs zero ChromaDB code — the integration
imposes no overhead.
