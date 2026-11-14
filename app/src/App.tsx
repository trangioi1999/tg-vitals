import { Pause, Play, Search } from "lucide-react";
import { useState } from "react";
import { Sidebar, type PageId } from "./components/Sidebar";
import { ACCENT } from "./components/ui";
import { pct, bytes, fileBytes } from "./format";
import { Cleanup } from "./pages/Cleanup";
import { Uninstall } from "./pages/Uninstall";
import { Detail, type MetricPage } from "./pages/Detail";
import { Overview } from "./pages/Overview";
import { Processes } from "./pages/Processes";
import { useAppVersion, useVitals } from "./useVitals";

const TITLES: Record<PageId, string> = {
  overview: "Tổng quan", processes: "Tiến trình", cpu: "CPU", memory: "Bộ nhớ", disk: "Ổ đĩa",
  network: "Mạng", energy: "Năng lượng", cleanup: "Dọn dẹp", uninstall: "Gỡ app", about: "Về máy này",
};

type Filter = "all" | "app" | "mine" | "system";
const FILTERS: [Filter, string][] = [["all", "Tất cả"], ["app", "App"], ["mine", "Của tôi"], ["system", "Hệ thống"]];

export default function App() {
  const initial = (new URLSearchParams(location.search).get("page") as PageId) || "overview";
  const [page, setPage] = useState<PageId>(initial);
  const [paused, setPaused] = useState(false);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const { snapshot: s, history: h } = useVitals(paused);

  const threads = s?.processes.reduce((a, p) => a + p.threads, 0) ?? 0;
  const subtitle =
    page === "processes" && s ? `${s.processes.length} process · ${threads} luồng`
      : page === "cleanup" ? "Chỉ chuyển vào Thùng rác, không xóa vĩnh viễn"
      : page === "uninstall" ? "Gỡ app cùng dữ liệu nó để lại · mọi thứ vào Thùng rác"
        : s?.machine.model ?? "";

  return (
    <div className="app">
      <Sidebar page={page} onSelect={setPage} snapshot={s} />
      <main>
        <header className="toolbar" data-tauri-drag-region>
          <div className="title" data-tauri-drag-region>
            <h1>{TITLES[page]}</h1>
            <div className="subtitle">{subtitle}</div>
          </div>
          {page === "processes" && (
            <>
              <label className="search">
                <Search size={14} />
                <input placeholder="Tìm tiến trình hoặc PID" value={search} onChange={(e) => setSearch(e.target.value)} />
              </label>
              <div className="segmented">
                {FILTERS.map(([id, label]) => (
                  <button key={id} className={filter === id ? "on" : ""} onClick={() => setFilter(id)}>{label}</button>
                ))}
              </div>
            </>
          )}
          {page !== "cleanup" && page !== "uninstall" && page !== "about" && <span className={`live-status ${paused ? "paused" : ""}`}><i/>{paused ? "Tạm dừng" : "Trực tiếp"}</span>}
          {page !== "cleanup" && page !== "uninstall" && (
            <button className="icon-btn boxed" onClick={() => setPaused((p) => !p)} title={paused ? "Tiếp tục" : "Tạm dừng"}>
              {paused ? <Play size={14} /> : <Pause size={14} />}
            </button>
          )}
        </header>

        <div className={`content ${page === "processes" || page === "uninstall" ? "flush" : ""}`}>
          {!s ? null
            : page === "overview" ? <Overview s={s} h={h} go={setPage} />
              : page === "processes" ? <Processes s={s} h={h} search={search} filter={filter} />
                : page === "cleanup" ? <Cleanup />
                : page === "uninstall" ? <Uninstall />
                  : page === "about" ? <About s={s} />
                    : <Detail page={page as MetricPage} s={s} h={h} />}
        </div>

        {s && page === "processes" && (
          <footer className="statusbar">
            <span><i style={{ background: ACCENT.cpu }} />CPU <b>{pct(s.cpu.total)}</b></span>
            <span><i style={{ background: ACCENT.memory }} />Bộ nhớ <b>{bytes(s.memory.app + s.memory.wired + s.memory.compressed)}</b></span>
            {s.battery && <span><i style={{ background: ACCENT.energy }} />Công suất <b>{s.battery.powerDraw.toFixed(1)} W</b></span>}
          </footer>
        )}
      </main>
    </div>
  );
}

function About({ s }: { s: NonNullable<ReturnType<typeof useVitals>["snapshot"]> }) {
  const version = useAppVersion();
  return (
    <section className="card about">
      <h2>{s.machine.model}</h2>
      <dl className="details">
        <dt>Chip</dt><dd>{s.machine.chip}</dd>
        <dt>Core</dt><dd>{s.cpu.pCores} hiệu năng + {s.cpu.perCore.length - s.cpu.pCores} tiết kiệm</dd>
        <dt>Bộ nhớ</dt><dd>{bytes(s.memory.total)}</dd>
        <dt>Ổ đĩa</dt><dd>{fileBytes(s.disk.total)}</dd>
        <dt>tg-vitals</dt><dd>{version ? `Phiên bản ${version}` : "—"}</dd>
      </dl>
    </section>
  );
}
