//! Places that typically hold junk. The scanner lists the *children* of each folder,
//! so the folder itself (e.g. ~/Library/Caches) is never removed, only its contents.
use super::Category;

pub struct Target {
    pub id: &'static str,
    pub title: &'static str,
    pub category: Category,
    /// Relative to home.
    pub folders: &'static [&'static str],
    /// Children skipped because another target covers them (or they must stay).
    pub excluded: &'static [&'static str],
    pub note: &'static str,
    pub safe: bool,
}

pub const DEFAULT_TARGETS: &[Target] = &[
    Target {
        id: "user-caches", title: "Cache ứng dụng", category: Category::System,
        folders: &["Library/Caches"],
        excluded: &["Homebrew", "pip", "Yarn", "CocoaPods", "com.apple.Safari", "com.apple.HomeKit",
            "CloudKit", "com.apple.bird", "tg-vitals", "com.trangioi.tgvitals"],
        note: "App tự tạo lại khi cần, lần mở đầu có thể chậm hơn một chút",
        safe: true,
    },
    Target {
        id: "user-logs", title: "Log", category: Category::System,
        folders: &["Library/Logs"], excluded: &[],
        note: "Chỉ cần khi debug sự cố cũ", safe: true,
    },
    Target {
        id: "xcode-derived", title: "Xcode DerivedData", category: Category::Xcode,
        folders: &["Library/Developer/Xcode/DerivedData"], excluded: &[],
        note: "Kết quả build tạm, lần build sau sẽ lâu hơn", safe: true,
    },
    Target {
        id: "xcode-device-support", title: "iOS DeviceSupport", category: Category::Xcode,
        folders: &["Library/Developer/Xcode/iOS DeviceSupport", "Library/Developer/Xcode/watchOS DeviceSupport"],
        excluded: &[],
        note: "Symbol của các bản iOS từng cắm máy, Xcode tải lại khi cần", safe: false,
    },
    Target {
        id: "xcode-archives", title: "Xcode Archives", category: Category::Xcode,
        folders: &["Library/Developer/Xcode/Archives"], excluded: &[],
        note: "Bản build đã archive, cần nếu muốn symbolicate crash của bản cũ", safe: false,
    },
    Target {
        id: "homebrew", title: "Homebrew", category: Category::Devtools,
        folders: &["Library/Caches/Homebrew"], excluded: &[],
        note: "File cài đặt đã tải về", safe: true,
    },
    Target {
        id: "npm", title: "npm / yarn", category: Category::Devtools,
        folders: &[".npm/_cacache", "Library/Caches/Yarn"], excluded: &[],
        note: "Package đã tải, cài lại sẽ tải về", safe: true,
    },
    Target {
        id: "pnpm", title: "pnpm store", category: Category::Devtools,
        folders: &["Library/pnpm/store"], excluded: &[],
        note: "Nên ưu tiên `pnpm store prune`; xóa hết thì lần install sau tải lại", safe: false,
    },
    Target {
        id: "pip", title: "pip", category: Category::Devtools,
        folders: &["Library/Caches/pip"], excluded: &[],
        note: "pip tải lại package khi cần", safe: true,
    },
    Target {
        id: "cocoapods", title: "CocoaPods", category: Category::Devtools,
        folders: &["Library/Caches/CocoaPods"], excluded: &[],
        note: "Pod tải lại khi `pod install`", safe: true,
    },
    Target {
        id: "gradle", title: "Gradle", category: Category::Devtools,
        folders: &[".gradle/caches"], excluded: &[],
        note: "Dependency Gradle đã tải, build sau sẽ tải lại", safe: false,
    },
];

/// Folders (relative to home) searched for project build artifacts (`care::projects`).
pub const PROJECT_ROOTS: &[&str] = &[
    "Documents", "WebstormProjects", "Projects", "projects", "code", "Code", "dev", "Developer", "Development",
    "src", "work", "Workspace", "GitHub", "Repos", "Sites", "www",
];
/// How deep under a project root to look, e.g. Documents/vibe-code/app/app/src-tauri/target.
pub const PROJECT_DEPTH: usize = 6;

/// Folders (relative to home) searched for large files.
pub const LARGE_FILE_ROOTS: &[&str] = &["Downloads", "Desktop", "Movies"];

pub const LARGE_FILE_BYTES: u64 = 500 * 1024 * 1024;
