import { useEffect, useRef, useState } from "react";
import { api, isTauri, type Snapshot } from "./api";

export const HISTORY = 150;

export interface History {
  cpuUser: number[];
  cpuSystem: number[];
  memory: number[];
  diskRead: number[];
  diskWrite: number[];
  netIn: number[];
  netOut: number[];
  power: number[];
  perProcess: Map<number, { cpu: number[]; memory: number[] }>;
}

const empty = (): History => ({
  cpuUser: [], cpuSystem: [], memory: [], diskRead: [], diskWrite: [],
  netIn: [], netOut: [], power: [], perProcess: new Map(),
});

const push = (list: number[], value: number) => {
  list.push(value);
  if (list.length > HISTORY) list.shift();
};

/** Polls the backend every second and keeps a rolling minute of history. */
export function useVitals(paused: boolean) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const history = useRef<History>(empty());

  useEffect(() => {
    if (paused) return;
    let alive = true;

    const record = (s: Snapshot) => {
      const h = history.current;
      push(h.cpuUser, s.cpu.user);
      push(h.cpuSystem, s.cpu.system);
      push(h.memory, (s.memory.app + s.memory.wired + s.memory.compressed) / s.memory.total);
      push(h.diskRead, s.disk.readPerSec);
      push(h.diskWrite, s.disk.writePerSec);
      push(h.netIn, s.network.inPerSec);
      push(h.netOut, s.network.outPerSec);
      push(h.power, s.battery?.powerDraw ?? 0);
      for (const p of s.processes) {
        const entry = h.perProcess.get(p.pid) ?? { cpu: [], memory: [] };
        push(entry.cpu, p.cpu);
        push(entry.memory, p.memory);
        h.perProcess.set(p.pid, entry);
      }
    };

    const tick = async () => {
      const s = await api.snapshot();
      if (!alive) return;
      record(s);
      setSnapshot(s);
    };

    // Browser preview: pre-roll a minute of mock data so charts aren't empty.
    if (!isTauri && history.current.cpuUser.length === 0) {
      (async () => {
        for (let i = 0; i < HISTORY - 1; i++) record(await api.snapshot());
        tick();
      })();
    } else {
      tick();
    }
    const id = setInterval(tick, 1000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [paused]);

  return { snapshot, history: history.current };
}
