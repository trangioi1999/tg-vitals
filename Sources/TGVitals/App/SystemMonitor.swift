import Foundation
import Observation
import VitalsCore

/// Owns all samplers, ticks every second, and publishes the latest values + history to the UI.
@MainActor
@Observable
final class SystemMonitor {
    static let historyLength = 60

    private(set) var cpu: CPUSnapshot?
    private(set) var memory: MemorySnapshot?
    private(set) var network: NetworkSnapshot?
    private(set) var processes: [ProcessInfoSnapshot] = []

    private(set) var cpuHistory = RingBuffer<Double>(capacity: historyLength)
    private(set) var memoryHistory = RingBuffer<Double>(capacity: historyLength)
    private(set) var networkInHistory = RingBuffer<Double>(capacity: historyLength)
    private(set) var networkOutHistory = RingBuffer<Double>(capacity: historyLength)

    /// True while the main window is visible; process sampling only runs then.
    var isWindowVisible = false

    @ObservationIgnored private let cpuSampler = CPUSampler()
    @ObservationIgnored private let memorySampler = MemorySampler()
    @ObservationIgnored private let networkSampler = NetworkSampler()
    @ObservationIgnored private let processSampler = ProcessSampler()
    @ObservationIgnored private var loop: Task<Void, Never>?
    @ObservationIgnored private var tickCount = 0

    init() {
        start()
    }

    func start() {
        guard loop == nil else { return }
        loop = Task { [weak self] in
            while !Task.isCancelled {
                self?.tick()
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    func stop() {
        loop?.cancel()
        loop = nil
    }

    private func tick() {
        if let value = cpuSampler.sample() {
            cpu = value
            cpuHistory.append(value.total)
        }
        if let value = memorySampler.sample() {
            memory = value
            memoryHistory.append(value.usedFraction)
        }
        if let value = networkSampler.sample() {
            network = value
            networkInHistory.append(value.bytesInPerSecond)
            networkOutHistory.append(value.bytesOutPerSecond)
        }
        // Process list is the expensive part: every 2s, and only when someone is looking.
        // TODO(M1): move off the main actor if it shows up in Instruments.
        if isWindowVisible, tickCount % 2 == 0 {
            processes = processSampler.sample()
        }
        tickCount += 1
    }
}
