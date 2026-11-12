import Foundation

/// The single gatekeeper deciding whether a path may ever be moved to the Trash.
///
/// Rules:
/// 1. Must be strictly inside the user's home folder (after resolving symlinks).
///    This alone rules out /System, /Library, /usr, /bin, /Applications, etc.
/// 2. Must not be one of the protected folders themselves, or inside a protected subtree.
public enum PathGuard {
    /// Folders that may contain removable children but must never be removed themselves.
    static let protectedRoots: Set<String> = [
        "Library", "Library/Caches", "Library/Logs", "Library/Application Support",
        "Library/Preferences", "Library/Developer", "Library/Developer/Xcode",
        "Desktop", "Documents", "Downloads", "Movies", "Music", "Pictures", "Public",
        "Applications",
    ]

    /// Subtrees that are never touched at all.
    static let forbiddenSubtrees: [String] = [
        ".ssh", ".gnupg", ".aws", ".config/gh",
        "Library/Keychains", "Library/Mobile Documents", "Library/Mail",
        "Library/Messages", "Library/Photos", "Library/CloudStorage",
    ]

    public static func isAllowed(_ url: URL, home: URL) -> Bool {
        let homePath = canonical(home)
        let path = canonical(url)

        guard path.hasPrefix(homePath + "/") else { return false }
        let relative = String(path.dropFirst(homePath.count + 1))
        guard !relative.isEmpty, !protectedRoots.contains(relative) else { return false }

        for subtree in forbiddenSubtrees where relative == subtree || relative.hasPrefix(subtree + "/") {
            return false
        }
        return true
    }

    private static func canonical(_ url: URL) -> String {
        var path = url.standardizedFileURL.resolvingSymlinksInPath().path
        while path.count > 1, path.hasSuffix("/") { path.removeLast() }
        return path
    }
}
