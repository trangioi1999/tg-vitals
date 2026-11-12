import {
  Battery, Cpu, HardDrive, Info, LayoutGrid, List, MemoryStick, Settings, Sparkles, Wifi,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { Snapshot } from "../api";
import { pct, rate, uptime } from "../format";
import { ACCENT, IconTile } from "./ui";

export type PageId = "overview" | "processes" | "cpu" | "memory" | "disk" | "network" | "energy" | "cleanup" | "about";

interface Item {
  id: PageId;
  label: string;
  icon: LucideIcon;
  color: string;
  value?: (s: Snapshot) => string;
}

const SECTIONS: { title?: string; items: Item[] }[] = [
  {
    items: [
      { id: "overview", label: "Tổng quan", icon: LayoutGrid, color: ACCENT.gray },
      { id: "processes", label: "Process", icon: List, color: ACCENT.gray, value: (s) => String(s.processes.length) },
    ],
  },
  {
    title: "Phần cứng",
    items: [
      { id: "cpu", label: "CPU", icon: Cpu, color: ACCENT.cpu, value: (s) => pct(s.cpu.total) },
      { id: "memory", label: "Bộ nhớ", icon: MemoryStick, color: ACCENT.memory,
        value: (s) => pct((s.memory.app + s.memory.wired + s.memory.compressed) / s.memory.total) },
      { id: "disk", label: "Ổ đĩa", icon: HardDrive, color: ACCENT.disk, value: (s) => rate(s.disk.readPerSec + s.disk.writePerSec) },
      { id: "network", label: "Mạng", icon: Wifi, color: ACCENT.network, value: (s) => rate(s.network.inPerSec) },
      { id: "energy", label: "Năng lượng", icon: Battery, color: ACCENT.energy, value: (s) => (s.battery ? pct(s.battery.percent) : "—") },
    ],
  },
  {
    title: "Hệ thống",
    items: [
      { id: "cleanup", label: "Dọn dẹp", icon: Sparkles, color: ACCENT.disk },
      { id: "about", label: "Về máy này", icon: Info, color: ACCENT.gray },
    ],
  },
];

export function Sidebar({ page, onSelect, snapshot }: { page: PageId; onSelect: (p: PageId) => void; snapshot: Snapshot | null }) {
  return (
    <aside className="sidebar">
      <div className="traffic" data-tauri-drag-region>
        <span /><span /><span />
      </div>
      <nav>
        {SECTIONS.map((section, i) => (
          <div key={i} className="nav-section">
            {section.title && <div className="nav-title">{section.title}</div>}
            {section.items.map((item) => (
              <button
                key={item.id}
                className={`nav-item ${page === item.id ? "active" : ""}`}
                onClick={() => onSelect(item.id)}
              >
                <IconTile icon={item.icon} color={page === item.id ? "#ffffff" : item.color} size={20} />
                <span className="nav-label">{item.label}</span>
                {snapshot && item.value && <span className="nav-value">{item.value(snapshot)}</span>}
              </button>
            ))}
          </div>
        ))}
      </nav>
      <footer className="sidebar-foot">
        <div>
          <div className="machine">{snapshot?.machine.model ?? "Mac"}</div>
          <div className="machine-sub">
            {snapshot ? `${snapshot.machine.chip} · Bật ${uptime(snapshot.machine.uptime)}` : ""}
          </div>
        </div>
        <Settings size={16} className="muted" />
      </footer>
    </aside>
  );
}
