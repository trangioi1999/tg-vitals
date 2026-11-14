// Realistic fake data so the UI can be developed and previewed in a plain browser.
import type { AppInfo, AppLeftover, CleanupGroup, ProcessRow, ScanEvent, Snapshot } from "./types";

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
  const artifact = (name: string, gb: number, daysAgo: number) =>
    ({ path: `~/Documents/${name}`, name, size: gb * GB, modified: Math.floor(Date.now() / 1000) - daysAgo * 86400 });
  return [
    { id: "xcode-derived", title: "Xcode DerivedData", category: "xcode", safe: true,
      note: "Kết quả build tạm, lần build sau sẽ lâu hơn",
      items: [item("TgStudio-abxq", 6.2), item("Pulse-fkew", 2.1), item("Sample-ccnd", 0.8)] },
    { id: "project-artifacts", title: "File build trong project", category: "projects", safe: false,
      note: "target, node_modules, .next, .venv… tạo lại khi build / cài lại; không đụng file Git đang theo dõi",
      items: [
        artifact("vibe-code/tg-studio/src-tauri/target", 42, 2),
        artifact("vibe-code/omni-monorepo/.angular", 1.2, 41),
        artifact("vibe-code/tg-labs-mfe/node_modules", 0.9, 96),
        artifact("WebstormProjects/onboarding-project/node_modules", 0.6, 210),
      ] },
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

/** Browser preview: replays a scan as progress events, then returns `mockCleanup()`. */
export async function mockScan(onEvent: (e: ScanEvent) => void): Promise<CleanupGroup[]> {
  const groups = mockCleanup();
  const steps = [
    ...groups.map((g) => ({
      title: g.title,
      paths: g.id === "project-artifacts" ? ["~/Documents", "~/WebstormProjects"] : g.items.slice(0, 2).map((i) => i.path),
      group: g,
    })),
    { title: "Log", paths: ["~/Library/Logs"], group: undefined },
  ];
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  for (const [index, s] of steps.entries()) {
    onEvent({ kind: "step", index, total: steps.length, title: s.title });
    for (const path of s.paths) {
      await wait(220);
      onEvent({ kind: "path", path });
    }
    if (s.group?.id === "project-artifacts") {
      for (const i of s.group.items) {
        await wait(260);
        onEvent({ kind: "found", path: i.path, bytes: i.size });
      }
    }
    await wait(260);
    const bytes = s.group?.items.reduce((a, i) => a + i.size, 0) ?? 0;
    onEvent({ kind: "stepDone", title: s.title, bytes, items: s.group?.items.length ?? 0 });
  }
  return groups;
}

const DAY = 86400;
const now = () => Math.floor(Date.now() / 1000);
const APPS: AppInfo[] = [
  { path: "/Applications/WebStorm.app", name: "WebStorm", bundleId: "com.jetbrains.WebStorm", version: "2025.3", size: 3.28e9, ownedByUser: true, lastUsed: now() - 195 * DAY, running: false, brewCask: null, teamId: "2ZEFAR8TH3" },
  { path: "/Applications/Microsoft Word.app", name: "Microsoft Word", bundleId: "com.microsoft.Word", version: "16.103", size: 2.98e9, ownedByUser: false, lastUsed: null, running: false, brewCask: null, teamId: "UBF8T346G9" },
  { path: "/Applications/Docker.app", name: "Docker", bundleId: "com.docker.docker", version: "4.48.0", size: 1.81e9, ownedByUser: true, lastUsed: now() - 207 * DAY, running: false, brewCask: null, teamId: "9BNSXJN65R" },
  { path: "/Applications/Google Chrome.app", name: "Google Chrome", bundleId: "com.google.Chrome", version: "142.0", size: 1.54e9, ownedByUser: true, lastUsed: now() - 600, running: true, brewCask: null, teamId: "EQHXZ8M8AV" },
  { path: "/Applications/DBeaver.app", name: "DBeaver", bundleId: "org.jkiss.dbeaver.core.product", version: "25.2", size: 0.52e9, ownedByUser: true, lastUsed: now() - 40 * DAY, running: false, brewCask: "dbeaver-community", teamId: "42B6MDKMW8" },
  { path: "/Applications/Spotify.app", name: "Spotify", bundleId: "com.spotify.client", version: "1.2.74", size: 0.41e9, ownedByUser: true, lastUsed: null, running: false, brewCask: null, teamId: "2FNC3A47ZF" },
];

/** Browser preview: replays reading each app, then returns the list. */
export async function mockApps(onEvent: (e: ScanEvent) => void): Promise<AppInfo[]> {
  for (const a of APPS) {
    await new Promise((r) => setTimeout(r, 180));
    onEvent({ kind: "path", path: a.path });
  }
  return APPS.map((a) => ({ ...a }));
}

export function mockLeftovers(bundlePath: string): AppLeftover[] {
  const app = APPS.find((a) => a.path === bundlePath);
  if (!app) return [];
  const lib = "/Users/me/Library";
  const id = app.bundleId;
  const list: AppLeftover[] = [
    { path: `${lib}/Application Support/${app.name}`, name: app.name, size: 0.9e9, needsAdmin: false },
    { path: `${lib}/Caches/${id}`, name: id, size: 0.31e9, needsAdmin: false },
    { path: `${lib}/Preferences/${id}.plist`, name: `${id}.plist`, size: 12e3, needsAdmin: false },
  ];
  if (!app.ownedByUser) {
    list.push({ path: `${lib}/Containers/${id}`, name: id, size: null, needsAdmin: false });
    list.push({ path: `/Library/LaunchDaemons/${id}.helper.plist`, name: `${id}.helper.plist`, size: 4e3, needsAdmin: true });
  }
  return list;
}
