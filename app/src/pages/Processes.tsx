import { ChevronDown, ChevronUp, FolderOpen, X } from "lucide-react";
import { useMemo, useState } from "react";
import { api, type ProcessRow, type Snapshot } from "../api";
import { DotChart } from "../components/DotChart";
import { ACCENT, AppIcon, dimOf } from "../components/ui";
import { bytes, duration } from "../format";
import type { History } from "../useVitals";

type SortKey = "name" | "cpu" | "memory" | "threads" | "cpuTime" | "pid";
type Filter = "all" | "app" | "mine" | "system";

const COLUMNS: { key: SortKey; label: string; align?: "right" }[] = [
  { key: "name", label: "Tên" },
  { key: "cpu", label: "% CPU", align: "right" },
  { key: "memory", label: "Bộ nhớ", align: "right" },
  { key: "threads", label: "Luồng", align: "right" },
  { key: "cpuTime", label: "Thời gian CPU", align: "right" },
  { key: "pid", label: "PID", align: "right" },
];

export function Processes({ s, h, search, filter }: { s: Snapshot; h: History; search: string; filter: Filter }) {
  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: "cpu", desc: true });
  const [selected, setSelected] = useState<number | null>(
    () => [...s.processes].filter((p) => p.kind === "app").sort((a, b) => b.cpu - a.cpu)[0]?.pid ?? null);

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = s.processes.filter((p) =>
      (filter === "all" || p.kind === filter) &&
      (!q || p.name.toLowerCase().includes(q) || String(p.pid) === q));
    const dir = sort.desc ? -1 : 1;
    return list.sort((a, b) => {
      const x = a[sort.key], y = b[sort.key];
      return (typeof x === "string" ? x.localeCompare(y as string) : (x as number) - (y as number)) * dir;
    });
  }, [s.processes, search, filter, sort]);

  const current = s.processes.find((p) => p.pid === selected) ?? null;

  return (
    <div className="proc-layout">
      <div className="table-wrap">
        <table className="proc-table">
          <thead>
            <tr>
              {COLUMNS.map((c) => (
                <th key={c.key} className={c.align} onClick={() =>
                  setSort((o) => ({ key: c.key, desc: o.key === c.key ? !o.desc : c.key !== "name" }))}>
                  {c.label}
                  {sort.key === c.key && (sort.desc ? <ChevronDown size={12} /> : <ChevronUp size={12} />)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((p) => (
              <tr key={p.pid} className={p.pid === selected ? "selected" : ""} onClick={() => setSelected(p.pid)}>
                <td><span className="name-cell"><AppIcon name={p.name} icon={p.icon} />{p.name}</span></td>
                <td className={`right ${heat(p.cpu, 20, 40)}`}>{p.cpu.toFixed(1)}</td>
                <td className={`right ${heat(p.memory, 1.5 * 1024 ** 3, 3 * 1024 ** 3)}`}>{bytes(p.memory)}</td>
                <td className="right">{p.threads}</td>
                <td className="right muted-num">{duration(p.cpuTime)}</td>
                <td className="right muted-num">{p.pid}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {current && <Inspector key={current.pid} p={current} h={h} onClose={() => setSelected(null)} />}
    </div>
  );
}

function Inspector({ p, h, onClose }: { p: ProcessRow; h: History; onClose: () => void }) {
  const hist = h.perProcess.get(p.pid);
  const [confirmForce, setConfirmForce] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const kill = (force: boolean) => api.killProcess(p.pid, force).then(() => setError(null), (e) => setError(String(e)));

  return (
    <aside className="inspector">
      <header>
        <AppIcon name={p.name} icon={p.icon} size={40} />
        <div>
          <div className="insp-name">{p.name}</div>
          <div className="insp-sub">PID {p.pid} · {p.user}</div>
        </div>
        <button className="icon-btn" onClick={onClose}><X size={16} /></button>
      </header>

      <div className="insp-actions">
        <button className="btn" onClick={() => kill(false)}>Thoát</button>
        <button className="btn danger" onClick={() => (confirmForce ? kill(true) : setConfirmForce(true))}>
          {confirmForce ? "Chắc chắn?" : "Buộc thoát"}
        </button>
        <button className="btn square" title="Hiện trong Finder" disabled={!p.path}
          onClick={() => api.revealPath(p.path).catch((e) => setError(String(e)))}><FolderOpen size={15} /></button>
      </div>
      {error && <div className="insp-error">{error}</div>}

      <div className="mini" style={{ background: `${ACCENT.cpu}1a` }}>
        <div className="mini-head"><span style={{ color: ACCENT.cpu }}>CPU</span><b>{p.cpu.toFixed(0)}%</b></div>
        <DotChart dim={dimOf(ACCENT.cpu)} series={[{ values: hist?.cpu ?? [], color: ACCENT.cpu }]} height={34} rows={4} pitch={5.5} />
      </div>
      <div className="mini" style={{ background: `${ACCENT.memory}1a` }}>
        <div className="mini-head"><span style={{ color: ACCENT.memory }}>Bộ nhớ</span><b>{bytes(p.memory)}</b></div>
        <DotChart dim={dimOf(ACCENT.memory)} series={[{ values: hist?.memory ?? [], color: ACCENT.memory }]} height={34} rows={4} pitch={5.5} />
      </div>

      <div className="insp-title">CHI TIẾT</div>
      <dl className="details">
        <dt>Thời gian CPU</dt><dd>{duration(p.cpuTime)}</dd>
        <dt>Luồng</dt><dd>{p.threads}</dd>
        <dt>Bộ nhớ</dt><dd>{bytes(p.memory)}</dd>
        <dt>Đã đọc đĩa</dt><dd>{bytes(p.diskRead)}</dd>
        <dt>Đã ghi đĩa</dt><dd>{bytes(p.diskWritten)}</dd>
        <dt>Bắt đầu</dt><dd>{new Date(p.startedAt * 1000).toLocaleString("vi-VN", { hour: "2-digit", minute: "2-digit", day: "2-digit", month: "2-digit" })}</dd>
        <dt>Loại</dt><dd>{p.kind === "app" ? "Ứng dụng" : p.kind === "mine" ? "Của tôi" : "Hệ thống"}</dd>
      </dl>
    </aside>
  );
}

const heat = (v: number, warn: number, hot: number) => (v >= hot ? "hot" : v >= warn ? "warm" : "");
