interface Slice {
  value: number;
  color: string;
}

export function Donut({ slices, size = 120, stroke = 14, center, sub }: {
  slices: Slice[];
  size?: number;
  stroke?: number;
  center: string;
  sub: string;
}) {
  const total = slices.reduce((s, x) => s + x.value, 0) || 1;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const gap = 3;
  let offset = 0;

  return (
    <div className="donut" style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        {slices.map((s, i) => {
          const len = Math.max(0, (s.value / total) * c - gap);
          const el = (
            <circle
              key={i}
              r={r} cx={size / 2} cy={size / 2}
              fill="none" stroke={s.color} strokeWidth={stroke} strokeLinecap="round"
              strokeDasharray={`${len} ${c - len}`}
              strokeDashoffset={-offset}
              transform={`rotate(-90 ${size / 2} ${size / 2})`}
            />
          );
          offset += (s.value / total) * c;
          return el;
        })}
      </svg>
      <div className="donut-center">
        <div className="donut-value">{center}</div>
        <div className="donut-sub">{sub}</div>
      </div>
    </div>
  );
}
