import { useId } from "react";

const AXIS = 18; // px reserved under the plot for the HTML axis labels

/**
 * Lightweight SVG history plot. Fixed scales are used for fractions; rates share a scale.
 * The SVG stretches horizontally (preserveAspectRatio="none") but its CSS height equals the
 * viewBox height, so only x is scaled; strokes are non-scaling and text lives in HTML.
 */
export function LineChart({ series, max, height = 88, axes = false, label = "Lịch sử hoạt động" }: {
  series: { values: number[]; color: string }[]; max?: number; height?: number; axes?: boolean; label?: string;
}) {
  const id = useId().replace(/:/g, "");
  const peak = max ?? Math.max(1, ...series.flatMap(s => s.values.filter(Number.isFinite)));
  const plot = height - (axes ? AXIS : 0);
  const width = 600, top = 8, bottom = plot - 4;
  const samples = Math.max(0, ...series.map(s => s.values.length - 1));
  return <div className="line-chart" role="img" aria-label={label}>
    <svg viewBox={`0 0 ${width} ${plot}`} preserveAspectRatio="none" style={{ height: plot }} aria-hidden="true">
      <defs>{series.map((s, i) => <linearGradient key={i} id={`${id}-${i}`} x1="0" y1="0" x2="0" y2="1"><stop stopColor={s.color} stopOpacity=".38"/><stop offset="1" stopColor={s.color} stopOpacity=".02"/></linearGradient>)}</defs>
      {[0, .5, 1].map(v => <line key={v} x1="0" x2={width} y1={top + v * (bottom - top)} y2={top + v * (bottom - top)} stroke="#ffffff12" strokeDasharray="3 4" vectorEffect="non-scaling-stroke"/>)}
      {series.map((s, i) => {
        if (!s.values.length) return null;
        const step = width / Math.max(1, s.values.length - 1);
        let points = "";
        for (let n = 0; n < s.values.length; n++) {
          const v = s.values[n];
          const y = bottom - Math.min(1, Math.max(0, Number.isFinite(v) ? v / peak : 0)) * (bottom - top);
          points += `${(n * step).toFixed(1)},${y.toFixed(1)} `;
        }
        return <g key={i}>
          <polygon points={`0,${bottom} ${points}${width},${bottom}`} fill={`url(#${id}-${i})`}/>
          <polyline points={points} fill="none" stroke={s.color} strokeWidth="2" vectorEffect="non-scaling-stroke" strokeLinejoin="round"/>
        </g>;
      })}
    </svg>
    {axes && <div className="line-axis" aria-hidden="true"><span>−{samples} s</span><span>Hiện tại</span></div>}
  </div>;
}
