# Ideas — NETRA

Backlog of improvements, grouped by area. Not committed work — a menu to pull from.
Each item: **what** + **why** + rough effort (S/M/L).

---

## Reliability & correctness

- **Crash-safe job recovery (M).** On startup, scan store for jobs stuck in
  `running` (process died mid-flight) and requeue or mark `failed`. Today a
  killed app likely leaves orphaned `running` rows.
- **Planner sidecar health gate (S).** Block `structured_reviewer` batches until
  `/healthz` passes; surface a clear UI banner when the sidecar is down instead
  of failing per-job. `planner_supervisor` already monitors — wire the state up.
- **Graceful pi version check (S).** Probe `pi --version` at startup; warn if not
  `v0.74.2`. RPC protocol is version-coupled (NDJSON), so a mismatch fails late
  and cryptically.
- **Per-job timeout + cancellation propagation (M).** Ensure cancel actually
  kills the `pi` child + frees the concurrency slot; add a configurable wall
  clock per job. Verify no zombie processes on the pool.
- **Retry backoff visibility (S).** `retry.rs` exists — expose attempt count +
  next-retry time in job model/UI so retries aren't silent.

## Testing & CI

- **netra-cli has 0 tests (M).** Add integration tests over the binary with the
  mock agent (`netra-agent/mock.rs`): `project add`, `queue`, `run`, `batch run`
  with each reduce strategy. Highest-value gap.
- **netra-chroma 1 test file (M).** Cover `ignore_rules`, `indexer` chunking,
  and `query` against a temp Chroma. Bootstrap path (`uv`) can stay mocked.
- **End-to-end batch fanout test (M).** Assert `prompts × targets × modules`
  expands to the right job set incl. the `global` skip-fanout path.
- **CI matrix (S).** `.github/` exists — confirm it runs `cargo test --workspace`
  + `clippy -D warnings` + `cargo fmt --check`; add the Python sidecar
  (`ruff`/`pytest`) and a `ui` `npm run check`.
- **Planner sidecar tests (S).** pytest hitting `/generate` with a tiny schema
  against a stub OpenAI server; assert non-conformant output raises 422 not 500.

## Features

- **Module auto-inference (M).** `netra-api/modules.rs` hints at inference —
  let an agent pass propose modules from repo layout (dirs, workspace members,
  language boundaries) and let user accept/edit before a batch.
- **Resumable / incremental batches (L).** Re-run only failed or changed items
  in a batch instead of the whole product. Pairs with crash recovery.
- **Findings export (S).** Export aggregated `structured_reviewer` findings to
  SARIF / JSON / Markdown so results feed into PRs or other tools.
- **Diff-scoped runs (M).** Target a batch at `git diff` (working tree or a range)
  instead of whole modules — review only what changed.
- **Template library (S).** `template` model exists; ship a starter pack
  (security review, dead-code, doc-gen, test-gen) + UI to manage them.
- **Live token / cost meter (S).** Surface tokens-per-job from the model server
  in the UI; helps tune `concurrency` + prompt size.
- **Multi-model routing (M).** Per-module or per-prompt model override (cheap
  model for map, strong model for reduce). `resolver.rs` precedence already
  exists — extend to module level.

## Performance

- **Chroma incremental reindex (M).** Index only changed files (mtime/hash) on
  re-enable rather than full re-walk. `indexer.rs` + `ignore_rules.rs`.
- **Concurrency autotuning (S).** Default `global_max` from CPU count; warn when
  the model server is the bottleneck (queue grows but jobs idle).
- **Streaming reduce (M).** Start the reducer/aggregator on partial map output
  instead of waiting for every map job — cuts batch latency on slow agents.

## UX / UI

- **Job log tailing in UI (M).** Stream per-job log files (`log_dir`) live in
  `OutputPane` instead of only final output.
- **Batch progress + ETA (S).** Show `done/total` + running items per batch in
  `QueuePanel`; today fanout size is only an estimate until modules resolve.
- **Keyboard-driven review (S).** j/k navigation + accept/dismiss across
  `FindingCard`s in the review subview.
- **Dark/light + density settings (S).** `SettingsPane` exists — round out theme
  + compact mode for big finding lists.
- **First-run setup wizard (M).** Detect missing `pi`, model server, planner,
  ChromaDB and walk the user through enabling each.

## Docs & DX

- **Finish the code-wiki (L).** `docs/TODO.md` lists 29 remaining HTML pages
  (netra-api, netra-cli, src-tauri, ui, cross-module map). Stale refs to
  `PLAN.md` (deleted) — clean those up.
- **Architecture pages drifted (S).** `docs/concepts/` predates `netra-chroma`,
  module fanout, and the planner sidecar. Refresh diagrams + glossary.
- **`just`/`make` task runner (S).** One-liners for `build`, `test`, `tauri dev`,
  `planner serve`, `lint`. Lowers the multi-workspace barrier.
- **Mock-agent dev mode doc (S).** Document running the whole stack with
  `netra-agent/mock.rs` so contributors don't need a model server.

## Packaging & ops

- **Bundle the planner sidecar (M).** Ship `netra-planner` inside the Tauri
  bundle (PyInstaller or `uv` venv on first run) so the desktop app is
  self-contained; `autostart` already expects it local.
- **Single-binary CLI release (S).** GitHub release workflow building
  `netra` for Linux/macOS/Windows (`.exe` handling already in chroma `win.rs`).
- **Telemetry opt-in (S).** Local-only structured `tracing` export (JSON logs)
  for debugging long batches; no network by default.

---

## Quick wins (start here)

1. netra-cli integration tests (closes the biggest test gap).
2. Crash-safe `running`-job recovery on startup.
3. Planner `/healthz` gate + UI banner.
4. `just`/`make` task runner.
5. Findings export to Markdown/SARIF.
