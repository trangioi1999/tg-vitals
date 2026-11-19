import { Check, CheckCircle2, Lock, Power, RefreshCw, Search, Terminal, XCircle } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api, isTauri, type AppInfo, type AppLeftover, type TrashReport } from "../api";
import { AppIcon, rowTabIndex, tableKeys } from "../components/ui";
import { ago, fileBytes, fileBytesParts } from "../format";

type Sort = "size" | "unused";

/** /Users/name/Library/… → ~/Library/… */
const tilde = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");
const iconOf = (path: string) => (isTauri ? `appicon://localhost${encodeURI(path)}` : null);

/** "Gỡ app": apps by size; pick one, review its leftovers, move everything to the Trash. */
export function Uninstall() {
  const [apps, setApps] = useState<AppInfo[] | null>(null);
  const [reading, setReading] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<Sort>("size");
  const [selected, setSelected] = useState<string | null>(null);
  // Last uninstall result, shown in the right panel once the app is gone from the list.
  const [result, setResult] = useState<{ app: AppInfo; report: TrashReport } | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  /** First load shows the reading screen; later refreshes keep the list on screen. */
  const load = async (silent = false) => {
    if (silent) setRefreshing(true);
    else { setApps(null); setReading(""); }
    try {
      const list = await api.listApps((e) => !silent && e.kind === "path" && setReading(e.path));
      setApps(list);
      setSelected((s) => (s && list.some((a) => a.path === s) ? s : null));
    } finally {
      setReading(null);
      setRefreshing(false);
    }
  };

  const finished = (app: AppInfo, report: TrashReport) => {
    setResult({ app, report });
    // The bundle left: drop it from the list right away, then refresh in the background.
    if (!report.failed.some((f) => f.path === app.path) && report.trashed > 0) {
      setApps((list) => list?.filter((a) => a.path !== app.path) ?? null);
      setSelected(null);
    }
    load(true);
  };
  useEffect(() => { load(); }, []);

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = (apps ?? []).filter((a) => !q || a.name.toLowerCase().includes(q) || a.bundleId.toLowerCase().includes(q));
    // "Lâu không dùng": never-used (null) first, then oldest.
    return sort === "size" ? list : [...list].sort((a, b) => (a.lastUsed ?? 0) - (b.lastUsed ?? 0));
  }, [apps, search, sort]);
  const current = apps?.find((a) => a.path === selected) ?? null;
  const ids = rows.map((a) => a.path);
  const total = (apps ?? []).reduce((s, a) => s + a.size, 0);

  if (!apps) {
    return (
      <div className="uninstall-loading">
        <section className="hero">
          <div className="hero-icon scanning"><Terminal size={28} /></div>
          <div className="hero-text">
            <div className="big-label">Đang đọc app…</div>
            <div className="hero-sub mono">{reading ? tilde(reading) : "/Applications"}</div>
            <div className="scan-track indeterminate" role="progressbar" aria-label="Đang đọc app"><div /></div>
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="proc-layout">
      <div className="uninstall-main">
        <div className="uninstall-bar">
          <label className="search">
            <Search size={14} />
            <input placeholder="Tìm app" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
          <div className="segmented">
            <button className={sort === "size" ? "on" : ""} onClick={() => setSort("size")}>Dung lượng</button>
            <button className={sort === "unused" ? "on" : ""} onClick={() => setSort("unused")}>Lâu không dùng</button>
          </div>
          <span className="uninstall-count">
            {refreshing && <RefreshCw size={12} className="spin" />}
            {apps.length} app · {fileBytes(total)}
          </span>
        </div>
        <div className="table-wrap">
        <table className="proc-table">
          <thead>
            <tr>
              <th>Tên</th>
              <th className="right sortable" aria-sort={sort === "size" ? "descending" : "none"}>
                <button type="button" className="th-sort" onClick={() => setSort("size")}>Dung lượng</button>
              </th>
              <th className="right sortable" aria-sort={sort === "unused" ? "ascending" : "none"}>
                <button type="button" className="th-sort" onClick={() => setSort("unused")}>Dùng lần cuối</button>
              </th>
            </tr>
          </thead>
          <tbody onKeyDown={tableKeys(ids, selected, setSelected)}>
            {rows.map((a, i) => (
              <tr key={a.path} data-row-id={a.path} className={a.path === selected ? "selected" : ""} onClick={() => setSelected(a.path)}
                tabIndex={rowTabIndex(a.path, i, selected, ids)} aria-selected={a.path === selected}>
                <td>
                  <span className="name-cell">
                    <AppIcon name={a.name} icon={iconOf(a.path)} size={22} />
                    <span className="app-name">{a.name}<small>{a.version}</small></span>
                    {a.running && <span className="pill-running">đang chạy</span>}
                    {!a.ownedByUser && <Lock size={11} className="lock" role="img" aria-label="cần mật khẩu admin" />}
                  </span>
                </td>
                <td className="right">{fileBytes(a.size)}</td>
                <td className="right muted-num">{a.lastUsed ? ago(a.lastUsed) : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
        </div>
      </div>
      {current
        ? <AppPanel key={current.path} app={current}
            lastReport={result?.app.path === current.path ? result.report : null}
            onFinished={(report) => finished(current, report)} onQuit={() => load(true)} />
        : result
          ? <ResultPanel app={result.app} report={result.report} onClose={() => setResult(null)} />
          : <aside className="inspector uninstall-panel empty-panel">Chọn một app để xem nó chiếm bao nhiêu và gỡ cùng dữ liệu nó để lại.</aside>}
    </div>
  );
}

function AppPanel({ app, lastReport, onFinished, onQuit }: {
  app: AppInfo; lastReport: TrashReport | null; onFinished: (r: TrashReport) => void; onQuit: () => void;
}) {
  const [leftovers, setLeftovers] = useState<AppLeftover[] | null>(null);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [quitError, setQuitError] = useState<string | null>(null);

  useEffect(() => {
    api.appLeftovers(app.path).then((l) => {
      setLeftovers(l);
      // Everything measurable is ticked; unreadable items stay unticked.
      setChecked(new Set([app.path, ...l.filter((x) => x.size !== null).map((x) => x.path)]));
    }, (e) => {
      // Still allow removing the .app itself; just say the leftovers are unknown.
      setLeftovers([]);
      setChecked(new Set([app.path]));
      setLoadError(String(e));
    });
  }, [app.path]);

  const sizeOf = (p: string) => (p === app.path ? app.size : leftovers?.find((l) => l.path === p)?.size ?? 0);
  const freed = [...checked].reduce((s, p) => s + sizeOf(p), 0);
  const [freedValue, freedUnit] = fileBytesParts(freed);
  const needsAdmin = (!app.ownedByUser && checked.has(app.path)) || (leftovers ?? []).some((l) => l.needsAdmin && checked.has(l.path));
  const toggle = (p: string) => { setConfirming(false); setChecked((s) => { const n = new Set(s); n.has(p) ? n.delete(p) : n.add(p); return n; }); };

  const uninstall = async () => {
    setConfirming(false);
    setBusy(true);
    try {
      onFinished(await api.uninstallApp(app.path, [...checked]));
    } catch (e) {
      onFinished({ trashed: 0, bytes: 0, failed: [{ path: app.path, reason: String(e) }] });
    } finally {
      setBusy(false);
    }
  };

  return (
    <aside className={`inspector uninstall-panel ${busy ? "is-busy" : ""}`}>
      {busy && (
        <div className="panel-busy" role="status">
          <RefreshCw size={22} className="spin" />
          <div className="busy-title">Đang gỡ {app.name}…</div>
          <div className="busy-sub">
            Chuyển {checked.size} mục ({fileBytes(freed)}) vào Thùng rác
            {needsAdmin && <><br />Finder sẽ hỏi mật khẩu admin</>}
          </div>
          <div className="scan-track indeterminate" role="progressbar" aria-label={`Đang gỡ ${app.name}`}><div /></div>
        </div>
      )}
      <header>
        <AppIcon name={app.name} icon={iconOf(app.path)} size={44} />
        <div>
          <div className="insp-name">{app.name}</div>
          <div className="insp-sub">{app.version} · {app.bundleId}</div>
        </div>
      </header>

      <div className="freed">
        <div className="big-label">Sẽ giải phóng</div>
        <div className="big-row"><span className="big-value">{freedValue}</span><span className="big-unit">{freedUnit}</span></div>
      </div>

      {app.brewCask && (
        <div className="note">Cài bằng Homebrew · gỡ bằng lệnh <code>brew uninstall --cask {app.brewCask}</code> để Homebrew không còn nhớ app này.</div>
      )}

      <ul className="leftovers">
        <Row path={app.path} name={`${app.name}.app`} size={app.size} admin={!app.ownedByUser}
          on={checked.has(app.path)} toggle={toggle} />
        {leftovers === null && <li className="muted">Đang tìm dữ liệu sót lại…</li>}
        {loadError && <li className="insp-error" role="alert">Không tìm được dữ liệu sót lại: {loadError}</li>}
        {leftovers?.length === 0 && !loadError && <li className="muted">Không có dữ liệu sót lại</li>}
        {leftovers?.map((l) => (
          <Row key={l.path} path={l.path} name={l.name} size={l.size} admin={l.needsAdmin}
            on={checked.has(l.path)} toggle={toggle} />
        ))}
      </ul>

      {lastReport && lastReport.failed.length > 0 && <ReportBox report={lastReport} />}
      {app.running ? (
        <>
          <button className="btn wide" onClick={() => {
            setQuitError(null);
            api.quitApp(app.bundleId).then(() => setTimeout(onQuit, 1500), (e) => setQuitError(String(e)));
          }}>
            <Power size={14} /> Thoát {app.name} trước khi gỡ
          </button>
          {quitError && <div className="insp-error" role="alert">Không thoát được {app.name}: {quitError}</div>}
        </>
      ) : confirming ? (
        <div className="confirm-row">
          <button className="btn" onClick={() => setConfirming(false)}>Huỷ</button>
          <button className="btn primary danger-fill" onClick={uninstall} autoFocus>{checked.has(app.path) ? "Xác nhận gỡ" : "Xác nhận chuyển"} · {fileBytes(freed)}</button>
        </div>
      ) : (
        <button className="btn primary wide" disabled={busy || checked.size === 0 || leftovers === null} onClick={() => setConfirming(true)}>
          {busy ? "Đang gỡ…" : checked.has(app.path) ? `Gỡ ${app.name}` : `Chuyển ${checked.size} mục vào Thùng rác`}
        </button>
      )}
      <div className="care-note">
        Mọi thứ vào Thùng rác, khôi phục được{needsAdmin ? " · macOS sẽ hỏi mật khẩu admin cho mục có ổ khoá" : ""}
      </div>
    </aside>
  );
}

/** Shown after the app left the list: what went to the Trash and what did not. */
function ResultPanel({ app, report, onClose }: { app: AppInfo; report: TrashReport; onClose: () => void }) {
  const ok = report.failed.length === 0;
  return (
    <aside className="inspector uninstall-panel result-panel">
      <div className={`result-icon ${ok ? "ok" : "bad"}`}>{ok ? <CheckCircle2 size={44} /> : <XCircle size={44} />}</div>
      <div className="insp-name">{ok ? `Đã gỡ ${app.name}` : `Gỡ ${app.name} chưa trọn vẹn`}</div>
      <div className="big-row"><span className="big-value">{fileBytesParts(report.bytes)[0]}</span><span className="big-unit">{fileBytesParts(report.bytes)[1]}</span></div>
      <div className="insp-sub">{report.trashed} mục đã vào Thùng rác · khôi phục được bằng "Put Back"</div>
      {!ok && <ReportBox report={report} />}
      <button className="btn wide" onClick={onClose}>Xong</button>
    </aside>
  );
}

function ReportBox({ report }: { report: TrashReport }) {
  return (
    <div className="uninstall-result bad">
      {report.failed.length} mục chưa gỡ được
      {report.failed.map((f) => <div key={f.path} className="fail"><span>{tilde(f.path)}</span>{f.reason}</div>)}
    </div>
  );
}

function Row({ path, name, size, admin, on, toggle }: {
  path: string; name: string; size: number | null; admin: boolean; on: boolean; toggle: (p: string) => void;
}) {
  return (
    <li title={path}>
      <button className={`check small ${on ? "on" : ""}`} onClick={() => toggle(path)}
        role="checkbox" aria-checked={on} aria-label={`Chọn ${name}`}>{on && <Check size={10} strokeWidth={3} />}</button>
      <div className="lo-text">
        <span className="lo-name">{name}{admin && <Lock size={10} className="lock" role="img" aria-label="cần mật khẩu admin" />}</span>
        <span className="lo-path">{tilde(path)}</span>
      </div>
      <span className="lo-size">{size === null ? "không đo được" : fileBytes(size)}</span>
    </li>
  );
}
