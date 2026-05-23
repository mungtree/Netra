// JSON import/export for task batch definitions.
//
// The file format is small on purpose — definition only, no run history — so a
// batch is trivial to share, edit by hand, and re-import.

const FORMAT = 'chatur.batch/v1';
const KNOWN_STRATEGIES = new Set([
  'concat',
  'reviewer',
  'structured_reviewer',
  'schema_merge',
]);

/** Serializes a preset to the v1 batch JSON. */
export function serializeBatch(preset) {
  return JSON.stringify(
    {
      format: FORMAT,
      name: preset.title ?? preset.name ?? 'Untitled batch',
      strategy: preset.strategy,
      prompts: preset.prompts,
      output_schema: preset.output_schema ?? null,
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
  const preset = {
    id: `custom-${cryptoId()}`,
    icon: 'bookmark',
    title: data.name.trim(),
    desc: `${data.prompts.length} prompts · ${data.strategy}`,
    strategy: data.strategy,
    prompts: data.prompts,
    output_schema: data.output_schema ?? null,
    custom: true,
  };
  return { ok: true, preset };
}

function cryptoId() {
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID().slice(0, 8);
  }
  return Math.random().toString(36).slice(2, 10);
}
