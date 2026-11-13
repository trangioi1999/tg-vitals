import { Battery, Cpu, HardDrive, List, MemoryStick, Sparkles, Thermometer, Trash2, Wifi, Zap } from "lucide-react";
import type { Snapshot } from "../api";
import type { PageId } from "../components/Sidebar";
import { LineChart } from "../components/LineChart";
import { Donut } from "../components/Donut";
import { ACCENT, AppIcon, BigNumber, Card, Stats } from "../components/ui";
import { bytes, bytesParts, fileBytes, fileBytesParts, pct, rate } from "../format";
import type { History } from "../useVitals";

const avg = (xs: number[]) => xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0;
const clamp = (v: number) => Math.max(0, Math.min(1, v));

export function Overview({ s, h, go }: { s: Snapshot; h: History; go: (p: PageId) => void }) {
  const m = s.memory, used = m.app + m.wired + m.compressed;
  const [memValue, memUnit] = bytesParts(used), [freeValue, freeUnit] = fileBytesParts(s.disk.free);
  const [netValue, netUnit] = bytesParts(s.network.inPerSec);
  const top = [...s.processes].sort((a, b) => b.cpu - a.cpu).slice(0, 4);
  const peak = Math.max(1, ...top.map(p => p.cpu));
  const diskUsed = s.disk.total > 0 ? clamp(1 - s.disk.free / s.disk.total) : 0;
  const free = Math.max(0, m.total - used - m.cached);
  const slices = [{ value: m.app, color: ACCENT.memory }, { value: m.wired, color: ACCENT.disk }, { value: m.compressed, color: ACCENT.network }, { value: m.cached, color: ACCENT.gray }, { value: free, color: "#41444c" }];
  return <div className="overview-grid">
    <Card className="overview-cpu" title="CPU" icon={Cpu} color={ACCENT.cpu} onOpen={() => go("cpu")}>
      <div className="cpu-summary"><BigNumber label="Hiện tại" value={(s.cpu.total * 100).toFixed(0)} unit="%"/>
        <Stats items={[["User", pct(s.cpu.user)], ["System", pct(s.cpu.system)]]}/>
        <span className="temperature"><Thermometer size={14}/>{s.cpu.temperature != null ? `${s.cpu.temperature.toFixed(0)} °C` : "—"}</span>
      </div>
      <LineChart axes height={110} max={1} label="Mức sử dụng CPU theo thời gian" series={[{ values: h.cpuUser.map((v, i) => v + (h.cpuSystem[i] ?? 0)), color: ACCENT.cpu }]}/>
      <div className="core-summary">{s.cpu.pCores > 0 ? <>
        <CoreMeter label="P-core" value={avg(s.cpu.perCore.slice(0, s.cpu.pCores))}/>
        {s.cpu.perCore.length > s.cpu.pCores && <CoreMeter label="E-core" value={avg(s.cpu.perCore.slice(s.cpu.pCores))}/>}
      </> : <CoreMeter label={`${s.cpu.perCore.length} core`} value={avg(s.cpu.perCore)}/>}</div>
    </Card>
    <Card className="overview-memory" title="Bộ nhớ" icon={MemoryStick} color={ACCENT.memory} onOpen={() => go("memory")}>
      <BigNumber label={`Đang dùng / ${bytes(m.total)}`} value={memValue} unit={memUnit} badge={<span className={`pill ${m.pressure}`}>{m.pressure === "normal" ? "Bình thường" : m.pressure === "warning" ? "Áp lực cao" : "Quá tải"}</span>}/>
      <div className="memory-breakdown"><ul className="legend">{[["App", m.app, ACCENT.memory], ["Wired", m.wired, ACCENT.disk], ["Nén", m.compressed, ACCENT.network], ["Cache", m.cached, ACCENT.gray]].map(([label, value, color]) => <li key={label}><i style={{ background: String(color) }}/>{label}<span>{bytes(Number(value))}</span></li>)}<li className="swap-label">Swap<span>{bytes(m.swapUsed)}</span></li></ul>
        <Donut size={132} stroke={16} center={m.total > 0 ? pct(used / m.total) : "—"} sub="đang dùng" slices={slices}/>
      </div>
    </Card>
    <Card className="overview-disk" title="Ổ đĩa" icon={HardDrive} color={ACCENT.disk} onOpen={() => go("disk")}>
      <BigNumber label={`Còn trống / ${fileBytes(s.disk.total)}`} value={freeValue} unit={freeUnit}/>
      <div className="capacity"><div className="capacity-track"><div style={{ width: pct(diskUsed) }}/></div><b>{s.disk.total > 0 ? pct(diskUsed) : "—"}</b></div>
      <Stats items={[["Đọc", rate(s.disk.readPerSec)], ["Ghi", rate(s.disk.writePerSec)]]}/>
    </Card>
    <Card className="overview-network" title="Mạng" icon={Wifi} color={ACCENT.network} onOpen={() => go("network")}>
      <div className="network-summary"><BigNumber label="↓ Tải xuống" value={netValue} unit={`${netUnit}/s`}/><span className="upload">↑ {rate(s.network.outPerSec)}</span></div>
      <LineChart height={54} label="Tốc độ tải xuống và tải lên" series={[{ values: h.netOut, color: ACCENT.cpu }, { values: h.netIn, color: ACCENT.network }]}/>
      <div className="metric-foot"><span>Đã nhận</span><b>{bytes(s.network.totalIn)}</b></div>
    </Card>
    <Card className="overview-energy" title="Năng lượng" icon={Zap} color={ACCENT.energy} onOpen={() => go("energy")}>
      {s.battery ? <><div className="energy-summary"><BigNumber label="Công suất báo cáo" value={s.battery.powerDraw.toFixed(1)} unit="W"/><div className="battery-number"><span><Battery size={14}/> Pin</span><b>{pct(s.battery.percent)}</b></div></div>
        <div className="battery-shell" role="img" aria-label={`Pin ${pct(s.battery.percent)}`}><div className="battery-fill" style={{ width: pct(clamp(s.battery.percent)) }}/>{s.battery.charging && <Zap size={22} fill="currentColor"/>}</div>
        <Stats items={[["Trạng thái", s.battery.charging ? "Đang sạc" : s.battery.onAdapter ? "Đã cắm" : "Dùng pin"], ["Chu kỳ", String(s.battery.cycles)], ["Độ bền", pct(s.battery.health)]]}/></> : <div className="no-battery"><Battery size={28}/><b>Không có dữ liệu pin</b><span>Máy bàn hoặc cảm biến chưa khả dụng</span></div>}
    </Card>
    <Card className="overview-processes" title="Tiến trình nổi bật" icon={List} color={ACCENT.cpu} right="Xem tất cả" onOpen={() => go("processes")}>
      <table className="overview-table"><thead><tr><th>Tiến trình</th><th>CPU</th><th>Bộ nhớ</th></tr></thead><tbody>{top.map(p => <tr key={p.pid}><td><button className="process-link" onClick={() => go("processes")} title={p.name}><AppIcon name={p.name} icon={p.icon} size={22}/><span>{p.name}</span></button></td><td><span className="cpu-cell"><span className="bar"><span style={{ width: pct(p.cpu / peak) }}/></span>{p.cpu.toFixed(1)}%</span></td><td>{bytes(p.memory)}</td></tr>)}</tbody></table>
      {!top.length && <div className="empty">Chưa có dữ liệu tiến trình</div>}
    </Card>
    <Card className="overview-care" title="Dọn dẹp" icon={Sparkles} color={ACCENT.disk} onOpen={() => go("cleanup")}>
      <h2>Dọn dữ liệu không cần thiết</h2><p>Cache, log và dữ liệu build.</p><div className="care-tags"><span>Xcode</span><span>npm</span><span>node_modules</span></div>
      <button className="btn care-action" onClick={() => go("cleanup")}><Sparkles size={18}/>Mở Dọn dẹp</button><div className="care-note"><Trash2 size={13}/>Quét và chọn trước khi chuyển vào Thùng rác.</div>
    </Card>
  </div>;
}
function CoreMeter({ label, value }: { label: string; value: number }) {
  return <div className="core-meter"><span>{label}</span><b>{pct(value)}</b><div><i style={{ width: pct(clamp(value)) }}/></div></div>;
}
