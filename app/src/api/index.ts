// Single entry point for data. Inside Tauri it calls Rust commands; in a browser it uses mocks.
import type { CleanupGroup, Snapshot, TrashReport } from "./types";
import { mockCleanup, mockSnapshot } from "./mock";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const api = {
  snapshot: (): Promise<Snapshot> => (isTauri ? call("snapshot") : Promise.resolve(mockSnapshot())),

  scanCleanup: (): Promise<CleanupGroup[]> =>
    isTauri ? call("scan_cleanup") : new Promise((r) => setTimeout(() => r(mockCleanup()), 600)),

  trash: (paths: string[]): Promise<TrashReport> =>
    isTauri ? call("trash_paths", { paths }) : Promise.resolve({ trashed: paths.length, bytes: 0, failed: [] }),

  killProcess: (pid: number, force: boolean): Promise<void> =>
    isTauri ? call("kill_process", { pid, force }) : Promise.resolve(),
};

export type * from "./types";
