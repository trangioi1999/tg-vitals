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
  // The latest draw closure, so the ResizeObserver (set up once) always paints current data.
  const drawRef = useRef<() => void>(() => {});

  drawRef.current = () => {
    const canvas = ref.current;
    if (!canvas) return;
    const width = canvas.clientWidth;
    if (width === 0) return;
    const dpr = window.devicePixelRatio || 1;
    const w = Math.round(width * dpr), hgt = Math.round(height * dpr);
    // Reallocating the backing store every tick is wasteful; only do it when the size changed.
    if (canvas.width !== w || canvas.height !== hgt) {
      canvas.width = w;
      canvas.height = hgt;
    }
    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
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
        let any = false;
        for (const s of series) {
          const v = s.values[sample] ?? 0;
          if (v > 0) any = true;
          const n = Math.round((v / peak) * rows);
          for (let k = 0; k < n && colors.length < rows; k++) colors.push(s.color);
        }
        // Some activity that rounds to zero still lights one dot (like Pulse); true idle stays dark.
        if (colors.length === 0 && any) colors.push(series[0].color);
      }
      for (let r = 0; r < rows; r++) {
        ctx.fillStyle = colors[r] ?? dim;
        ctx.beginPath();
        ctx.arc(offsetX + c * pitch, height - rowGap * (r + 0.5), radius, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  };

  // Repaint on every render: the history arrays are mutated in place, so props can't tell us what changed.
  useEffect(() => drawRef.current());

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const observer = new ResizeObserver(() => drawRef.current());
    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);

  return <canvas ref={ref} className="dot-chart" style={{ height }} />;
}
