export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let i = -1;
  let v = bytes;
  do {
    v /= 1024;
    i++;
  } while (v >= 1024 && i < units.length - 1);
  return `${v.toFixed(1)} ${units[i]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.max(0, Math.round(ms))}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  const totalSec = Math.round(s);
  const m = Math.floor(totalSec / 60);
  const rem = totalSec - m * 60;
  return `${m}m ${rem}s`;
}

export function formatPing(micros: number): string {
  if (!Number.isFinite(micros) || micros < 0) return "—";
  if (micros < 1000) return `${Math.round(micros)}µs`;
  const ms = micros / 1000;
  return ms < 10 ? `${ms.toFixed(1)}ms` : `${Math.round(ms)}ms`;
}
