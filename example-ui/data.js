// Sample state for the Chatur mockup
window.CHATUR_DATA = {
  projects: [
    { id: "chatur",      name: "chatur",            path: "~/code/chatur",            status: "run",  count: 142 },
    { id: "linear-x",    name: "linear-clone",      path: "~/code/linear-x",          status: "done", count: 87  },
    { id: "portfolio",   name: "portfolio-site",    path: "~/sites/portfolio",        status: "idle", count: 24  },
    { id: "rust-fork",   name: "rust-analyzer-fork",path: "~/oss/rust-analyzer",      status: "done", count: 312 },
    { id: "ml-nb",       name: "ml-notebooks",      path: "~/research/ml-nb",         status: "err",  count: 9   },
    { id: "lampshade",   name: "lampshade-api",     path: "~/work/lampshade-api",     status: "idle", count: 0   },
  ],
  activeProjectId: "chatur",

  tasks: [
    { id: "bugs",   icon: "bug",      title: "Find Bugs",            desc: "Detect logic errors, edge cases, and broken assumptions.",      meta: "~3m · 47 prompts", shortcut: "1", featured: true },
    { id: "vulns",  icon: "shield",   title: "Find Vulnerabilities", desc: "OWASP, injection, secrets, unsafe patterns and dependencies.",  meta: "~5m · 38 prompts", shortcut: "2" },
    { id: "ideas",  icon: "bulb",     title: "Generate Ideas",       desc: "Surface new features, UX improvements, and product directions.", meta: "~2m · 18 prompts", shortcut: "3" },
    { id: "refactor", icon: "wand",   title: "Refactor",             desc: "Identify duplication, complexity, and cleaner abstractions.",   meta: "~4m · 29 prompts", shortcut: "4" },
    { id: "perf",   icon: "gauge",    title: "Performance",          desc: "Hot paths, allocations, async bottlenecks, and N+1 patterns.",  meta: "~6m · 22 prompts", shortcut: "5" },
    { id: "docs",   icon: "book",     title: "Documentation",        desc: "Find missing docstrings, stale comments, and unclear APIs.",    meta: "~2m · 14 prompts", shortcut: "6" },
    { id: "tests",  icon: "test",     title: "Test Coverage",        desc: "Untested branches, missing edge cases, and weak assertions.",   meta: "~4m · 31 prompts", shortcut: "7" },
    { id: "deps",   icon: "package",  title: "Dependency Audit",     desc: "Outdated, unused, or risky packages and license issues.",       meta: "~1m · 9 prompts",  shortcut: "8" },
  ],

  lastRun: {
    task: "Find Bugs",
    icon: "bug",
    finishedAt: "2 min ago",
    duration: "3m 14s",
    files: 218,
    prompts: 47,
    tokens: "1.8M",
    findings: [
      { sev: "high", msg: "Potential SQL injection: user input interpolated into raw query string.",   file: "src/db/queries.ts",        line: 142, range: "L142–149" },
      { sev: "high", msg: "Hardcoded API key committed to source.",                                     file: "src/config/secrets.ts",    line: 8,   range: "L8" },
      { sev: "med",  msg: "Race condition: shared mutable state accessed without lock.",                file: "src/sync/handler.ts",      line: 67,  range: "L67–82" },
      { sev: "med",  msg: "Memory leak: event listener registered without matching cleanup on unmount.",file: "src/components/Live.vue",  line: 204, range: "L204" },
      { sev: "med",  msg: "Unhandled rejection in async iterator; will surface as silent failure.",     file: "src/jobs/runner.ts",       line: 318, range: "L318–326" },
      { sev: "low",  msg: "Magic number 86400 used without named constant (seconds in a day).",         file: "src/api/limits.ts",        line: 23,  range: "L23" },
      { sev: "low",  msg: "Unused import: `formatDate` is declared but never read.",                    file: "src/utils/format.ts",      line: 1,   range: "L1" },
      { sev: "info", msg: "Consider extracting repeated validation into a shared helper.",              file: "src/forms/SignupForm.vue", line: 88,  range: "L88–141" },
    ],
  },

  queue: {
    running: {
      task: "Find Vulnerabilities",
      icon: "shield",
      project: "chatur",
      step: "Scanning src/auth/*",
      progress: 0.62,
      eta: "1m 47s",
      promptIdx: 18,
      promptTotal: 38,
    },
    pending: [
      { task: "Generate Ideas",  icon: "bulb",    project: "chatur",     prompts: 18 },
      { task: "Refactor",        icon: "wand",    project: "chatur",     prompts: 29 },
      { task: "Test Coverage",   icon: "test",    project: "linear-x",   prompts: 31 },
    ],
    done: [
      { task: "Find Bugs",          icon: "bug",     project: "chatur",   findings: 8,  duration: "3m 14s",  highSev: 2 },
      { task: "Dependency Audit",   icon: "package", project: "chatur",   findings: 4,  duration: "47s",     highSev: 0 },
      { task: "Documentation",      icon: "book",    project: "linear-x", findings: 12, duration: "1m 52s",  highSev: 0 },
    ],
    stats: { running: 1, pending: 3, done: 14 },
  },
};
