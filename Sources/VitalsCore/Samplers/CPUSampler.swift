import Darwin

/// Raw cumulative tick counters for one core.
public struct CPUTicks: Sendable, Equatable {
    public var user: UInt64
    public var system: UInt64
    public var idle: UInt64
    public var nice: UInt64

    public init(user: UInt64, system: UInt64, idle: UInt64, nice: UInt64) {
        self.user = user
        self.system = system
        self.idle = idle
        self.nice = nice
    }
}

/// Reads per-core CPU load via `host_processor_info`. Needs two samples to produce usage.
public final class CPUSampler {
    private var previous: [CPUTicks]?

    public init() {}

    /// Returns nil on the first call (no previous sample yet) or if the kernel call fails.
    public func sample() -> CPUSnapshot? {
        guard let current = Self.readTicks() else { return nil }
        defer { previous = current }
        guard let previous, previous.count == current.count else { return nil }
        return Self.usage(from: previous, to: current)
    }

    /// Pure delta math, kept separate so it can be unit-tested.
    public static func usage(from old: [CPUTicks], to new: [CPUTicks]) -> CPUSnapshot {
        var userSum = 0.0, systemSum = 0.0, totalSum = 0.0
        var perCore: [Double] = []
        perCore.reserveCapacity(new.count)

        for (o, n) in zip(old, new) {
            let user = Double(delta(o.user, n.user) + delta(o.nice, n.nice))
            let system = Double(delta(o.system, n.system))
            let idle = Double(delta(o.idle, n.idle))
            let total = user + system + idle
            perCore.append(total > 0 ? (user + system) / total : 0)
            userSum += user
            systemSum += system
            totalSum += total
        }

        guard totalSum > 0 else { return CPUSnapshot(user: 0, system: 0, perCore: perCore) }
        return CPUSnapshot(user: userSum / totalSum, system: systemSum / totalSum, perCore: perCore)
    }

    /// Counters are 32-bit in the kernel and may wrap; treat a decrease as a wrap.
    static func delta(_ old: UInt64, _ new: UInt64) -> UInt64 {
        new >= old ? new - old : (UInt64(UInt32.max) - old) + new + 1
    }

    static func readTicks() -> [CPUTicks]? {
        var cpuCount: natural_t = 0
        var info: processor_info_array_t?
        var infoCount: mach_msg_type_number_t = 0

        let result = host_processor_info(
            mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &cpuCount, &info, &infoCount
        )
        guard result == KERN_SUCCESS, let info else { return nil }
        defer {
            vm_deallocate(
                mach_task_self_,
                vm_address_t(bitPattern: info),
                vm_size_t(Int(infoCount) * MemoryLayout<integer_t>.stride)
            )
        }

        let stride = Int(CPU_STATE_MAX)
        func tick(_ core: Int, _ state: Int32) -> UInt64 {
            UInt64(UInt32(bitPattern: info[core * stride + Int(state)]))
        }

        return (0..<Int(cpuCount)).map { core in
            CPUTicks(
                user: tick(core, CPU_STATE_USER),
                system: tick(core, CPU_STATE_SYSTEM),
                idle: tick(core, CPU_STATE_IDLE),
                nice: tick(core, CPU_STATE_NICE)
            )
        }
    }
}
