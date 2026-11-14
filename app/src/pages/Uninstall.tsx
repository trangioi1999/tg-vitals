import { Check, Lock, Power, Search, Terminal, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { api, isTauri, type AppInfo, type AppLeftover, type TrashReport } from "../api";
import { AppIcon } from "../components/ui";
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
  // Last uninstall result: kept here so it survives the list reloading after every attempt.
  const [result, setResult] = useState<{ name: string; report: TrashReport } | null>(null);

  const load = async () => {
    setApps(null);
    setReading("");
    try {
      const list = await api.listApps((e) => e.kind === "path" && setReading(e.path));
      setApps(list);
      setSelected((s) => (s && list.some((a) => a.path === s) ? s : null));
    } finally {
      setReading(null);
    }
  };
  useEffect(() => { load(); }, []);

  const rows = useMemo(() => {
    const q = search.trim().toLowerCase();
    const list = (apps ?? []).filter((a) => !q || a.name.toLowerCase().includes(q) || a.bundleId.toLowerCase().includes(q));
    // "Lâu không dùng": never-used (null) first, then oldest.
    return sort === "size" ? list : [...list].sort((a, b) => (a.lastUsed ?? 0) - (b.lastUsed ?? 0));
  }, [apps, search, sort]);
  const current = apps?.find((a) => a.path === selected) ?? null;
  const total = (apps ?? []).reduce((s, a) => s + a.size, 0);

  if (!apps) {
    return (
      <div className="uninstall-loading">
        <section className="hero">
          <div className="hero-icon scanning"><Terminal size={28} /></div>
          <div className="hero-text">
            <div className="big-label">Đang đọc app…</div>
            <div className="hero-sub mono">{reading ? tilde(reading) : "/Applications"}</div>
            <div className="scan-track"><div style={{ width: "100%" }} /></div>
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="proc-layout">
      <div className="table-wrap">
        <div className="uninstall-bar">
          <label className="search">
            <Search size={14} />
            <input placeholder="Tìm app" value={search} onChange={(e) => setSearch(e.target.value)} />
          </label>
          <div className="segmented">
            <button className={sort === "size" ? "on" : ""} onClick={() => setSort("size")}>Dung lượng</button>
            <button className={sort === "unused" ? "on" : ""} onClick={() => setSort("unused")}>Lâu không dùng</button>
          </div>
          <span className="uninstall-count">{apps.length} app · {fileBytes(total)}</span>
        </div>
        {result && (
          <div className={`uninstall-result banner ${result.report.failed.length ? "bad" : ""}`}>
            <button className="icon-btn close" onClick={() => setResult(null)} title="Đóng"><X size={13} /></button>
            Gỡ {result.name}: {result.report.trashed} mục vào Thùng rác · {fileBytes(result.report.bytes)}
            {result.report.failed.length > 0 && " · chọn lại app để thử lại"}
            {result.report.failed.map((f) => <div key={f.path} className="fail"><span>{tilde(f.path)}</span>{f.reason}</div>)}
          </div>
        )}
        <table className="proc-table">
          <thead>
            <tr><th>Tên</th><th className="right">Dung lượng</th><th className="right">Dùng lần cuối</th></tr>
          </thead>
          <tbody>
            {rows.map((a) => (
              <tr key={a.path} className={a.path === selected ? "selected" : ""} onClick={() => setSelected(a.path)}>
                <td>
                  <span className="name-cell">
                    <AppIcon name={a.name} icon={iconOf(a.path)} size={22} />
                    <span className="app-name">{a.name}<small>{a.version}</small></span>
                    {a.running && <span className="pill-running">đang chạy</span>}
                    {!a.ownedByUser && <Lock size={11} className="lock" aria-label="cần mật khẩu admin" />}
                  </span>
                </td>
                <td className="right">{fileBytes(a.size)}</td>
                <td className="right muted-num">{a.lastUsed ? ago(a.lastUsed) : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {current
        ? <AppPanel key={current.path} app={current}
            onDone={(report) => { if (report) setResult({ name: current.name, report }); load(); }} />
        : <aside className="inspector uninstall-panel empty-panel">Chọn một app để xem nó chiếm bao nhiêu và gỡ cùng dữ liệu nó để lại.</aside>}
    </div>
  );
}

function AppPanel({ app, onDone }: { app: AppInfo; onDone: (report?: TrashReport) => void }) {
  const [leftovers, setLeftovers] = useState<AppLeftover[] | null>(null);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api.appLeftovers(app.path).then((l) => {
      setLeftovers(l);
      // Everything measurable is ticked; unreadable items stay unticked.
      setChecked(new Set([app.path, ...l.filter((x) => x.size !== null).map((x) => x.path)]));
    });
  }, [app.path]);

  const sizeOf = (p: string) => (p === app.path ? app.size : leftovers?.find((l) => l.path === p)?.size ?? 0);
  const freed = [...checked].reduce((s, p) => s + sizeOf(p), 0);
  const [freedValue, freedUnit] = fileBytesParts(freed);
  const needsAdmin = (!app.ownedByUser && checked.has(app.path)) || (leftovers ?? []).some((l) => l.needsAdmin && checked.has(l.path));
  const toggle = (p: string) => setChecked((s) => { const n = new Set(s); n.has(p) ? n.delete(p) : n.add(p); return n; });

  const uninstall = async () => {
    setConfirming(false);
    setBusy(true);
    try {
      // Always reload afterwards: even a partial result changes what is installed.
      onDone(await api.uninstallApp(app.path, [...checked]));
    } catch (e) {
      onDone({ trashed: 0, bytes: 0, failed: [{ path: app.path, reason: String(e) }] });
    } finally {
      setBusy(false);
    }
  };

  return (
    <aside className="inspector uninstall-panel">
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
        {leftovers?.length === 0 && <li className="muted">Không có dữ liệu sót lại</li>}
        {leftovers?.map((l) => (
          <Row key={l.path} path={l.path} name={l.name} size={l.size} admin={l.needsAdmin}
            on={checked.has(l.path)} toggle={toggle} />
        ))}
      </ul>

      {app.running ? (
        <button className="btn wide" onClick={() => api.quitApp(app.bundleId).then(() => setTimeout(() => onDone(), 1500))}>
          <Power size={14} /> Thoát {app.name} trước khi gỡ
        </button>
      ) : confirming ? (
        <div className="confirm-row">
          <button className="btn" onClick={() => setConfirming(false)}>Huỷ</button>
          <button className="btn primary danger-fill" onClick={uninstall}>Xác nhận gỡ · {fileBytes(freed)}</button>
        </div>
      ) : (
        <button className="btn primary wide" disabled={busy || checked.size === 0 || leftovers === null} onClick={() => setConfirming(true)}>
          {busy ? "Đang gỡ…" : `Gỡ ${app.name}`}
        </button>
      )}
      <div className="care-note">
        Mọi thứ vào Thùng rác, khôi phục được{needsAdmin ? " · macOS sẽ hỏi mật khẩu admin cho mục có ổ khoá" : ""}
      </div>
    </aside>
  );
}

function Row({ path, name, size, admin, on, toggle }: {
  path: string; name: string; size: number | null; admin: boolean; on: boolean; toggle: (p: string) => void;
}) {
  return (
    <li title={path}>
      <button className={`check small ${on ? "on" : ""}`} onClick={() => toggle(path)}>{on && <Check size={10} strokeWidth={3} />}</button>
      <div className="lo-text">
        <span className="lo-name">{name}{admin && <Lock size={10} className="lock" />}</span>
        <span className="lo-path">{tilde(path)}</span>
      </div>
      <span className="lo-size">{size === null ? "không đo được" : fileBytes(size)}</span>
    </li>
  );
}
