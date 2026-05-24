/**
 * Format a duration in milliseconds as a compact human label.
 *
 *   45s, 1m 23s, 1h 04m
 *
 * Returns "—" for null/undefined/negative inputs so callers can render
 * unconditionally.
 */
export function formatDuration(ms) {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return '—';
  const totalSec = Math.floor(ms / 1000);
  if (totalSec < 60) return `${totalSec}s`;
  const min = Math.floor(totalSec / 60);
  const sec = totalSec % 60;
  if (min < 60) return `${min}m ${String(sec).padStart(2, '0')}s`;
  const hr = Math.floor(min / 60);
  const remMin = min % 60;
  return `${hr}h ${String(remMin).padStart(2, '0')}m`;
}

/** Parse a backend ISO-8601 timestamp into epoch ms, or null. */
export function toEpochMs(iso) {
  if (!iso) return null;
  const t = Date.parse(iso);
  return Number.isFinite(t) ? t : null;
}
