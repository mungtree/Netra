// Sample state for the Modules / Batch / Queue mockups.
// Mirrors the shape of data.js so components can be composed alongside.
window.MODULES_DATA = {
  /* ---------- Module sets per project ---------- */
  projects: [
    {
      id: "chatur", name: "chatur", path: "~/code/chatur", status: "run", filesTotal: 218,
      // 5 well-scoped modules — used in populated, validation, queue states
      modules: [
        { id: "m_fe",    name: "frontend",   desc: "SvelteKit UI, components, routes",         root: "ui/src",            files: 64,  lines: "12.4k" },
        { id: "m_be",    name: "backend",    desc: "Tauri commands, IPC handlers, services",   root: "src-tauri/src",     files: 41,  lines: "8.1k"  },
        { id: "m_engine",name: "engine",     desc: "Job runner, LLM client, agent loop",       root: "src-tauri/src/engine", files: 22,  lines: "5.3k" },
        { id: "m_db",    name: "db",         desc: "SQLite schema, migrations, repositories",  root: "src-tauri/src/db",  files: 14,  lines: "2.1k"  },
        { id: "m_shared",name: "shared",     desc: "Cross-cutting types and helpers",          root: "src-tauri/src/shared", files: 9, lines: "0.9k" },
      ],
    },
    {
      id: "linear-x", name: "linear-clone", path: "~/code/linear-x", status: "done", filesTotal: 87,
      modules: [
        { id: "lx_root", name: "root", desc: "Whole repository (default)", root: ".", files: 87, lines: "14.2k" },
      ],
    },
    {
      id: "rust-fork", name: "rust-analyzer-fork", path: "~/oss/rust-analyzer", status: "done", filesTotal: 312,
      modules: [
        { id: "ra_ide",  name: "ide",      desc: "Interactive editor surface",        root: "crates/ide",         files: 84, lines: "21.0k" },
        { id: "ra_hir",  name: "hir",      desc: "High-level IR, name resolution",    root: "crates/hir",         files: 51, lines: "16.4k" },
        { id: "ra_syn",  name: "syntax",   desc: "Lexing, parsing, syntax trees",     root: "crates/syntax",      files: 28, lines: "8.7k"  },
        { id: "ra_cli",  name: "cli",      desc: "Command-line entrypoints",          root: "crates/rust-analyzer", files: 33, lines: "9.1k" },
      ],
    },
    {
      id: "portfolio", name: "portfolio-site", path: "~/sites/portfolio", status: "idle", filesTotal: 24,
      modules: [
        { id: "pf_root", name: "root", desc: "Whole repository (default)", root: ".", files: 24, lines: "3.8k" },
      ],
    },
    {
      id: "ml-nb", name: "ml-notebooks", path: "~/research/ml-nb", status: "err", filesTotal: 9,
      modules: [
        { id: "ml_root", name: "root", desc: "Whole repository (default)", root: ".", files: 9, lines: "2.4k" },
      ],
    },
  ],

  /* ---------- AI inference proposal (diff state) ----------
     Compared against chatur's current modules. Each row carries one of:
       added   — new module proposed
       changed — existing module with field changes (showing before/after)
       kept    — exactly matches current
       removed — current module the AI suggests dropping  */
  inferredFor: "chatur",
  inferProposal: [
    { kind: "kept",    id: "m_fe",
      after:  { name: "frontend",  desc: "SvelteKit UI, components, routes",       root: "ui/src",             files: 64 } },
    { kind: "changed", id: "m_be",
      before: { name: "backend",   desc: "Tauri commands, IPC handlers, services", root: "src-tauri/src",      files: 41 },
      after:  { name: "backend",   desc: "Tauri command surface and IPC handlers", root: "src-tauri/src/cmds", files: 18 } },
    { kind: "kept",    id: "m_engine",
      after:  { name: "engine",    desc: "Job runner, LLM client, agent loop",     root: "src-tauri/src/engine", files: 22 } },
    { kind: "added",   id: "m_new_models",
      after:  { name: "models",    desc: "Model providers, prompts, tool policy",  root: "src-tauri/src/models", files: 11 } },
    { kind: "added",   id: "m_new_chroma",
      after:  { name: "vectorstore", desc: "Chroma client + embedding helpers",    root: "src-tauri/src/chroma", files: 7 } },
    { kind: "changed", id: "m_db",
      before: { name: "db",        desc: "SQLite schema, migrations, repositories", root: "src-tauri/src/db",  files: 14 },
      after:  { name: "persistence", desc: "SQLite schema, migrations, repositories", root: "src-tauri/src/db", files: 14 } },
    { kind: "removed", id: "m_shared",
      before: { name: "shared",    desc: "Cross-cutting types and helpers",        root: "src-tauri/src/shared", files: 9 } },
  ],

  /* ---------- Batch builder state ---------- */
  batch: {
    promptIds: ["bugs", "vulns"],            // 2 prompts
    targetProjectIds: ["chatur", "rust-fork"], // 2 projects
    // Per project, which modules are picked (default = all)
    moduleSelection: {
      chatur:    ["m_fe", "m_be", "m_engine", "m_db"],          // user de-selected "shared"
      "rust-fork": ["ra_ide", "ra_hir", "ra_syn", "ra_cli"],
    },
  },

  /* ---------- Queue items annotated with module ---------- */
  queueWithModules: {
    running: {
      task: "Find Vulnerabilities", icon: "shield",
      project: "chatur", module: "backend",
      step: "Scanning src-tauri/src/auth/*",
      progress: 0.62, eta: "1m 47s",
      promptIdx: 18, promptTotal: 38,
    },
    pending: [
      { task: "Find Vulnerabilities", icon: "shield", project: "chatur",    module: "engine",   prompts: 38 },
      { task: "Find Vulnerabilities", icon: "shield", project: "chatur",    module: "db",       prompts: 38 },
      { task: "Find Vulnerabilities", icon: "shield", project: "chatur",    module: "frontend", prompts: 38 },
      { task: "Find Bugs",            icon: "bug",    project: "rust-fork", module: "ide",      prompts: 47 },
      { task: "Find Bugs",            icon: "bug",    project: "rust-fork", module: "hir",      prompts: 47 },
      { task: "Find Bugs",            icon: "bug",    project: "rust-fork", module: "syntax",   prompts: 47 },
      { task: "Find Bugs",            icon: "bug",    project: "rust-fork", module: "cli",      prompts: 47 },
    ],
    done: [
      { task: "Find Vulnerabilities", icon: "shield", project: "chatur",    module: "frontend", findings: 3, duration: "1m 22s", highSev: 0 },
      { task: "Find Bugs",            icon: "bug",    project: "chatur",    module: "backend",  findings: 8, duration: "2m 04s", highSev: 2 },
      { task: "Find Bugs",            icon: "bug",    project: "chatur",    module: "engine",   findings: 5, duration: "1m 51s", highSev: 1 },
    ],
    stats: { running: 1, pending: 7, done: 14 },
  },

  /* ---------- Resume banner state ---------- */
  resume: {
    queuedAtRestart: 9,
    discarded: 1,
    resumedAt: "12:04",
  },
};
