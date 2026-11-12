import { useEffect, useRef } from "react";

interface Series {
  values: number[];
  color: string;
}

interface Props {
  /** Stacked bottom-up; all series share the same scale. */
  series: Series[];
  /** Fixed max (e.g. 1 for fractions). Omit to auto-scale to the peak. */
  max?: number;
  rows?: number;
  height?: number;
  /** Colour of unlit dots. */
  dim: string;
  /** Horizontal distance between dot centres, in CSS px. */
  pitch?: number;
}

/** LED-style dot matrix: one column per sample, newest on the right. */
export function DotChart({ series, max, rows = 5, height = 44, dim, pitch = 6.5 }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;

    const draw = () => {
      const width = canvas.clientWidth;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      const ctx = canvas.getContext("2d")!;
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, width, height);

      const cols = Math.max(1, Math.floor(width / pitch));
      const offsetX = (width - cols * pitch) / 2 + pitch / 2;
      const rowGap = height / rows;
      const radius = Math.min(pitch, rowGap) * 0.3;
      const length = series[0]?.values.length ?? 0;

      let peak = max ?? 0;
      if (max === undefined) {
        for (let i = 0; i < length; i++) {
          peak = Math.max(peak, series.reduce((sum, s) => sum + (s.values[i] ?? 0), 0));
        }
        peak = peak || 1;
      }

      for (let c = 0; c < cols; c++) {
        const sample = length - cols + c;
        const colors: string[] = [];
        if (sample >= 0) {
          for (const s of series) {
            const n = Math.round(((s.values[sample] ?? 0) / peak) * rows);
            for (let k = 0; k < n && colors.length < rows; k++) colors.push(s.color);
          }
          if (colors.length === 0) colors.push(series[0].color); // baseline, like Pulse
        }
        for (let r = 0; r < rows; r++) {
          ctx.fillStyle = colors[r] ?? dim;
          ctx.beginPath();
          ctx.arc(offsetX + c * pitch, height - rowGap * (r + 0.5), radius, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    };

    draw();
    const observer = new ResizeObserver(draw);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [series, max, rows, height, dim, pitch]);

  return <canvas ref={ref} className="dot-chart" style={{ height }} />;
}
