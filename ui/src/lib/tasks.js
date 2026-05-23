// Batch presets shown in the task grid.
//
// Each preset is a prompt-set (see PLAN §6.5): a series of prompts a batch runs
// over the selected project, whose outputs are then aggregated by `strategy`.
// Clicking a card calls `createBatch` + `runBatch` with these prompts.
//
// Prompts are tuned for small local models (7B-class). Each one:
//   - States the goal in one sentence.
//   - Names a hard scope cap (file count or finding cap) so the agent stops.
//   - Lists what to skip, so the agent does not wander into adjacent concerns.
//   - Requires concrete evidence (file:line) for every finding.
//   - Tells the agent to emit "no findings" instead of inventing weak ones.

export const DEFAULT_STOP_RULES = [
  'Hard limits:',
  '- Inspect at most 6 files. Pick the most relevant by name; skip vendored, generated, test fixtures, and any file > 800 lines.',
  '- Report at most 4 findings. If fewer real issues exist, return fewer.',
  '- Each finding needs a concrete location (file path, plus line number when visible).',
  '- Do not rewrite large blocks.',
  '- If nothing meaningful surfaces, return an empty findings list — do not fabricate.',
  '- Stop after listing findings. No closing summary, no next-steps, no questions.',
].join('\n');

/**
 * Builds the final prompt strings for a preset by appending `stopRules`
 * to every body when `preset.useStopRules` is true. Custom-imported presets
 * (no flag) are passed through verbatim.
 */
export function composePrompts(preset, stopRules) {
  if (!preset.useStopRules) return preset.prompts.slice();
  const rules = (stopRules ?? '').trim();
  if (!rules) return preset.prompts.slice();
  return preset.prompts.map((body) => `${body}\n\n${rules}`);
}

// Bodies only — STOP_RULES is appended at run-time from the live setting.
const p = (body) => body;

export const TASK_PRESETS = [
  {
    id: 'bugs',
    icon: 'bug',
    title: 'Find Bugs',
    desc: 'Detect logic errors, edge cases, and broken assumptions.',
    featured: true,
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find logic bugs: wrong operators (== vs !=, < vs <=), off-by-one indexing, inverted booleans, swapped arguments, mismatched branches.\n' +
          'In scope: source files in the main module tree only.\n' +
          'Out of scope: style, naming, missing docs, performance, test code.\n' +
          'For each bug: quote the offending line, name the wrong behavior, give the corrected line.',
      ),
      p(
        'Find broken edge-case handling: empty inputs, null/None, zero/negative numbers, empty collections, missing optional fields, malformed input that the code does not guard.\n' +
          'In scope: public functions and request/response handlers.\n' +
          'Out of scope: defensive checks against impossible internal state, panics in test helpers.\n' +
          'For each issue: name the input that breaks it, quote the unguarded line, give the minimal guard to add.',
      ),
      p(
        'Find concurrency hazards: shared mutable state accessed without a lock, data races, await inside a held lock, missing cancellation on error paths, fire-and-forget tasks that swallow errors.\n' +
          'In scope: code that spawns threads, tasks, goroutines, or async handlers.\n' +
          'Out of scope: single-threaded paths, hypothetical races behind feature flags that are off.\n' +
          'For each hazard: name the shared resource, quote the racing access, describe the interleaving that breaks it.',
      ),
    ],
  },
  {
    id: 'vulns',
    icon: 'shield',
    title: 'Find Vulnerabilities',
    desc: 'Injection, unsafe input, secrets in source.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find injection sinks: string-built SQL, shell exec with user input, path joins from request data without normalization, template rendering with unescaped input, deserialization of untrusted bytes.\n' +
          'In scope: any code path reachable from an HTTP handler, CLI arg, or file read from a user-controlled location.\n' +
          'Out of scope: hypothetical injections in test fixtures, hardcoded internal strings, generated migrations.\n' +
          'For each sink: quote the dangerous call, name the tainted variable, name the source (handler / arg / env), give the safe replacement (parameterized query, exec with arg array, etc.).',
      ),
      p(
        'Find committed secrets and weak credential handling: API keys, tokens, passwords, private keys in source; secrets logged to stdout; credentials passed in URLs; default/empty passwords accepted at runtime.\n' +
          'In scope: source files, config files, scripts. Look for high-entropy strings and assignments like "secret =", "token =", "password =", "Bearer ...", "-----BEGIN ...".\n' +
          'Out of scope: example values clearly marked as placeholders ("xxx", "your-key-here").\n' +
          'For each finding: quote the line, name what kind of credential it appears to be, say whether it looks live or placeholder.',
      ),
      p(
        'Find unsafe input handling at trust boundaries: missing auth checks, missing authorization (logged-in but wrong tenant), missing CSRF protection, unchecked redirects/SSRF, file uploads without type/size limits, integer parsing without range checks.\n' +
          'In scope: HTTP handlers, RPC endpoints, message-queue consumers.\n' +
          'Out of scope: internal-only helpers, code behind a verified gateway.\n' +
          'For each issue: quote the entry point, name the missing check, give one line of code that adds it.',
      ),
    ],
  },
  {
    id: 'ideas',
    icon: 'bulb',
    title: 'Generate Ideas',
    desc: 'Concrete feature proposals grounded in the code.',
    strategy: 'concat',
    useStopRules: true,
    prompts: [
      'Propose exactly 3 feature ideas that fit this project. Each must:\n' +
        '- Build on a capability the code already has (name the module or function it would extend).\n' +
        '- Fit in one focused PR (no architectural rewrites).\n' +
        '- Have a one-sentence user-visible win.\n' +
        'Format each idea as: TITLE — one-line pitch — extends <module/function> — user win.\n' +
        'Skip generic suggestions ("add dark mode", "add tests", "improve docs") unless the code clearly lacks them and they unlock something specific.\n' +
        'Stop after 3. No preamble, no closing summary.',
      'Identify the single weakest UX or DX rough edge in this project and propose one concrete fix.\n' +
        'Pick the rough edge by reading actual code/config — not by guessing. Quote the file or behavior that shows the problem.\n' +
        'The fix must be:\n' +
        '- Small enough to land in one PR.\n' +
        '- Stated as a code or config change, not a vague principle.\n' +
        'Format: PROBLEM (with file evidence) — FIX (concrete change) — WHY IT HELPS.\n' +
        'One issue only. No alternates, no list.',
    ],
  },
  {
    id: 'refactor',
    icon: 'wand',
    title: 'Refactor',
    desc: 'Duplication, tangled functions, leaky abstractions.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find true duplication: 2+ functions or blocks with near-identical logic that should be one helper. Require structural similarity, not just shared keywords.\n' +
          'In scope: functions ≥ 10 lines.\n' +
          'Out of scope: trivial getters, test setup boilerplate, distinct functions that happen to share a name.\n' +
          'For each: name both locations (file:line), describe the shared logic in one sentence, propose the helper signature.',
      ),
      p(
        'Find functions that are too tangled: high cyclomatic complexity, deep nesting (≥ 4 levels), multiple unrelated responsibilities in one body, > 80 lines without a clear single purpose.\n' +
          'In scope: non-test source files.\n' +
          'Out of scope: generated code, parser tables, dispatch tables that are inherently flat-list-shaped.\n' +
          'For each: name the function (file:line), point to the specific tangle (the nested block or the mixed concerns), propose how to split it (extract / early-return / strategy object) in one sentence.',
      ),
      p(
        "Find leaky abstractions: modules whose internals callers reach into, types that expose mutable state callers depend on, helpers whose signatures betray the caller's logic.\n" +
          'In scope: public types and exported functions.\n' +
          'Out of scope: private helpers used only inside their own file.\n' +
          'For each: name the leak (file:line), quote a caller that depends on the leaked detail, propose a tighter boundary.',
      ),
    ],
  },
  {
    id: 'perf',
    icon: 'gauge',
    title: 'Performance',
    desc: 'Hot paths, wasteful allocations, blocking calls.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find quadratic-or-worse loops on data that can grow: nested loops over the same collection, repeated linear scans inside a loop, list-membership checks in a tight loop, N+1 database calls.\n' +
          'In scope: code on request, render, or batch-processing paths.\n' +
          'Out of scope: one-shot setup code, code on inputs known to be small (≤ 16 items, constant-bounded enums).\n' +
          'For each: quote the loop (file:line), name what scales (rows? requests? items?), give the fix (set/dict lookup, batched query, caching) in one line.',
      ),
      p(
        'Find avoidable allocations and copies on hot paths: clones inside loops, repeated string concatenation, intermediate collections built only to be iterated once, defensive copies of immutable data.\n' +
          'In scope: loops, request handlers, render paths.\n' +
          'Out of scope: code outside hot paths, allocations needed for thread-safety.\n' +
          'For each: quote the allocation (file:line), name why it is avoidable, give the alternative (iterator, reuse buffer, borrow, etc.).',
      ),
      p(
        'Find blocking calls on async or latency-sensitive paths: sync I/O inside an async function, time.sleep / thread::sleep in handlers, long synchronous work between awaits, locks held across awaits.\n' +
          'In scope: async functions, request handlers, event loops.\n' +
          'Out of scope: startup code, CLI scripts, tests.\n' +
          'For each: quote the blocking call (file:line), name what it blocks, give the non-blocking replacement.',
      ),
    ],
  },
  {
    id: 'docs',
    icon: 'book',
    title: 'Documentation',
    desc: 'Missing or misleading docs on public surface.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find undocumented public surface: exported functions/types/classes/endpoints with no doc comment, or with a one-word comment that adds nothing ("the user", "helper", "TODO").\n' +
          'In scope: items visible outside their module (public/exported keyword, default-public in the language).\n' +
          'Out of scope: private helpers, test code, generated code.\n' +
          'For each: name the item (file:line), say what a caller needs to know (purpose / inputs / failure modes / side effects), draft the missing doc in one or two sentences.',
      ),
      p(
        'Find stale or misleading comments: comments that describe behavior the code no longer has, parameter docs that name wrong arguments, TODO/FIXME notes for work that is already done, copy-pasted comments from a different function.\n' +
          'In scope: doc comments and inline comments on functions ≥ 5 lines.\n' +
          'Out of scope: trivial inline notes that are merely terse.\n' +
          'For each: quote the comment (file:line), quote the code it disagrees with, say what the comment should say.',
      ),
    ],
  },
  {
    id: 'tests',
    icon: 'test',
    title: 'Test Coverage',
    desc: 'Untested branches and weak assertions.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        'Find untested branches in code that already has tests nearby: error paths, fallback branches, validation rejections, retries, cancellation. Only call out gaps in functions that already have at least one test — do not demand tests where none exist.\n' +
          'In scope: functions with a matching test file/section.\n' +
          'Out of scope: experimental code, mock helpers, modules with no tests at all.\n' +
          'For each: name the function and the missing branch (file:line), describe the input that would hit it, sketch the assertion in one line.',
      ),
      p(
        'Find weak assertions that would not catch a real regression: tests that only check "no exception", assertions against truthy/not-None when a specific value is expected, snapshot tests on volatile output, tests that mock the unit under test.\n' +
          'In scope: existing test files.\n' +
          'Out of scope: smoke tests that are explicitly meant to be shallow.\n' +
          'For each: quote the assertion (file:line), name what behavior it fails to pin, give a stronger assertion.',
      ),
    ],
  },
  {
    id: 'deps',
    icon: 'package',
    title: 'Dependency Audit',
    desc: 'Unused, risky, or stale packages.',
    strategy: 'structured_reviewer',
    useStopRules: true,
    prompts: [
      p(
        "Find dependencies declared in the manifest (package.json / Cargo.toml / pyproject / go.mod / requirements) that are not used in source.\n" +
          'Method: read the manifest, then check whether each name appears in an import / use / require statement anywhere in the project.\n' +
          'In scope: direct dependencies only.\n' +
          'Out of scope: transitive deps, peerDependencies, build-only deps that are used by scripts.\n' +
          'For each: name the dependency, name the manifest file, say where you looked for usages and found none.',
      ),
      p(
        'Find risky dependency patterns: pinned to an old major when newer is widely adopted, pulled from an unusual registry/git URL, package names that look typo-squatted (e.g. "reqeusts", "loadsh"), wildcards/floating versions on security-sensitive libs (auth, crypto, parsers).\n' +
          "In scope: the project's dependency manifests.\n" +
          'Out of scope: judging individual libraries you do not recognize — flag the pattern, not the name.\n' +
          'For each: name the dependency and version line (file:line), name the specific risk pattern, give the safer alternative spec.',
      ),
    ],
  },
];
