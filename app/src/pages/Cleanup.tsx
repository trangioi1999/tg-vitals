import { Archive, Box, Check, ChevronRight, Code2, FileText, Film, Hammer, Package, RefreshCw, Smartphone, Sparkles, Terminal, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { api, isTauri, type CleanupGroup, type ScanEvent, type TrashReport } from "../api";
import { ACCENT, IconTile } from "../components/ui";
import { ago, fileBytes, fileBytesParts } from "../format";

const GROUP_STYLE: Record<string, { icon: LucideIcon; color: string }> = {
  "xcode-derived": { icon: Hammer, color: ACCENT.cpu },
  "project-artifacts": { icon: Code2, color: ACCENT.memory },
  docker: { icon: Box, color: ACCENT.network },
  "user-caches": { icon: Sparkles, color: ACCENT.purple },
  npm: { icon: Package, color: ACCENT.gpu },
  homebrew: { icon: Package, color: ACCENT.disk },
  "user-logs": { icon: FileText, color: ACCENT.gray },
  "xcode-device-support": { icon: Smartphone, color: ACCENT.cpu },
  "xcode-archives": { icon: Archive, color: ACCENT.cpu },
  pnpm: { icon: Package, color: ACCENT.gpu },
  pip: { icon: Package, color: ACCENT.network },
  cocoapods: { icon: Package, color: ACCENT.gpu },
  gradle: { icon: Package, color: ACCENT.memory },
  "large-files": { icon: Film, color: ACCENT.purple },
};

/** One line of the scan terminal. */
type LogLine = { kind: "step" | "path" | "found" | "done" | "empty"; text: string; bytes?: number };

/** /Users/name/Library/Caches → ~/Library/Caches */
const tilde = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

export function Cleanup() {
  const [groups, setGroups] = useState<CleanupGroup[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<TrashReport | null>(null);
  const [confirming, setConfirming] = useState(false);
  // Scan progress: terminal log while scanning, folded into one summary line afterwards.
  const [scanning, setScanning] = useState(false);
  const [log, setLog] = useState<LogLine[]>([]);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const [found, setFound] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const [logOpen, setLogOpen] = useState(false);
  // What is being moved to the Trash right now (shown over the dimmed list).
  const [cleaning, setCleaning] = useState<{ count: number; bytes: number } | null>(null);

  const onScanEvent = (e: ScanEvent) => {
    if (e.kind === "step") {
      setProgress({ done: e.index, total: e.total });
      setLog((l) => [...l, { kind: "step", text: e.title }]);
    } else if (e.kind === "path") {
      setLog((l) => [...l, { kind: "path", text: tilde(e.path) }]);
    } else if (e.kind === "found") {
      setLog((l) => [...l, { kind: "found", text: tilde(e.path), bytes: e.bytes }]);
    } else {
      setProgress((p) => ({ ...p, done: p.done + 1 }));
      setFound((f) => f + e.bytes);
      setLog((l) => [...l, e.bytes > 0
        ? { kind: "done", text: `${e.title} · ${e.items} mục`, bytes: e.bytes }
        : { kind: "empty", text: `${e.title} · không có gì` }]);
    }
  };

  const scan = async () => {
    setBusy(true);
    setScanning(true);
    setReport(null);
    setGroups(null);
    setLog([]);
    setFound(0);
    setProgress({ done: 0, total: 0 });
    setLogOpen(false);
    const started = performance.now();
    try {
      const result = (await api.scanCleanup(onScanEvent)).sort((a, b) => total(b) - total(a));
      setGroups(result);
      setSelected(new Set(result.filter((g) => g.safe).flatMap((g) => g.items.map((i) => i.path))));
    } finally {
      setElapsed(performance.now() - started);
      setScanning(false);
      setBusy(false);
    }
  };
  const percent = progress.total ? Math.round((progress.done / progress.total) * 100) : 0;

  const all = useMemo(() => groups?.flatMap((g) => g.items) ?? [], [groups]);
  const selectedBytes = all.filter((i) => selected.has(i.path)).reduce((a, i) => a + i.size, 0);
  const totalBytes = all.reduce((a, i) => a + i.size, 0);
  const [bigValue, bigUnit] = fileBytesParts(selectedBytes);

  const toggleGroup = (g: CleanupGroup) => {
    const paths = g.items.map((i) => i.path);
    const allOn = paths.every((p) => selected.has(p));
    const next = new Set(selected);
    paths.forEach((p) => (allOn ? next.delete(p) : next.add(p)));
    setSelected(next);
    setReport(null);
  };

  const clean = async () => {
    const paths = [...selected];
    setConfirming(false);
    setBusy(true);
    setCleaning({ count: paths.length, bytes: selectedBytes });
    try {
      const r = await api.trash(paths);
      setReport({ ...r, bytes: isTauri ? r.bytes : selectedBytes });
      // Only what went to the Trash leaves the list; the rest stays cleanable without a new scan.
      const failed = new Set(r.failed.map((f) => f.path));
      const gone = new Set(paths.filter((p) => !failed.has(p)));
      setGroups((list) => list?.map((g) => ({ ...g, items: g.items.filter((i) => !gone.has(i.path)) }))
        .filter((g) => g.items.length > 0) ?? null);
      setSelected((s) => new Set([...s].filter((p) => !gone.has(p))));
    } finally {
      setCleaning(null);
      setBusy(false);
    }
  };

  return (
    <div className="cleanup">
      <section className="hero">
        <div className={`hero-icon ${scanning ? "scanning" : ""}`}><Trash2 size={30} /></div>
        <div className="hero-text">
          {cleaning ? (
            <>
              <div className="big-label">Đang dọn…</div>
              <div className="big-row"><span className="big-value">{fileBytesParts(cleaning.bytes)[0]}</span><span className="big-unit">{fileBytesParts(cleaning.bytes)[1]}</span></div>
              <div className="scan-track"><div style={{ width: "100%" }} /></div>
            </>
          ) : report ? (
            <>
              <div className="big-label">Đã chuyển vào Thùng rác</div>
              <div className="big-row"><span className="big-value">{fileBytesParts(report.bytes)[0]}</span><span className="big-unit">{fileBytesParts(report.bytes)[1]}</span></div>
              <div className="hero-sub">
                {report.trashed} mục · vẫn khôi phục được từ Thùng rác cho tới khi bạn đổ nó
                {report.failed.length > 0 && ` · ${report.failed.length} mục không dọn được`}
              </div>
            </>
          ) : (
            scanning ? (
            <>
              <div className="big-label">Đang quét <span className="live-status scan-live"><i />{percent}%</span></div>
              <div className="big-row"><span className="big-value">{fileBytesParts(found)[0]}</span><span className="big-unit">{fileBytesParts(found)[1]} tìm thấy</span></div>
              <div className="scan-track"><div style={{ width: `${percent}%` }} /></div>
            </>
            ) : (
            <>
              <div className="big-label">{groups ? "Có thể dọn" : "Chưa quét"}</div>
              <div className="big-row"><span className="big-value">{!groups ? "—" : bigValue}</span><span className="big-unit">{!groups ? "" : bigUnit}</span></div>
              <div className="hero-sub">
                {groups
                  ? `Đã chọn ${fileBytes(selectedBytes)} trong ${fileBytes(totalBytes)} tìm thấy · mọi thứ đều vào Thùng rác, không xóa vĩnh viễn`
                  : "Bấm Quét để tìm cache, log, build tạm và file lớn · chỉ đọc, chưa xóa gì"}
              </div>
            </>
            )
          )}
        </div>
        <div className="hero-actions">
          <button className="btn" onClick={scan} disabled={busy}>{groups ? "Quét lại" : "Quét"}</button>
          {groups && (
            confirming ? (
              <button className="btn primary" onClick={clean}>Xác nhận dọn {fileBytes(selectedBytes)}</button>
            ) : (
              <button className="btn primary" disabled={!selectedBytes || busy} onClick={() => setConfirming(true)}>Dọn dẹp</button>
            )
          )}
        </div>
      </section>

      {(scanning || logOpen) && log.length > 0 && <ScanTerminal log={log} live={scanning} />}
      {!scanning && groups && log.length > 0 && (
        <button className="scan-summary" onClick={() => setLogOpen((o) => !o)}>
          <span className="ok">✓</span>
          Quét {progress.total} bước · {fileBytes(found)} · {(elapsed / 1000).toFixed(1)}s
          <span className="scan-summary-toggle"><Terminal size={13} />{logOpen ? "Ẩn log" : "Xem log"}</span>
        </button>
      )}

      {groups && totalBytes > 0 && (
        <div className="stack-bar">
          {groups.map((g) => (
            <div key={g.id} title={g.title} style={{ flex: total(g), background: GROUP_STYLE[g.id]?.color ?? ACCENT.gray }} />
          ))}
        </div>
      )}

      {report && report.failed.length > 0 && (
        <ul className="failures">
          {report.failed.map((f) => <li key={f.path}><span className="item-path">{tilde(f.path)}</span><span>{f.reason}</span></li>)}
        </ul>
      )}

      {groups && groups.length === 0 && <div className="empty">Không tìm thấy gì để dọn 🎉</div>}

      <div className={`groups ${cleaning ? "is-busy" : ""}`}>
        {cleaning && (
          <div className="groups-busy" role="status">
            <div className="busy-card">
              <RefreshCw size={22} className="spin" />
              <div className="busy-title">Đang chuyển {cleaning.count} mục vào Thùng rác</div>
              <div className="busy-sub">{fileBytes(cleaning.bytes)} · khôi phục được từ Thùng rác</div>
            </div>
          </div>
        )}
        {groups?.map((g) => {
          const style = GROUP_STYLE[g.id] ?? { icon: Sparkles, color: ACCENT.gray };
          const paths = g.items.map((i) => i.path);
          const on = paths.filter((p) => selected.has(p)).length;
          const isOpen = open.has(g.id);
          return (
            <section key={g.id} className="group">
              <div className="group-row">
                <button className={`check ${on === paths.length ? "on" : on ? "some" : ""}`} onClick={() => toggleGroup(g)}>
                  {on > 0 && <Check size={12} strokeWidth={3} />}
                </button>
                <IconTile icon={style.icon} color={style.color} size={32} />
                <div className="group-text" onClick={() => setOpen((o) => toggleIn(o, g.id))}>
                  <div className="group-title">{g.title}{!g.safe && <span className="tag">xem trước khi dọn</span>}</div>
                  <div className="group-note">{g.note}</div>
                </div>
                <div className="group-size">{fileBytes(total(g))}</div>
                <button className="icon-btn" onClick={() => setOpen((o) => toggleIn(o, g.id))}>
                  <ChevronRight size={16} style={{ transform: isOpen ? "rotate(90deg)" : undefined, transition: "transform .15s" }} />
                </button>
              </div>
              {isOpen && (
                <ul className="items">
                  {g.items.map((i) => (
                    <li key={i.path}>
                      <button className={`check small ${selected.has(i.path) ? "on" : ""}`}
                        onClick={() => { setSelected((s) => toggleIn(s, i.path)); setReport(null); }}>
                        {selected.has(i.path) && <Check size={10} strokeWidth={3} />}
                      </button>
                      <span className="item-name">{i.name}</span>
                      <span className="item-path">{g.id === "project-artifacts" && i.modified ? `sửa lần cuối ${ago(i.modified)}` : tilde(i.path)}</span>
                      <span className="item-size">{fileBytes(i.size)}</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          );
        })}
      </div>
    </div>
  );
}

/** macOS-style terminal window streaming the scan log; follows the newest line. */
function ScanTerminal({ log, live }: { log: LogLine[]; live: boolean }) {
  const body = useRef<HTMLDivElement>(null);
  useEffect(() => {
    body.current?.scrollTo({ top: body.current.scrollHeight });
  }, [log.length]);
  return (
    <div className="term">
      <div className="term-head"><b /><b /><b /><span>tg-vitals — scan</span></div>
      <div className="term-body" ref={body}>
        {log.map((l, i) => (
          <div key={i} className={`ln ln-${l.kind}`}>
            {l.kind === "step" && <><span className="p">❯</span> {l.text}</>}
            {l.kind === "path" && <>  quét {l.text}</>}
            {l.kind === "found" && <><span className="plus">  +</span> {l.text}<span className="sz">{fileBytes(l.bytes ?? 0)}</span></>}
            {l.kind === "done" && <><span className="ok">  ✓</span> {l.text}<span className="sz">{fileBytes(l.bytes ?? 0)}</span></>}
            {l.kind === "empty" && <>  – {l.text}</>}
          </div>
        ))}
        <div className="ln">{!live && <span className="ok">✓ xong </span>}<span className={`cursor ${live ? "" : "idle"}`} /></div>
      </div>
    </div>
  );
}

const total = (g: CleanupGroup) => g.items.reduce((a, i) => a + i.size, 0);

function toggleIn<T>(set: Set<T>, value: T): Set<T> {
  const next = new Set(set);
  next.has(value) ? next.delete(value) : next.add(value);
  return next;
}
