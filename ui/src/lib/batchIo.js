// JSON import/export for task batch definitions.
//
// v1 format: definition only, no run history — a batch is trivial to share,
// edit by hand, and re-import. Stop-condition + schema fields are additive and
// optional; older files without them still parse.

import { STOP_CONDITIONS } from './prompts/promptsData.js';

const FORMAT = 'chatur.batch/v1';
const KNOWN_STRATEGIES = new Set([
  'concat',
  'reviewer',
  'structured_reviewer',
  'schema_merge',
]);

/** Resolves the stop-condition text for a preset, returns '' when none applies. */
function resolveStopText(preset) {
  const id = preset.stopConditionId ?? 'default';
  if (id === 'default') return '';
  if (id === 'custom') return (preset.customStopText ?? '').trim();
  const cond = STOP_CONDITIONS.find((s) => s.id === id);
  return cond?.text ?? '';
}

/** Serializes output_schema for prompt-baking. Returns '' when unusable. */
function schemaBlock(preset) {
  if (preset.strategy !== 'schema_merge') return '';
  const schema = preset.output_schema;
  if (!schema) return '';
  let body;
  try {
    body = JSON.stringify(schema, null, 2);
  } catch {
    return '';
  }
  return `— Output schema —\n${body}`;
}

/**
 * Bakes the stop text (per appendToAll) and the schema_merge schema (every
 * prompt) into the prompt bodies.
 */
function bakedPrompts(preset) {
  const stop = resolveStopText(preset);
  const schema = schemaBlock(preset);
  const prompts = preset.prompts ?? [];
  if (!stop && !schema) return prompts.slice();
  const all = !!preset.appendToAll;
  return prompts.map((p, i) => {
    const parts = [p];
    if (schema) parts.push(schema);
    const stopApplies = stop && (all || i === prompts.length - 1);
    if (stopApplies) parts.push(`— Stop condition —\n${stop}`);
    return parts.join('\n\n');
  });
}

/** Serializes a preset to the v1 batch JSON. */
export function serializeBatch(preset) {
  return JSON.stringify(
    {
      format: FORMAT,
      name: preset.title ?? preset.name ?? 'Untitled batch',
      strategy: preset.strategy,
      prompts: bakedPrompts(preset),
      output_schema: preset.output_schema ?? null,
      stop_condition_id: preset.stopConditionId ?? 'default',
      custom_stop_text: preset.customStopText ?? '',
      append_to_all: !!preset.appendToAll,
    },
    null,
    2,
  );
}

/**
 * Parses and validates a batch JSON file. On success returns `{ok: true, preset}`;
 * otherwise `{ok: false, error}` with a user-facing message.
 */
export function parseBatch(text) {
  let data;
  try {
    data = JSON.parse(text);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${e.message}` };
  }
  if (!data || typeof data !== 'object') {
    return { ok: false, error: 'Expected a JSON object.' };
  }
  if (data.format !== FORMAT) {
    return {
      ok: false,
      error: `Unsupported format "${data.format}" — expected "${FORMAT}".`,
    };
  }
  if (typeof data.name !== 'string' || !data.name.trim()) {
    return { ok: false, error: 'Missing "name".' };
  }
  if (typeof data.strategy !== 'string' || !KNOWN_STRATEGIES.has(data.strategy)) {
    return {
      ok: false,
      error: `Unknown strategy "${data.strategy}". Allowed: ${[...KNOWN_STRATEGIES].join(', ')}.`,
    };
  }
  if (
    !Array.isArray(data.prompts) ||
    data.prompts.length === 0 ||
    !data.prompts.every((p) => typeof p === 'string' && p.trim())
  ) {
    return { ok: false, error: '"prompts" must be a non-empty array of strings.' };
  }
  const stopId =
    typeof data.stop_condition_id === 'string' &&
    STOP_CONDITIONS.some((s) => s.id === data.stop_condition_id)
      ? data.stop_condition_id
      : 'default';
  const preset = {
    id: `custom-${cryptoId()}`,
    icon: 'bookmark',
    title: data.name.trim(),
    desc: `${data.prompts.length} prompts · ${data.strategy}`,
    strategy: data.strategy,
    prompts: data.prompts,
    output_schema: data.output_schema ?? null,
    stopConditionId: stopId,
    customStopText:
      typeof data.custom_stop_text === 'string' ? data.custom_stop_text : '',
    appendToAll: !!data.append_to_all,
    custom: true,
    builtin: false,
  };
  return { ok: true, preset };
}

function cryptoId() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID().slice(0, 8);
  }
  return Math.random().toString(36).slice(2, 10);
}
