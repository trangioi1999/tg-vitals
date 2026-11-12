import Foundation

public struct ScanItem: Sendable, Identifiable, Hashable {
    public let url: URL
    public let size: UInt64
    public let lastModified: Date?
    public var id: URL { url }
    public var name: String { url.lastPathComponent }
}

public struct ScanResult: Sendable, Identifiable {
    public let target: CleanupTarget
    public let items: [ScanItem]
    public var id: String { target.id }
    public var totalSize: UInt64 { items.reduce(0) { $0 + $1.size } }
}

/// Read-only: never modifies the disk. Deleting is `Trasher`'s job.
public struct CleanupScanner {
    private let home: URL
    private let fileManager: FileManager

    public init(home: URL = FileManager.default.homeDirectoryForCurrentUser,
                fileManager: FileManager = .default) {
        self.home = home
        self.fileManager = fileManager
    }

    public func scan(_ targets: [CleanupTarget] = CleanupTarget.defaults) -> [ScanResult] {
        targets.compactMap { target in
            let items = target.relativePaths.flatMap { children(of: $0, excluding: target.excludedChildren) }
                .filter { $0.size > 0 }
                .sorted { $0.size > $1.size }
            return items.isEmpty ? nil : ScanResult(target: target, items: items)
        }
    }

    /// Finds `node_modules` folders under `roots` whose project hasn't changed for `staleDays`.
    public func staleNodeModules(roots: [URL], staleDays: Int = 30, maxDepth: Int = 4,
                                 now: Date = Date()) -> [ScanItem] {
        let cutoff = now.addingTimeInterval(-Double(staleDays) * 86_400)
        var found: [ScanItem] = []

        func walk(_ dir: URL, depth: Int) {
            guard depth <= maxDepth,
                  let entries = try? fileManager.contentsOfDirectory(
                      at: dir, includingPropertiesForKeys: [.isDirectoryKey, .isSymbolicLinkKey],
                      options: [.skipsHiddenFiles])
            else { return }

            for entry in entries {
                let values = try? entry.resourceValues(forKeys: [.isDirectoryKey, .isSymbolicLinkKey])
                guard values?.isDirectory == true, values?.isSymbolicLink != true else { continue }

                if entry.lastPathComponent == "node_modules" {
                    let projectModified = modificationDate(of: dir)
                    if let projectModified, projectModified < cutoff {
                        found.append(ScanItem(url: entry, size: DirectorySizer.size(of: entry),
                                              lastModified: projectModified))
                    }
                    // Never descend into node_modules.
                } else {
                    walk(entry, depth: depth + 1)
                }
            }
        }

        roots.forEach { walk($0, depth: 0) }
        return found.sorted { $0.size > $1.size }
    }

    private func children(of relativePath: String, excluding: Set<String>) -> [ScanItem] {
        let root = home.appendingPathComponent(relativePath, isDirectory: true)
        guard let entries = try? fileManager.contentsOfDirectory(
            at: root, includingPropertiesForKeys: [.contentModificationDateKey], options: []
        ) else { return [] }

        return entries
            .filter { !excluding.contains($0.lastPathComponent) && $0.lastPathComponent != ".DS_Store" }
            .map { ScanItem(url: $0, size: DirectorySizer.size(of: $0), lastModified: modificationDate(of: $0)) }
    }

    private func modificationDate(of url: URL) -> Date? {
        try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
    }
}

public enum DirectorySizer {
    private static let keys: Set<URLResourceKey> = [
        .totalFileAllocatedSizeKey, .fileAllocatedSizeKey, .isRegularFileKey,
    ]

    /// On-disk (allocated) size, matching what Finder shows. Follows no symlinks.
    public static func size(of url: URL) -> UInt64 {
        if let values = try? url.resourceValues(forKeys: keys), values.isRegularFile == true {
            return UInt64(values.totalFileAllocatedSize ?? values.fileAllocatedSize ?? 0)
        }

        guard let enumerator = FileManager.default.enumerator(
            at: url, includingPropertiesForKeys: Array(keys), options: [], errorHandler: { _, _ in true }
        ) else { return 0 }

        var total: UInt64 = 0
        for case let file as URL in enumerator {
            guard let values = try? file.resourceValues(forKeys: keys), values.isRegularFile == true else { continue }
            total += UInt64(values.totalFileAllocatedSize ?? values.fileAllocatedSize ?? 0)
        }
        return total
    }
}
