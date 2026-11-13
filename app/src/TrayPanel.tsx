import { Cpu, ExternalLink, MemoryStick, Power, Thermometer, Battery, Wifi } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { api } from "./api";
import { DotChart } from "./components/DotChart";
import { ACCENT, AppIcon, IconTile, dimOf } from "./components/ui";
import { bytes, bytesParts, pct, rate, uptime } from "./format";
import { useVitals } from "./useVitals";

/** Compact Pulse-style panel shown under the menu bar item. */
export default function TrayPanel() {
  const { snapshot: s, history: h } = useVitals(false);

  if (!s) return <div className="tray"><div className="tray-loading">Đang đọc số liệu…</div></div>;

  const m = s.memory;
  const used = m.app + m.wired + m.compressed;
  const top = [...s.processes].sort((a, b) => b.cpu - a.cpu).slice(0, 5);
  const [inValue, inUnit] = bytesParts(s.network.inPerSec);

  return (
    <div className="tray">
      <header className="tray-head">
        <div>
          <div className="tray-title">{s.machine.model}</div>
          <div className="tray-sub">{s.machine.chip} · Bật {uptime(s.machine.uptime)}</div>
        </div>
        <button className="icon-btn" title="Mở tg-vitals" onClick={() => api.showMain()}><ExternalLink size={15} /></button>
      </header>

      <div className="tray-grid">
        <Tile icon={Cpu} color={ACCENT.cpu} label="CPU" value={(s.cpu.total * 100).toFixed(0)} unit="%">
          <DotChart dim={dimOf(ACCENT.cpu)} max={1} height={30} rows={4} pitch={5}
            series={[{ values: h.cpuSystem, color: ACCENT.gpu }, { values: h.cpuUser, color: ACCENT.cpu }]} />
        </Tile>
        <Tile icon={MemoryStick} color={ACCENT.memory} label="Bộ nhớ" value={(used / m.total * 100).toFixed(0)} unit="%">
          <DotChart dim={dimOf(ACCENT.memory)} max={1} height={30} rows={4} pitch={5}
            series={[{ values: h.memory, color: ACCENT.memory }]} />
        </Tile>
        <Tile icon={Wifi} color={ACCENT.network} label="Mạng" value={inValue} unit={`${inUnit}/s`}>
          <div className="tray-note">↑ {rate(s.network.outPerSec)}</div>
        </Tile>
        {s.cpu.temperature != null ? (
          <Tile icon={Thermometer} color={ACCENT.disk} label="Nhiệt độ" value={s.cpu.temperature.toFixed(0)} unit="°C">
            <div className="tray-note">{s.battery ? `Pin ${pct(s.battery.percent)}` : `${bytes(used)} / ${bytes(m.total)}`}</div>
          </Tile>
        ) : (
          <Tile icon={Battery} color={ACCENT.battery} label="Pin" value={s.battery ? (s.battery.percent * 100).toFixed(0) : "—"} unit={s.battery ? "%" : ""}>
            <div className="tray-note">{s.battery ? `${s.battery.powerDraw.toFixed(1)} W` : "Máy bàn"}</div>
          </Tile>
        )}
      </div>

      <div className="tray-section">Dùng CPU nhiều nhất</div>
      <ul className="tray-list">
        {top.map((p) => (
          <li key={p.pid}>
            <AppIcon name={p.name} icon={p.icon} size={16} />
            <span className="bar-name">{p.name}</span>
            <span className="bar-value">{p.cpu.toFixed(1)}%</span>
          </li>
        ))}
      </ul>

      <footer className="tray-foot">
        <button className="btn" onClick={() => api.showMain()}>Mở tg-vitals</button>
        <button className="btn square" title="Thoát" onClick={() => api.quit()}><Power size={14} /></button>
      </footer>
    </div>
  );
}

function Tile({ icon, color, label, value, unit, children }: {
  icon: LucideIcon; color: string; label: string; value: string; unit: string; children?: ReactNode;
}) {
  return (
    <div className="tray-tile">
      <div className="tray-tile-head"><IconTile icon={icon} color={color} size={18} /><span style={{ color }}>{label}</span></div>
      <div className="big-row"><span className="tray-value">{value}</span><span className="big-unit small">{unit}</span></div>
      {children}
    </div>
  );
}
