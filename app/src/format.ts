const UNITS = ["B", "KB", "MB", "GB", "TB"];

/**
 * Splits into number + unit so the UI can style the unit smaller, like Pulse.
 * Memory uses base 1024 (like Activity Monitor); disk and file sizes use base 1000 (like Finder).
 */
export function bytesParts(value: number, base: 1024 | 1000 = 1024): [string, string] {
  let v = Math.max(0, value);
  let u = 0;
  while (v >= base && u < UNITS.length - 1) {
    v /= base;
    u++;
  }
  const digits = u === 0 ? 0 : v < 10 ? 2 : v < 100 ? 1 : 0;
  return [v.toFixed(digits), UNITS[u]];
}

export const bytes = (v: number) => bytesParts(v).join(" ");
/** Disk / file sizes, matching Finder (1 GB = 10⁹ bytes). */
export const fileBytesParts = (v: number) => bytesParts(v, 1000);
export const fileBytes = (v: number) => fileBytesParts(v).join(" ");
export const rate = (v: number) => `${bytes(v)}/s`;
export const pct = (fraction: number, digits = 0) => `${(fraction * 100).toFixed(digits)}%`;

export function duration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export function uptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  return d > 0 ? `${d} ngày ${h} giờ` : `${h} giờ`;
}

/** Unix seconds → "hôm nay" / "3 ngày trước" / "2 tháng trước". */
export function ago(seconds: number): string {
  const days = Math.floor((Date.now() / 1000 - seconds) / 86400);
  if (days < 1) return "hôm nay";
  if (days < 60) return `${days} ngày trước`;
  if (days < 730) return `${Math.floor(days / 30)} tháng trước`;
  return `${Math.floor(days / 365)} năm trước`;
}
