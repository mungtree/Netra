// Batch presets shown in the task grid.
//
// Each preset is a prompt-set (see PLAN §6.5): a series of prompts a batch runs
// over the selected project, whose outputs are then aggregated by `strategy`.
// Clicking a card calls `createBatch` + `runBatch` with these prompts.
export const TASK_PRESETS = [
  {
    id: 'bugs',
    icon: 'bug',
    title: 'Find Bugs',
    desc: 'Detect logic errors, edge cases, and broken assumptions.',
    featured: true,
    strategy: 'structured_reviewer',
    prompts: [
      'Review this project for logic errors and incorrect assumptions. List each with file and line.',
      'Find unhandled edge cases: empty inputs, boundary values, and error paths.',
      'Identify race conditions, unsafe shared state, and concurrency bugs.',
    ],
  },
  {
    id: 'vulns',
    icon: 'shield',
    title: 'Find Vulnerabilities',
    desc: 'OWASP, injection, secrets, and unsafe patterns.',
    strategy: 'structured_reviewer',
    prompts: [
      'Scan for injection vulnerabilities: SQL, command, and path traversal.',
      'Find hardcoded secrets, API keys, and credentials committed to source.',
      'Identify unsafe input handling and missing validation at trust boundaries.',
    ],
  },
  {
    id: 'ideas',
    icon: 'bulb',
    title: 'Generate Ideas',
    desc: 'Surface new features and product directions.',
    strategy: 'concat',
    prompts: [
      'Suggest three new features that would improve this project.',
      'Identify the weakest part of the user experience and how to fix it.',
    ],
  },
  {
    id: 'refactor',
    icon: 'wand',
    title: 'Refactor',
    desc: 'Identify duplication, complexity, and cleaner abstractions.',
    strategy: 'structured_reviewer',
    prompts: [
      'Find duplicated code that should be extracted into shared helpers.',
      'Identify overly complex functions and propose simpler structures.',
      'Suggest cleaner abstractions for the most tangled modules.',
    ],
  },
  {
    id: 'perf',
    icon: 'gauge',
    title: 'Performance',
    desc: 'Hot paths, allocations, and async bottlenecks.',
    strategy: 'structured_reviewer',
    prompts: [
      'Identify hot paths and expensive operations in this project.',
      'Find unnecessary allocations and copies.',
      'Detect blocking calls and async bottlenecks.',
    ],
  },
  {
    id: 'docs',
    icon: 'book',
    title: 'Documentation',
    desc: 'Missing docstrings, stale comments, unclear APIs.',
    strategy: 'concat',
    prompts: [
      'Find public items missing documentation.',
      'Identify stale or misleading comments.',
    ],
  },
  {
    id: 'tests',
    icon: 'test',
    title: 'Test Coverage',
    desc: 'Untested branches and weak assertions.',
    strategy: 'structured_reviewer',
    prompts: [
      'Identify untested branches and error paths.',
      'Find weak assertions that would not catch real regressions.',
    ],
  },
  {
    id: 'deps',
    icon: 'package',
    title: 'Dependency Audit',
    desc: 'Outdated, unused, or risky packages.',
    strategy: 'concat',
    prompts: [
      'List dependencies that appear outdated or unmaintained.',
      'Identify unused dependencies that could be removed.',
    ],
  },
];
