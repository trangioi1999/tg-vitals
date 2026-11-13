import type { LucideIcon } from "lucide-react";
import { ChevronRight } from "lucide-react";
import type { CSSProperties, ReactNode } from "react";

/** Apple system colours, used as the accent for each metric. */
export const ACCENT = {
  cpu: "#3a82f7",
  memory: "#32d74b",
  gpu: "#ff375f",
  disk: "#ff9f0a",
  network: "#40c8e0",
  battery: "#32d74b",
  energy: "#ffd60a",
  purple: "#bf5af2",
  gray: "#8e8e93",
} as const;

export const dimOf = (hex: string) => `${hex}2e`;

export function IconTile({ icon: Icon, color, size = 26 }: { icon: LucideIcon; color: string; size?: number }) {
  return (
    <span className="icon-tile" style={{ width: size, height: size, background: `${color}26`, color }}>
      <Icon size={size * 0.58} strokeWidth={2.2} />
    </span>
  );
}

export function Card({
  title, icon, color, onOpen, right, children, className = "", style,
}: {
  title: string;
  icon: LucideIcon;
  color: string;
  onOpen?: () => void;
  right?: ReactNode;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <section className={`card ${className}`} style={style}>
      <header className="card-head" onClick={onOpen} role={onOpen ? "button" : undefined} tabIndex={onOpen ? 0 : undefined} onKeyDown={onOpen ? (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onOpen(); } } : undefined}>
        <IconTile icon={icon} color={color} />
        <h3 style={{ color }}>{title}</h3>
        <span className="card-right">{right}</span>
        {onOpen && <ChevronRight size={16} className="chev" />}
      </header>
      {children}
    </section>
  );
}

export function BigNumber({ label, value, unit, badge }: { label: string; value: string; unit?: string; badge?: ReactNode }) {
  return (
    <div className="big">
      <div className="big-label">{label}</div>
      <div className="big-row">
        <span className="big-value">{value}</span>
        {unit && <span className="big-unit">{unit}</span>}
        {badge && <span className="big-badge">{badge}</span>}
      </div>
    </div>
  );
}

export function Stats({ items }: { items: [string, ReactNode][] }) {
  return (
    <div className="stats">
      {items.map(([label, value]) => (
        <div key={label}>
          <div className="stat-label">{label}</div>
          <div className="stat-value">{value}</div>
        </div>
      ))}
    </div>
  );
}

/** Coloured letter tile used when a process has no bundle icon. */
export function AppIcon({ name, icon, size = 18 }: { name: string; icon: string | null; size?: number }) {
  if (icon) return <img src={icon} width={size} height={size} className="app-icon" alt="" />;
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  const hue = hash % 360;
  return (
    <span className="app-icon letter" style={{ width: size, height: size, background: `hsl(${hue} 55% 46%)`, fontSize: size * 0.55 }}>
      {name[0]?.toUpperCase()}
    </span>
  );
}
