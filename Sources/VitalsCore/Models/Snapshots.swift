import Foundation

/// CPU usage at one moment. All values are fractions in 0...1.
public struct CPUSnapshot: Sendable, Equatable {
    public var user: Double
    public var system: Double
    public var perCore: [Double]

    public var total: Double { min(1, user + system) }

    public init(user: Double, system: Double, perCore: [Double]) {
        self.user = user
        self.system = system
        self.perCore = perCore
    }
}

/// Memory breakdown in bytes, matching Activity Monitor's categories.
public struct MemorySnapshot: Sendable, Equatable {
    public var total: UInt64
    public var app: UInt64
    public var wired: UInt64
    public var compressed: UInt64
    public var cached: UInt64

    public var used: UInt64 { app + wired + compressed }
    public var usedFraction: Double { total == 0 ? 0 : Double(used) / Double(total) }

    public init(total: UInt64, app: UInt64, wired: UInt64, compressed: UInt64, cached: UInt64) {
        self.total = total
        self.app = app
        self.wired = wired
        self.compressed = compressed
        self.cached = cached
    }
}

/// Network throughput in bytes per second since the previous sample.
public struct NetworkSnapshot: Sendable, Equatable {
    public var bytesInPerSecond: Double
    public var bytesOutPerSecond: Double

    public init(bytesInPerSecond: Double, bytesOutPerSecond: Double) {
        self.bytesInPerSecond = bytesInPerSecond
        self.bytesOutPerSecond = bytesOutPerSecond
    }
}

public struct ProcessInfoSnapshot: Sendable, Equatable, Identifiable {
    public var pid: Int32
    public var name: String
    /// Fraction of ONE core (can exceed 1 for multithreaded processes, like `top`).
    public var cpu: Double
    /// Resident memory in bytes.
    public var memory: UInt64
    public var threads: Int32

    public var id: Int32 { pid }

    public init(pid: Int32, name: String, cpu: Double, memory: UInt64, threads: Int32) {
        self.pid = pid
        self.name = name
        self.cpu = cpu
        self.memory = memory
        self.threads = threads
    }
}
