import Foundation

/// The ONLY code in tg-vitals allowed to delete anything — and it only moves to the Trash.
public struct Trasher {
    public struct Report: Sendable, Equatable {
        public var trashed: [URL] = []
        public var trashedBytes: UInt64 = 0
        /// Refused by `PathGuard`.
        public var rejected: [URL] = []
        /// Allowed, but macOS refused (permissions, file in use…).
        public var failed: [URL: String] = [:]
    }

    private let home: URL
    private let fileManager: FileManager

    public init(home: URL = FileManager.default.homeDirectoryForCurrentUser,
                fileManager: FileManager = .default) {
        self.home = home
        self.fileManager = fileManager
    }

    /// - Parameter items: URLs with their pre-computed size (for the report).
    public func trash(_ items: [(url: URL, size: UInt64)]) -> Report {
        var report = Report()
        for item in items {
            guard PathGuard.isAllowed(item.url, home: home) else {
                report.rejected.append(item.url)
                continue
            }
            do {
                try fileManager.trashItem(at: item.url, resultingItemURL: nil)
                report.trashed.append(item.url)
                report.trashedBytes += item.size
            } catch {
                report.failed[item.url] = error.localizedDescription
            }
        }
        return report
    }
}
