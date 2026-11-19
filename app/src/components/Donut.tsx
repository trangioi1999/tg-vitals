interface Slice {
  value: number;
  color: string;
}

export function Donut({ slices, size = 120, stroke = 14, center, sub, label }: {
  slices: Slice[];
  size?: number;
  stroke?: number;
  center: string;
  sub: string;
  /** Accessible description; defaults to the centre text. */
  label?: string;
}) {
  const total = slices.reduce((s, x) => s + x.value, 0) || 1;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const gap = 3; // butt caps, so this is the real gap between slices
  let offset = 0;

  return (
    <div className="donut" style={{ width: size, height: size }} role="img" aria-label={label ?? `${center} ${sub}`}>
      <svg aria-hidden="true" width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {slices.map((s, i) => {
          const len = Math.max(0, (s.value / total) * c - gap);
          const el = (
            <circle
              key={i}
              r={r} cx={size / 2} cy={size / 2}
              fill="none" stroke={s.color} strokeWidth={stroke} strokeLinecap="butt"
              strokeDasharray={`${len} ${c - len}`}
              strokeDashoffset={-offset}
              transform={`rotate(-90 ${size / 2} ${size / 2})`}
            />
          );
          offset += (s.value / total) * c;
          return el;
        })}
      </svg>
      <div className="donut-center" aria-hidden="true">
        <div className="donut-value">{center}</div>
        <div className="donut-sub">{sub}</div>
      </div>
    </div>
  );
}
