import Darwin
import Foundation

/// Lists processes and their CPU / memory via libproc (no root needed for own-user processes;
/// other users' processes may return partial data).
///
/// TODO(M3): use `proc_pid_rusage` → `ri_phys_footprint` so memory matches Activity Monitor.
public final class ProcessSampler {
    /// pid → cumulative CPU time in nanoseconds at the previous sample.
    private var previousCPUTime: [Int32: UInt64] = [:]
    private var previousTime: UInt64 = 0
    private let timebase: mach_timebase_info_data_t

    public init() {
        var info = mach_timebase_info_data_t()
        mach_timebase_info(&info)
        timebase = info
    }

    public func sample() -> [ProcessInfoSnapshot] {
        let now = DispatchTime.now().uptimeNanoseconds
        let elapsed = previousTime == 0 ? 0 : now - previousTime
        var nextCPUTime: [Int32: UInt64] = [:]
        var result: [ProcessInfoSnapshot] = []

        for pid in Self.allPIDs() where pid > 0 {
            var task = proc_taskinfo()
            let size = Int32(MemoryLayout<proc_taskinfo>.size)
            guard proc_pidinfo(pid, PROC_PIDTASKINFO, 0, &task, size) == size else { continue }

            let cpuNanos = machToNanos(task.pti_total_user + task.pti_total_system)
            nextCPUTime[pid] = cpuNanos

            let cpu = Self.cpuFraction(
                previousCPU: previousCPUTime[pid], currentCPU: cpuNanos, elapsedNanos: elapsed
            )

            result.append(ProcessInfoSnapshot(
                pid: pid,
                name: Self.name(of: pid),
                cpu: cpu,
                memory: task.pti_resident_size,
                threads: task.pti_threadnum
            ))
        }

        previousCPUTime = nextCPUTime
        previousTime = now
        return result
    }

    /// CPU used between two samples, as a fraction of one core.
    public static func cpuFraction(previousCPU: UInt64?, currentCPU: UInt64, elapsedNanos: UInt64) -> Double {
        guard let previousCPU, elapsedNanos > 0, currentCPU >= previousCPU else { return 0 }
        return Double(currentCPU - previousCPU) / Double(elapsedNanos)
    }

    /// On Apple silicon `pti_total_*` is in Mach ticks, not nanoseconds.
    private func machToNanos(_ ticks: UInt64) -> UInt64 {
        guard timebase.denom != 0 else { return ticks }
        return ticks * UInt64(timebase.numer) / UInt64(timebase.denom)
    }

    static func allPIDs() -> [Int32] {
        let estimate = proc_listallpids(nil, 0)
        guard estimate > 0 else { return [] }
        var pids = [Int32](repeating: 0, count: Int(estimate) + 64)
        let count = pids.withUnsafeMutableBufferPointer { buffer in
            proc_listallpids(buffer.baseAddress, Int32(buffer.count * MemoryLayout<Int32>.size))
        }
        guard count > 0 else { return [] }
        return Array(pids.prefix(Int(count)))
    }

    static func name(of pid: Int32) -> String {
        var buffer = [CChar](repeating: 0, count: 1024)
        let length = proc_name(pid, &buffer, UInt32(buffer.count))
        if length > 0 { return String(cString: buffer) }
        return "pid \(pid)"
    }
}
