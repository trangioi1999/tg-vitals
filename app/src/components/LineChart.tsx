import { useId } from "react";

/** Lightweight SVG history plot. Fixed scales are used for fractions; rates share a scale. */
export function LineChart({ series, max, height = 88, axes = false, label = "Lịch sử hoạt động" }: {
  series: { values: number[]; color: string }[]; max?: number; height?: number; axes?: boolean; label?: string;
}) {
  const id = useId().replace(/:/g, "");
  const peak = max ?? Math.max(1, ...series.flatMap(s => s.values.filter(Number.isFinite)));
  const width = 600, top = 8, bottom = height - (axes ? 22 : 4);
  return <svg className="line-chart" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label={label}>
    <defs>{series.map((s, i) => <linearGradient key={i} id={`${id}-${i}`} x1="0" y1="0" x2="0" y2="1"><stop stopColor={s.color} stopOpacity=".38"/><stop offset="1" stopColor={s.color} stopOpacity=".02"/></linearGradient>)}</defs>
    {[0, .5, 1].map(v => <line key={v} x1="0" x2={width} y1={top + v * (bottom - top)} y2={top + v * (bottom - top)} stroke="#ffffff12" strokeDasharray="3 4"/>)}
    {series.map((s, i) => {
      const points = s.values.map((v, n) => `${n * width / Math.max(1, s.values.length - 1)},${bottom - Math.min(1, Math.max(0, Number.isFinite(v) ? v / peak : 0)) * (bottom - top)}`);
      if (!points.length) return null;
      return <g key={i}><polygon points={`0,${bottom} ${points.join(" ")} ${width},${bottom}`} fill={`url(#${id}-${i})`}/><polyline points={points.join(" ")} fill="none" stroke={s.color} strokeWidth="2" vectorEffect="non-scaling-stroke" strokeLinejoin="round"/></g>;
    })}
    {axes && <g fill="#9699a2" fontSize="10"><text x="0" y={height - 3}>−{Math.max(0, ...series.map(s => s.values.length - 1))} s</text><text x={width} y={height - 3} textAnchor="end">Hiện tại</text></g>}
  </svg>;
}
