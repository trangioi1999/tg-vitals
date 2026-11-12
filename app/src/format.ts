const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Splits into number + unit so the UI can style the unit smaller, like Pulse. */
export function bytesParts(value: number): [string, string] {
  let v = Math.max(0, value);
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u++;
  }
  const digits = u === 0 ? 0 : v < 10 ? 2 : v < 100 ? 1 : 0;
  return [v.toFixed(digits), UNITS[u]];
}

export const bytes = (v: number) => bytesParts(v).join(" ");
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
