import Foundation

/// A place that typically holds junk. The scanner lists the *children* of each path,
/// so the folder itself (e.g. ~/Library/Caches) is never removed, only its contents.
public struct CleanupTarget: Sendable, Identifiable, Hashable {
    public enum Category: String, Sendable, CaseIterable {
        case system = "Hệ thống"
        case xcode = "Xcode"
        case devTools = "Công cụ dev"
        case projects = "Project"
    }

    public let id: String
    public let title: String
    public let category: Category
    /// Paths relative to the home folder, e.g. "Library/Caches".
    public let relativePaths: [String]
    /// Children skipped because another target already covers them.
    public let excludedChildren: Set<String>
    /// Shown under the title so the user knows what deleting means.
    public let note: String
    /// Pre-ticked in the UI. Only for things that are always safe to regenerate.
    public let selectedByDefault: Bool

    public init(
        id: String, title: String, category: Category, relativePaths: [String],
        excludedChildren: Set<String> = [], note: String, selectedByDefault: Bool
    ) {
        self.id = id
        self.title = title
        self.category = category
        self.relativePaths = relativePaths
        self.excludedChildren = excludedChildren
        self.note = note
        self.selectedByDefault = selectedByDefault
    }
}

public extension CleanupTarget {
    static let defaults: [CleanupTarget] = [
        CleanupTarget(
            id: "user-caches", title: "Cache ứng dụng", category: .system,
            relativePaths: ["Library/Caches"],
            excludedChildren: ["Homebrew", "pip", "Yarn", "CocoaPods", "com.apple.Safari"],
            note: "App sẽ tự tạo lại khi cần. Có thể chậm hơn một chút ở lần mở đầu.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "user-logs", title: "Log", category: .system,
            relativePaths: ["Library/Logs"],
            note: "Chỉ dùng khi cần debug sự cố cũ.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "xcode-derived", title: "Xcode DerivedData", category: .xcode,
            relativePaths: ["Library/Developer/Xcode/DerivedData"],
            note: "Kết quả build tạm. Lần build sau sẽ lâu hơn.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "xcode-device-support", title: "iOS DeviceSupport", category: .xcode,
            relativePaths: ["Library/Developer/Xcode/iOS DeviceSupport"],
            note: "Symbol của các bản iOS từng cắm máy. Xcode tải lại khi cần.",
            selectedByDefault: false
        ),
        CleanupTarget(
            id: "xcode-archives", title: "Xcode Archives", category: .xcode,
            relativePaths: ["Library/Developer/Xcode/Archives"],
            note: "Bản build đã archive — cần nếu muốn symbolicate crash của bản cũ.",
            selectedByDefault: false
        ),
        CleanupTarget(
            id: "homebrew", title: "Homebrew cache", category: .devTools,
            relativePaths: ["Library/Caches/Homebrew"],
            note: "File cài đặt đã tải về.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "npm", title: "npm cache", category: .devTools,
            relativePaths: [".npm/_cacache"],
            note: "npm tải lại package khi cần.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "pnpm", title: "pnpm store", category: .devTools,
            relativePaths: ["Library/pnpm/store"],
            note: "Nên dùng `pnpm store prune` nếu có thể; xóa hết thì lần install sau tải lại.",
            selectedByDefault: false
        ),
        CleanupTarget(
            id: "yarn", title: "Yarn cache", category: .devTools,
            relativePaths: ["Library/Caches/Yarn"],
            note: "Yarn tải lại package khi cần.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "pip", title: "pip cache", category: .devTools,
            relativePaths: ["Library/Caches/pip"],
            note: "pip tải lại package khi cần.",
            selectedByDefault: true
        ),
        CleanupTarget(
            id: "cocoapods", title: "CocoaPods cache", category: .devTools,
            relativePaths: ["Library/Caches/CocoaPods"],
            note: "Pod tải lại khi `pod install`.",
            selectedByDefault: true
        ),
    ]
}
