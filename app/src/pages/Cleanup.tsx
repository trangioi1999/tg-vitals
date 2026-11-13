import { Archive, Box, Check, ChevronRight, Code2, FileText, Film, Hammer, Package, Smartphone, Sparkles, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useMemo, useState } from "react";
import { api, isTauri, type CleanupGroup, type TrashReport } from "../api";
import { ACCENT, IconTile } from "../components/ui";
import { bytes, bytesParts } from "../format";

const GROUP_STYLE: Record<string, { icon: LucideIcon; color: string }> = {
  "xcode-derived": { icon: Hammer, color: ACCENT.cpu },
  "node-modules": { icon: Code2, color: ACCENT.memory },
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

/** /Users/name/Library/Caches → ~/Library/Caches */
const tilde = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

export function Cleanup() {
  const [groups, setGroups] = useState<CleanupGroup[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<TrashReport | null>(null);
  const [confirming, setConfirming] = useState(false);

  const scan = async () => {
    setBusy(true);
    setReport(null);
    const result = (await api.scanCleanup()).sort((a, b) => total(b) - total(a));
    setGroups(result);
    setSelected(new Set(result.filter((g) => g.safe).flatMap((g) => g.items.map((i) => i.path))));
    setBusy(false);
  };

  const all = useMemo(() => groups?.flatMap((g) => g.items) ?? [], [groups]);
  const selectedBytes = all.filter((i) => selected.has(i.path)).reduce((a, i) => a + i.size, 0);
  const totalBytes = all.reduce((a, i) => a + i.size, 0);
  const [bigValue, bigUnit] = bytesParts(selectedBytes);

  const toggleGroup = (g: CleanupGroup) => {
    const paths = g.items.map((i) => i.path);
    const allOn = paths.every((p) => selected.has(p));
    const next = new Set(selected);
    paths.forEach((p) => (allOn ? next.delete(p) : next.add(p)));
    setSelected(next);
  };

  const clean = async () => {
    setConfirming(false);
    setBusy(true);
    const r = await api.trash([...selected]);
    setReport({ ...r, bytes: isTauri ? r.bytes : selectedBytes });
    setGroups(null);
    setBusy(false);
  };

  return (
    <div className="cleanup">
      <section className="hero">
        <div className="hero-icon"><Trash2 size={30} /></div>
        <div className="hero-text">
          {report ? (
            <>
              <div className="big-label">Đã chuyển vào Thùng rác</div>
              <div className="big-row"><span className="big-value">{bytesParts(report.bytes)[0]}</span><span className="big-unit">{bytesParts(report.bytes)[1]}</span></div>
              <div className="hero-sub">
                {report.trashed} mục · vẫn khôi phục được từ Thùng rác cho tới khi bạn đổ nó
                {report.failed.length > 0 && ` · ${report.failed.length} mục không dọn được`}
              </div>
            </>
          ) : (
            <>
              <div className="big-label">{busy ? "Đang quét…" : groups ? "Có thể dọn" : "Chưa quét"}</div>
              <div className="big-row"><span className="big-value">{!groups ? "—" : bigValue}</span><span className="big-unit">{!groups ? "" : bigUnit}</span></div>
              <div className="hero-sub">
                {groups
                  ? `Đã chọn ${bytes(selectedBytes)} trong ${bytes(totalBytes)} tìm thấy · mọi thứ đều vào Thùng rác, không xóa vĩnh viễn`
                  : "Bấm Quét để tìm cache, log, build tạm và file lớn · chỉ đọc, chưa xóa gì"}
              </div>
            </>
          )}
        </div>
        <div className="hero-actions">
          <button className="btn" onClick={scan} disabled={busy}>{groups ? "Quét lại" : "Quét"}</button>
          {groups && (
            confirming ? (
              <button className="btn primary" onClick={clean}>Xác nhận dọn {bytes(selectedBytes)}</button>
            ) : (
              <button className="btn primary" disabled={!selectedBytes || busy} onClick={() => setConfirming(true)}>Dọn dẹp</button>
            )
          )}
        </div>
      </section>

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

      <div className="groups">
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
                <div className="group-size">{bytes(total(g))}</div>
                <button className="icon-btn" onClick={() => setOpen((o) => toggleIn(o, g.id))}>
                  <ChevronRight size={16} style={{ transform: isOpen ? "rotate(90deg)" : undefined, transition: "transform .15s" }} />
                </button>
              </div>
              {isOpen && (
                <ul className="items">
                  {g.items.map((i) => (
                    <li key={i.path}>
                      <button className={`check small ${selected.has(i.path) ? "on" : ""}`}
                        onClick={() => setSelected((s) => toggleIn(s, i.path))}>
                        {selected.has(i.path) && <Check size={10} strokeWidth={3} />}
                      </button>
                      <span className="item-name">{i.name}</span>
                      <span className="item-path">{tilde(i.path)}</span>
                      <span className="item-size">{bytes(i.size)}</span>
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

const total = (g: CleanupGroup) => g.items.reduce((a, i) => a + i.size, 0);

function toggleIn<T>(set: Set<T>, value: T): Set<T> {
  const next = new Set(set);
  next.has(value) ? next.delete(value) : next.add(value);
  return next;
}
