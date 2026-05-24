// Catalogs for the prompt editor: aggregation strategies the runner knows
// about, and the canned stop conditions the editor can append to prompts.
// Ported from demo-prompt-editor/promptsData.js.

export const STRATEGIES = [
  {
    id: 'concat',
    name: 'Concatenate',
    desc: 'Run each prompt sequentially; concatenate raw outputs into one report.',
  },
  {
    id: 'reviewer',
    name: 'Reviewer',
    desc: 'First prompt produces a draft. Following prompts critique and refine it.',
  },
  {
    id: 'structured_reviewer',
    name: 'Structured Reviewer',
    desc: 'Reviewer with a schema. Each pass must emit JSON matching output_schema.',
  },
  {
    id: 'schema_merge',
    name: 'Schema Merge',
    desc: 'Each prompt fills a slice of output_schema. Results are merged by key.',
  },
];

export const STOP_CONDITIONS = [
  {
    id: 'default',
    name: 'Strategy default',
    desc: "Use the strategy's built-in halt rule (recommended).",
    text: '',
  },
  {
    id: 'tight_findings',
    name: 'Tight findings',
    desc: 'Cap findings to a small number with concrete locations. Good for small local models.',
    text: [
      'Hard limits:',
      '- Skip vendored, generated, test fixtures.',
      '- Report at most 2 findings. If fewer real issues exist, return fewer.',
      '- Each finding needs a concrete location (file path, plus line number when visible).',
      '- Do not propose refactors, tests, or unrelated improvements. Do not rewrite large blocks.',
      '- If nothing meaningful surfaces, return an empty findings list — do not fabricate.',
      '- Stop after listing findings. No closing summary, no next-steps, no questions.',
    ].join('\n'),
  },
  {
    id: 'broad_findings',
    name: 'Broad findings',
    desc: 'Same shape as tight, but allows up to 4 findings and a wider file budget.',
    text: [
      'Hard limits:',
      '- Inspect at most 6 files. Pick the most relevant by name; skip vendored, generated, test fixtures, and any file > 800 lines.',
      '- Report at most 4 findings. If fewer real issues exist, return fewer.',
      '- Each finding needs a concrete location (file path, plus line number when visible).',
      '- Do not rewrite large blocks.',
      '- If nothing meaningful surfaces, return an empty findings list — do not fabricate.',
      '- Stop after listing findings. No closing summary, no next-steps, no questions.',
    ].join('\n'),
  },
  {
    id: 'single_issue',
    name: 'Single issue',
    desc: 'Force exactly one finding (or none). Useful for first-pass triage.',
    text: [
      'Hard limits:',
      '- Skip vendored, generated, test fixtures.',
      '- Report exactly one finding, or none. Pick the highest-severity real issue.',
      '- The finding needs a concrete location (file path, plus line number when visible).',
      '- Do not propose refactors, tests, or unrelated improvements.',
      '- If nothing meaningful surfaces, return an empty findings list — do not fabricate.',
      '- Stop after the finding. No closing summary, no next-steps, no questions.',
    ].join('\n'),
  },
  {
    id: 'no_new',
    name: 'No new findings',
    desc: 'Halt the reviewer loop when a pass would only repeat earlier findings.',
    text: [
      'Hard limits:',
      '- Skip vendored, generated, test fixtures.',
      '- Do not repeat any finding already reported on an earlier pass.',
      '- If this pass would produce no findings beyond those already reported, return an empty findings list.',
      '- Each finding needs a concrete location (file path, plus line number when visible).',
      '- Do not propose refactors or unrelated improvements. Do not rewrite large blocks.',
      '- Stop after listing new findings. No closing summary, no next-steps, no questions.',
    ].join('\n'),
  },
  {
    id: 'custom',
    name: 'Custom…',
    desc: 'Write your own hard-limits block. Appended verbatim to the prompt.',
    text: '',
  },
];

export const BATCH_FORMAT = 'chatur.batch/v1';

/** Returns a normalized preset shape, filling defaults for missing fields. */
export function normalizePreset(raw) {
  return {
    id: raw.id,
    icon: raw.icon ?? 'bookmark',
    title: raw.title ?? raw.name ?? 'Untitled batch',
    desc: raw.desc ?? `${(raw.prompts ?? []).length} prompts · ${raw.strategy ?? 'concat'}`,
    strategy: raw.strategy ?? 'concat',
    stopConditionId: raw.stopConditionId ?? 'default',
    customStopText: raw.customStopText ?? '',
    appendToAll: !!raw.appendToAll,
    output_schema: raw.output_schema ?? null,
    prompts: Array.isArray(raw.prompts) && raw.prompts.length > 0 ? raw.prompts.slice() : [''],
    builtin: !!raw.builtin,
    custom: !raw.builtin,
    useStopRules: raw.useStopRules,
  };
}
