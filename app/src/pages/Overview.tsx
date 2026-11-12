import { Battery, Cpu, HardDrive, LayoutGrid, MemoryStick, Wifi, Zap } from "lucide-react";
import type { Snapshot } from "../api";
import type { PageId } from "../components/Sidebar";
import { DotChart } from "../components/DotChart";
import { Donut } from "../components/Donut";
import { ACCENT, AppIcon, BigNumber, Card, Stats, dimOf } from "../components/ui";
import { bytes, bytesParts, pct, rate } from "../format";
import type { History } from "../useVitals";

export function Overview({ s, h, go }: { s: Snapshot; h: History; go: (p: PageId) => void }) {
  const m = s.memory;
  const used = m.app + m.wired + m.compressed;
  const [memValue, memUnit] = bytesParts(used);
  const [freeValue, freeUnit] = bytesParts(s.disk.free);
  const [netValue, netUnit] = bytesParts(s.network.inPerSec);
  const topMemory = [...s.processes].sort((a, b) => b.memory - a.memory).slice(0, 6);
  const topCpu = [...s.processes].sort((a, b) => b.cpu - a.cpu).slice(0, 5);
  const maxCpu = Math.max(1, topCpu[0]?.cpu ?? 1);
  const free = Math.max(0, m.total - used - m.cached);

  return (
    <div className="grid-3">
      <Card title="CPU" icon={Cpu} color={ACCENT.cpu} onOpen={() => go("cpu")}>
        <BigNumber label="Hiện tại" value={(s.cpu.total * 100).toFixed(0)} unit="%" />
        <Stats items={[
          ["User", pct(s.cpu.user)],
          ["System", pct(s.cpu.system)],
          ["Nhiệt độ", s.cpu.temperature ? `${s.cpu.temperature.toFixed(0)} °C` : "—"],
        ]} />
        <DotChart dim={dimOf(ACCENT.cpu)} max={1}
          series={[{ values: h.cpuSystem, color: ACCENT.gpu }, { values: h.cpuUser, color: ACCENT.cpu }]} />
      </Card>

      <Card title="Bộ nhớ" icon={MemoryStick} color={ACCENT.memory} onOpen={() => go("memory")}>
        <BigNumber label={`Đang dùng / ${bytes(m.total)}`} value={memValue} unit={memUnit}
          badge={<span className={`pill ${m.pressure}`}>{m.pressure === "normal" ? "✓ Bình thường" : m.pressure === "warning" ? "Căng" : "Quá tải"}</span>} />
        <Stats items={[["App", bytes(m.app)], ["Wired", bytes(m.wired)], ["Nén", bytes(m.compressed)]]} />
        <DotChart dim={dimOf(ACCENT.memory)} max={1} series={[{ values: h.memory, color: ACCENT.memory }]} />
      </Card>

      <Card title="Năng lượng" icon={Zap} color={ACCENT.energy} onOpen={() => go("energy")}>
        <BigNumber label="Công suất toàn máy" value={(s.battery?.powerDraw ?? 0).toFixed(1)} unit="W" />
        <Stats items={[
          ["P-core", pct(avg(s.cpu.perCore.slice(0, s.cpu.pCores)))],
          ["E-core", pct(avg(s.cpu.perCore.slice(s.cpu.pCores)))],
          ["Swap", bytes(m.swapUsed)],
        ]} />
        <DotChart dim={dimOf(ACCENT.energy)} series={[{ values: h.power, color: ACCENT.energy }]} />
      </Card>

      <Card title="Ổ đĩa" icon={HardDrive} color={ACCENT.disk} onOpen={() => go("disk")}>
        <BigNumber label={`Còn trống / ${bytes(s.disk.total)}`} value={freeValue} unit={freeUnit} />
        <Stats items={[
          ["Đọc", rate(s.disk.readPerSec)],
          ["Ghi", rate(s.disk.writePerSec)],
          ["Đã dùng", pct(1 - s.disk.free / s.disk.total)],
        ]} />
        <DotChart dim={dimOf(ACCENT.disk)}
          series={[{ values: h.diskWrite, color: ACCENT.network }, { values: h.diskRead, color: ACCENT.disk }]} />
      </Card>

      <Card title="Mạng" icon={Wifi} color={ACCENT.network} onOpen={() => go("network")}>
        <BigNumber label="Đang tải xuống" value={netValue} unit={`${netUnit}/s`} />
        <Stats items={[
          ["Tải lên", rate(s.network.outPerSec)],
          ["Đã nhận", bytes(s.network.totalIn)],
          ["Đã gửi", bytes(s.network.totalOut)],
        ]} />
        <DotChart dim={dimOf(ACCENT.network)}
          series={[{ values: h.netOut, color: ACCENT.cpu }, { values: h.netIn, color: ACCENT.network }]} />
      </Card>

      <Card title="Pin" icon={Battery} color={ACCENT.battery} onOpen={() => go("energy")}>
        {s.battery ? (
          <>
            <BigNumber label={s.battery.onAdapter ? "Đang cắm sạc" : "Dùng pin"} value={(s.battery.percent * 100).toFixed(0)} unit="%" />
            <Stats items={[
              ["Trạng thái", s.battery.charging ? "Đang sạc" : s.battery.onAdapter ? "Đã cắm" : "Đang xả"],
              ["Chu kỳ", String(s.battery.cycles)],
              ["Độ chai", pct(s.battery.health)],
            ]} />
            <div className="battery-bar">
              <div style={{ width: pct(s.battery.percent) }} />
            </div>
          </>
        ) : (
          <BigNumber label="Máy bàn" value="—" />
        )}
      </Card>

      <Card title="Bộ nhớ theo loại" icon={MemoryStick} color={ACCENT.memory}>
        <div className="donut-row">
          <Donut center={pct(used / m.total)} sub="đang dùng" slices={[
            { value: m.app, color: ACCENT.memory },
            { value: m.wired, color: ACCENT.disk },
            { value: m.compressed, color: ACCENT.network },
            { value: m.cached, color: ACCENT.gray },
            { value: free, color: "#3a3a3c" },
          ]} />
          <ul className="legend">
            {([["App", m.app, ACCENT.memory], ["Wired", m.wired, ACCENT.disk], ["Nén", m.compressed, ACCENT.network],
              ["Cache", m.cached, ACCENT.gray], ["Trống", free, "#48484a"]] as const).map(([label, v, color]) => (
              <li key={label}><i style={{ background: color }} />{label}<span>{bytes(v)}</span></li>
            ))}
          </ul>
        </div>
      </Card>

      <Card title="Bộ nhớ theo app" icon={LayoutGrid} color={ACCENT.memory} right={`${bytes(topMemory.reduce((a, p) => a + p.memory, 0))}`}>
        <div className="treemap">
          {topMemory.map((p, i) => (
            <div key={p.pid} className={`tm tm-${i}`} style={{ background: `${ACCENT.memory}${i === 0 ? "26" : "14"}` }}>
              <AppIcon name={p.name} icon={p.icon} size={i < 2 ? 20 : 16} />
              {i < 4 && <div className="tm-text"><b>{p.name}</b><span>{bytes(p.memory)}</span></div>}
            </div>
          ))}
        </div>
      </Card>

      <Card title="CPU theo app" icon={Cpu} color={ACCENT.cpu} onOpen={() => go("processes")}>
        <ul className="bar-list">
          {topCpu.map((p) => (
            <li key={p.pid}>
              <div className="bar-row">
                <AppIcon name={p.name} icon={p.icon} />
                <span className="bar-name">{p.name}</span>
                <span className="bar-value">{p.cpu.toFixed(1)}%</span>
              </div>
              <div className="bar"><div style={{ width: `${(p.cpu / maxCpu) * 100}%`, background: ACCENT.cpu }} /></div>
            </li>
          ))}
        </ul>
      </Card>
    </div>
  );
}

const avg = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);
