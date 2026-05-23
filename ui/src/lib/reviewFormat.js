// Helpers for the Review page: output-type detection, markdown rendering,
// and JSON syntax highlighting. Ported from demo-review-ui/review.jsx.

const escapeHtml = (s) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** Returns `'structured'` when output has a findings array, else `'text'`. */
export function detectOutputType(output) {
  if (typeof output === 'string') return 'text';
  if (output && Array.isArray(output.findings)) return 'structured';
  return 'text';
}

/** Inline `<code>` for `\`literals\`` inside suggested-fix text. */
export function highlightInlineCode(text) {
  return escapeHtml(text).replace(/`([^`]+)`/g, '<code>$1</code>');
}

/** Tiny JSON syntax highlighter for the Raw JSON tab. */
export function highlightJson(value) {
  const json = JSON.stringify(value, null, 2);
  const esc = escapeHtml(json);
  return esc.replace(
    /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(\.\d+)?([eE][+-]?\d+)?)/g,
    (match) => {
      let cls = 'n';
      if (/^"/.test(match)) cls = /:$/.test(match) ? 'k' : 's';
      else if (/true|false/.test(match)) cls = 'b';
      else if (/null/.test(match)) cls = 'nl';
      return `<span class="${cls}">${match}</span>`;
    },
  );
}

/**
 * Minimal Markdown renderer (headings, lists, blockquotes, code blocks,
 * inline code/bold/italic/link). Sufficient for plaintext agent output.
 */
export function renderMarkdown(src) {
  const inline = (line) =>
    line
      .replace(/`([^`]+)`/g, (_, c) => `<code>${c}</code>`)
      .replace(/\*\*([^*]+)\*\*/g, (_, c) => `<strong>${c}</strong>`)
      .replace(/(^|[^*])\*([^*\n]+)\*/g, (_, p, c) => `${p}<em>${c}</em>`)
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, t, h) => `<a href="${h}">${t}</a>`);

  const lines = src.split('\n');
  const out = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];

    if (/^```/.test(line)) {
      const buf = [];
      i++;
      while (i < lines.length && !/^```/.test(lines[i])) {
        buf.push(escapeHtml(lines[i]));
        i++;
      }
      i++;
      out.push(`<pre><code>${buf.join('\n')}</code></pre>`);
      continue;
    }

    const h = line.match(/^(#{1,4})\s+(.*)$/);
    if (h) {
      const lvl = h[1].length;
      out.push(`<h${lvl}>${inline(escapeHtml(h[2]))}</h${lvl}>`);
      i++;
      continue;
    }

    if (/^---+\s*$/.test(line)) {
      out.push('<hr/>');
      i++;
      continue;
    }

    if (/^>\s?/.test(line)) {
      const buf = [];
      while (i < lines.length && /^>\s?/.test(lines[i])) {
        buf.push(lines[i].replace(/^>\s?/, ''));
        i++;
      }
      out.push(`<blockquote>${renderMarkdown(buf.join('\n'))}</blockquote>`);
      continue;
    }

    if (/^[-*]\s+/.test(line)) {
      const buf = [];
      while (i < lines.length && /^[-*]\s+/.test(lines[i])) {
        buf.push(`<li>${inline(escapeHtml(lines[i].replace(/^[-*]\s+/, '')))}</li>`);
        i++;
      }
      out.push(`<ul>${buf.join('')}</ul>`);
      continue;
    }

    if (/^\d+\.\s+/.test(line)) {
      const buf = [];
      while (i < lines.length && /^\d+\.\s+/.test(lines[i])) {
        buf.push(`<li>${inline(escapeHtml(lines[i].replace(/^\d+\.\s+/, '')))}</li>`);
        i++;
      }
      out.push(`<ol>${buf.join('')}</ol>`);
      continue;
    }

    if (/^\s*$/.test(line)) {
      i++;
      continue;
    }

    const buf = [];
    while (
      i < lines.length &&
      !/^\s*$/.test(lines[i]) &&
      !/^(#{1,4}\s|```|>\s?|[-*]\s+|\d+\.\s+|---+\s*$)/.test(lines[i])
    ) {
      buf.push(inline(escapeHtml(lines[i])));
      i++;
    }
    if (buf.length) out.push(`<p>${buf.join(' ')}</p>`);
  }

  return out.join('\n');
}

/** Compact severity → short class name used by `.sev`/`.finding-card.<x>`. */
export const SEV_SHORT = {
  critical: 'crit',
  high: 'high',
  medium: 'med',
  low: 'low',
  info: 'info',
};

/** Human-readable kind labels for the kind-badge. */
export const KIND_LABEL = {
  bug: 'BUG',
  fix: 'FIX',
  suggestion: 'SUGGESTION',
  idea: 'IDEA',
  warning: 'WARNING',
  vulnerability: 'VULN',
  change: 'CHANGE',
  other: 'OTHER',
};

/** Relative-time formatter for the recent-runs list. */
export function relativeTime(isoString) {
  const then = new Date(isoString).getTime();
  if (!Number.isFinite(then)) return '';
  const secs = Math.max(0, Math.round((Date.now() - then) / 1000));
  if (secs < 60) return `${secs}s ago`;
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.round(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.round(hrs / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(isoString).toLocaleDateString();
}

/** Pretty-prints a wall-clock duration between two ISO timestamps. */
export function formatDuration(startIso, endIso) {
  const start = new Date(startIso).getTime();
  const end = new Date(endIso).getTime();
  if (!Number.isFinite(start) || !Number.isFinite(end) || end < start) return '';
  const secs = Math.round((end - start) / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const rem = secs % 60;
  if (mins < 60) return rem ? `${mins}m ${rem}s` : `${mins}m`;
  const hrs = Math.floor(mins / 60);
  return `${hrs}h ${mins % 60}m`;
}

/** Shortens a token count to `1.8M` / `84.2k` style. */
export function formatTokens(n) {
  if (!n && n !== 0) return '';
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

/** Counts findings by severity. Missing severities default to 0. */
export function countSeverities(findings) {
  const c = { critical: 0, high: 0, medium: 0, low: 0, info: 0 };
  for (const f of findings ?? []) {
    if (c[f.severity] != null) c[f.severity] += 1;
  }
  return c;
}
