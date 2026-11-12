import Foundation
import Observation
import VitalsCore

@MainActor
@Observable
final class CareModel {
    enum Phase: Equatable {
        case idle
        case scanning
        case ready
        case cleaning
        case done(Trasher.Report)
    }

    private(set) var phase: Phase = .idle
    private(set) var results: [ScanResult] = []
    var selected: Set<URL> = []

    var selectedBytes: UInt64 {
        results.flatMap(\.items).filter { selected.contains($0.url) }.reduce(0) { $0 + $1.size }
    }

    var totalBytes: UInt64 { results.reduce(0) { $0 + $1.totalSize } }

    func scan() {
        phase = .scanning
        Task {
            // Disk walking can take seconds; keep it off the main actor.
            let found = await Task.detached(priority: .userInitiated) {
                CleanupScanner().scan()
            }.value
            results = found
            selected = Set(found.filter(\.target.selectedByDefault).flatMap(\.items).map(\.url))
            phase = .ready
        }
    }

    func toggle(_ result: ScanResult) {
        let urls = Set(result.items.map(\.url))
        if urls.isSubset(of: selected) {
            selected.subtract(urls)
        } else {
            selected.formUnion(urls)
        }
    }

    func cleanSelected() {
        let items = results.flatMap(\.items)
            .filter { selected.contains($0.url) }
            .map { (url: $0.url, size: $0.size) }
        phase = .cleaning
        Task {
            let report = await Task.detached(priority: .userInitiated) {
                Trasher().trash(items)
            }.value
            results = []
            selected = []
            phase = .done(report)
        }
    }
}
