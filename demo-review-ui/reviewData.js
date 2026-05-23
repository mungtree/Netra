// Sample run-detail data for the Review page
window.CHATUR_REVIEW = {
  recentRuns: [
    { id: "r-014", task: "Find Bugs",          icon: "bug",     project: "chatur",    findings: 8,  highSev: 2, when: "2m ago",   active: true },
    { id: "r-013", task: "Dependency Audit",   icon: "package", project: "chatur",    findings: 4,  highSev: 0, when: "12m ago" },
    { id: "r-012", task: "Documentation",      icon: "book",    project: "linear-x",  findings: 12, highSev: 0, when: "47m ago" },
    { id: "r-011", task: "Find Vulnerabilities",icon: "shield", project: "chatur",    findings: 6,  highSev: 1, when: "1h ago"  },
    { id: "r-010", task: "Refactor",           icon: "wand",    project: "rust-fork", findings: 19, highSev: 0, when: "2h ago"  },
    { id: "r-009", task: "Performance",        icon: "gauge",   project: "linear-x",  findings: 5,  highSev: 1, when: "3h ago"  },
    { id: "r-008", task: "Test Coverage",      icon: "test",    project: "chatur",    findings: 11, highSev: 0, when: "yesterday" },
    { id: "r-007", task: "Generate Ideas",     icon: "bulb",    project: "portfolio", findings: 7,  highSev: 0, when: "yesterday" },
  ],

  run: {
    id: "r-014",
    task: "Find Bugs",
    icon: "bug",
    project: "chatur",
    branch: "main",
    commit: "8a3f1d2",
    started: "2026-05-22 14:32:08",
    finished: "2026-05-22 14:35:22",
    duration: "3m 14s",
    files: 218,
    tokens: "1.8M",
    model: "qwen2.5-coder:7b",
    totalFindings: 8,
    severities: { critical: 1, high: 1, medium: 3, low: 2, info: 1 },
  },

  prompts: [
    {
      id: "p-01",
      name: "Detect syntax & parsing issues",
      status: "done",
      duration: "12s",
      tokens: "84.2k",
      output: {
        summary: "Three findings identified across three files: a syntax error, a potential race condition, and an incorrect comparison operator that may cause logic errors.",
        findings: [
          {
            title: "Syntax issue on line 1",
            description: "A syntax problem exists on line 1 of an unspecified file. The parser reports an unexpected token before the module's first import statement.",
            kind: "bug",
            location: null,
            severity: "high",
            suggested_fix: "Inspect and correct the syntax on line 1 of the affected file.",
            tags: ["syntax", "parsing"],
          },
          {
            title: "Potential race condition in src/main.cpp",
            description: "A race condition may exist in src/main.cpp, leading to unpredictable behavior under concurrent access. Two threads append to the same buffer without synchronization.",
            kind: "suggestion",
            location: "src/main.cpp",
            severity: "high",
            suggested_fix: "Add synchronization primitives (mutex, lock, or atomics) to protect shared state in src/main.cpp.",
            tags: ["concurrency", "race-condition"],
          },
          {
            title: "Incorrect equality comparison in src/auth.ts",
            description: "The if statement uses == instead of !=, causing inverted logic that may allow unauthorized access or block legitimate operations.",
            kind: "fix",
            location: "src/auth.ts",
            severity: "critical",
            suggested_fix: "Change `if (x == 2)` to `if (x != 2)` to restore the intended conditional logic.",
            tags: ["auth", "logic-error", "comparison"],
          },
        ],
      },
    },
    {
      id: "p-02",
      name: "Identify null & undefined dereferences",
      status: "done",
      duration: "18s",
      tokens: "112.4k",
      output: {
        summary: "Two locations may dereference values that could be null at runtime; one is reachable from public API surface.",
        findings: [
          {
            title: "Possible null dereference in src/db/queries.ts",
            description: "`row.user` is accessed without a nullish check after a left-join that may return undefined for missing relations.",
            kind: "bug",
            location: "src/db/queries.ts",
            severity: "medium",
            suggested_fix: "Guard with `if (!row.user) return null;` or use optional chaining: `row.user?.id`.",
            tags: ["null-safety", "database"],
          },
          {
            title: "Optional config accessed without default",
            description: "`config.retry.attempts` is read directly; `retry` may be undefined when no override is provided.",
            kind: "fix",
            location: "src/jobs/runner.ts",
            severity: "low",
            suggested_fix: "Destructure with defaults: `const { retry = { attempts: 3 } } = config;`.",
            tags: ["null-safety", "config"],
          },
        ],
      },
    },
    {
      id: "p-03",
      name: "Detect unhandled async rejections",
      status: "done",
      duration: "23s",
      tokens: "138.7k",
      output: {
        summary: "One unhandled rejection in an async iterator that will surface as a silent failure under load.",
        findings: [
          {
            title: "Unhandled rejection in async iterator",
            description: "The async generator in `runner.ts` awaits a network call inside a for-await loop without a try/catch. A single rejection will terminate the iterator silently and drop downstream items.",
            kind: "bug",
            location: "src/jobs/runner.ts",
            severity: "medium",
            suggested_fix: "Wrap the inner await in try/catch and either continue, retry with backoff, or surface the error to the caller via a result type.",
            tags: ["async", "error-handling"],
          },
        ],
      },
    },
    {
      id: "p-04",
      name: "Find memory leaks & resource handles",
      status: "done",
      duration: "29s",
      tokens: "164.1k",
      output: {
        summary: "One event listener registered without a matching cleanup on unmount.",
        findings: [
          {
            title: "Memory leak: event listener not cleaned up",
            description: "Component registers a `resize` listener on `window` in `mounted()` but the matching `removeEventListener` call is missing from `beforeUnmount()`. Long sessions will accumulate listeners.",
            kind: "bug",
            location: "src/components/Live.vue",
            severity: "medium",
            suggested_fix: "Store the handler in a ref and remove it in `beforeUnmount`: `window.removeEventListener('resize', handler)`.",
            tags: ["memory-leak", "vue", "lifecycle"],
          },
        ],
      },
    },
    {
      id: "p-05",
      name: "Detect dead code & unused exports",
      status: "done",
      duration: "8s",
      tokens: "62.0k",
      output: {
        summary: "One unused import detected; no dead exports in this scope.",
        findings: [
          {
            title: "Unused import: formatDate",
            description: "`formatDate` is declared at the top of `src/utils/format.ts` but never read in the file.",
            kind: "fix",
            location: "src/utils/format.ts",
            severity: "low",
            suggested_fix: "Remove the import statement on line 1.",
            tags: ["dead-code", "imports"],
          },
        ],
      },
    },
    {
      id: "p-06",
      name: "Find magic numbers & unclear constants",
      status: "done",
      duration: "14s",
      tokens: "78.9k",
      output: {
        summary: "One magic number used without a named constant.",
        findings: [
          {
            title: "Magic number 86400 in rate limiter",
            description: "The literal `86400` appears in `src/api/limits.ts` without context. This is seconds in a day, but readers must infer that.",
            kind: "suggestion",
            location: "src/api/limits.ts",
            severity: "info",
            suggested_fix: "Extract to `const SECONDS_PER_DAY = 86_400;` at module top.",
            tags: ["readability", "constants"],
          },
        ],
      },
    },
    {
      id: "p-07",
      name: "Identify duplicated logic",
      status: "done",
      duration: "31s",
      tokens: "187.3k",
      output: {
        summary: "No structural duplication detected across the scanned modules.",
        findings: [],
      },
    },
    {
      id: "p-08",
      name: "Summarize architectural risk areas",
      status: "done",
      duration: "26s",
      tokens: "142.8k",
      outputType: "text",
      output: `## Architectural risk summary

The codebase is structured around three coupled subsystems: **the job runner**, **the streaming output layer**, and **the persistence boundary**. Most of the bug surface lives where these meet.

### Top concerns

1. **Shared mutable state in the runner.** \`src/jobs/runner.ts\` keeps an in-memory queue that is mutated from both the HTTP handler and the worker pool without a lock. This is the source of the race condition flagged in prompt 1.
2. **Error propagation is inconsistent.** Some modules throw, some return \`Result<T, E>\`-style tuples, some swallow. The async iterator path is the worst offender — a single rejected promise terminates the generator silently.
3. **Database access is not centralized.** Raw query strings appear in five different files. One of them interpolates user input (already flagged as a critical vulnerability in prompt 1).

### Recommended next steps

- Introduce a small \`Mutex\` wrapper around the runner queue, or migrate to a single-writer worker model.
- Pick one error convention and apply it across the boundary — \`Result\` types pair well with the existing Rust-side code.
- Move all SQL into a \`src/db/queries.ts\` module with parameterized helpers; forbid raw template literals at the lint level.

> The good news: none of this is structural. All three concerns can be addressed incrementally without breaking the module boundaries that already exist.`,
    },
    {
      id: "p-09",
      name: "Free-form review notes",
      status: "done",
      duration: "11s",
      tokens: "48.2k",
      outputType: "text",
      output: `Overall the code reads cleanly and the module boundaries are sensible. A few impressions worth recording:

The Vue components are well-scoped — most are under 200 lines and have a clear single responsibility. \`Live.vue\` is the exception and is probably worth splitting; the resize listener leak flagged earlier is symptomatic of how much that file is doing.

Naming is consistent across the TypeScript and Rust halves, which is unusual and helpful. The token \`Job\` means the same thing in both layers, which makes cross-boundary debugging much easier than it usually is.

I did not find evidence of test coverage for the streaming path. That's the area where the race condition lives and where regressions would be hardest to detect by inspection.`,
    },
  ],

  activePromptId: "p-01",
};
