// Realistic fake data so the UI can be developed and previewed in a plain browser.
import type { CleanupGroup, ProcessRow, Snapshot } from "./types";

const GB = 1024 ** 3;
const MB = 1024 ** 2;

let t = 0;
const wave = (base: number, amp: number, speed: number, phase = 0) =>
  Math.max(0, base + amp * Math.sin(t * speed + phase) + (Math.random() - 0.5) * amp * 0.6);

const PROCS: Omit<ProcessRow, "cpu" | "memory">[] = [
  ["tg-studio", "app", 48], ["Google Chrome", "app", 61], ["node", "mine", 11],
  ["WebStorm", "app", 72], ["Slack", "app", 33], ["WindowServer", "system", 24],
  ["kernel_task", "system", 520], ["Docker Desktop", "app", 28], ["python3.12", "mine", 4],
  ["Spotify", "app", 39], ["mds_stores", "system", 7], ["Finder", "app", 9],
  ["Telegram", "app", 22], ["coreaudiod", "system", 9], ["zsh", "mine", 1],
  ["Notion", "app", 31], ["bluetoothd", "system", 6], ["git", "mine", 1],
  ["launchd", "system", 4], ["Postman", "app", 27], ["cfprefsd", "system", 4],
  ["Preview", "app", 5], ["trustd", "system", 3], ["Messages", "app", 12],
].map(([name, kind, threads], i) => ({
  pid: [412, 437, 2210, 618, 702, 158, 0, 891, 3302, 1104, 301, 512, 1401, 211, 3410,
    1520, 199, 3420, 1, 1633, 297, 1740, 350, 1801][i],
  name: name as string,
  kind: kind as ProcessRow["kind"],
  threads: threads as number,
  user: kind === "system" ? "root" : "trangioi",
  icon: null,
  cpuTime: 400 + i * 913,
  diskRead: (20 + i * 37) * MB,
  diskWritten: (5 + i * 11) * MB,
  startedAt: Date.now() / 1000 - 3600 * (2 + i),
  path: kind === "app" ? `/Applications/${name}.app/Contents/MacOS/${name}` : `/usr/bin/${name}`,
}));

const BASE: [number, number][] = [
  [24, 2.1], [18, 1.9], [21, 0.64], [12, 2.4], [6, 0.42], [8, 0.51], [6.4, 0.04], [5.5, 1.6],
  [9.4, 0.29], [3, 0.39], [3.1, 0.1], [1.1, 0.26], [1.6, 0.31], [1.2, 0.02], [0.2, 0.006],
  [0.9, 0.48], [0.4, 0.018], [0.3, 0.012], [0.2, 0.03], [0.6, 0.55], [0.2, 0.008], [0, 0.09],
  [0.1, 0.014], [0.3, 0.2],
];

export function mockSnapshot(): Snapshot {
  t += 1;
  const perCore = Array.from({ length: 10 }, (_, i) =>
    Math.min(1, wave(i < 4 ? 0.45 : 0.22, i < 4 ? 0.25 : 0.15, 0.3 + i * 0.07, i)));
  const user = wave(0.32, 0.12, 0.25);
  const system = wave(0.14, 0.05, 0.4, 1);
  const inPerSec = wave(1.6 * MB, 1.4 * MB, 0.5);

  return {
    time: Date.now(),
    machine: { model: "MacBook Pro", chip: "Apple M3 Pro", uptime: 6 * 86400 + 4 * 3600 },
    cpu: { total: Math.min(1, user + system), user, system, perCore, pCores: 4, temperature: wave(47, 4, 0.1) },
    memory: {
      total: 18 * GB, app: 7.4 * GB + wave(0, 0.2 * GB, 0.1), wired: 2.3 * GB,
      compressed: 2.9 * GB, cached: 3.1 * GB, swapUsed: 1.2 * GB, pressure: "normal",
    },
    disk: { total: 494 * GB, free: 162 * GB, readPerSec: wave(2 * MB, 2 * MB, 0.7), writePerSec: wave(0.8 * MB, 0.8 * MB, 0.9, 2) },
    network: { inPerSec, outPerSec: wave(90_000, 80_000, 0.6, 1), totalIn: 48.2 * GB, totalOut: 6.1 * GB },
    battery: { percent: 0.82, charging: false, onAdapter: true, health: 0.94, cycles: 213, powerDraw: wave(14, 4, 0.2) },
    processes: PROCS.map((p, i) => ({
      ...p,
      cpu: wave(BASE[i][0], BASE[i][0] * 0.4, 0.3 + i * 0.1, i),
      memory: BASE[i][1] * GB,
    })),
  };
}

export function mockCleanup(): CleanupGroup[] {
  const item = (name: string, gb: number) => ({ path: `~/${name}`, name: name.split("/").pop()!, size: gb * GB, modified: null });
  return [
    { id: "xcode-derived", title: "Xcode DerivedData", category: "xcode", safe: true,
      note: "Kết quả build tạm, lần build sau sẽ lâu hơn",
      items: [item("TgStudio-abxq", 6.2), item("Pulse-fkew", 2.1), item("Sample-ccnd", 0.8)] },
    { id: "node-modules", title: "node_modules bỏ lâu", category: "projects", safe: false,
      note: "Project không sửa hơn 30 ngày, chạy lại npm install khi cần",
      items: [item("WebstormProjects/onboarding-project/node_modules", 1.4), item("WebstormProjects/untitled/node_modules", 0.6)] },
    { id: "docker", title: "Docker", category: "devtools", safe: false,
      note: "Image và build cache không dùng (docker system prune)",
      items: [item("Docker/build-cache", 3.8)] },
    { id: "user-caches", title: "Cache ứng dụng", category: "system", safe: true,
      note: "App tự tạo lại khi cần",
      items: [item("Library/Caches/Google", 1.1), item("Library/Caches/com.spotify.client", 0.9), item("Library/Caches/JetBrains", 0.7), item("Library/Caches/Slack", 0.3)] },
    { id: "npm", title: "npm / yarn / pnpm", category: "devtools", safe: true,
      note: "Package đã tải, cài lại sẽ tải về",
      items: [item(".npm/_cacache", 2.3), item("Library/Caches/Yarn", 0.5)] },
    { id: "homebrew", title: "Homebrew", category: "devtools", safe: true,
      note: "File cài đặt cũ", items: [item("Library/Caches/Homebrew", 1.2)] },
    { id: "user-logs", title: "Log", category: "system", safe: true,
      note: "Chỉ cần khi debug sự cố cũ", items: [item("Library/Logs/DiagnosticReports", 0.2)] },
  ];
}
