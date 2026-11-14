// Single entry point for data. Inside Tauri it calls Rust commands; in a browser it uses mocks.
import type { AppInfo, AppLeftover, CleanupGroup, ScanEvent, Snapshot, TrashReport } from "./types";
import { mockApps, mockLeftovers, mockScan, mockSnapshot } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const api = {
  snapshot: (): Promise<Snapshot> => (isTauri ? call("snapshot") : Promise.resolve(mockSnapshot())),

  /** Read-only scan; `onEvent` receives progress (steps, folders) while it runs. */
  scanCleanup: async (onEvent: (e: ScanEvent) => void): Promise<CleanupGroup[]> => {
    if (!isTauri) return mockScan(onEvent);
    const { Channel } = await import("@tauri-apps/api/core");
    const channel = new Channel<ScanEvent>();
    channel.onmessage = onEvent;
    return call("scan_cleanup", { onEvent: channel });
  },

  trash: (paths: string[]): Promise<TrashReport> =>
    isTauri ? call("trash_paths", { paths })
      : new Promise((r) => setTimeout(() => r({ trashed: paths.length, bytes: 0, failed: [] }), 900)),

  /** Installed apps, biggest first; `onEvent` gets each app path as it is read. */
  listApps: async (onEvent: (e: ScanEvent) => void): Promise<AppInfo[]> => {
    if (!isTauri) return mockApps(onEvent);
    const { Channel } = await import("@tauri-apps/api/core");
    const channel = new Channel<ScanEvent>();
    channel.onmessage = onEvent;
    return call("list_apps", { onEvent: channel });
  },

  appLeftovers: (bundlePath: string): Promise<AppLeftover[]> =>
    isTauri ? call("app_leftovers", { bundlePath }) : Promise.resolve(mockLeftovers(bundlePath)),

  quitApp: (bundleId: string): Promise<void> => (isTauri ? call("quit_app", { bundleId }) : Promise.resolve()),

  /** Moves the chosen paths to the Trash (Finder asks for the admin password when needed). */
  uninstallApp: (bundlePath: string, paths: string[]): Promise<TrashReport> =>
    isTauri ? call("uninstall_app", { bundlePath, paths })
      : new Promise((r) => setTimeout(() => r({ trashed: paths.length, bytes: 0, failed: [] }), 700)),

  killProcess: (pid: number, force: boolean): Promise<void> =>
    isTauri ? call("kill_process", { pid, force }) : Promise.resolve(),

  revealPath: (path: string): Promise<void> => (isTauri ? call("reveal_path", { path }) : Promise.resolve()),

  /** Menu bar popup → open the main window. */
  showMain: (): Promise<void> => (isTauri ? call("show_main") : Promise.resolve()),

  quit: (): Promise<void> => (isTauri ? call("quit_self") : Promise.resolve()),
};

export type * from "./types";
