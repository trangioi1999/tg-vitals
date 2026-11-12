import { Cpu, HardDrive, MemoryStick, Wifi, Zap } from "lucide-react";
import type { Snapshot } from "../api";
import { DotChart } from "../components/DotChart";
import { ACCENT, AppIcon, BigNumber, Card, Stats, dimOf } from "../components/ui";
import { bytes, pct, rate } from "../format";
import type { History } from "../useVitals";

export type MetricPage = "cpu" | "memory" | "disk" | "network" | "energy";

/** Hardware pages: one large chart, key numbers, and the processes driving that metric. */
export function Detail({ page, s, h }: { page: MetricPage; s: Snapshot; h: History }) {
  const m = s.memory;
  const used = m.app + m.wired + m.compressed;

  const config = {
    cpu: {
      title: "CPU", icon: Cpu, color: ACCENT.cpu, big: [(s.cpu.total * 100).toFixed(0), "%"],
      stats: [["User", pct(s.cpu.user)], ["System", pct(s.cpu.system)], ["Nhiệt độ", s.cpu.temperature ? `${s.cpu.temperature.toFixed(0)} °C` : "—"], ["Số core", String(s.cpu.perCore.length)]],
      series: [{ values: h.cpuSystem, color: ACCENT.gpu }, { values: h.cpuUser, color: ACCENT.cpu }], max: 1,
      top: [...s.processes].sort((a, b) => b.cpu - a.cpu).map((p) => [p, `${p.cpu.toFixed(1)}%`, p.cpu] as const),
    },
    memory: {
      title: "Bộ nhớ", icon: MemoryStick, color: ACCENT.memory, big: bytes(used).split(" "),
      stats: [["App", bytes(m.app)], ["Wired", bytes(m.wired)], ["Nén", bytes(m.compressed)], ["Swap", bytes(m.swapUsed)]],
      series: [{ values: h.memory, color: ACCENT.memory }], max: 1,
      top: [...s.processes].sort((a, b) => b.memory - a.memory).map((p) => [p, bytes(p.memory), p.memory] as const),
    },
    disk: {
      title: "Ổ đĩa", icon: HardDrive, color: ACCENT.disk, big: bytes(s.disk.free).split(" "),
      stats: [["Đọc", rate(s.disk.readPerSec)], ["Ghi", rate(s.disk.writePerSec)], ["Dung lượng", bytes(s.disk.total)], ["Đã dùng", pct(1 - s.disk.free / s.disk.total)]],
      series: [{ values: h.diskWrite, color: ACCENT.network }, { values: h.diskRead, color: ACCENT.disk }], max: undefined,
      top: [...s.processes].sort((a, b) => b.diskWritten - a.diskWritten).map((p) => [p, bytes(p.diskWritten), p.diskWritten] as const),
    },
    network: {
      title: "Mạng", icon: Wifi, color: ACCENT.network, big: rate(s.network.inPerSec).split(" "),
      stats: [["Tải lên", rate(s.network.outPerSec)], ["Đã nhận", bytes(s.network.totalIn)], ["Đã gửi", bytes(s.network.totalOut)], ["Kết nối", "—"]],
      series: [{ values: h.netOut, color: ACCENT.cpu }, { values: h.netIn, color: ACCENT.network }], max: undefined,
      top: [] as (readonly [Snapshot["processes"][number], string, number])[],
    },
    energy: {
      title: "Năng lượng", icon: Zap, color: ACCENT.energy, big: [(s.battery?.powerDraw ?? 0).toFixed(1), "W"],
      stats: [["Pin", s.battery ? pct(s.battery.percent) : "—"], ["Độ chai", s.battery ? pct(s.battery.health) : "—"], ["Chu kỳ", s.battery ? String(s.battery.cycles) : "—"], ["Sạc", s.battery?.onAdapter ? "Đang cắm" : "Không"]],
      series: [{ values: h.power, color: ACCENT.energy }], max: undefined,
      top: [...s.processes].sort((a, b) => b.cpu - a.cpu).map((p) => [p, `${(p.cpu * 0.08).toFixed(2)} W`, p.cpu] as const),
    },
  }[page];

  const peak = Math.max(1e-9, ...config.top.slice(0, 8).map(([, , v]) => v));

  return (
    <div className="detail">
      <Card title={config.title} icon={config.icon} color={config.color} className="detail-main">
        <div className="detail-head">
          <BigNumber label="Hiện tại" value={config.big[0]} unit={config.big.slice(1).join(" ")} />
          <Stats items={config.stats as [string, string][]} />
        </div>
        <DotChart dim={dimOf(config.color)} series={config.series} max={config.max} rows={10} height={150} pitch={8} />
      </Card>

      {page === "cpu" && (
        <Card title="Từng core" icon={Cpu} color={ACCENT.cpu}>
          <div className="cores">
            {s.cpu.perCore.map((v, i) => (
              <div key={i} className="core">
                <div className="core-bar"><div style={{ height: pct(v), background: i < s.cpu.pCores ? ACCENT.cpu : ACCENT.network }} /></div>
                <span>{i < s.cpu.pCores ? "P" : "E"}{i + 1}</span>
              </div>
            ))}
          </div>
        </Card>
      )}

      {config.top.length > 0 && (
        <Card title="App dùng nhiều nhất" icon={config.icon} color={config.color}>
          <ul className="bar-list">
            {config.top.slice(0, 8).map(([p, label, v]) => (
              <li key={p.pid}>
                <div className="bar-row">
                  <AppIcon name={p.name} icon={p.icon} />
                  <span className="bar-name">{p.name}</span>
                  <span className="bar-value">{label}</span>
                </div>
                <div className="bar"><div style={{ width: `${(v / peak) * 100}%`, background: config.color }} /></div>
              </li>
            ))}
          </ul>
        </Card>
      )}
    </div>
  );
}
