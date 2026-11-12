import XCTest
@testable import VitalsCore

final class CPUMathTests: XCTestCase {
    func testUsageFromTickDeltas() {
        let old = [CPUTicks(user: 100, system: 50, idle: 850, nice: 0)]
        let new = [CPUTicks(user: 130, system: 60, idle: 910, nice: 0)]
        // delta: user 30, system 10, idle 60 → total 100
        let snapshot = CPUSampler.usage(from: old, to: new)
        XCTAssertEqual(snapshot.user, 0.30, accuracy: 0.0001)
        XCTAssertEqual(snapshot.system, 0.10, accuracy: 0.0001)
        XCTAssertEqual(snapshot.perCore, [0.40])
        XCTAssertEqual(snapshot.total, 0.40, accuracy: 0.0001)
    }

    func testIdleMachineIsZero() {
        let ticks = [CPUTicks(user: 5, system: 5, idle: 90, nice: 0)]
        XCTAssertEqual(CPUSampler.usage(from: ticks, to: ticks).total, 0)
    }

    func testCounterWrap() {
        XCTAssertEqual(CPUSampler.delta(UInt64(UInt32.max) - 4, 5), 10)
    }

    func testProcessCPUFraction() {
        XCTAssertEqual(ProcessSampler.cpuFraction(previousCPU: 1_000, currentCPU: 501_000, elapsedNanos: 1_000_000), 0.5)
        XCTAssertEqual(ProcessSampler.cpuFraction(previousCPU: nil, currentCPU: 10, elapsedNanos: 10), 0)
    }
}

final class RingBufferTests: XCTestCase {
    func testKeepsNewestInOrder() {
        var buffer = RingBuffer<Int>(capacity: 3)
        (1...5).forEach { buffer.append($0) }
        XCTAssertEqual(buffer.elements, [3, 4, 5])
        XCTAssertEqual(buffer.last, 5)
        XCTAssertEqual(buffer.count, 3)
    }
}

final class FormatTests: XCTestCase {
    func testBytes() {
        XCTAssertEqual(Format.bytes(512), "512 B")
        XCTAssertEqual(Format.bytes(1536), "1.5 KB")
        XCTAssertEqual(Format.bytes(200 * 1024 * 1024), "200 MB")
    }

    func testPercent() {
        XCTAssertEqual(Format.percent(0.423), "42%")
    }
}

final class PathGuardTests: XCTestCase {
    let home = URL(fileURLWithPath: "/Users/tester")

    func testAllowsChildrenOfCaches() {
        XCTAssertTrue(PathGuard.isAllowed(home.appendingPathComponent("Library/Caches/com.foo"), home: home))
        XCTAssertTrue(PathGuard.isAllowed(home.appendingPathComponent("code/app/node_modules"), home: home))
    }

    func testBlocksProtectedRootsThemselves() {
        for path in ["Library", "Library/Caches", "Documents", "Desktop", "Downloads"] {
            XCTAssertFalse(PathGuard.isAllowed(home.appendingPathComponent(path), home: home), path)
        }
        XCTAssertFalse(PathGuard.isAllowed(home, home: home))
    }

    func testBlocksOutsideHome() {
        for path in ["/System/Library", "/Library/Caches", "/usr/local", "/Applications/Safari.app", "/Users/other/x"] {
            XCTAssertFalse(PathGuard.isAllowed(URL(fileURLWithPath: path), home: home), path)
        }
    }

    func testBlocksSecretsAndTraversal() {
        XCTAssertFalse(PathGuard.isAllowed(home.appendingPathComponent(".ssh/id_ed25519"), home: home))
        XCTAssertFalse(PathGuard.isAllowed(home.appendingPathComponent("Library/Keychains/login.keychain-db"), home: home))
        XCTAssertFalse(PathGuard.isAllowed(home.appendingPathComponent("Library/Caches/../../.ssh"), home: home))
    }

    func testTrasherRejectsWithoutTouchingDisk() {
        let report = Trasher(home: home).trash([(url: URL(fileURLWithPath: "/System/Library"), size: 1)])
        XCTAssertEqual(report.rejected.count, 1)
        XCTAssertTrue(report.trashed.isEmpty)
    }
}

final class ScannerTests: XCTestCase {
    func testScanListsChildrenAndHonoursExclusions() throws {
        let home = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let caches = home.appendingPathComponent("Library/Caches")
        try FileManager.default.createDirectory(at: caches.appendingPathComponent("com.app"), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: caches.appendingPathComponent("Homebrew"), withIntermediateDirectories: true)
        try Data(repeating: 1, count: 10_000).write(to: caches.appendingPathComponent("com.app/blob"))
        try Data(repeating: 1, count: 10_000).write(to: caches.appendingPathComponent("Homebrew/pkg"))
        defer { try? FileManager.default.removeItem(at: home) }  // test temp dir only

        let target = CleanupTarget.defaults.first { $0.id == "user-caches" }!
        let results = CleanupScanner(home: home).scan([target])

        XCTAssertEqual(results.count, 1)
        XCTAssertEqual(results[0].items.map(\.name), ["com.app"])
        XCTAssertGreaterThan(results[0].totalSize, 0)
    }
}
