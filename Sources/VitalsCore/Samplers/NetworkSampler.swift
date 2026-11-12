import Darwin
import Foundation

/// Total network throughput across physical interfaces, via `getifaddrs`.
///
/// TODO(M3): `if_data` counters are 32-bit and wrap at 4 GB. Switch to
/// `sysctl(NET_RT_IFLIST2)` + `if_data64` for exact totals.
public final class NetworkSampler {
    private var previous: (bytesIn: UInt64, bytesOut: UInt64, time: Date)?

    public init() {}

    public func sample(now: Date = Date()) -> NetworkSnapshot? {
        guard let counters = Self.readCounters() else { return nil }
        defer { previous = (counters.bytesIn, counters.bytesOut, now) }
        guard let previous else { return nil }

        let elapsed = now.timeIntervalSince(previous.time)
        guard elapsed > 0 else { return nil }

        return NetworkSnapshot(
            bytesInPerSecond: Double(Self.safeDelta(previous.bytesIn, counters.bytesIn)) / elapsed,
            bytesOutPerSecond: Double(Self.safeDelta(previous.bytesOut, counters.bytesOut)) / elapsed
        )
    }

    /// A summed counter going backwards means one interface wrapped or disappeared;
    /// report 0 for that tick instead of a bogus spike.
    static func safeDelta(_ old: UInt64, _ new: UInt64) -> UInt64 {
        new >= old ? new - old : 0
    }

    static func readCounters() -> (bytesIn: UInt64, bytesOut: UInt64)? {
        var head: UnsafeMutablePointer<ifaddrs>?
        guard getifaddrs(&head) == 0, let first = head else { return nil }
        defer { freeifaddrs(head) }

        var bytesIn: UInt64 = 0
        var bytesOut: UInt64 = 0

        for pointer in sequence(first: first, next: { $0.pointee.ifa_next }) {
            let entry = pointer.pointee
            guard let address = entry.ifa_addr,
                  address.pointee.sa_family == UInt8(AF_LINK),
                  let rawData = entry.ifa_data
            else { continue }

            let name = String(cString: entry.ifa_name)
            // Skip loopback and virtual tunnels so traffic isn't double-counted.
            if name.hasPrefix("lo") || name.hasPrefix("utun") || name.hasPrefix("awdl")
                || name.hasPrefix("llw") || name.hasPrefix("bridge") {
                continue
            }

            let data = rawData.assumingMemoryBound(to: if_data.self).pointee
            bytesIn += UInt64(data.ifi_ibytes)
            bytesOut += UInt64(data.ifi_obytes)
        }
        return (bytesIn, bytesOut)
    }
}
