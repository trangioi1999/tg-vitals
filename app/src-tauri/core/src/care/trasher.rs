//! The ONLY code in tg-vitals allowed to delete anything — and it only moves to the Trash
//! (`NSFileManager.trashItemAtURL`). Never `remove_file` / `remove_dir_all` / `rm -rf`.
use super::path_guard;
use super::{TrashFailure, TrashReport};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// `allowed` maps every path the user may trash (the last scan result) to its size.
/// Anything not in that list, or refused by [`path_guard`], is reported as failed.
pub fn trash(paths: &[String], allowed: &HashMap<String, u64>, home: &Path) -> TrashReport {
    let mut report = TrashReport::default();
    for raw in paths {
        let fail = |reason: &str| TrashFailure { path: raw.clone(), reason: reason.into() };
        let Some(size) = allowed.get(raw) else {
            report.failed.push(fail("Không có trong kết quả quét gần nhất"));
            continue;
        };
        let path = PathBuf::from(raw);
        if !path_guard::is_allowed(&path, home) {
            report.failed.push(fail("Đường dẫn được bảo vệ, không cho phép xóa"));
            continue;
        }
        match move_to_trash(&path) {
            Ok(()) => {
                report.trashed += 1;
                report.bytes += size;
            }
            Err(reason) => report.failed.push(fail(&reason)),
        }
    }
    report
}

#[cfg(target_os = "macos")]
fn move_to_trash(path: &Path) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|e| e.localizedDescription().to_string())
}

#[cfg(not(target_os = "macos"))]
fn move_to_trash(_path: &Path) -> Result<(), String> {
    Err("Chỉ hỗ trợ macOS".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn refuses_paths_not_in_scan_and_protected_paths() {
        let home = std::env::temp_dir();
        let mut allowed = HashMap::new();
        allowed.insert("/System".to_string(), 1);
        let report = trash(&["/System".into(), "/usr/bin".into()], &allowed, &home);
        assert_eq!(report.trashed, 0);
        assert_eq!(report.failed.len(), 2);
        assert!(report.failed[1].reason.contains("quét"));
        assert!(fs::metadata("/usr/bin").is_ok());
    }

    /// Moves a real file to the Trash. Ignored by default; CI runs it on macOS.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn really_moves_a_file_to_the_trash() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let dir = home.join(format!("tg-vitals-trash-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("junk.txt");
        fs::write(&file, b"junk").unwrap();
        let key = file.to_string_lossy().into_owned();
        let allowed = HashMap::from([(key.clone(), 4)]);
        let report = trash(&[key.clone(), dir.to_string_lossy().into_owned()], &allowed, &home);
        assert_eq!(report.trashed, 1, "{report:?}");
        assert!(!file.exists());
        let report = trash(&[dir.to_string_lossy().into_owned()], &HashMap::from([(dir.to_string_lossy().into_owned(), 0)]), &home);
        assert_eq!(report.trashed, 1, "{report:?}");
    }
}
