//! The single gatekeeper deciding whether a path may ever be moved to the Trash.
//!
//! 1. Must be strictly inside the user's home folder after resolving symlinks.
//!    This alone rules out /System, /Library, /usr, /bin, /sbin, /Applications…
//! 2. Must not be a protected folder itself, nor inside a forbidden subtree.
use super::protected_cache::is_protected_cache_child;
use std::path::{Path, PathBuf};

/// Folders that may hold removable children but must never be removed themselves.
pub const PROTECTED_ROOTS: &[&str] = &[
    "Library", "Library/Caches", "Library/Logs", "Library/Application Support",
    "Library/Preferences", "Library/Developer", "Library/Developer/Xcode",
    "Library/Developer/Xcode/DerivedData", "Library/Developer/Xcode/Archives",
    "Library/Developer/Xcode/iOS DeviceSupport", "Library/pnpm", "Library/pnpm/store",
    ".npm", ".npm/_cacache",
    "Desktop", "Documents", "Downloads", "Movies", "Music", "Pictures", "Public",
    "Applications",
];

/// Subtrees that are never touched at all.
pub const FORBIDDEN_SUBTREES: &[&str] = &[
    ".ssh", ".gnupg", ".aws", ".kube", ".docker", ".config", ".Trash",
    "Library/Keychains", "Library/Mobile Documents", "Library/Mail", "Library/Messages",
    "Library/Photos", "Library/CloudStorage", "Library/Containers", "Library/Group Containers",
    "Library/Preferences", "Library/Application Support/MobileSync",
];

/// Absolute system locations. Rule 1 already excludes them; listed so tests document intent.
pub const SYSTEM_PATHS: &[&str] = &["/System", "/Library", "/usr", "/bin", "/sbin", "/Applications", "/private", "/var", "/etc"];

pub fn is_allowed(path: &Path, home: &Path) -> bool {
    // Protect the named store even when it is a symlink to another home folder.
    if path.strip_prefix(home).is_ok_and(has_protected_cache_child) {
        return false;
    }
    let (Some(home), Some(path)) = (canonical(home), canonical(path)) else {
        return false;
    };
    let Ok(relative) = path.strip_prefix(&home) else {
        return false;
    };
    if has_protected_cache_child(relative) {
        return false;
    }
    let relative = relative.to_string_lossy();
    if relative.is_empty() || PROTECTED_ROOTS.contains(&relative.as_ref()) {
        return false;
    }
    if SYSTEM_PATHS.iter().any(|s| path.starts_with(s) && !home.starts_with(s)) {
        return false;
    }
    !FORBIDDEN_SUBTREES
        .iter()
        .any(|sub| relative == *sub || relative.starts_with(&format!("{sub}/")))
}

/// Folders whose direct children the app uninstaller may move to the Trash
/// (system-level leftovers matched by bundle id; Finder asks for the admin password).
pub const SYSTEM_LEFTOVER_ROOTS: &[&str] = &[
    "/Library/LaunchAgents", "/Library/LaunchDaemons", "/Library/PrivilegedHelperTools",
    "/Library/Application Support", "/Library/Caches", "/Library/Logs", "/Library/Preferences",
];

/// Shape check for the app uninstaller, no disk access: an `.app` directly in `/Applications` or
/// `~/Applications`, or one entry directly inside a [`SYSTEM_LEFTOVER_ROOTS`] folder.
pub fn is_app_path_shape(path: &Path, home: &Path) -> bool {
    use std::path::Component;
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return false;
    }
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else { return false };
    let is_app = Path::new(name).extension().is_some_and(|e| e == "app");
    (is_app && (parent == Path::new("/Applications") || parent == home.join("Applications")))
        || SYSTEM_LEFTOVER_ROOTS.iter().any(|root| parent == Path::new(root))
}

/// The app uninstaller's guard: [`is_app_path_shape`] on an existing, non-symlink path whose
/// resolved location is itself; anything else falls back to [`is_allowed`].
pub fn is_allowed_app_path(path: &Path, home: &Path) -> bool {
    if !is_app_path_shape(path, home) {
        return is_allowed(path, home);
    }
    let Ok(meta) = std::fs::symlink_metadata(path) else { return false };
    if meta.file_type().is_symlink() {
        return false;
    }
    // Resolve the parent too (e.g. a temp home behind /var → /private/var).
    match (canonical(path), path.parent().and_then(canonical), path.file_name()) {
        (Some(real), Some(parent), Some(name)) => real == parent.join(name),
        _ => false,
    }
}

/// Only direct children of the home cache roots identify protected subtrees.
fn has_protected_cache_child(relative: &Path) -> bool {
    let mut components = relative.components();
    let Some(first) = components.next() else { return false };
    let first = first.as_os_str().to_string_lossy();
    if first.eq_ignore_ascii_case("Library") {
        let Some(second) = components.next() else { return false };
        if !second.as_os_str().to_string_lossy().eq_ignore_ascii_case("Caches") {
            return false;
        }
    } else if !first.eq_ignore_ascii_case(".cache") {
        return false;
    }
    components.next().is_some_and(|child| {
        is_protected_cache_child(&child.as_os_str().to_string_lossy())
    })
}

/// Resolves symlinks and `..`. Missing paths are never allowed.
fn canonical(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    std::fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn sandbox() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tgv-guard-{}-{}", std::process::id(), rand()));
        fs::create_dir_all(dir.join("Library/Caches/com.example")).unwrap();
        fs::create_dir_all(dir.join("Library/Keychains")).unwrap();
        fs::create_dir_all(dir.join(".ssh")).unwrap();
        fs::create_dir_all(dir.join("Documents/notes")).unwrap();
        fs::create_dir_all(dir.join("Downloads")).unwrap();
        fs::write(dir.join("Downloads/big.dmg"), b"x").unwrap();
        fs::write(dir.join(".ssh/id_ed25519"), b"x").unwrap();
        fs::canonicalize(dir).unwrap()
    }

    fn rand() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    }

    #[test]
    fn identifies_only_direct_protected_cache_children() {
        for root in ["Library/Caches", ".cache", "library/CACHES"] {
            for child in ["ms-playwright-mcp", "CYPRESS", "HuggingFace-hub"] {
                assert!(has_protected_cache_child(Path::new(&format!("{root}/{child}"))));
                assert!(has_protected_cache_child(Path::new(&format!("{root}/{child}/nested/x"))));
            }
        }
        for path in ["Library/Caches/com.app", "Library/Caches/com.app/ollama",
            "Documents/ms-playwright", "other/.cache/ollama", ".cache", "Library/Caches"] {
            assert!(!has_protected_cache_child(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn refuses_runtime_and_model_stores_and_their_contents() {
        let home = sandbox();
        for root in ["Library/Caches", ".cache"] {
            for child in ["ms-playwright-mcp", "MS-PLAYWRIGHT-go", "Buzz-models", "ollama"] {
                let store = home.join(root).join(child);
                fs::create_dir_all(store.join("chromium-1")).unwrap();
                let file = store.join("chromium-1/x");
                fs::write(&file, b"runtime or model").unwrap();
                assert!(!is_allowed(&store, &home));
                assert!(!is_allowed(&file, &home));
            }
            let ordinary = home.join(root).join("com.app");
            fs::create_dir_all(&ordinary).unwrap();
            assert!(is_allowed(&ordinary, &home));
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_aliases_of_protected_cache_stores() {
        let home = sandbox();
        let store = home.join("Library/Caches/ms-playwright-mcp");
        fs::create_dir_all(&store).unwrap();
        let alias = home.join("Downloads/browser");
        std::os::unix::fs::symlink(&store, &alias).unwrap();
        assert!(!is_allowed(&alias, &home));
        let named_store = home.join(".cache/ollama");
        fs::create_dir_all(named_store.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(home.join("Documents/notes"), &named_store).unwrap();
        assert!(!is_allowed(&named_store, &home));
    }

    #[test]
    fn allows_children_of_cache_folders() {
        let home = sandbox();
        assert!(is_allowed(&home.join("Library/Caches/com.example"), &home));
        assert!(is_allowed(&home.join("Downloads/big.dmg"), &home));
        assert!(is_allowed(&home.join("Documents/notes"), &home));
    }

    #[test]
    fn refuses_protected_roots_and_home_itself() {
        let home = sandbox();
        for p in ["", "Library", "Library/Caches", "Documents", "Downloads"] {
            assert!(!is_allowed(&home.join(p), &home), "{p} must be refused");
        }
    }

    #[test]
    fn refuses_forbidden_subtrees() {
        let home = sandbox();
        assert!(!is_allowed(&home.join(".ssh"), &home));
        assert!(!is_allowed(&home.join(".ssh/id_ed25519"), &home));
        assert!(!is_allowed(&home.join("Library/Keychains"), &home));
    }

    #[test]
    fn refuses_system_paths_and_outside_home() {
        let home = sandbox();
        for p in ["/System", "/Library", "/usr", "/bin", "/sbin", "/Applications", "/", "/tmp"] {
            assert!(!is_allowed(Path::new(p), &home), "{p} must be refused");
        }
    }

    #[test]
    fn refuses_relative_missing_and_dotdot_escapes() {
        let home = sandbox();
        assert!(!is_allowed(Path::new("Library/Caches/com.example"), &home));
        assert!(!is_allowed(&home.join("Library/Caches/missing"), &home));
        assert!(!is_allowed(&home.join("Library/Caches/../.."), &home));
        assert!(!is_allowed(&home.join("Library/Caches/com.example/../../Keychains"), &home));
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinks_pointing_outside_or_at_protected() {
        let home = sandbox();
        let outside = std::env::temp_dir().join(format!("tgv-outside-{}", rand()));
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, home.join("Library/Caches/escape")).unwrap();
        std::os::unix::fs::symlink(home.join(".ssh"), home.join("Library/Caches/keys")).unwrap();
        assert!(!is_allowed(&home.join("Library/Caches/escape"), &home));
        assert!(!is_allowed(&home.join("Library/Caches/keys"), &home));
    }

    #[test]
    fn app_paths_allowed_one_level_only() {
        let home = Path::new("/Users/me");
        for ok in ["/Applications/Foo.app", "/Applications/Microsoft Word.app", "/Users/me/Applications/Bar.app",
            "/Library/LaunchDaemons/com.foo.plist", "/Library/Application Support/Foo", "/Library/PrivilegedHelperTools/com.foo.helper"] {
            assert!(is_app_path_shape(Path::new(ok), home), "{ok}");
        }
        for no in ["/Applications/Foo.app/Contents", "/Applications", "/Applications/notes.txt", "/System/Applications/Mail.app",
            "/Library/LaunchDaemons", "/Library/LaunchDaemons/a/b.plist", "/Library/Keychains/x", "/Users/other/Applications/X.app",
            "/Applications/../Library/Foo.app"] {
            assert!(!is_app_path_shape(Path::new(no), home), "{no}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn app_path_must_not_be_a_symlink() {
        let home = sandbox();
        let real = home.join("Documents/Real.app");
        fs::create_dir_all(&real).unwrap();
        fs::create_dir_all(home.join("Applications")).unwrap();
        std::os::unix::fs::symlink(&real, home.join("Applications/Link.app")).unwrap();
        fs::create_dir_all(home.join("Applications/Mine.app")).unwrap();
        assert!(!is_allowed_app_path(&home.join("Applications/Link.app"), &home));
        assert!(is_allowed_app_path(&home.join("Applications/Mine.app"), &home));
        assert!(!is_allowed_app_path(&home.join("Applications/Missing.app"), &home));
    }
}
