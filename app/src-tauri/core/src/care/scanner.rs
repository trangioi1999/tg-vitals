//! Read-only: never modifies the disk. Deleting is `trasher`'s job.
use super::targets::{Target, DEFAULT_TARGETS, LARGE_FILE_BYTES, LARGE_FILE_ROOTS, PROJECT_ROOTS, STALE_DAYS};
use super::{Category, CleanupGroup, CleanupItem};
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
        let mut groups: Vec<CleanupGroup> = DEFAULT_TARGETS.iter().filter_map(|t| self.scan_target(t)).collect();

        let roots: Vec<PathBuf> = PROJECT_ROOTS.iter().map(|r| self.home.join(r)).collect();
        let stale = self.stale_node_modules(&roots, STALE_DAYS, 4);
        if !stale.is_empty() {
            groups.push(CleanupGroup {
                id: "node-modules".into(),
                title: "node_modules bỏ lâu".into(),
                category: Category::Projects,
                note: format!("Project không sửa hơn {STALE_DAYS} ngày, chạy lại npm install khi cần"),
                safe: false,
                items: stale,
            });
        }

        let roots: Vec<PathBuf> = LARGE_FILE_ROOTS.iter().map(|r| self.home.join(r)).collect();
        let large = large_files(&roots, LARGE_FILE_BYTES, 3);
        if !large.is_empty() {
            groups.push(CleanupGroup {
                id: "large-files".into(),
                title: "File lớn".into(),
                category: Category::Files,
                note: "File trên 500 MB trong Downloads, Desktop, Movies — xem kỹ trước khi dọn".into(),
                safe: false,
                items: large,
            });
        }

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

    /// `node_modules` folders under `roots` whose project folder hasn't changed for `stale_days`.
    pub fn stale_node_modules(&self, roots: &[PathBuf], stale_days: u64, max_depth: usize) -> Vec<CleanupItem> {
        let cutoff = self.now - std::time::Duration::from_secs(stale_days * 86_400);
        let mut found = Vec::new();
        for root in roots {
            walk_projects(root, 0, max_depth, cutoff, &mut found);
        }
        found.sort_by_key(|i| std::cmp::Reverse(i.size));
        found
    }
}

fn walk_projects(dir: &Path, depth: usize, max_depth: usize, cutoff: SystemTime, found: &mut Vec<CleanupItem>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        // file_type() does not follow symlinks.
        let Ok(kind) = entry.file_type() else { continue };
        if !kind.is_dir() {
            continue;
        }
        let path = entry.path();
        if name == "node_modules" {
            if let Some(project_modified) = project_modified(dir) {
                if project_modified < cutoff {
                    found.push(item(&path, dir_size(&path), Some(project_modified)));
                }
            }
            // Never descend into node_modules.
        } else {
            walk_projects(&path, depth + 1, max_depth, cutoff, found);
        }
    }
}

/// Newest modification among the project folder and its direct children (except node_modules),
/// so editing `src/` or `package.json` counts as activity.
fn project_modified(dir: &Path) -> Option<SystemTime> {
    let mut newest = fs::symlink_metadata(dir).and_then(|m| m.modified()).ok()?;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name() == "node_modules" {
                continue;
            }
            if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                newest = newest.max(m);
            }
        }
    }
    Some(newest)
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
            name != ".DS_Store" && !excluded.contains(&name.as_ref())
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
    use std::time::Duration;

    fn home() -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!("tgv-scan-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
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
    fn finds_only_stale_node_modules_and_never_descends() {
        let home = home();
        let projects = home.join("WebstormProjects");
        write(&projects.join("old/node_modules/left-pad/index.js"), 5_000);
        write(&projects.join("old/node_modules/x/node_modules/y/i.js"), 5_000);
        write(&projects.join("fresh/node_modules/react/index.js"), 5_000);
        // Pretend "now" is 60 days ahead: everything is stale except what we touch later.
        let mut scanner = Scanner::new(home.clone());
        scanner.now = SystemTime::now() + Duration::from_secs(60 * 86_400);
        let found = scanner.stale_node_modules(std::slice::from_ref(&projects), 30, 4);
        assert_eq!(found.len(), 2, "one per project, nested node_modules not listed separately");
        assert!(found.iter().all(|i| i.name == "node_modules"));

        // Real "now": nothing is 30 days old.
        let scanner = Scanner::new(home);
        assert!(scanner.stale_node_modules(&[projects], 30, 4).is_empty());
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
