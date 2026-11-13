//! The single gatekeeper deciding whether a path may ever be moved to the Trash.
//!
//! 1. Must be strictly inside the user's home folder after resolving symlinks.
//!    This alone rules out /System, /Library, /usr, /bin, /sbin, /Applications…
//! 2. Must not be a protected folder itself, nor inside a forbidden subtree.
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
    let (Some(home), Some(path)) = (canonical(home), canonical(path)) else {
        return false;
    };
    let Ok(relative) = path.strip_prefix(&home) else {
        return false;
    };
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
}
