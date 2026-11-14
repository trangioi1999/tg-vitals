//! Read-only: never modifies the disk. Deleting is `trasher`'s job.
use super::protected_cache::is_protected_cache_child;
use super::targets::{Target, DEFAULT_TARGETS, LARGE_FILE_BYTES, LARGE_FILE_ROOTS, PROJECT_DEPTH, PROJECT_ROOTS};
use super::{Category, CleanupGroup, CleanupItem, ScanEvent};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Scanner {
    pub home: PathBuf,
    pub now: SystemTime,
}

impl Scanner {
    pub fn new(home: PathBuf) -> Self {
        Self { home, now: SystemTime::now() }
    }

    /// Every group with at least one non-empty item, biggest first.
    pub fn scan_all(&self) -> Vec<CleanupGroup> {
        self.scan_all_with(&mut |_| {})
    }

    /// Same as `scan_all`, reporting each step and folder to `on` as it goes.
    pub fn scan_all_with(&self, on: &mut dyn FnMut(ScanEvent)) -> Vec<CleanupGroup> {
        let total = DEFAULT_TARGETS.len() + 2;
        let mut index = 0;
        let mut groups = Vec::new();
        type Scan<'a> = &'a mut dyn FnMut(&mut dyn FnMut(ScanEvent)) -> Option<CleanupGroup>;
        let mut run = |title: &str, folders: Vec<PathBuf>, on: &mut dyn FnMut(ScanEvent), scan: Scan| {
            on(ScanEvent::Step { index, total, title: title.into() });
            index += 1;
            for folder in folders.iter().filter(|f| f.is_dir()) {
                on(ScanEvent::Path { path: folder.to_string_lossy().into_owned() });
            }
            let group = scan(on);
            let (bytes, items) = group.as_ref().map_or((0, 0), |g| (g.total(), g.items.len()));
            on(ScanEvent::StepDone { title: title.into(), bytes, items });
            groups.extend(group);
        };

        for target in DEFAULT_TARGETS {
            let folders = target.folders.iter().map(|f| self.home.join(f)).collect();
            run(target.title, folders, on, &mut |_| self.scan_target(target));
        }

        let roots: Vec<PathBuf> = PROJECT_ROOTS.iter().map(|r| self.home.join(r)).collect();
        run("File build trong project", roots.clone(), on, &mut |on| {
            let artifacts = super::projects::find_artifacts_with(&roots, PROJECT_DEPTH, &mut |i| {
                on(ScanEvent::Found { path: i.path.clone(), bytes: i.size })
            });
            (!artifacts.is_empty()).then(|| CleanupGroup {
                id: "project-artifacts".into(),
                title: "File build trong project".into(),
                category: Category::Projects,
                note: "target, node_modules, .next, .venv… tạo lại khi build / cài lại; không đụng file Git đang theo dõi".into(),
                safe: false,
                items: artifacts,
            })
        });

        let roots: Vec<PathBuf> = LARGE_FILE_ROOTS.iter().map(|r| self.home.join(r)).collect();
        run("File lớn", roots.clone(), on, &mut |_| {
            let large = large_files(&roots, LARGE_FILE_BYTES, 3);
            (!large.is_empty()).then(|| CleanupGroup {
                id: "large-files".into(),
                title: "File lớn".into(),
                category: Category::Files,
                note: "File trên 500 MB trong Downloads, Desktop, Movies — xem kỹ trước khi dọn".into(),
                safe: false,
                items: large,
            })
        });

        groups.sort_by_key(|g| std::cmp::Reverse(g.total()));
        groups
    }

    pub fn scan_target(&self, target: &Target) -> Option<CleanupGroup> {
        let mut items: Vec<CleanupItem> = target
            .folders
            .iter()
            .flat_map(|folder| children(&self.home.join(folder), target.excluded))
            .filter(|i| i.size > 0)
            .collect();
        if items.is_empty() {
            return None;
        }
        items.sort_by_key(|i| std::cmp::Reverse(i.size));
        Some(CleanupGroup {
            id: target.id.into(),
            title: target.title.into(),
            category: target.category,
            note: target.note.into(),
            safe: target.safe,
            items,
        })
    }
}

fn large_files(roots: &[PathBuf], min_bytes: u64, max_depth: usize) -> Vec<CleanupItem> {
    fn walk(dir: &Path, depth: usize, max_depth: usize, min: u64, out: &mut Vec<CleanupItem>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let path = entry.path();
            let is_bundle = path.extension().is_some_and(|e| e == "app" || e == "photoslibrary" || e == "fcpbundle");
            if meta.file_type().is_symlink() || is_bundle {
                continue;
            }
            if meta.is_file() {
                let size = allocated(&meta);
                if size >= min {
                    out.push(item(&path, size, meta.modified().ok()));
                }
            } else if meta.is_dir() && depth < max_depth {
                walk(&path, depth + 1, max_depth, min, out);
            }
        }
    }
    let mut out = Vec::new();
    for root in roots {
        walk(root, 0, max_depth, min_bytes, &mut out);
    }
    out.sort_by_key(|i| std::cmp::Reverse(i.size));
    out
}

fn children(folder: &Path, excluded: &[&str]) -> Vec<CleanupItem> {
    let Ok(entries) = fs::read_dir(folder) else { return Vec::new() };
    entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name != ".DS_Store"
                && !excluded.contains(&name.as_ref())
                && !is_protected_cache_child(&name)
        })
        .map(|e| {
            let path = e.path();
            let modified = fs::symlink_metadata(&path).and_then(|m| m.modified()).ok();
            item(&path, dir_size(&path), modified)
        })
        .collect()
}

fn item(path: &Path, size: u64, modified: Option<SystemTime>) -> CleanupItem {
    CleanupItem {
        path: path.to_string_lossy().into_owned(),
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        size,
        modified: modified.and_then(|m| m.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()),
    }
}

/// On-disk (allocated) size, like Finder. Never follows symlinks.
pub fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else { return 0 };
    if !meta.is_dir() {
        return allocated(&meta);
    }
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += allocated(&meta);
            }
        }
    }
    total
}

#[cfg(unix)]
fn allocated(meta: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn allocated(meta: &fs::Metadata) -> u64 {
    meta.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home() -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("tgv-scan-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reports_every_step_in_order() {
        let home = home();
        write(&home.join("Library/Caches/com.app/a.bin"), 10_000);
        let mut events = Vec::new();
        let groups = Scanner::new(home.clone()).scan_all_with(&mut |e| events.push(e));

        let total = DEFAULT_TARGETS.len() + 2;
        let steps: Vec<_> = events.iter().filter_map(|e| match e {
            ScanEvent::Step { index, total: t, .. } => Some((*index, *t)),
            _ => None,
        }).collect();
        assert_eq!(steps, (0..total).map(|i| (i, total)).collect::<Vec<_>>());
        assert_eq!(events.iter().filter(|e| matches!(e, ScanEvent::StepDone { .. })).count(), total);
        assert!(!events.iter().any(|e| matches!(e, ScanEvent::Found { .. })), "no projects in this home");
        // The caches folder exists, so it is announced, then its step reports what was found.
        let caches = home.join("Library/Caches").to_string_lossy().into_owned();
        assert!(events.contains(&ScanEvent::Path { path: caches }));
        let found: u64 = events.iter().map(|e| match e { ScanEvent::StepDone { bytes, .. } => *bytes, _ => 0 }).sum();
        assert_eq!(found, groups.iter().map(|g| g.total()).sum::<u64>());
        assert!(found > 0);
        // Missing folders are not announced.
        assert!(!events.iter().any(|e| matches!(e, ScanEvent::Path { path } if path.contains("DerivedData"))));
    }

    #[test]
    fn scan_event_json_shape() {
        let e = ScanEvent::StepDone { title: "Log".into(), bytes: 5, items: 1 };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"kind":"stepDone","title":"Log","bytes":5,"items":1}"#);
    }

    fn write(path: &Path, bytes: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![7u8; bytes]).unwrap();
    }

    #[test]
    fn lists_children_not_the_folder_and_skips_excluded() {
        let home = home();
        write(&home.join("Library/Caches/com.app/a.bin"), 10_000);
        write(&home.join("Library/Caches/Homebrew/b.tar"), 10_000);
        write(&home.join("Library/Caches/.DS_Store"), 10);
        let scanner = Scanner::new(home.clone());
        let group = scanner.scan_target(&DEFAULT_TARGETS[0]).unwrap();
        assert_eq!(group.id, "user-caches");
        let names: Vec<_> = group.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["com.app"]);
        assert!(group.items[0].size >= 10_000);
        assert!(group.items[0].path.ends_with("Library/Caches/com.app"));
    }

    #[test]
    fn missing_folders_give_no_group() {
        let scanner = Scanner::new(home());
        assert!(scanner.scan_target(&DEFAULT_TARGETS[2]).is_none());
    }

    #[test]
    fn skips_protected_cache_children_for_every_target() {
        let home = home();
        let scanner = Scanner::new(home.clone());
        for target in DEFAULT_TARGETS {
            for folder in target.folders {
                write(&home.join(folder).join("ms-playwright-mcp/chromium-1/x"), 10_000);
                write(&home.join(folder).join("HuggingFace-hub/model"), 10_000);
                write(&home.join(folder).join("com.app/a.bin"), 10_000);
            }
            let group = scanner.scan_target(target).unwrap();
            assert_eq!(group.items.len(), target.folders.len(), "{}", target.id);
            assert!(group.items.iter().all(|item| item.name == "com.app"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn size_does_not_follow_symlinks() {
        let home = home();
        write(&home.join("big/file.bin"), 200_000);
        fs::create_dir_all(home.join("cache")).unwrap();
        std::os::unix::fs::symlink(home.join("big"), home.join("cache/link")).unwrap();
        assert!(dir_size(&home.join("cache")) < 100_000);
        assert!(dir_size(&home.join("big")) >= 200_000);
    }

    #[test]
    fn large_files_threshold() {
        let home = home();
        write(&home.join("Downloads/small.zip"), 1_000);
        write(&home.join("Downloads/sub/big.iso"), 300_000);
        let found = large_files(&[home.join("Downloads")], 200_000, 3);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "big.iso");
    }
}
