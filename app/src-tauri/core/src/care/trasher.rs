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

/// Paths of a request that actually went to the Trash (everything not reported as failed).
pub fn trashed_paths(requested: &[String], report: &TrashReport) -> Vec<String> {
    requested.iter().filter(|p| !report.failed.iter().any(|f| &f.path == *p)).cloned().collect()
}

/// App uninstaller: like [`trash`], but items the user does not own (root-owned apps such as
/// Microsoft Office, `/Library/LaunchDaemons/...`) go to the Trash through Finder, which asks for
/// the admin password. `NSWorkspace.recycleURLs` was tried and fails without asking (spike,
/// 2026-11-14). Guarded by [`path_guard::is_allowed_app_path`].
pub fn recycle_with_auth(paths: &[String], allowed: &HashMap<String, u64>, home: &Path) -> TrashReport {
    recycle_with(paths, allowed, home, &super::scanner::owned_by_user, &mut finder_delete)
}

/// [`recycle_with_auth`] with ownership and Finder injected (tests). Items needing the password go
/// first: if Finder fails or the user cancels, **nothing** else is moved, so an app is never left
/// installed with its data already in the Trash.
pub fn recycle_with(
    paths: &[String],
    allowed: &HashMap<String, u64>,
    home: &Path,
    owned: &dyn Fn(&Path) -> bool,
    finder: &mut dyn FnMut(&[String]) -> Result<(), String>,
) -> TrashReport {
    let mut report = TrashReport::default();
    let (mut mine, mut root) = (Vec::new(), Vec::new());
    for raw in paths {
        let fail = |reason: &str| TrashFailure { path: raw.clone(), reason: reason.into() };
        let Some(size) = allowed.get(raw) else {
            report.failed.push(fail("Không có trong kết quả quét gần nhất"));
            continue;
        };
        let path = PathBuf::from(raw);
        if !path_guard::is_allowed_app_path(&path, home) {
            report.failed.push(fail("Đường dẫn được bảo vệ, không cho phép xóa"));
            continue;
        }
        if owned(&path) { mine.push((raw.clone(), *size)) } else { root.push((raw.clone(), *size)) }
    }
    if !root.is_empty() {
        let list: Vec<String> = root.iter().map(|(p, _)| p.clone()).collect();
        let outcome = finder(&list);
        if let Err(_) = &outcome {
            let reason = finder_failure_reason(&outcome);
            for (raw, _) in root.into_iter().chain(mine) {
                report.failed.push(TrashFailure { path: raw, reason: reason.clone() });
            }
            return report;
        }
        for (raw, size) in root {
            if Path::new(&raw).exists() {
                report.failed.push(TrashFailure { path: raw, reason: finder_failure_reason(&outcome) });
            } else {
                report.trashed += 1;
                report.bytes += size;
            }
        }
    }
    for (raw, size) in mine {
        match move_to_trash(Path::new(&raw)) {
            Ok(()) => {
                report.trashed += 1;
                report.bytes += size;
            }
            Err(reason) => report.failed.push(TrashFailure { path: raw, reason }),
        }
    }
    report
}

/// Why an item Finder was asked to trash is still there. `-128` = the user cancelled the prompt.
pub fn finder_failure_reason(outcome: &Result<(), String>) -> String {
    match outcome {
        Err(e) if e.contains("(-128)") => "Đã huỷ".to_string(),
        Err(e) => e.clone(),
        Ok(()) => "Finder không chuyển được vào Thùng rác".to_string(),
    }
}

/// `osascript` arguments that make Finder move `paths` to the Trash. Paths travel as argv items
/// after the script and are never spliced into it, so quotes or spaces cannot change the script.
pub fn finder_delete_args(paths: &[String]) -> Vec<String> {
    let script = [
        "on run argv",
        "set targets to {}",
        "repeat with p in argv",
        "set end of targets to (POSIX file (contents of p))",
        "end repeat",
        "tell application \"Finder\" to delete targets",
        "end run",
    ];
    script.iter().flat_map(|line| ["-e".to_string(), (*line).to_string()]).chain(paths.iter().cloned()).collect()
}

#[cfg(target_os = "macos")]
fn finder_delete(paths: &[String]) -> Result<(), String> {
    let out = std::process::Command::new("/usr/bin/osascript").args(finder_delete_args(paths)).output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn finder_delete(_paths: &[String]) -> Result<(), String> {
    Err("Chỉ hỗ trợ macOS".into())
}

/// Installer receipts (`pkgutil --pkgs`) that belong to `bundle_id`: equal to it or starting with
/// `bundle_id.`, and made only of characters a package id can contain (they reach a shell).
pub fn receipts_to_forget(pkgs_output: &str, bundle_id: &str) -> Vec<String> {
    let id = bundle_id.to_lowercase();
    if id.is_empty() {
        return Vec::new();
    }
    let safe = |p: &str| p.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    pkgs_output
        .lines()
        .map(str::trim)
        .filter(|p| safe(p) && { let l = p.to_lowercase(); l == id || l.starts_with(&format!("{id}.")) })
        .map(str::to_owned)
        .collect()
}

/// Forgets the installer receipts of an uninstalled app (needs admin: one password prompt).
#[cfg(target_os = "macos")]
pub fn forget_receipts(bundle_id: &str) {
    let Ok(out) = std::process::Command::new("/usr/sbin/pkgutil").arg("--pkgs").output() else { return };
    let ids = receipts_to_forget(&String::from_utf8_lossy(&out.stdout), bundle_id);
    if ids.is_empty() {
        return;
    }
    // Ids are validated to [A-Za-z0-9._-] by receipts_to_forget.
    let cmd = ids.iter().map(|id| format!("/usr/sbin/pkgutil --forget {id}")).collect::<Vec<_>>().join("; ");
    let script = format!("do shell script \"{cmd}\" with prompt \"tg-vitals: xoá thông tin cài đặt của app đã gỡ\" with administrator privileges");
    let _ = std::process::Command::new("/usr/bin/osascript").args(["-e", &script]).output();
}

#[cfg(not(target_os = "macos"))]
pub fn forget_receipts(_bundle_id: &str) {}

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

    #[test]
    fn finder_args_keep_paths_verbatim() {
        let paths = ["/Applications/Bob's App.app".to_string(), "/Library/LaunchDaemons/com.x \"y\".plist".to_string()];
        let args = finder_delete_args(&paths);
        // Paths are passed as separate argv items after the script, never spliced into it.
        assert_eq!(&args[args.len() - 2..], &paths);
        assert!(args[..args.len() - 2].iter().all(|a| !a.contains("Bob") && !a.contains("com.x")));
        assert!(args.iter().any(|a| a.contains("on run argv")));
    }

    #[test]
    fn receipts_to_forget_filters_and_validates() {
        let pkgs = "com.apple.pkg.Core\ncom.paloaltonetworks.globalprotect.pkg\ncom.paloaltonetworks.gp.helper\ncom.paloaltonetworks; rm -rf ~\ncom.paloaltonetworksX.pkg\n";
        assert_eq!(receipts_to_forget(pkgs, "com.paloaltonetworks"),
            ["com.paloaltonetworks.globalprotect.pkg", "com.paloaltonetworks.gp.helper"]);
        assert!(receipts_to_forget(pkgs, "").is_empty());
    }

    #[test]
    fn recycle_refuses_unlisted_paths() {
        let home = std::env::temp_dir();
        let report = recycle_with_auth(&["/Applications/Foo.app".into()], &HashMap::new(), &home);
        assert_eq!(report.trashed, 0);
        assert_eq!(report.failed[0].reason, "Không có trong kết quả quét gần nhất");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn recycle_moves_user_owned_fake_app() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let home = std::env::temp_dir().join(format!("tgv-recycle-{n}"));
        let app = home.join("Applications/Fake.app");
        fs::create_dir_all(app.join("Contents")).unwrap();
        fs::write(app.join("Contents/x"), b"fake").unwrap();
        let key = app.to_string_lossy().into_owned();
        let report = recycle_with_auth(&[key.clone()], &HashMap::from([(key, 4)]), &home);
        assert_eq!((report.trashed, report.failed.len()), (1, 0), "{:?}", report.failed);
        assert!(!app.exists());
    }

    #[test]
    fn finder_cancel_is_reported_as_cancelled() {
        let cancelled = "execution error: Finder got an error: User canceled. (-128)";
        assert_eq!(finder_failure_reason(&Err(cancelled.into())), "Đã huỷ");
        assert_eq!(finder_failure_reason(&Err("boom".into())), "boom");
        assert_eq!(finder_failure_reason(&Ok(())), "Finder không chuyển được vào Thùng rác");
    }

    #[test]
    fn cancelled_password_moves_nothing() {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let home = std::env::temp_dir().join(format!("tgv-cancel-{n}"));
        let app = home.join("Applications/Root.app");
        let data = home.join("Library/Application Support/Root");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        let (a, d) = (app.to_string_lossy().into_owned(), data.to_string_lossy().into_owned());
        let allowed = HashMap::from([(a.clone(), 1), (d.clone(), 1)]);
        // The app is "root-owned"; the user cancels Finder's password prompt.
        let report = recycle_with(&[a.clone(), d.clone()], &allowed, &home, &|p| p != app.as_path(),
            &mut |_| Err("Finder got an error: User canceled. (-128)".into()));
        assert_eq!(report.trashed, 0);
        assert!(app.exists() && data.exists(), "nothing may move when the prompt is cancelled");
        assert!(report.failed.iter().all(|f| f.reason == "Đã huỷ"), "{:?}", report.failed);
    }

    #[test]
    fn trashed_paths_are_requested_minus_failed() {
        let report = TrashReport { trashed: 1, bytes: 5, failed: vec![TrashFailure { path: "/b".into(), reason: "x".into() }] };
        assert_eq!(trashed_paths(&["/a".into(), "/b".into()], &report), ["/a"]);
    }
}
