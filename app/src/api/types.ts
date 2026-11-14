// Shapes shared with the Rust backend (src-tauri/core/src/model.rs, care/mod.rs). Keep both in sync.

export interface CpuInfo {
  total: number;      // 0..1
  user: number;       // 0..1
  system: number;     // 0..1
  perCore: number[];  // 0..1, P-cores first
  pCores: number;
  temperature: number | null; // °C
}

export interface MemoryInfo {
  total: number;      // bytes
  app: number;
  wired: number;
  compressed: number;
  cached: number;
  swapUsed: number;
  pressure: "normal" | "warning" | "critical";
}

export interface DiskInfo {
  total: number;
  free: number;
  readPerSec: number;
  writePerSec: number;
}

export interface NetworkInfo {
  inPerSec: number;
  outPerSec: number;
  totalIn: number;
  totalOut: number;
}

export interface BatteryInfo {
  percent: number;    // 0..1
  charging: boolean;
  onAdapter: boolean;
  health: number;     // 0..1
  cycles: number;
  powerDraw: number;  // watts
}

export interface ProcessRow {
  pid: number;
  name: string;
  /** Percent of one core, like Activity Monitor (can exceed 100). */
  cpu: number;
  memory: number;     // bytes
  threads: number;
  user: string;
  kind: "app" | "system" | "mine";
  /** Bundle icon as data URL; null for plain executables. */
  icon: string | null;
  cpuTime: number;    // seconds
  diskRead: number;   // bytes total
  diskWritten: number;
  startedAt: number;  // unix seconds
  /** Executable path; empty when macOS refuses to tell. */
  path: string;
}

export interface Snapshot {
  time: number;       // unix ms
  machine: { model: string; chip: string; uptime: number };
  cpu: CpuInfo;
  memory: MemoryInfo;
  disk: DiskInfo;
  network: NetworkInfo;
  battery: BatteryInfo | null;
  processes: ProcessRow[];
}

export interface CleanupItem {
  path: string;
  name: string;
  size: number;
  modified: number | null; // unix seconds
}

export interface CleanupGroup {
  id: string;
  title: string;
  category: "system" | "xcode" | "devtools" | "projects" | "files";
  note: string;
  safe: boolean;      // pre-selected
  items: CleanupItem[];
}

/** Scan progress, streamed while `scanCleanup` runs (mirrors `care::ScanEvent`). */
export type ScanEvent =
  | { kind: "step"; index: number; total: number; title: string }
  | { kind: "path"; path: string }
  | { kind: "stepDone"; title: string; bytes: number; items: number };

export interface TrashReport {
  trashed: number;
  bytes: number;
  failed: { path: string; reason: string }[];
}
