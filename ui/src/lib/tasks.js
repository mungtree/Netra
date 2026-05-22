// Batch presets shown in the task grid.
//
// Each entry will, once P5's BatchExecutor lands, map to a prompt-set JSON file
// under `prompts/` and run as a batch over the selected project (see PLAN §10).
// Until then the cards render disabled.
export const TASK_PRESETS = [
  {
    id: 'bugs',
    icon: 'bug',
    title: 'Find Bugs',
    desc: 'Detect logic errors, edge cases, and broken assumptions.',
    featured: true,
  },
  {
    id: 'vulns',
    icon: 'shield',
    title: 'Find Vulnerabilities',
    desc: 'OWASP, injection, secrets, and unsafe patterns.',
  },
  {
    id: 'ideas',
    icon: 'bulb',
    title: 'Generate Ideas',
    desc: 'Surface new features and product directions.',
  },
  {
    id: 'refactor',
    icon: 'wand',
    title: 'Refactor',
    desc: 'Identify duplication, complexity, and cleaner abstractions.',
  },
  {
    id: 'perf',
    icon: 'gauge',
    title: 'Performance',
    desc: 'Hot paths, allocations, and async bottlenecks.',
  },
  {
    id: 'docs',
    icon: 'book',
    title: 'Documentation',
    desc: 'Missing docstrings, stale comments, unclear APIs.',
  },
  {
    id: 'tests',
    icon: 'test',
    title: 'Test Coverage',
    desc: 'Untested branches and weak assertions.',
  },
  {
    id: 'deps',
    icon: 'package',
    title: 'Dependency Audit',
    desc: 'Outdated, unused, or risky packages.',
  },
];
